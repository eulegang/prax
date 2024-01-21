use std::str::FromStr;

use hyper::{
    header::{HeaderName, HeaderValue},
    HeaderMap, StatusCode, Uri,
};

pub trait LinesImprint {
    type Error: std::error::Error;

    fn imprint(&mut self, lines: Vec<String>) -> Result<(), Self::Error>;
}

impl LinesImprint for hyper::Request<Vec<u8>> {
    type Error = crate::Error;

    fn imprint(&mut self, lines: Vec<String>) -> Result<(), Self::Error> {
        let Some(status) = lines.first() else {
            return Err(crate::Error::InterceptMalformed);
        };

        let (method, uri) = extract_status(self.uri(), status)?;

        let mut headermap = HeaderMap::new();
        let mut i = 1;
        for line in lines.iter().skip(1) {
            if line.is_empty() {
                break;
            }

            if let Some((name, value)) = line.split_once(": ") {
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
    type Error = crate::Error;

    fn imprint(&mut self, lines: Vec<String>) -> Result<(), Self::Error> {
        let Some(status) = lines.first() else {
            return Err(crate::Error::InterceptMalformed);
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
                let value = HeaderValue::from_str(value.trim())?;

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

fn extract_status(uri: &Uri, lines: &str) -> crate::Result<(hyper::Method, hyper::Uri)> {
    let Some((method, path)) = lines.split_once(' ') else {
        return Err(crate::Error::InterceptMalformed);
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
