use std::collections::HashMap;

use hyper::body::Bytes;
use serde::{Deserialize, Serialize};

mod deser;

#[derive(Debug, Clone)]
pub struct Body(Bytes);

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Request {
    method: String,
    path: String,
    query: HashMap<String, String>,
    version: String,
    headers: HashMap<String, String>,
    body: Body,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Response {
    status: u32,
    headers: HashMap<String, String>,
    body: Body,
}

#[derive(Debug)]
pub struct Entry {
    pub request: Request,
    pub response: Option<Response>,
}

#[derive(Default, Debug)]
pub struct History(Vec<Entry>);

impl History {
    pub fn get(&self, index: usize) -> Option<&Entry> {
        self.0.get(index)
    }

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
}
