use std::sync::Arc;

use nvim_rs::{Buffer, Neovim, Value};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::{cli::NvimConnInfo, io::IoConn};

lazy_static::lazy_static! {
    static ref COMMS: Arc<Mutex<Option<NvimComms>>> = Arc::new(Mutex::new(None));
}

#[allow(dead_code)]
struct NvimComms {
    nvim: Neovim<IoConn>,
    list: Buffer<IoConn>,
    detail: Buffer<IoConn>,
    namespace: i64,
}

#[derive(Clone)]
struct Handler {
    token: CancellationToken,
}

impl NvimComms {
    async fn report_req(&self, id: usize, path: &str) -> eyre::Result<()> {
        self.list
            .set_lines(id as i64, id as i64, false, vec![format!("{}", path)])
            .await?;

        Ok(())
    }

    async fn report_res(&self, id: usize, status: u16) -> eyre::Result<()> {
        let group: Value = match status {
            100..=199 => "AtkpxStatusInfo".into(),
            200..=299 => "AtkpxStatusOk".into(),
            300..=399 => "AtkpxStatusRedirect".into(),
            400..=499 => "AtkpxStatusClientError".into(),
            500..=599 => "AtkpxStatusServerError".into(),

            _ => eyre::bail!("Invalid status code"),
        };

        log::info!("group: {group}");

        self.list
            .set_extmark(
                self.namespace,
                id as i64,
                -1,
                vec![
                    (
                        "virt_text".into(),
                        Value::Array(vec![Value::Array(vec![format!("{status}").into(), group])]),
                    ),
                    ("virt_text_pos".into(), "eol".into()),
                ],
            )
            .await?;

        Ok(())
    }
}

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

    let list = nvim.create_buf(true, true).await?;
    let detail = nvim.create_buf(false, true).await?;
    list.set_name("atkpx").await?;

    let namespace = nvim.create_namespace("atkpx").await?;

    let win = nvim.get_current_win().await?;
    win.set_buf(&list).await?;

    {
        let mut comms = COMMS.lock().await;
        comms.replace(NvimComms {
            nvim,
            list,
            detail,
            namespace,
        });
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
