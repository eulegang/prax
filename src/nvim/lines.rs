use std::{convert::Infallible, str::FromStr};

use hyper::{
    header::{HeaderName, HeaderValue},
    HeaderMap, StatusCode, Uri,
};

use crate::{
    hist::{Body, Encoding},
    srv,
};

pub trait ToLines {
    type Error: std::error::Error;

    fn to_lines(&self) -> Result<Vec<String>, Self::Error>;
}

pub trait LinesImprint {
    type Error: std::error::Error;

    fn imprint(&mut self, lines: Vec<String>) -> Result<(), Self::Error>;
}

impl ToLines for hyper::Request<Vec<u8>> {
    type Error = srv::Error;

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

impl ToLines for crate::hist::Request {
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
    type Error = srv::Error;

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

impl LinesImprint for hyper::Request<Vec<u8>> {
    type Error = srv::Error;

    fn imprint(&mut self, lines: Vec<String>) -> Result<(), Self::Error> {
        let Some(status) = lines.first() else {
            return Err(srv::Error::InterceptMalformed);
        };

        let (method, uri) = extract_status(self.uri(), status)?;

        let mut headermap = HeaderMap::new();
        let mut i = 1;
        for line in lines.iter().skip(1) {
            if line.is_empty() {
                break;
            }

            if let Some((name, value)) = line.split_once(':') {
                let name = HeaderName::from_str(name)?;
                let value = HeaderValue::from_str(value)?;

                headermap.insert(name, value);
            }

            i += 1;
        }

        let mut body = Vec::new();

        for line in &lines[i..] {
            body.extend_from_slice(line.as_bytes());
            body.push(b'\n');
        }

        *self.method_mut() = method;
        *self.uri_mut() = uri;
        *self.headers_mut() = headermap;
        *self.body_mut() = body;

        Ok(())
    }
}

impl LinesImprint for hyper::Response<Vec<u8>> {
    type Error = srv::Error;

    fn imprint(&mut self, lines: Vec<String>) -> Result<(), Self::Error> {
        let Some(status) = lines.first() else {
            return Err(srv::Error::InterceptMalformed);
        };

        let code = StatusCode::from_str(status)?;

        let mut headermap = HeaderMap::new();
        let mut i = 1;
        for line in lines.iter().skip(1) {
            if line.is_empty() {
                break;
            }

            if let Some((name, value)) = line.split_once(':') {
                let name = HeaderName::from_str(name)?;
                let value = HeaderValue::from_str(value)?;

                headermap.insert(name, value);
            }

            i += 1;
        }

        let mut body = Vec::new();

        for line in &lines[i..] {
            body.extend_from_slice(line.as_bytes());
            body.push(b'\n');
        }

        *self.status_mut() = code;
        *self.headers_mut() = headermap;
        *self.body_mut() = body;

        Ok(())
    }
}

fn extract_status(uri: &Uri, lines: &str) -> srv::Result<(hyper::Method, hyper::Uri)> {
    let Some((method, path)) = lines.split_once(' ') else {
        return Err(srv::Error::InterceptMalformed);
    };

    let method = hyper::Method::from_str(method)?;
    let mut builder = Uri::builder();

    if let Some(scheme) = uri.scheme() {
        builder = builder.scheme(scheme.clone())
    }

    if let Some(auth) = uri.authority() {
        builder = builder.authority(auth.clone())
    }

    let uri = builder.path_and_query(path).build()?;

    log::debug!("modiified {method:?} {uri:?}");

    Ok((method, uri))
}
