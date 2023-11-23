use std::net::SocketAddr;

use hyper::{server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
use tokio::{io, net::TcpSocket};

mod srv;

use self::srv::service;

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
            // Finally, we bind the incoming connection to our `hello` service
            if let Err(err) = http1::Builder::new()
                // `service_fn` converts our function in a `Service`
                .serve_connection(io, service_fn(service))
                .await
            {
                println!("Error serving connection: {:?}", err);
            }
        });
    }
}
