use prax::hist::Hist;
use srv::Tls;
use std::{fs::File, sync::Arc};
use tokio_util::sync::CancellationToken;

use clap::Parser;
use log::LevelFilter;
use prax::proxy::Config;

mod cli;
mod srv;

mod nvim;

#[tokio::main(flavor = "multi_thread", worker_threads = 8)]
async fn main() -> eyre::Result<()> {
    let cli = cli::Cli::parse();

    if let Some(path) = cli.log {
        simplelog::WriteLogger::init(
            LevelFilter::Trace,
            simplelog::Config::default(),
            File::create(path)?,
        )?;
    } else if std::env::var("RUST_LOG").is_err() {
        pretty_env_logger::formatted_builder()
            .filter_level(LevelFilter::Info)
            .init();
    } else {
        pretty_env_logger::init()
    }

    let tls = Tls::load(cli.tls)?;
    let token = CancellationToken::new();

    if let Some(nvim) = cli.nvim {
        log::trace!("loading nvim connection");
        let history: &'static Hist = Box::leak(Box::default());
        let nvim = nvim::NVim::connect(nvim, token.clone(), history).await?;
        let intercept = nvim.intercept();

        if let Some(path) = cli.configure {
            log::debug!("path is correct");

            let config = Config::load(&path, intercept).await?;

            #[cfg(not(target_os = "linux"))]
            let reload = None;

            #[cfg(target_os = "linux")]
            let reload = if cli.watch {
                Some(config.watch(path))
            } else {
                None
            };

            let server = srv::Server::new(cli.listen, token, config, history, tls);
            let server = Arc::new(server);

            let s = server.clone();
            if let Some(mut reload) = reload {
                log::debug!("watching for reloads");
                tokio::spawn(async move {
                    while let Some(filter) = reload.recv().await {
                        log::debug!("found reload");
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
