use std::{net::SocketAddr, sync::Arc};

use futures::Future;
use tokio::io;
use tokio_util::sync::CancellationToken;

mod srv;

mod listen;
mod null;
mod service;

pub type Req<T> = hyper::Request<T>;
pub type Res<T> = hyper::Response<T>;

/// A trait for modifying in flight requests
#[allow(async_fn_in_trait)]
pub trait Filter {
    fn modify_request(&self, req: &mut Req<Vec<u8>>)
        -> impl Future<Output = io::Result<()>> + Send;

    fn modify_response(
        &self,
        req: &mut Res<Vec<u8>>,
    ) -> impl Future<Output = io::Result<()>> + Send;
}

/// A trait to add to a history store
#[allow(async_fn_in_trait)]
pub trait Scribe {
    type Ticket: Send;

    fn report_request(&self, req: &Req<Vec<u8>>) -> impl Future<Output = Self::Ticket> + Send;
    fn report_response(
        &self,
        ticket: Self::Ticket,
        req: &Res<Vec<u8>>,
    ) -> impl Future<Output = ()> + Send;
}

pub struct Server<F, S> {
    addr: SocketAddr,
    token: CancellationToken,
    filter: Arc<F>,
    scribe: Arc<S>,
}

impl<F, S> Server<F, S> {
    pub fn new(addr: SocketAddr, token: CancellationToken, filter: F, scribe: S) -> Self {
        let filter = Arc::new(filter);
        let scribe = Arc::new(scribe);

        Server {
            addr,
            token,
            filter,
            scribe,
        }
    }
}
