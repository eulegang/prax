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

pub async fn intercept_request(req: &mut hyper::Request<Vec<u8>>) -> eyre::Result<bool> {
    let recv = {
        let mut comms = COMMS.lock().await;

        if let Some(ref mut comms) = *comms {
            comms.intercept_request(req).await?
        } else {
            return Ok(false);
        }
    };

    let lines = recv.await?;
    {
        let mut comms = COMMS.lock().await;
        if let Some(ref mut comms) = *comms {
            if comms.retrieve_intercept_request(lines, req).await? {}
        };
    }

    Ok(true)
}

pub async fn submit_intercept() -> eyre::Result<()> {
    let mut comms = COMMS.lock().await;

    if let Some(ref mut comms) = *comms {
        comms.submit_intercept().await?;
    }

    Ok(())
}
