use std::collections::HashMap;

use serde::{Deserialize, Serialize};

mod body;
mod deser;

pub use body::Body;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub query: HashMap<String, String>,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: Body,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Body,
}

#[derive(Debug)]
pub struct Entry {
    pub request: Request,
    pub response: Option<Response>,
}

#[derive(Default, Debug)]
pub struct History(Vec<Entry>);

impl History {
    pub fn request(&mut self, request: Request) -> usize {
        let response = None;
        let ent = Entry { request, response };

        let idx = self.0.len();

        self.0.push(ent);

        idx
    }

    pub fn response(&mut self, index: usize, res: Response) -> bool {
        let Some(ent) = self.0.get_mut(index) else {
            return false;
        };

        if ent.response.is_some() {
            return false;
        }

        ent.response = Some(res);

        true
    }

    pub fn entry(&self, index: usize) -> Option<&Entry> {
        self.0.get(index)
    }
}

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
