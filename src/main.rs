use once_cell::sync::Lazy;
use std::sync::Arc;

use clap::Parser;
use log::LevelFilter;
use proxy::Proxy;
use tokio::sync::RwLock;

mod cli;
mod config;
mod listen;
mod proxy;

static PROXY: Lazy<Arc<RwLock<Proxy>>> = Lazy::new(|| Arc::new(RwLock::new(Proxy::default())));

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

    {
        let proxy = PROXY.read().await;
        eprintln!("Config: {:#?}", proxy);
    }

    listen::listen(cli.listen).await?;

    Ok(())
}
