use clap::Parser;
use log::LevelFilter;
use proxy::{Mode, Proxy};

mod cli;
mod listen;
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

    listen::listen(cli.listen, Proxy { mode }.into()).await?;

    Ok(())
}
