use prax::hist::Hist;
use srv::Tls;
use std::{fs::File, sync::Arc};
use tokio_util::sync::CancellationToken;
use tracing::Level;

use clap::Parser;
use prax::proxy::Config;

mod cli;
mod srv;

mod nvim;

#[tokio::main(flavor = "multi_thread", worker_threads = 8)]
async fn main() -> eyre::Result<()> {
    let cli = cli::Cli::parse();

    if let Some(path) = cli.log {
        let subscriber = tracing_subscriber::fmt()
            .json()
            .with_max_level(Level::TRACE)
            .with_file(true)
            .with_line_number(true)
            .with_thread_ids(true)
            .with_target(true)
            .with_writer(File::options().create(true).append(true).open(path)?)
            .finish();

        tracing::subscriber::set_global_default(subscriber)?;
    }

    let tls = Tls::load(cli.tls)?;
    let token = CancellationToken::new();

    if let Some(nvim) = cli.nvim {
        let span = tracing::trace_span!("loading nvim connection", info = ?nvim);
        let _conn = span.enter();

        let history: &'static Hist = Box::leak(Box::default());
        let nvim = nvim::NVim::connect(nvim, token.clone(), history).await?;
        let intercept = nvim.intercept();

        if let Some(path) = cli.configure {
            tracing::debug!(?path, "configuring proxy");

            let config = Config::load(&path, intercept).await?;

            #[cfg(not(target_os = "linux"))]
            let reload = None::<tokio::sync::mpsc::Receiver<Config<nvim::Intercept>>>;

            #[cfg(target_os = "linux")]
            let reload = if cli.watch {
                Some(config.watch(path.clone()))
            } else {
                None
            };

            let server = srv::Server::new(cli.listen, token, config, history, tls);
            let server = Arc::new(server);

            let s = server.clone();
            if let Some(mut reload) = reload {
                let watch_span =
                    tracing::debug_span!("watching for reloads", path = %path.display());

                tokio::spawn(async move {
                    while let Some(filter) = reload.recv().await {
                        tracing::debug!(parent: &watch_span, "found reload");
                        s.replace(filter).await;
                    }
                });
            }

            server.listen().await?;
        } else {
            let config = Config::<()>::default();
            let server = srv::Server::new(cli.listen, token, config, history, tls);
            server.listen().await?;
        };
    } else {
        let config = if let Some(path) = cli.configure {
            Config::load(&path, ()).await?
        } else {
            Config::default()
        };

        let server = srv::Server::new(cli.listen, token, config, &(), tls);
        server.listen().await?;
    };

    Ok(())
}
