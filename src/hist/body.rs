use std::str::Split;

use hyper::body::Bytes;

use super::Encoding;

#[derive(Debug, Clone)]
pub struct Body(Bytes);

impl From<Vec<u8>> for Body {
    fn from(value: Vec<u8>) -> Self {
        Body(value.into())
    }
}

impl AsRef<[u8]> for Body {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl Body {
    pub fn lines(&self) -> Option<Split<char>> {
        let s = std::str::from_utf8(&self.0);

        let s = s.ok()?;
        Some(s.split('\n'))
    }

    pub fn encode(&self, encoding: Encoding) -> std::io::Result<Body> {
        Ok(Body(encoding.encode(&self.0)?))
    }

    pub fn decode(&self, encoding: Encoding) -> std::io::Result<Body> {
        Ok(Body(encoding.decode(&self.0)?))
    }
}
