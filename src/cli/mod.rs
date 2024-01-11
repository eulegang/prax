use clap::Parser;
use std::{net::SocketAddr, path::PathBuf, str::FromStr};

#[derive(Parser)]
pub struct Cli {
    /// attack endpoint
    #[clap(short, long, default_value = "127.0.0.1:8091")]
    pub listen: SocketAddr,

    /// configure script
    #[clap(short = 'f', long = "file")]
    pub configure: Option<PathBuf>,

    /// log file
    #[clap(short = 'L', long)]
    pub log: Option<PathBuf>,

    /// trace all requests
    #[clap(short, long)]
    pub trace: bool,

    /// options to connect neovim
    #[clap(short, long)]
    pub nvim: Option<NvimConnInfo>,

    #[clap(flatten)]
    pub tls: CertOpts,
}

#[derive(Clone)]
pub enum NvimConnInfo {
    Stdin,
    Unix(PathBuf),
}

#[derive(Parser)]
pub struct CertOpts {
    /// private key for a tls session
    #[clap(short, long, requires = "cert")]
    pub key: Option<PathBuf>,

    /// certificate for a tls session
    #[clap(short, long, requires = "key")]
    pub cert: Option<PathBuf>,
}

impl FromStr for NvimConnInfo {
    type Err = <PathBuf as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "-" {
            Ok(NvimConnInfo::Stdin)
        } else {
            Ok(NvimConnInfo::Unix(PathBuf::from_str(s)?))
        }
    }
}

impl NvimConnInfo {
    /// whether this connection method should kill the proxy
    pub fn singleton(&self) -> bool {
        matches!(self, NvimConnInfo::Stdin)
    }
}
