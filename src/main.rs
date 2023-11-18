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

    if let Some(path) = cli.configure {
        let lua = mlua::Lua::new();

        let content = tokio::fs::read_to_string(path).await?;

        let chunk = lua.load(content).set_name("atkpx-config");

        chunk.exec_async().await?;
    }

    let mode = if cli.trace { Mode::Trace } else { Mode::Pass };

    listen::listen(cli.listen, Proxy { mode }.into()).await?;

    Ok(())
}
