use super::{Filter, Scribe};

impl Filter for () {
    async fn modify_request(
        &self,
        _: &str,
        _: &mut super::Req<Vec<u8>>,
    ) -> Result<(), super::Error> {
        Ok(())
    }

    async fn modify_response(
        &self,
        _: &str,
        _: &mut super::Res<Vec<u8>>,
    ) -> Result<(), super::Error> {
        Ok(())
    }
}

impl Scribe for () {
    type Ticket = ();

    async fn report_request(&self, _: &super::Req<Vec<u8>>) -> Self::Ticket {}
    async fn report_response(&self, _: Self::Ticket, _: &super::Res<Vec<u8>>) {}
}
