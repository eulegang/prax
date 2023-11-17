use std::future::Future;
use std::pin::Pin;

use http_body_util::Full;
use hyper::body::{Bytes, Incoming};
use hyper::client::conn::http1::Builder;
use hyper::service::Service;
use hyper::{Request, Response};
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
    type Response = Response<Full<Bytes>>;

    type Error = hyper::Error;

    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<Incoming>) -> Self::Future {
        fn mk_response(s: String) -> Result<Response<Full<Bytes>>, hyper::Error> {
            Ok(Response::builder().body(Full::new(Bytes::from(s))).unwrap())
        }

        let host = req.uri().host().unwrap().to_string();
        let port = req.uri().port_u16().unwrap_or(80);

        if self.mode == Mode::Trace {
            log::info!("[{} {} {} {}]", host, port, req.method(), req.uri());
            log::info!("headers: {:?}", req.headers());

            log::info!("{}:{}", host, port);
        }

        let res = mk_response(format!("Hello, World! {:?}", self.mode));

        Box::pin(async move { res })
    }
}
