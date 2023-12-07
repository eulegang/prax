use hist::History;
use once_cell::sync::Lazy;
use std::sync::{atomic::AtomicBool, Arc};

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

static HIST: Lazy<Arc<RwLock<History>>> = Lazy::new(|| Arc::new(RwLock::new(History::default())));
static PROXY: Lazy<Arc<RwLock<Proxy>>> = Lazy::new(|| Arc::new(RwLock::new(Proxy::default())));
static COMM: AtomicBool = AtomicBool::new(false);

#[tokio::main(flavor = "multi_thread", worker_threads = 8)]
async fn main() -> eyre::Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        pretty_env_logger::formatted_builder()
            .filter_level(LevelFilter::Info)
            .init();
    } else {
        pretty_env_logger::init()
    }

    let cli = cli::Cli::parse();

    if let Some(path) = cli.configure {
        config::config(path)?
    };

    if cli.stdin {
        COMM.swap(true, std::sync::atomic::Ordering::SeqCst);
        tokio::spawn(async { comm::main().await });
    }

    listen::listen(cli.listen).await?;

    Ok(())
}
