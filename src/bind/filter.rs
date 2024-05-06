use futures::Future;

use super::{Req, Res};

/// A trait for modifying in flight requests
pub trait Filter {
    fn modify_request(
        &self,
        hostname: &mut String,
        req: &mut Req<Vec<u8>>,
    ) -> impl Future<Output = crate::Result<()>> + Send;

    fn modify_response(
        &self,
        hostname: &mut String,
        req: &mut Res<Vec<u8>>,
    ) -> impl Future<Output = crate::Result<()>> + Send;
}

impl Filter for () {
    async fn modify_request(
        &self,
        _: &mut String,
        _: &mut super::Req<Vec<u8>>,
    ) -> crate::Result<()> {
        Ok(())
    }

    async fn modify_response(
        &self,
        _: &mut String,
        _: &mut super::Res<Vec<u8>>,
    ) -> crate::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn test_null_filter() {
    let mut req = Req::builder()
        .method("GET")
        .header("Host", "example.com")
        .body(Vec::new())
        .unwrap();
    let init_req = req.clone();

    let mut res = Res::builder().status(200).body(Vec::new()).unwrap();
    let init_res = res.clone();

    let mut host = String::from("example.com");

    ().modify_request(&mut host, &mut req).await.unwrap();
    ().modify_response(&mut host, &mut res).await.unwrap();

    assert_eq!(req.uri(), init_req.uri());
    assert_eq!(req.headers(), init_req.headers());
    assert_eq!(req.body(), init_req.body());

    assert_eq!(res.status(), init_res.status());
    assert_eq!(res.headers(), init_res.headers());
    assert_eq!(res.body(), init_res.body());
}
