use std::future::Future;
use std::pin::Pin;

use http_body_util::Full;
use hyper::body::{Bytes, Incoming};
use hyper::service::Service;
use hyper::{Request, Response};

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

    fn call(&self, _: Request<Incoming>) -> Self::Future {
        fn mk_response(s: String) -> Result<Response<Full<Bytes>>, hyper::Error> {
            Ok(Response::builder().body(Full::new(Bytes::from(s))).unwrap())
        }

        let res = mk_response(format!("Hello, World! {:?}", self.mode));

        Box::pin(async { res })
    }
}
