use std::{net::SocketAddr, sync::Arc};

use futures::Future;
use tokio_util::sync::CancellationToken;

//mod srv;

mod listen;
mod null;
mod service;

pub use service::{Error, Result};

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
#[allow(async_fn_in_trait)]
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
}

impl<F, S> Server<F, S> {
    pub fn new(addr: SocketAddr, token: CancellationToken, filter: F, scribe: &'static S) -> Self {
        let filter = Arc::new(filter);

        Server {
            addr,
            token,
            filter,
            scribe,
        }
    }
}
