use std::{net::SocketAddr, sync::Arc};

use futures::Future;
use http_body_util::Full;
use hyper::{body::Bytes, client::conn::http1::SendRequest};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

mod err;
mod listen;
mod null;
mod service;
mod tls;

pub use err::{Error, Result};

pub use self::tls::Tls;

pub type Req<T> = hyper::Request<T>;
pub type Res<T> = hyper::Response<T>;

/// A trait for modifying in flight requests
pub trait Filter {
    fn modify_request(
        &self,
        hostname: &str,
        req: &mut Req<Vec<u8>>,
    ) -> impl Future<Output = Result<()>> + Send;

    fn modify_response(
        &self,
        hostname: &str,
        req: &mut Res<Vec<u8>>,
    ) -> impl Future<Output = Result<()>> + Send;
}

/// A trait to add to a history store
pub trait Scribe {
    type Ticket: Send;

    fn report_request(&self, req: &Req<Vec<u8>>) -> impl Future<Output = Self::Ticket> + Send;
    fn report_response(
        &self,
        ticket: Self::Ticket,
        res: &Res<Vec<u8>>,
    ) -> impl Future<Output = ()> + Send;
}

pub struct Server<F, S: 'static> {
    addr: SocketAddr,
    token: CancellationToken,
    filter: Arc<F>,
    scribe: &'static S,
    tls: Option<Tls>,
}

pub struct Tunnel<F, S: 'static> {
    sender: Arc<Mutex<SendRequest<Full<Bytes>>>>,
    server: Server<F, S>,
}

impl<F, S: 'static> Clone for Server<F, S> {
    fn clone(&self) -> Self {
        Server {
            token: self.token.clone(),
            addr: self.addr.clone(),
            filter: self.filter.clone(),
            scribe: self.scribe,
            tls: self.tls.clone(),
        }
    }
}

impl<F, S> Server<F, S> {
    pub fn new(
        addr: SocketAddr,
        token: CancellationToken,
        filter: F,
        scribe: &'static S,
        tls: Option<Tls>,
    ) -> Self {
        let filter = Arc::new(filter);

        Server {
            addr,
            token,
            filter,
            scribe,
            tls,
        }
    }
}
