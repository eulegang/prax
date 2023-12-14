use crate::hist::{Request, Response};

use super::COMMS;

pub async fn report_req(id: usize, req: &Request) -> eyre::Result<()> {
    let comms = COMMS.lock().await;

    if let Some(ref comms) = *comms {
        return comms.report_req(id, req).await;
    }

    Ok(())
}

pub async fn report_res(id: usize, res: &Response) -> eyre::Result<()> {
    let comms = COMMS.lock().await;

    if let Some(ref comms) = *comms {
        return comms.report_res(id, res).await;
    }

    Ok(())
}
