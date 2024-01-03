use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

mod body;
mod conv;
mod deser;
//mod store;

pub use body::Body;
use tokio::sync::Mutex;

use crate::srv::Scribe;

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

pub struct Winner {
    history: Arc<Mutex<History>>,
}

impl Winner {
    pub fn new(history: Arc<Mutex<History>>) -> Self {
        Winner { history }
    }
}

impl Scribe for Winner {
    type Ticket = usize;

    async fn report_request(&self, req: &crate::srv::Req<Vec<u8>>) -> Self::Ticket {
        let req = Request::from(req);
        let mut hist = self.history.lock().await;
        hist.request(req)
    }

    async fn report_response(&self, ticket: Self::Ticket, res: &crate::srv::Res<Vec<u8>>) {
        let res = Response::from(res);
        let mut hist = self.history.lock().await;
        hist.response(ticket, res);
    }
}
