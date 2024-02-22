use std::collections::HashMap;

use crate::{
    hist::{Body, Ent, HistoryEvent},
    Scribe,
};

use super::Hist;

#[tokio::test]
async fn test_record() {
    let hist = Hist::default();

    let empty = hist.entry(0);
    assert_eq!(empty, None);

    let req = hyper::Request::new(b"ping".to_vec());
    let res = hyper::Response::new(b"pong".to_vec());

    let hreq = super::Request {
        method: "GET".to_string(),
        path: "/".to_string(),
        query: HashMap::default(),
        version: "HTTP/1.1".to_string(),
        headers: HashMap::default(),
        body: Body::from(b"ping".to_vec()),
    };

    let hres = super::Response {
        status: 200,
        headers: HashMap::default(),
        body: Body::from(b"pong".to_vec()),
    };

    let id = hist.report_request(&req).await;

    let partial = hist.entry(0);
    assert_eq!(
        partial,
        Some(Ent {
            request: &hreq,
            response: None
        })
    );

    hist.report_response(id, &res).await;

    let full = hist.entry(0);

    assert_eq!(
        full,
        Some(Ent {
            request: &hreq,
            response: Some(&hres),
        })
    )
}

#[tokio::test]
async fn test_listen() {
    let hist = Hist::default();

    let req = hyper::Request::new(b"ping".to_vec());
    let res = hyper::Response::new(b"pong".to_vec());

    let mut listener = hist.listen();

    assert!(listener.try_recv().is_err());
    let id = hist.report_request(&req).await;
    assert_eq!(listener.try_recv(), Ok(HistoryEvent::Request { index: 0 }));

    assert!(listener.try_recv().is_err());
    hist.report_response(id, &res).await;
    assert_eq!(listener.try_recv(), Ok(HistoryEvent::Response { index: 0 }));
}

#[test]
fn test_binary_body() {
    let body = Body::from([1, 2, 3, 4].to_vec());

    let mut repr = Vec::new();
    body.hex(&mut repr);

    assert_eq!(repr, vec!["00000000  01 02 03 04".to_string()]);

    let body = Body::from(
        [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
        ]
        .to_vec(),
    );

    let mut repr = Vec::new();
    body.hex(&mut repr);

    assert_eq!(
        repr,
        vec![
            "00000000  00 01 02 03 04 05 06 07  08 09 0A 0B 0C 0D 0E 0F".to_string(),
            "00000010  10 11 12 13 14".to_string()
        ]
    );
}

#[tokio::test]
async fn test_arbitrary_access() {
    let hist = Hist::default();

    let req = hyper::Request::new(b"ping".to_vec());
    let res = hyper::Response::new(b"pong".to_vec());

    let mut listener = hist.listen();

    assert!(listener.try_recv().is_err());
    let id = hist.report_request(&req).await;
    assert_eq!(listener.try_recv(), Ok(HistoryEvent::Request { index: 0 }));

    assert!(listener.try_recv().is_err());
    hist.report_response(id, &res).await;
    assert_eq!(listener.try_recv(), Ok(HistoryEvent::Response { index: 0 }));

    assert!(hist.request(0).is_some());
    assert!(hist.response(0).is_some());

    assert!(hist.request(1).is_none());
    assert!(hist.response(1).is_none());
}
