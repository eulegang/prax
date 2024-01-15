use serde::{Deserialize, Serialize};
use std::collections::HashMap;

mod body;
mod conv;
mod deser;
mod encoding;
mod store;

pub use body::Body;
pub use encoding::Encoding;
use tokio::sync::broadcast;

use crate::srv::Scribe;

use self::store::{Append, Random, Store};

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
pub struct Ent<'a> {
    pub request: &'a Request,
    pub response: Option<&'a Response>,
}

#[derive(Debug)]
pub struct Entry {
    pub request: Request,
    pub response: Option<Response>,
}

#[derive(Clone)]
pub enum HistoryEvent {
    Request { index: usize },
    Response { index: usize },
}

#[derive(Debug)]
pub struct Hist {
    requests: Store<Request, Append>,
    responses: Store<Response, Random>,

    events: broadcast::Sender<HistoryEvent>,
}

#[derive(Default, Debug)]
pub struct History(Vec<Entry>);

impl Scribe for Hist {
    type Ticket = usize;

    async fn report_request(&self, req: &crate::srv::Req<Vec<u8>>) -> usize {
        let req = Request::from(req);
        let index = self.requests.push(req);

        let _ = self.events.send(HistoryEvent::Request { index });

        index
    }

    async fn report_response(&self, index: Self::Ticket, res: &crate::srv::Res<Vec<u8>>) {
        let res = Response::from(res);

        if self.responses.insert(index, res) {
            let _ = self.events.send(HistoryEvent::Response { index });
        }
    }
}

impl Hist {
    pub fn entry(&self, index: usize) -> Option<Ent> {
        let request = self.requests.get(index)?;

        let response = self.responses.get(index);

        Some(Ent { request, response })
    }

    pub fn request(&self, index: usize) -> Option<&Request> {
        self.requests.get(index)
    }

    pub fn response(&self, index: usize) -> Option<&Response> {
        self.responses.get(index)
    }

    pub fn listen(&self) -> broadcast::Receiver<HistoryEvent> {
        self.events.subscribe()
    }
}

impl Default for Hist {
    fn default() -> Self {
        let requests = Store::<Request, Append>::default();
        let responses = Store::default();

        let events = broadcast::Sender::new(16);

        Hist {
            requests,
            responses,
            events,
        }
    }
}
