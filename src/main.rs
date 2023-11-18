use std::sync::Arc;

use clap::Parser;
use log::LevelFilter;
use proxy::Proxy;
use tokio::sync::RwLock;

mod cli;
mod config;
mod listen;
mod proxy;

fn main() -> eyre::Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        pretty_env_logger::formatted_builder()
            .filter_level(LevelFilter::Info)
            .init();
    } else {
        pretty_env_logger::init()
    }

    let cli = cli::Cli::parse();

    let rt = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(8)
            .thread_name("atkpx")
            .enable_all()
            .build()?,
    );

    let proxy = Arc::new(RwLock::new(Proxy::default()));

    if let Some(path) = cli.configure {
        config::config(path, rt.clone(), proxy.clone())?
    };

    rt.block_on(async { listen::listen(cli.listen, proxy).await })?;

    Ok(())
}
