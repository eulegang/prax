use config::Config;
use std::{fs::File, sync::Arc};
use tokio_util::sync::CancellationToken;

use clap::Parser;
use log::LevelFilter;
use tokio::sync::Mutex;

mod cli;
mod config;
mod hist;
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

    let hist = Arc::new(Mutex::new(hist::History::default()));
    let winner = hist::Winner::new(hist.clone());

    let token = CancellationToken::new();

    let nvim = if let Some(nvim) = cli.nvim {
        let token = token.clone();

        Arc::new(Some(Mutex::new(
            nvim::NVim::connect(nvim, token, hist.clone()).await?,
        )))
    } else {
        Arc::new(None)
    };

    let config = if let Some(path) = cli.configure {
        Config::load(path, nvim)?
    } else {
        Config::default()
    };

    let server = srv::Server::new(cli.listen, token, config, winner);
    server.listen().await?;

    Ok(())
}
