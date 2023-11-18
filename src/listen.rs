use std::{net::SocketAddr, sync::Arc};

use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;
use tokio::{io, net::TcpListener};

use crate::proxy::Proxy;

pub async fn listen(addr: SocketAddr, proxy: Arc<Proxy>) -> Result<(), io::Error> {
    // We create a TcpListener and bind it to 127.0.0.1:3000
    let listener = TcpListener::bind(addr).await?;

    // We start a loop to continuously accept incoming connections
    loop {
        let (stream, _) = listener.accept().await?;

        // Use an adapter to access something implementing `tokio::io` traits as if they implement
        // `hyper::rt` IO traits.
        let io = TokioIo::new(stream);
        let proxy = proxy.clone();

        // Spawn a tokio task to serve multiple connections concurrently
        tokio::task::spawn(async move {
            // Finally, we bind the incoming connection to our `hello` service
            if let Err(err) = http1::Builder::new()
                // `service_fn` converts our function in a `Service`
                .serve_connection(io, proxy.as_ref())
                .await
            {
                println!("Error serving connection: {:?}", err);
            }
        });
    }
}
