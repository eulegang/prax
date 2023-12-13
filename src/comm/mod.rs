use std::sync::Arc;

use nvim_rs::{Buffer, Neovim, Value};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::{
    cli::NvimConnInfo,
    hist::{Request, Response},
    io::IoConn,
    HIST,
};

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

    async fn find_line(&self) -> eyre::Result<i64> {
        let win = self.nvim.get_current_win().await?;
        let buf = win.get_buf().await?;

        if buf == self.list {
            let (line, _) = win.get_cursor().await?;

            Ok(line - 1)
        } else {
            eyre::bail!("list is not the current window")
        }
    }

    async fn show_detail(&self, req: &Request, res: Option<&Response>) -> eyre::Result<()> {
        let mut lines: Vec<String> = Vec::new();

        lines.push(format!("{} {} {}", req.method, req.path, req.version));
        for (key, value) in &req.headers {
            lines.push(format!("{}: {}", key, value));
        }

        lines.push(String::new());

        self.detail.set_lines(0, -1, false, lines).await?;

        let win = self.nvim.get_current_win().await?;
        win.set_buf(&self.detail).await?;

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
        match name.as_str() {
            "shutdown" => self.token.cancel(),
            "detail" => {
                let hist = HIST.read().await;
                let Some(ref comms) = *COMMS.lock().await else {
                    return;
                };

                let (req, res) = match comms.find_line().await {
                    Ok(line) => match hist.entry(line as usize) {
                        Some(entry) => (&entry.request, &entry.response),
                        None => {
                            log::error!("No history line");
                            return;
                        }
                    },
                    Err(e) => {
                        log::error!("failed to find_line: {e}");
                        return;
                    }
                };

                if let Err(err) = comms.show_detail(req, res.as_ref()).await {
                    log::error!("failed to show detail {err}")
                }
            }
            _ => (),
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

    list.set_keymap("n", "<cr>", ":lua require(\"atkpx\").detail()<cr>", vec![])
        .await?;

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
