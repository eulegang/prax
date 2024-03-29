use futures::Future;

use super::{Req, Res};

/// A trait to add to a history store
pub trait Scribe {
    type Ticket: Send;

    fn report_request(&self, req: &Req<Vec<u8>>) -> impl Future<Output = Self::Ticket> + Send;
    fn report_response(
        &self,
        ticket: Self::Ticket,
        res: &Res<Vec<u8>>,
    ) -> impl Future<Output = ()> + Send;
}

impl Scribe for () {
    type Ticket = ();

    async fn report_request(&self, _: &super::Req<Vec<u8>>) -> Self::Ticket {}
    async fn report_response(&self, _: Self::Ticket, _: &super::Res<Vec<u8>>) {}
}

#[tokio::test]
async fn test_null() {
    let req = Req::builder()
        .method("GET")
        .header("Host", "example.com")
        .body(Vec::new())
        .unwrap();

    let res = Res::builder().status(200).body(Vec::new()).unwrap();

    let ticket = ().report_request(&req).await;
    ().report_response(ticket, &res).await;
}
