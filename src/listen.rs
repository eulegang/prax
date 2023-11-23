use std::net::SocketAddr;

use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;
use tokio::{io, net::TcpSocket};

use crate::PROXY;

pub async fn listen(addr: SocketAddr) -> Result<(), io::Error> {
    let socket = TcpSocket::new_v4()?;
    socket.bind(addr)?;
    socket.set_reuseaddr(true)?;
    socket.set_reuseport(true)?;

    let listener = socket.listen(1024)?;

    // We create a TcpListener and bind it to 127.0.0.1:3000
    //let mut listener = TcpListener::bind(addr).await?;

    // We start a loop to continuously accept incoming connections
    loop {
        let (stream, _) = listener.accept().await?;

        // Use an adapter to access something implementing `tokio::io` traits as if they implement
        // `hyper::rt` IO traits.
        let io = TokioIo::new(stream);

        // Spawn a tokio task to serve multiple connections concurrently
        tokio::task::spawn(async move {
            let proxy = PROXY.as_ref().read().await;

            // Finally, we bind the incoming connection to our `hello` service
            if let Err(err) = http1::Builder::new()
                // `service_fn` converts our function in a `Service`
                .serve_connection(io, &*proxy)
                .await
            {
                println!("Error serving connection: {:?}", err);
            }
        });
    }
}
