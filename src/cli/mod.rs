use clap::Parser;
use std::{net::SocketAddr, path::PathBuf};

#[derive(Parser)]
pub struct Cli {
    /// Attack endpoint
    #[clap(short, long, default_value = "127.0.0.1:8091")]
    pub listen: SocketAddr,

    /// Configure script
    #[clap(short = 'f', long = "file")]
    pub configure: Option<PathBuf>,

    /// trace all requests
    #[clap(short, long)]
    pub trace: bool,
}
