use futures::Future;

use super::{Req, Res};

/// A trait for modifying in flight requests
pub trait Filter {
    fn modify_request(
        &self,
        hostname: &str,
        req: &mut Req<Vec<u8>>,
    ) -> impl Future<Output = crate::Result<()>> + Send;

    fn modify_response(
        &self,
        hostname: &str,
        req: &mut Res<Vec<u8>>,
    ) -> impl Future<Output = crate::Result<()>> + Send;
}

impl Filter for () {
    async fn modify_request(&self, _: &str, _: &mut super::Req<Vec<u8>>) -> crate::Result<()> {
        Ok(())
    }

    async fn modify_response(&self, _: &str, _: &mut super::Res<Vec<u8>>) -> crate::Result<()> {
        Ok(())
    }
}
