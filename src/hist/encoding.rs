use std::str::FromStr;

use flate2::{
    bufread::{GzDecoder, GzEncoder},
    Compression,
};
use hyper::body::Bytes;
use std::io::Read;

#[derive(Copy, Clone)]
pub enum Encoding {
    Bare,
    Gzip,
}

impl Encoding {
    #[allow(dead_code)]
    pub fn encode(&self, bytes: &Bytes) -> std::io::Result<Bytes> {
        match self {
            Encoding::Bare => Ok(bytes.clone()),
            Encoding::Gzip => {
                let mut gz = GzEncoder::new(bytes.as_ref(), Compression::default());
                let mut buf = Vec::new();
                gz.read_to_end(&mut buf)?;

                Ok(Bytes::from(buf))
            }
        }
    }

    pub fn decode(&self, bytes: &Bytes) -> std::io::Result<Bytes> {
        match self {
            Encoding::Bare => Ok(bytes.clone()),
            Encoding::Gzip => {
                let mut gz = GzDecoder::new(bytes.as_ref());
                let mut buf = Vec::new();

                gz.read_to_end(&mut buf)?;

                Ok(Bytes::from(buf))
            }
        }
    }
}

impl FromStr for Encoding {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "gzip" => Ok(Encoding::Gzip),

            _ => Err(()),
        }
    }
}
