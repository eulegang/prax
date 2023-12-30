use http_body_util::{BodyExt, Full};
use hyper::{
    body::{Bytes, Incoming},
    client::conn::http1::Builder,
    Request, Response, Uri,
};
use hyper_util::rt::TokioIo;
use tokio::{io::AsyncWriteExt, net::TcpStream};

use crate::{
    comm::{report_req, report_res},
    hist::{self},
    proxy::service::{apply_request, apply_response},
    HIST, PROXY,
};

pub async fn service(req: Request<Incoming>) -> eyre::Result<Response<Full<Bytes>>> {
    let Some(host) = req.uri().host() else {
        let body = "Bad Request".as_bytes().into();

        let builder = Response::builder().status(400).body(body).unwrap();
        return Ok(builder);
    };

    let host = host.to_string();
    let port = req.uri().port_u16().unwrap_or(80);
    let lookup = format!("{host}:{port}");

    let (parts, body) = req.into_parts();
    let buf = collect(body).await?;
    let mut req = Request::from_parts(parts, buf);

    let proxy = PROXY.read().await;

    let target = proxy.find_target(&lookup);
    let log = target.is_some() || !proxy.focus;

    let mut id = None::<usize>;

    let mut resp_rules = None;
    if let Some(target) = target {
        apply_request(&mut req, &target.req).await;

        resp_rules = Some(target.resp.clone());
    }

    if log {
        log::info!("[{} {} {} {}]", host, port, req.method(), req.uri().path());

        let req = hist::Request::from(&req);
        let mut hist = HIST.write().await;
        let xid = hist.request(req);

        let req = &hist.entry(xid).expect("just inserted").request;

        if let Err(err) = report_req(xid, req).await {
            log::error!("failed to report request: {err}");
        }

        id = Some(xid);
    }

    let stream = TcpStream::connect(format!("{host}:{port}")).await.unwrap();
    let io = TokioIo::new(stream);

    let (mut sender, conn) = Builder::new().handshake::<_, Full<Bytes>>(io).await?;
    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            log::error!("Connection failed: {:?}", err);
        }
    });

    let mut builder = Uri::builder();
    if let Some(pq) = req.uri().path_and_query() {
        builder = builder.path_and_query(pq.clone());
    }

    *req.uri_mut() = builder.build().unwrap();

    let resp = sender.send_request(req.map(|b| b.into())).await?;
    let (parts, body) = resp.into_parts();
    let buf = collect(body).await?;
    let mut resp = Response::from_parts(parts, buf);

    if let Some(rules) = resp_rules {
        apply_response(&mut resp, &rules).await;
    }

    if log {
        log::info!("[{} {} {}]", host, port, resp.status());
    }

    if let Some(id) = id {
        let res = hist::Response::from(&resp);
        let mut hist = HIST.write().await;
        if hist.response(id, res) {
            let res = hist
                .entry(id)
                .expect("inserted")
                .response
                .as_ref()
                .expect("response populated");

            if let Err(err) = report_res(id, res).await {
                log::error!("failed to report response: {err}");
            }
        }
    }

    Ok(resp.map(|b| b.into()))
}

async fn collect(mut incoming: Incoming) -> eyre::Result<Vec<u8>> {
    let mut buf = Vec::with_capacity(0x2000);
    while let Some(next) = incoming.frame().await {
        let frame = next?;
        if let Some(chunk) = frame.data_ref() {
            buf.write_all(chunk).await?;
        }
    }

    Ok(buf)
}