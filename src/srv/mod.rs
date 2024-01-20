use std::{net::SocketAddr, sync::Arc};

use http_body_util::Full;
use hyper::{body::Bytes, client::conn::http1::SendRequest};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

mod listen;
mod service;
mod tls;

pub use self::tls::Tls;

pub struct Server<F, S: 'static> {
    addr: SocketAddr,
    token: CancellationToken,
    filter: Arc<F>,
    scribe: &'static S,
    tls: Option<Tls>,
}

pub struct Tunnel<F, S: 'static> {
    sender: Arc<Mutex<SendRequest<Full<Bytes>>>>,
    host: String,
    server: Server<F, S>,
}

impl<F, S: 'static> Clone for Server<F, S> {
    fn clone(&self) -> Self {
        Server {
            token: self.token.clone(),
            addr: self.addr,
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
