use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use hyper::client::conn::http1::SendRequest;
use hyper::Uri;
use hyper::{client::conn::http1::Builder, Method};
use hyper_util::rt::TokioIo;

use http_body_util::{BodyExt, Full};
use hyper::{
    body::{Bytes, Incoming},
    service::Service,
    Response,
};
use rustls::pki_types::ServerName;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::{Mutex, RwLock};
use tokio_rustls::{TlsAcceptor, TlsConnector};
use tokio_util::sync::CancellationToken;

use crate::srv::Tunnel;

use super::{Server, Tls};
use prax::{Error, Filter, Req, Res, Result, Scribe};

impl<F, S> Service<Req<Incoming>> for Server<F, S>
where
    F: Filter + Send + Sync + 'static,
    S: Scribe + Send + Sync + 'static,
{
    type Response = Res<Full<Bytes>>;
    type Error = Error;

    type Future = Pin<Box<dyn Future<Output = Result<Self::Response>> + Send>>;

    fn call(&self, req: Req<Incoming>) -> Self::Future {
        tracing::trace!("starting to service request");
        let Some(host) = req.uri().host() else {
            return Box::pin(async { Err(Error::NoHost) });
        };

        let host = host.to_string();
        let port = req.uri().port_u16().unwrap_or(80);
        let lookup = format!("{host}:{port}");

        tracing::trace!("request host detected: {lookup:?}");

        let filter = self.filter.clone();
        let scribe = self.scribe;

        if req.method() == Method::CONNECT {
            let tls = self.tls.clone();
            let srv = self.clone();
            let token = self.token.clone();
            let host = host.clone();

            return Box::pin(async move { connect(req, tls, srv, host, lookup, token).await });
        }

        Box::pin(async move {
            handle(
                filter,
                scribe,
                req,
                lookup.clone(),
                Connection::Lookup(lookup),
            )
            .await
        })
    }
}

impl<F, S> Service<Req<Incoming>> for Tunnel<F, S>
where
    F: Filter + Send + Sync + 'static,
    S: Scribe + Send + Sync + 'static,
{
    type Response = Res<Full<Bytes>>;
    type Error = Error;

    type Future = Pin<Box<dyn Future<Output = Result<Self::Response>> + Send>>;

    fn call(&self, req: Req<Incoming>) -> Self::Future {
        tracing::trace!("starting to service request");

        tracing::trace!("servicing tunneled request: {req:?}");
        let host = req.uri().host().unwrap_or_else(|| &self.host);

        let host = host.to_string();
        let port = req.uri().port_u16().unwrap_or(80);
        let lookup = format!("{host}:{port}");

        tracing::trace!("request host detected: {lookup:?}");

        let filter = self.server.filter.clone();
        let scribe = self.server.scribe;

        if req.method() == Method::CONNECT {
            let tls = self.server.tls.clone();
            let srv = self.server.clone();
            let token = self.server.token.clone();
            let host = host.clone();

            return Box::pin(async move { connect(req, tls, srv, host, lookup, token).await });
        }

        let sender = self.sender.clone();
        let conn = Connection::Tunnel(sender);
        Box::pin(async move { handle(filter, scribe, req, lookup, conn).await })
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

const WAIT: [u64; 5] = [250, 500, 1000, 2000, 4000];
async fn retry<T, E, F>(op: impl Fn() -> F) -> std::result::Result<T, E>
where
    F: Future<Output = std::result::Result<T, E>>,
{
    let mut it = WAIT.into_iter();

    loop {
        match op().await {
            Ok(t) => return Ok(t),
            Err(e) => {
                if let Some(wait) = it.next() {
                    tokio::time::sleep(tokio::time::Duration::from_millis(wait)).await;
                    continue;
                } else {
                    return Err(e);
                }
            }
        }
    }
}

async fn connect<F, S>(
    req: Req<Incoming>,
    tls: Option<Tls>,
    srv: Server<F, S>,
    host: String,
    lookup: String,
    token: CancellationToken,
) -> Result<Res<Full<Bytes>>>
where
    F: Filter + Send + Sync + 'static,
    S: Scribe + Send + Sync + 'static,
{
    let (client_tls, server_tls) = match tls {
        Some(tls) => (tls.client, tls.server),
        None => return Err(Error::NoTlsConfig),
    };

    tokio::spawn(async move {
        tracing::trace!("upgrading connection");
        let upgrade = match hyper::upgrade::on(req).await {
            Ok(u) => u,
            Err(e) => {
                tracing::error!("failed to upgrade connection {e}");
                return;
            }
        };

        tracing::trace!("upgraded connection");

        let io = TokioIo::new(upgrade);

        tracing::trace!("creating acceptor connection");
        let acceptor = TlsAcceptor::from(server_tls);
        let incoming = match acceptor.accept(io).await {
            Ok(i) => i,
            Err(e) => {
                tracing::error!("failed to accept {e}");
                return;
            }
        };
        let tunnel = TokioIo::new(incoming);

        tracing::trace!("connecting to target");
        let servername = ServerName::try_from(host.clone()).unwrap();
        let stream = retry(|| TcpStream::connect(&lookup)).await.unwrap();
        let io = TokioIo::new(stream);

        let connector = TlsConnector::from(client_tls);

        let connect = match connector.connect(servername, TokioIo::new(io)).await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("failed to make connection to target {e}");
                return;
            }
        };

        tracing::trace!("creating sender");
        let (sender, conn) =
            match hyper::client::conn::http1::handshake(TokioIo::new(connect)).await {
                Ok(o) => o,
                Err(e) => {
                    tracing::error!("failed to handshake {e}");
                    return;
                }
            };

        tracing::trace!("spawning sender poller");
        tokio::spawn(async move {
            if let Err(e) = conn.await {
                eprintln!("Error in connection: {}", e);
            }
        });

        let sender = Arc::new(Mutex::new(sender));

        tracing::trace!("spawning tunneled server");
        tokio::spawn(async move {
            tokio::select! {
                () = token.cancelled() => { }

                res = hyper::server::conn::http1::Builder::new().serve_connection(tunnel, Tunnel { sender, host, server: srv } ).with_upgrades() => {
                    if let Err(err) = res {
                        tracing::error!("Error service connection: {:?}", err);
                    }
                }
            }
        });
    });

    let body = "".as_bytes().into();
    let builder = Response::builder().status(200).body(body).unwrap();
    Ok(builder)
}

