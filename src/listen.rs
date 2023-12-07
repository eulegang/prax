use std::net::SocketAddr;

use hyper::{server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
use tokio::{io, net::TcpSocket};
use tokio_util::sync::CancellationToken;

mod srv;

use self::srv::service;

pub async fn listen(addr: SocketAddr, token: CancellationToken) -> Result<(), io::Error> {
    let socket = TcpSocket::new_v4()?;
    socket.bind(addr)?;
    socket.set_reuseaddr(true)?;
    socket.set_reuseport(true)?;

    let listener = socket.listen(1024)?;

    loop {
        tokio::select! {
            _ = token.cancelled() => {
                return Ok(())
            }

            res = listener.accept() => {
                let (stream, _) = res?;

                let io = TokioIo::new(stream);

                let token = token.clone();
                tokio::task::spawn(async move {
                    tokio::select! {
                        _ = token.cancelled() => { }
                        res = http1::Builder::new().serve_connection(io, service_fn(service)) => {
                            if let Err(err) = res {
                                log::error!("Error service connection: {:?}", err);
                            }
                        }
                    }
                });
            }
        };
    }
}
