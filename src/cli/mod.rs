use clap::Parser;
use std::net::SocketAddr;

#[derive(Parser)]
pub struct Cli {
    /// Attack endpoint
    #[clap(short, long)]
    pub listen: SocketAddr,

    /// trace all requests
    #[clap(short, long)]
    pub trace: bool,
}
