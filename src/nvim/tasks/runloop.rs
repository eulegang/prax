use nvim_rs::error::LoopError;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

pub fn runloop(join: JoinHandle<Result<(), Box<LoopError>>>, token: Option<CancellationToken>) {
    tokio::spawn(async move {
        match join.await {
            Ok(Ok(())) => {}
            Err(loop_err) => {
                tracing::error!("io loop error: {loop_err}");
            }

            Ok(Err(e)) => {
                if !e.is_channel_closed() {
                    tracing::error!("interaction error: {e}");
                }
            }
        }

        if let Some(token) = token {
            token.cancel()
        }
    });
}
