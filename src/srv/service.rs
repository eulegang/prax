use std::future::Future;
use std::pin::Pin;

use hyper::client::conn::http1::Builder;
use hyper::Uri;
use hyper_util::rt::TokioIo;
use thiserror::Error;

use http_body_util::{BodyExt, Full};
use hyper::{
    body::{Bytes, Incoming},
    service::Service,
    Response,
};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use super::{Filter, Req, Res, Scribe, Server};

type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("io error \"{0}\"")]
    IO(#[from] tokio::io::Error),

    #[error("hyper error \"{0}\"")]
    Hyper(#[from] hyper::Error),
}

impl<F, S> Service<Req<Incoming>> for &Server<F, S>
where
    F: Filter + Send + Sync + 'static,
    S: Scribe + Send + Sync + 'static,
{
    type Response = Res<Full<Bytes>>;
    type Error = Error;

    type Future = Pin<Box<dyn Future<Output = Result<Self::Response>> + Send>>;

    fn call(&self, req: Req<Incoming>) -> Self::Future {
        let Some(host) = req.uri().host() else {
            let body = "Bad Request".as_bytes().into();

            let builder = Response::builder().status(400).body(body).unwrap();
            return Box::pin(async { Ok(builder) });
        };

        let host = host.to_string();
        let port = req.uri().port_u16().unwrap_or(80);
        let lookup = format!("{host}:{port}");

        let filter = self.filter.clone();
        let scribe = self.scribe.clone();

        Box::pin(async move {
            let mut req = collect_req(req).await?;

            filter.modify_request(&mut req).await?;
            let ticket = scribe.report_request(&req).await;

            let stream = TcpStream::connect(lookup).await.unwrap();
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

            let res = sender.send_request(req.map(|b| b.into())).await?;
            let mut res = collect_res(res).await?;

            filter.modify_response(&mut res).await?;
            scribe.report_response(ticket, &res);

            Ok(res.map(|b| b.into()))
        })
    }
}

async fn collect_req(req: Req<Incoming>) -> Result<Req<Vec<u8>>> {
    let (parts, body) = req.into_parts();
    let buf = collect(body).await?;
    Ok(Req::from_parts(parts, buf))
}

async fn collect_res(res: Res<Incoming>) -> Result<Res<Vec<u8>>> {
    let (parts, body) = res.into_parts();
    let buf = collect(body).await?;
    Ok(Res::from_parts(parts, buf))
}

async fn collect(mut incoming: Incoming) -> Result<Vec<u8>> {
    let mut buf = Vec::with_capacity(0x2000);
    while let Some(next) = incoming.frame().await {
        let frame = next?;
        if let Some(chunk) = frame.data_ref() {
            buf.write_all(chunk).await?;
        }
    }

    Ok(buf)
}
