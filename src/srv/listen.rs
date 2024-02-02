use super::Server;
use hyper::server::conn::http1;
use prax::{Filter, Scribe};
use tokio::{io, net::TcpSocket};

use hyper_util::rt::TokioIo;

impl<F, S> Server<F, S>
where
    F: Filter + Sync + Send + 'static,
    S: Scribe + Sync + Send + 'static,
{
    pub async fn listen(&self) -> Result<(), io::Error> {
        let socket = TcpSocket::new_v4()?;
        socket.bind(self.addr)?;
        socket.set_reuseaddr(true)?;
        socket.set_reuseport(true)?;

        let listener = socket.listen(1024)?;
        let token = self.token.clone();

        loop {
            tokio::select! {
                _ = token.cancelled() => {
                    return Ok(())
                }

                res = listener.accept() => {
                    let (stream, _) = res?;

                    let io = TokioIo::new(stream);

                    let token = token.clone();

                    let srv = self.clone();

                    tokio::task::spawn(async move {
                        tokio::select! {
                            _ = token.cancelled() => { }
                            res = http1::Builder::new().serve_connection(io, srv).with_upgrades() => {
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
}
