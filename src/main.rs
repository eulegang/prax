use clap::Parser;
use log::LevelFilter;
use proxy::Mode;
use tokio::net::TcpListener;

use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;

mod cli;
mod proxy;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        pretty_env_logger::formatted_builder()
            .filter_level(LevelFilter::Info)
            .init();
    } else {
        pretty_env_logger::init()
    }

    let cli = cli::Cli::parse();

    let mode = if cli.trace { Mode::Trace } else { Mode::Pass };

    // We create a TcpListener and bind it to 127.0.0.1:3000
    let listener = TcpListener::bind(cli.listen).await?;

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
                .serve_connection(io, proxy::Proxy { mode })
                .await
            {
                println!("Error serving connection: {:?}", err);
            }
        });
    }
}
