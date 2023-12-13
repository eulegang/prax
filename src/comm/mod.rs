use std::sync::Arc;

use nvim_rs::Value;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::cli::NvimConnInfo;
use comms::NvimComms;
use io::IoConn;

use self::handler::Handler;

mod bindings;
mod comms;
mod handler;
mod io;

pub use bindings::*;

pub(crate) type Neovim = nvim_rs::Neovim<IoConn>;
pub(crate) type Buffer = nvim_rs::Buffer<IoConn>;

lazy_static::lazy_static! {
    static ref COMMS: Arc<Mutex<Option<NvimComms>>> = Arc::new(Mutex::new(None));
}

pub async fn main(conn_info: NvimConnInfo, token: CancellationToken) -> eyre::Result<()> {
    let handler = Handler::init(token.clone());
    let single = conn_info.singleton();

    let (nvim, join) = IoConn::connect(&conn_info, handler).await?;

    let list = nvim.create_buf(true, true).await?;
    let detail = nvim.create_buf(false, true).await?;
    list.set_name("atkpx").await?;

    let namespace = nvim.create_namespace("atkpx").await?;

    let win = nvim.get_current_win().await?;
    win.set_buf(&list).await?;

    list.set_keymap("n", "<cr>", ":lua require(\"atkpx\").detail()<cr>", vec![])
        .await?;

    {
        let mut comms = COMMS.lock().await;
        comms.replace(NvimComms::init(nvim, list, detail, namespace));
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
