use clap::Parser;
use std::net::SocketAddr;

#[derive(Parser)]
pub struct Cli {
    /// Attack endpoint
    #[clap(short, long, default_value = "127.0.0.1:8091")]
    pub listen: SocketAddr,

    /// trace all requests
    #[clap(short, long)]
    pub trace: bool,
}
