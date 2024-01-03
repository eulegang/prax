use std::collections::HashMap;

use super::{Request, Response};

impl From<&hyper::Request<Vec<u8>>> for Request {
    fn from(value: &hyper::Request<Vec<u8>>) -> Self {
        let method = value.method().to_string();
        let path = value.uri().path().to_string();
        let version = format!("{:?}", value.version());

        let mut headers = HashMap::new();
        let mut query = HashMap::new();

        for (key, value) in value.headers() {
            if let Ok(s) = value.to_str() {
                headers.insert(key.to_string(), s.to_string());
            }
        }

        if let Some(q) = value.uri().path_and_query() {
            if let Some(q) = q.query() {
                for kv in q.split('&') {
                    if let Some((key, value)) = kv.split_once('=') {
                        query.insert(key.to_string(), value.to_string());
                    }
                }
            }
        }

        let body = value.body().clone().into();

        Request {
            method,
            path,
            query,
            version,
            headers,
            body,
        }
    }
}

impl From<&hyper::Response<Vec<u8>>> for Response {
    fn from(value: &hyper::Response<Vec<u8>>) -> Self {
        let status = value.status().as_u16();

        let mut headers = HashMap::new();
        for (key, value) in value.headers() {
            if let Ok(s) = value.to_str() {
                headers.insert(key.to_string(), s.to_string());
            }
        }

        let body = value.body().clone().into();

        Response {
            status,
            headers,
            body,
        }
    }
}
