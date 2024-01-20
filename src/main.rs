use config::Config;
use prax::hist::Hist;
use srv::Tls;
use std::{fs::File, sync::Arc};
use tokio_util::sync::CancellationToken;

use clap::Parser;
use log::LevelFilter;
use tokio::sync::Mutex;

mod cli;
mod config;
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

        let nvim = Arc::new(Some(Mutex::new(
            nvim::NVim::connect(nvim, token.clone(), history).await?,
        )));

        let config = if let Some(path) = cli.configure {
            Config::load(&path, nvim)?
        } else {
            Config::default()
        };

        let server = srv::Server::new(cli.listen, token, config, history, tls);
        server.listen().await?;
    } else {
        let config = if let Some(path) = cli.configure {
            Config::load(&path, Arc::new(None))?
        } else {
            Config::default()
        };

        let server = srv::Server::new(cli.listen, token, config, &(), tls);
        server.listen().await?;
    };

    Ok(())
}
