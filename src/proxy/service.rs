use std::future::Future;
use std::pin::Pin;

use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::client::conn::http1::Builder;
use hyper::service::Service;
use hyper::{Request, Response, Uri};
use hyper_util::rt::TokioIo;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use super::{Elem, Proxy, Rule};

impl Service<Request<Incoming>> for &Proxy {
    type Response = Response<Full<Bytes>>;

    type Error = hyper::Error;

    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, mut req: Request<Incoming>) -> Self::Future {
        let Some(host) = req.uri().host() else {
            return Box::pin(async {
                let body = "Bad Request".as_bytes().into();

                let builder = Response::builder().status(400).body(body).unwrap();
                Ok(builder)
            });
        };

        let host = host.to_string();
        let port = req.uri().port_u16().unwrap_or(80);
        let lookup = format!("{host}:{port}");

        let target = self.find_target(&lookup);
        let log = target.is_some() || !self.focus;

        if log {
            log::info!("[{} {} {} {}]", host, port, req.method(), req.uri().path());
        }

        let mut resp_rules = None;
        if let Some(target) = target {
            apply_request(&mut req, &target.req);

            resp_rules = Some(target.resp.clone());
        }

        Box::pin(async move {
            let stream = TcpStream::connect(format!("{host}:{port}")).await.unwrap();
            let io = TokioIo::new(stream);

            let (mut sender, conn) = Builder::new().handshake(io).await?;
            tokio::task::spawn(async move {
                if let Err(err) = conn.await {
                    println!("Connection failed: {:?}", err);
                }
            });

            let mut builder = Uri::builder();
            if let Some(pq) = req.uri().path_and_query() {
                builder = builder.path_and_query(pq.clone());
            }

            *req.uri_mut() = builder.build().unwrap();

            let mut buf = Vec::with_capacity(0x2000);
            let resp = sender.send_request(req).await?;
            let (parts, mut body) = resp.into_parts();

            // Stream the body, writing each frame to stdout as it arrives
            while let Some(next) = body.frame().await {
                let frame = next?;
                if let Some(chunk) = frame.data_ref() {
                    let _ = buf.write(&chunk).await;
                }
            }

            let mut resp = Response::from_parts(parts, buf);

            if log {
                log::info!("[{} {} {}]", host, port, resp.status());
            }

            if let Some(rules) = resp_rules {
                apply_response(&mut resp, &rules);
            }

            Ok(resp.map(|b| b.into()))
        })
    }
}

fn apply_request(req: &mut Request<Incoming>, rules: &[Rule]) {
    for rule in rules {
        match rule {
            Rule::SetHeader(_, _) => (),
            Rule::Log(Elem::Path) => log::info!("Path: {}", req.uri().path()),
            Rule::Log(Elem::Method) => log::info!("Method: {}", req.method()),
            Rule::Log(Elem::Header(h)) => {
                if let Some(value) = req.headers().get(h) {
                    log::info!("Header ({}): {}", h, value.to_str().unwrap_or("not string"));
                } else {
                    log::info!("Header ({}): (none)", h);
                }
            }
            Rule::Log(Elem::Query(q)) => {
                if let Some(qa) = req.uri().query() {
                    log::info!("todo query! {qa}");
                } else {
                    log::info!("Query ({}): (none)", q)
                }
            }

            Rule::Log(Elem::Body) => (),
            Rule::Log(Elem::Status) => (),
        }
    }
}

fn apply_response(resp: &mut Response<Vec<u8>>, rules: &[Rule]) {
    for rule in rules {
        match rule {
            Rule::SetHeader(_, _) => (),
            Rule::Log(Elem::Path) => (),
            Rule::Log(Elem::Method) => (),
            Rule::Log(Elem::Query(_)) => (),

            Rule::Log(Elem::Header(h)) => {
                if let Some(value) = resp.headers().get(h) {
                    log::info!("Header ({}): {}", h, value.to_str().unwrap_or("not string"));
                } else {
                    log::info!("Header ({}): (none)", h);
                }
            }
            Rule::Log(Elem::Status) => {
                log::info!("Status: {}", resp.status())
            }
            Rule::Log(Elem::Body) => {
                if let Ok(content) = std::str::from_utf8(resp.body()) {
                    log::info!("Body: {}", content);
                } else {
                    log::info!("Body: (binary)");
                }
            }
        }
    }
}
