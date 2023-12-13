use hist::History;
use once_cell::sync::Lazy;
use std::{fs::File, sync::Arc};
use tokio_util::sync::CancellationToken;

use clap::Parser;
use log::LevelFilter;
use proxy::Proxy;
use tokio::sync::RwLock;

mod cli;
mod comm;
mod config;
mod hist;
mod listen;
mod proxy;

mod io;

static HIST: Lazy<Arc<RwLock<History>>> = Lazy::new(|| Arc::new(RwLock::new(History::default())));
static PROXY: Lazy<Arc<RwLock<Proxy>>> = Lazy::new(|| Arc::new(RwLock::new(Proxy::default())));

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

    if let Some(path) = cli.configure {
        config::config(path)?
    };

    let token = CancellationToken::new();

    if let Some(nvim) = cli.nvim {
        let token = token.clone();
        tokio::spawn(async { comm::main(nvim, token).await });
    }

    listen::listen(cli.listen, token).await?;

    Ok(())
}
