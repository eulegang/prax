use nvim_rs::{Neovim, Value};
use tokio_util::sync::CancellationToken;

use crate::{cli::NvimConnInfo, io::IoConn, NVIM};

#[derive(Clone)]
pub struct Handler {
    token: CancellationToken,
}

#[async_trait::async_trait]
impl nvim_rs::Handler for Handler {
    type Writer = IoConn;

    async fn handle_notify(&self, name: String, _: Vec<Value>, _: Neovim<Self::Writer>) {
        if let "shutdown" = name.as_str() {
            self.token.cancel()
        }
    }
}

pub async fn main(conn_info: NvimConnInfo, token: CancellationToken) -> eyre::Result<()> {
    let handler = Handler {
        token: token.clone(),
    };
    let single = conn_info.singleton();

    let (nvim, join) = IoConn::connect(&conn_info, handler).await?;

    {
        let mut n = NVIM.lock().await;
        n.replace(nvim);
    }

    match join.await {
        Ok(Ok(())) => {}
        Err(loop_err) => {
            log::error!("io loop error: {loop_err}");
        }

        Ok(Err(e)) => {
            if !e.is_channel_closed() {
                log::error!("interaction error: {e}");
            }
        }
    }

    if single {
        token.cancel()
    }

    Ok(())
}
