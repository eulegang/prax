use std::sync::Arc;

use crate::{
    store::{Append, Store},
    Filter,
};

#[derive(Default, Clone)]
pub struct Trace {
    pub requests: Arc<Store<(String, crate::Req<Vec<u8>>), Append>>,
    pub responses: Arc<Store<(String, crate::Res<Vec<u8>>), Append>>,
}

impl Filter for Trace {
    async fn modify_request(
        &self,
        hostname: &str,
        req: &mut crate::Req<Vec<u8>>,
    ) -> crate::Result<()> {
        let hostname = hostname.to_string();
        let req = req.clone();
        self.requests.push((hostname, req));

        Ok(())
    }

    async fn modify_response(
        &self,
        hostname: &str,
        res: &mut crate::Res<Vec<u8>>,
    ) -> crate::Result<()> {
        let hostname = hostname.to_string();
        let res = res.clone();
        self.responses.push((hostname, res));

        Ok(())
    }
}
