use std::future::Future;
use std::pin::Pin;

use http_body_util::combinators::BoxBody;
use http_body_util::BodyExt;
use hyper::body::{Bytes, Incoming};
use hyper::client::conn::http1::Builder;
use hyper::service::Service;
use hyper::{Request, Response, Uri};
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;

pub struct Proxy {
    pub mode: Mode,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Mode {
    Pass,
    Trace,
}

impl Service<Request<Incoming>> for Proxy {
    type Response = Response<BoxBody<Bytes, hyper::Error>>;

    type Error = hyper::Error;

    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, mut req: Request<Incoming>) -> Self::Future {
        let Some(host) = req.uri().host() else {
            return Box::pin(async {
                let body = "Bad Request"
                    .to_string()
                    .boxed()
                    .map_err(|_| todo!())
                    .boxed();

                let builder = Response::builder().status(400).body(body).unwrap();

                Ok(builder)
            });
        };

        let host = host.to_string();
        let port = req.uri().port_u16().unwrap_or(80);

        if self.mode == Mode::Trace {
            log::info!("[{} {} {} {}]", host, port, req.method(), req.uri().path());
            log::info!("headers: {:?}", req.headers());

            log::info!("{}:{}", host, port);
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

            let resp = sender.send_request(req).await?;

            Ok(resp.map(|b| b.boxed()))
        })
    }
}