pub enum Connection {
    Tunnel(Arc<Mutex<SendRequest<Full<Bytes>>>>),
    Lookup(String),
}

impl Connection {
    async fn send(&self, req: Req<Full<Bytes>>) -> Result<Response<Vec<u8>>> {
        match self {
            Connection::Tunnel(sender) => {
                let mut sender = sender.lock().await;
                let res = sender.send_request(req.map(|b| b.into())).await?;

                Ok(collect_res(res).await?)
            }

            Connection::Lookup(lookup) => {
                let Ok(stream) = retry(|| TcpStream::connect(&lookup)).await else {
                    let body = "".as_bytes().into();
                    let builder = Response::builder().status(502).body(body).unwrap();
                    return Ok(builder);
                };

                let io = TokioIo::new(stream);

                tracing::trace!("starting connection to requested host");
                let (mut sender, conn) = Builder::new().handshake::<_, Full<Bytes>>(io).await?;
                tokio::task::spawn(async move {
                    if let Err(err) = conn.await {
                        tracing::error!("Connection failed: {:?}", err);
                    }
                });

                tracing::trace!("established connection to requested host");

                let res = sender.send_request(req.map(|b| b.into())).await?;
                Ok(collect_res(res).await?)
            }
        }
    }

    fn inject(&mut self, lookup: &str) {
        match self {
            Connection::Tunnel(_) => (),
            Connection::Lookup(internal) => {
                internal.clear();
                internal.push_str(lookup);
            }
        }
    }
}

async fn handle<F, S>(
    filter: Arc<RwLock<Arc<F>>>,
    scribe: &S,
    req: Req<Incoming>,
    mut lookup: String,
    mut conn: Connection,
) -> Result<Res<Full<Bytes>>>
where
    F: Filter + Send + Sync + 'static,
    S: Scribe + Send + Sync + 'static,
{
    let filter = filter.read().await.clone();

    let mut req = collect_req(req).await?;

    filter.modify_request(&mut lookup, &mut req).await?;
    conn.inject(&lookup);

    tracing::trace!("sending modified request to scribe");
    let ticket = scribe.report_request(&req).await;
    tracing::trace!("done sending modified request to scribe");

    let mut builder = Uri::builder();
    if let Some(pq) = req.uri().path_and_query() {
        builder = builder.path_and_query(pq.clone());
    }

    *req.uri_mut() = builder.build().unwrap();

    let mut res = conn.send(req.map(|b| b.into())).await?;

    filter.modify_response(&mut lookup, &mut res).await?;

    tracing::trace!("sending modified response to scribe");
    scribe.report_response(ticket, &res).await;
    tracing::trace!("done sending modified response to scribe");

    tracing::trace!("finished to service request");
    Ok(res.map(|b| b.into()))
}
