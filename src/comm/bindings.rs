use super::COMMS;

pub async fn report_req(id: usize, path: &str) -> eyre::Result<()> {
    let comms = COMMS.lock().await;

    if let Some(ref comms) = *comms {
        return comms.report_req(id, path).await;
    }

    Ok(())
}

pub async fn report_res(id: usize, status: u16) -> eyre::Result<()> {
    let comms = COMMS.lock().await;

    if let Some(ref comms) = *comms {
        return comms.report_res(id, status).await;
    }

    Ok(())
}
