use std::convert::Infallible;
use std::str::FromStr;

use crate::hist::{self, Body, Encoding};

/// Generates a representation line by line
pub trait ToLines {
    /// Error associated with generating line by line representation
    type Error: std::error::Error;

    /// generate a line by line text representation
    fn to_lines(&self) -> Result<Vec<String>, Self::Error>;
}

impl ToLines for hyper::Request<Vec<u8>> {
    type Error = crate::Error;

    fn to_lines(&self) -> Result<Vec<String>, Self::Error> {
        let mut res = Vec::new();

        let mut status = String::new();

        status.push_str(self.method().to_string().as_ref());
        status.push(' ');
        status.push_str(self.uri().path());
        if let Some(s) = self.uri().query() {
            status.push('?');
            status.push_str(s);
        }

        res.push(status);

        for (k, v) in self.headers() {
            res.push(format!("{}: {}", k, v.to_str()?));
        }

        res.push(String::new());

        let body = std::str::from_utf8(self.body())?;

        for line in body.lines() {
            res.push(line.to_string());
        }

        Ok(res)
    }
}

impl ToLines for hist::Request {
    type Error = Infallible;

    fn to_lines(&self) -> Result<Vec<String>, Self::Error> {
        let mut res = Vec::new();
        let mut status = String::new();

        status.push_str(&self.method);
        status.push(' ');
        status.push_str(&self.path);

        let mut query = String::new();
        for (k, v) in &self.query {
            if !query.is_empty() {
                query.push('&');
            }

            query.push_str(k);
            query.push('=');
            query.push_str(v);
        }

        if !query.is_empty() {
            status.push('?');
            status.push_str(&query);
        }

        res.push(status);

        let mut encoding = Encoding::Bare;

        for (k, v) in &self.headers {
            if k == "content-encoding" {
                if let Ok(e) = Encoding::from_str(v) {
                    encoding = e;
                }
            }
            res.push(format!("{}: {}", k, v));
        }

        res.push(String::new());

        let b: Body;

        let body = if let Ok(body) = self.body.decode(encoding) {
            b = body;
            &b
        } else {
            &self.body
        };

        if let Some(lines) = body.lines() {
            for line in lines {
                res.push(line.to_string());
            }
        } else {
            res.push("[binary]".to_string());
            body.hex(&mut res);
        }

        Ok(res)
    }
}

impl ToLines for hyper::Response<Vec<u8>> {
    type Error = crate::Error;

    fn to_lines(&self) -> Result<Vec<String>, Self::Error> {
        let mut res = Vec::new();

        res.push(self.status().as_u16().to_string());

        for (k, v) in self.headers() {
            res.push(format!("{}: {}", k, v.to_str()?));
        }

        res.push(String::new());

        let body = std::str::from_utf8(self.body())?;

        for line in body.lines() {
            res.push(line.to_string());
        }

        Ok(res)
    }
}
impl ToLines for crate::hist::Response {
    type Error = Infallible;

    fn to_lines(&self) -> Result<Vec<String>, Self::Error> {
        let mut res = Vec::new();

        res.push(self.status.to_string());

        let mut encoding = Encoding::Bare;

        for (k, v) in &self.headers {
            if k == "content-encoding" {
                if let Ok(e) = Encoding::from_str(v) {
                    encoding = e;
                }
            }
            res.push(format!("{}: {}", k, v));
        }

        res.push(String::new());
        let b: Body;

        let body = if let Ok(body) = self.body.decode(encoding) {
            b = body;
            &b
        } else {
            &self.body
        };

        if let Some(lines) = body.lines() {
            for line in lines {
                res.push(line.to_string());
            }
        } else {
            res.push("[binary]".to_string());
            body.hex(&mut res);
        }

        Ok(res)
    }
}
