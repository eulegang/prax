use std::str::FromStr;

use http::{uri::PathAndQuery, HeaderName, HeaderValue, Uri};
use hyper::Method;

use super::Attr;

#[derive(thiserror::Error, Debug)]
pub enum AttrError {
    #[error("Value not utf8")]
    Str(#[from] std::str::Utf8Error),

    #[error("Value not utf8")]
    String(#[from] std::string::FromUtf8Error),

    #[error("{0}")]
    Method(#[from] http::method::InvalidMethod),

    #[error("")]
    Uri(#[from] http::uri::InvalidUri),

    #[error("{0}")]
    HeaderName(#[from] http::header::InvalidHeaderName),

    #[error("{0}")]
    HeaderValue(#[from] http::header::InvalidHeaderValue),
}

pub trait Attributable {
    fn set(&mut self, attr: &Attr, value: Vec<u8>) -> Result<(), AttrError>;
}

impl Attributable for hyper::Request<Vec<u8>> {
    fn set(&mut self, attr: &Attr, value: Vec<u8>) -> Result<(), AttrError> {
        match attr {
            super::Attr::Method => {
                let update = Method::from_bytes(&value)?;

                *self.method_mut() = update;
            }
            Attr::Status => {}
            Attr::Path => {
                let value = String::from_utf8(value)?;

                let mut parts = self.uri().clone().into_parts();
                let pq = if let Some(pq) = parts.path_and_query {
                    if let Some(query) = pq.query() {
                        PathAndQuery::from_str(&format!("{}?{}", value, query))?
                    } else {
                        PathAndQuery::from_str(&value)?
                    }
                } else {
                    PathAndQuery::from_str(&value)?
                };

                parts.path_and_query = Some(pq);

                *self.uri_mut() = Uri::from_parts(parts).unwrap();
            }
            Attr::Query(key) => {
                let value = String::from_utf8(value)?;

                let val = if value.is_empty() {
                    "".to_string()
                } else {
                    format!("={value}")
                };

                let mut parts = self.uri().clone().into_parts();
                let pq = if let Some(pq) = parts.path_and_query {
                    if let Some(query) = pq.query() {
                        if query.is_empty() {
                            PathAndQuery::from_str(&format!("{}?{}{}", value, key, value))?
                        } else {
                            PathAndQuery::from_str(&format!("{}?{}&{}{}", value, query, key, val))?
                        }
                    } else {
                        PathAndQuery::from_str(&format!("{}?{}{}", pq.path(), key, val))?
                    }
                } else {
                    PathAndQuery::from_str(&format!("/?{key}{val}"))?
                };

                parts.path_and_query = Some(pq);

                *self.uri_mut() = Uri::from_parts(parts).unwrap();
            }
            Attr::Header(key) => {
                let header = HeaderValue::from_bytes(&value)?;

                self.headers_mut()
                    .insert(HeaderName::from_bytes(key.as_bytes())?, header);
            }
            Attr::Body => {
                *self.body_mut() = value;
            }
        }

        Ok(())
    }
}
