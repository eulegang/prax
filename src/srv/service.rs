use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

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
use tokio::sync::Mutex;
use tokio_rustls::{TlsAcceptor, TlsConnector};

use crate::srv::Tunnel;

use super::{Error, Filter, Req, Res, Result, Scribe, Server};

impl<F, S> Service<Req<Incoming>> for Server<F, S>
where
    F: Filter + Send + Sync + 'static,
    S: Scribe + Send + Sync + 'static,
{
    type Response = Res<Full<Bytes>>;
    type Error = Error;

    type Future = Pin<Box<dyn Future<Output = Result<Self::Response>> + Send>>;

    fn call(&self, req: Req<Incoming>) -> Self::Future {
        log::trace!("starting to service request");
        let Some(host) = req.uri().host() else {
            return Box::pin(async { Err(Error::NoHost) });
        };

        let host = host.to_string();
        let port = req.uri().port_u16().unwrap_or(80);
        let lookup = format!("{host}:{port}");

        log::trace!("request host detected: {lookup:?}");

        let filter = self.filter.clone();
        let scribe = self.scribe;

        if req.method() == Method::CONNECT {
            let client_tls = self.tls.as_ref().map(|t| t.client.clone());
            let server_tls = self.tls.as_ref().map(|t| t.server.clone());
            let srv = self.clone();
            let token = self.token.clone();
            let host = host.clone();

            return Box::pin(async move {
                let Some(client_tls) = client_tls else {
                    return Err(Error::NoTlsConfig);
                };

                let Some(server_tls) = server_tls else {
                    return Err(Error::NoTlsConfig);
                };
                let host = host.clone();

                tokio::spawn(async move {
                    log::trace!("upgrading connection");
                    let upgrade = match hyper::upgrade::on(req).await {
                        Ok(u) => u,
                        Err(e) => {
                            log::error!("failed to upgrade connection {e}");
                            return;
                        }
                    };

                    log::trace!("upgraded connection");

                    let io = TokioIo::new(upgrade);

                    log::trace!("creating acceptor connection");
                    let acceptor = TlsAcceptor::from(server_tls);
                    let incoming = match acceptor.accept(io).await {
                        Ok(i) => i,
                        Err(e) => {
                            log::error!("failed to accept {e}");
                            return;
                        }
                    };
                    let tunnel = TokioIo::new(incoming);

                    log::trace!("connecting to target");
                    let servername = ServerName::try_from(host.clone()).unwrap();
                    let stream = TcpStream::connect(&lookup).await.unwrap();
                    let io = TokioIo::new(stream);

                    let connector = TlsConnector::from(client_tls);

                    let connect = match connector.connect(servername, TokioIo::new(io)).await {
                        Ok(c) => c,
                        Err(e) => {
                            log::error!("failed to make connection to target {e}");
                            return;
                        }
                    };

                    log::trace!("creating sender");
                    let (sender, conn) =
                        match hyper::client::conn::http1::handshake(TokioIo::new(connect)).await {
                            Ok(o) => o,
                            Err(e) => {
                                log::error!("failed to handshake {e}");
                                return;
                            }
                        };

                    log::trace!("spawning sender poller");
                    tokio::spawn(async move {
                        if let Err(e) = conn.await {
                            eprintln!("Error in connection: {}", e);
                        }
                    });

                    let sender = Arc::new(Mutex::new(sender));

                    log::trace!("spawning tunneled server");
                    tokio::spawn(async move {
                        tokio::select! {
                            () = token.cancelled() => { }

                            res = hyper::server::conn::http1::Builder::new().serve_connection(tunnel, Tunnel { sender, host, server: srv } ).with_upgrades() => {
                                if let Err(err) = res {
                                    log::error!("Error service connection: {:?}", err);
                                }
                            }
                        }
                    });
                });

                let body = "".as_bytes().into();
                let builder = Response::builder().status(200).body(body).unwrap();
                Ok(builder)
            });
        }

        Box::pin(async move {
            let mut req = collect_req(req).await?;

            filter.modify_request(&lookup, &mut req).await?;

            log::trace!("sending modified request to scribe");
            let ticket = scribe.report_request(&req).await;
            log::trace!("done sending modified request to scribe");

            let stream = TcpStream::connect(&lookup).await.unwrap();
            let io = TokioIo::new(stream);

            log::trace!("starting connection to requested host");
            let (mut sender, conn) = Builder::new().handshake::<_, Full<Bytes>>(io).await?;
            tokio::task::spawn(async move {
                if let Err(err) = conn.await {
                    log::error!("Connection failed: {:?}", err);
                }
            });

            log::trace!("established connection to requested host");

            let mut builder = Uri::builder();
            if let Some(pq) = req.uri().path_and_query() {
                builder = builder.path_and_query(pq.clone());
            }

            *req.uri_mut() = builder.build().unwrap();

            let res = sender.send_request(req.map(|b| b.into())).await?;
            let mut res = collect_res(res).await?;

            filter.modify_response(&lookup, &mut res).await?;

            log::trace!("sending modified response to scribe");
            scribe.report_response(ticket, &res).await;
            log::trace!("done sending modified response to scribe");

            log::trace!("finished to service request");
            Ok(res.map(|b| b.into()))
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
        log::trace!("starting to service request");

        log::trace!("servicing tunneled request: {req:?}");
        let host = req.uri().host().unwrap_or_else(|| &self.host);

        let host = host.to_string();
        let port = req.uri().port_u16().unwrap_or(80);
        let lookup = format!("{host}:{port}");

        log::trace!("request host detected: {lookup:?}");

        let filter = self.server.filter.clone();
        let scribe = self.server.scribe;

        if req.method() == Method::CONNECT {
            let client_tls = self.server.tls.as_ref().map(|t| t.client.clone());
            let server_tls = self.server.tls.as_ref().map(|t| t.server.clone());
            let srv = self.server.clone();
            let token = self.server.token.clone();
            let host = host.clone();

            return Box::pin(async move {
                let Some(client_tls) = client_tls else {
                    return Err(Error::NoTlsConfig);
                };

                let Some(server_tls) = server_tls else {
                    return Err(Error::NoTlsConfig);
                };
                let host = host.clone();

                let upgrade = hyper::upgrade::on(req).await?;

                let io = TokioIo::new(upgrade);

                let acceptor = TlsAcceptor::from(server_tls);
                let incoming = acceptor.accept(io).await?;
                let tunnel = TokioIo::new(incoming);

                let servername = ServerName::try_from(host.clone()).unwrap();
                let stream = TcpStream::connect(&lookup).await.unwrap();
                let io = TokioIo::new(stream);

                let connector = TlsConnector::from(client_tls);

                let connect = connector.connect(servername, TokioIo::new(io)).await?;

                let (sender, conn) =
                    hyper::client::conn::http1::handshake(TokioIo::new(connect)).await?;

                tokio::spawn(async move {
                    if let Err(e) = conn.await {
                        eprintln!("Error in connection: {}", e);
                    }
                });

                let sender = Arc::new(Mutex::new(sender));
                tokio::spawn(async move {
                    tokio::select! {
                        () = token.cancelled() => { }

                        res = hyper::server::conn::http1::Builder::new().serve_connection(tunnel, Tunnel { sender, host, server: srv } ) => {
                            if let Err(err) = res {
                                log::error!("Error service connection: {:?}", err);
                            }
                        }
                    }
                });

                let body = "".as_bytes().into();
                let builder = Response::builder().status(200).body(body).unwrap();
                Ok(builder)
            });
        }

        let sender = self.sender.clone();
        Box::pin(async move {
            let mut req = collect_req(req).await?;

            filter.modify_request(&lookup, &mut req).await?;

            log::trace!("sending modified request to scribe");
            let ticket = scribe.report_request(&req).await;
            log::trace!("done sending modified request to scribe");

            let mut builder = Uri::builder();
            if let Some(pq) = req.uri().path_and_query() {
                builder = builder.path_and_query(pq.clone());
            }

            *req.uri_mut() = builder.build().unwrap();

            let mut res = {
                let mut sender = sender.lock().await;
                let res = sender.send_request(req.map(|b| b.into())).await?;

                collect_res(res).await?
            };

            filter.modify_response(&lookup, &mut res).await?;

            log::trace!("sending modified response to scribe");
            scribe.report_response(ticket, &res).await;
            log::trace!("done sending modified response to scribe");

            log::trace!("finished to service request");
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
