use nvim_rs::{error::CallError, Value};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio_util::sync::CancellationToken;

use super::{Buffer, Neovim, Window};

pub struct View {
    neovim: Neovim,
    pub chan: u64,
    cancel: CancellationToken,

    list: Buffer,
    intercept: Buffer,

    req_detail: Buffer,
    res_detail: Buffer,

    intercept_win: Option<Window>,
    req_win: Option<Window>,
    res_win: Option<Window>,

    detail_group: i64,
    intercept_group: i64,
    namespace: i64,
}

impl View {
    pub async fn new(
        neovim: Neovim,
        cancel: CancellationToken,
    ) -> eyre::Result<(Arc<Mutex<Self>>, mpsc::Sender<ViewOp>)> {
        let list = neovim.create_buf(true, true).await?;
        let intercept = neovim.create_buf(false, true).await?;
        list.set_name("prax-history").await?;
        let intercept_win = None;
        let namespace = neovim.create_namespace("prax").await?;

        let win = neovim.get_current_win().await?;
        win.set_buf(&list).await?;

        intercept
            .set_keymap(
                "n",
                "<c-q>",
                ":lua require(\"prax\").submit_intercept()<cr>",
                vec![],
            )
            .await?;

        list.set_keymap("n", "<cr>", ":lua require(\"prax\").detail()<cr>", vec![])
            .await?;

        let intercept_group = neovim
            .create_augroup("PraxIntercept", vec![("clear".into(), true.into())])
            .await?;

        let detail_group = neovim
            .create_augroup("PraxGroup", vec![("clear".into(), true.into())])
            .await?;

        let req_detail = neovim.create_buf(false, true).await?;
        let res_detail = neovim.create_buf(false, true).await?;

        let req_win = None;
        let res_win = None;
        let chan = 0;

        let s = Self {
            neovim,
            chan,
            cancel: cancel.clone(),

            list,
            intercept,

            req_detail,
            res_detail,
            intercept_win,
            req_win,
            res_win,
            namespace,
            intercept_group,
            detail_group,
        };

        let handler = Arc::new(Mutex::new(s));
        let res = handler.clone();

        let (send, mut recv) = mpsc::channel(16);
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        break;
                    }

                    next = recv.recv() => {
                        if let Some(op) = next {
                            let mut view = handler.lock().await;

                            view.handle(op).await;
                        } else {
                            break;
                        }

                    }

                }
            }
        });

        Ok((res, send))
    }

    pub async fn find_line(&self) -> eyre::Result<i64> {
        let win = self.neovim.get_current_win().await?;
        let buf = win.get_buf().await?;

        if buf == self.list {
            let (line, _) = win.get_cursor().await?;

            Ok(line)
        } else {
            eyre::bail!("list is not the current window")
        }
    }

    pub async fn intercept_buffer(&self) -> Result<Vec<String>, Box<CallError>> {
        self.intercept.get_lines(0, -1, true).await
    }

    async fn handle(&mut self, op: ViewOp) {
        log::trace!("handling view operation: {op:?}");
        let res = match op {
            ViewOp::NewRequest {
                entry,
                method,
                path,
            } => self.handle_new_request(entry, method, path).await,
            ViewOp::NewResponse { entry, status } => self.handle_new_response(entry, status).await,

            ViewOp::Detail { req, res } => self.handle_detail(req, res).await,
            ViewOp::Intercept { title, content } => self.handle_intercept(title, content).await,

            ViewOp::DismissIntercept => self.handle_dismiss_intercept().await,
            ViewOp::DismissDetail => self.handle_dismiss_detail().await,
        };

        if let Err(e) = res {
            log::error!("Failed to handle view op {e}");
        }
    }

    async fn handle_new_request(
        &mut self,
        entry: usize,
        method: String,
        path: String,
    ) -> eyre::Result<()> {
        let color = color_method(&method);
        let method_len = method.len();

        self.list
            .set_lines(
                entry as i64,
                entry as i64,
                false,
                vec![format!("{} {}", method, path)],
            )
            .await?;

        self.list
            .add_highlight(self.namespace, color, entry as i64, 0, method_len as i64)
            .await?;
        Ok(())
    }

    async fn handle_new_response(&mut self, entry: usize, status: u16) -> eyre::Result<()> {
        let color: Value = color_status(status).into();
        self.list
            .set_extmark(
                self.namespace,
                entry as i64,
                -1,
                vec![
                    (
                        "virt_text".into(),
                        Value::Array(vec![Value::Array(vec![
                            format!("{}", status).into(),
                            color,
                        ])]),
                    ),
                    ("virt_text_pos".into(), "eol".into()),
                ],
            )
            .await?;

        Ok(())
    }

    async fn handle_detail(&mut self, req: Vec<String>, res: Vec<String>) -> eyre::Result<()> {
        let pad = 4;

        self.req_detail.set_lines(0, -1, false, req).await?;
        self.res_detail.set_lines(0, -1, false, res).await?;

        let win = self.neovim.get_current_win().await?;
        let height = win.get_height().await?;
        let width = win.get_width().await?;

        let height = height.saturating_sub(2 * pad);
        let width = width.saturating_sub(2 * pad);

        let width = (width / 2).saturating_sub(pad / 2);

        let req_win = self
            .neovim
            .open_win(
                &self.req_detail,
                true,
                vec![
                    ("relative".into(), "editor".into()),
                    ("style".into(), "minimal".into()),
                    ("row".into(), pad.into()),
                    ("col".into(), pad.into()),
                    ("title".into(), "Request".into()),
                    ("height".into(), height.into()),
                    ("width".into(), width.into()),
                    ("border".into(), "rounded".into()),
                ],
            )
            .await?;

        let res_win = self
            .neovim
            .open_win(
                &self.res_detail,
                true,
                vec![
                    ("relative".into(), "editor".into()),
                    ("style".into(), "minimal".into()),
                    ("row".into(), pad.into()),
                    ("col".into(), ((2 * pad) + width).into()),
                    ("title".into(), "Response".into()),
                    ("height".into(), height.into()),
                    ("width".into(), width.into()),
                    ("border".into(), "rounded".into()),
                ],
            )
            .await?;

        self.neovim
            .clear_autocmds(vec![("group".into(), self.detail_group.into())])
            .await?;

        self.neovim
            .create_autocmd(
                "WinClosed".into(),
                vec![
                    ("group".into(), self.detail_group.into()),
                    ("pattern".into(), get_id(&req_win).to_string().into()),
                    (
                        "command".into(),
                        format!(":lua vim.fn.rpcnotify({}, \"dismiss_detail\")", self.chan).into(),
                    ),
                ],
            )
            .await?;

        self.neovim
            .create_autocmd(
                "WinClosed".into(),
                vec![
                    ("group".into(), self.detail_group.into()),
                    ("pattern".into(), get_id(&res_win).to_string().into()),
                    (
                        "command".into(),
                        format!(":lua vim.fn.rpcnotify({}, \"dismiss_detail\")", self.chan).into(),
                    ),
                ],
            )
            .await?;

        self.req_win = Some(req_win);
        self.res_win = Some(res_win);

        Ok(())
    }

    async fn handle_intercept(&mut self, title: String, content: Vec<String>) -> eyre::Result<()> {
        if let Some(s) = &self.intercept_win {
            if s.is_valid().await? {
                s.close(true).await?;
            }
        }

        let win = self.neovim.get_current_win().await?;
        let height = win.get_height().await?;
        let width = win.get_width().await?;

        let height = height.saturating_sub(10);
        let width = width.saturating_sub(10);

        self.intercept.set_lines(0, -1, true, content).await?;

        self.neovim
            .clear_autocmds(vec![("group".into(), self.intercept_group.into())])
            .await?;

        let win = self
            .neovim
            .open_win(
                &self.intercept,
                true,
                vec![
                    ("relative".into(), "editor".into()),
                    ("title".into(), title.into()),
                    ("row".into(), 5.into()),
                    ("col".into(), 5.into()),
                    ("height".into(), height.into()),
                    ("width".into(), width.into()),
                    ("border".into(), "rounded".into()),
                ],
            )
            .await?;

        self.neovim
            .create_autocmd(
                "WinClosed".into(),
                vec![
                    ("group".into(), self.intercept_group.into()),
                    ("pattern".into(), get_id(&win).to_string().into()),
                    (
                        "command".into(),
                        format!(":lua vim.fn.rpcnotify({}, \"submit_intercept\")", self.chan)
                            .into(),
                    ),
                ],
            )
            .await?;

        self.intercept_win = Some(win);

        Ok(())
    }

    async fn handle_dismiss_detail(&mut self) -> eyre::Result<()> {
        if let Some(win) = self.req_win.take() {
            let _ = win.close(true).await;
        }

        if let Some(win) = self.res_win.take() {
            let _ = win.close(true).await;
        }

        Ok(())
    }

    async fn handle_dismiss_intercept(&mut self) -> eyre::Result<()> {
        if let Some(win) = self.intercept_win.take() {
            let _ = win.close(true).await;
        }

        Ok(())
    }

    pub async fn shutdown(&mut self) -> eyre::Result<()> {
        log::debug!("looking for windows to close");
        for win in self.neovim.list_wins().await? {
            let Ok(buf) = win.get_buf().await else {
                continue;
            };

            let mut close = false;

            close |= close || buf == self.list;
            close |= close || buf == self.req_detail;
            close |= close || buf == self.res_detail;
            close |= close || buf == self.intercept;

            if close {
                log::debug!("closing window");
                let _ = win.close(true).await;
            }
        }
        log::debug!("looking for windows to close");

        self.cancel.cancel();

        Ok(())
    }
}

#[derive(Debug)]
pub enum ViewOp {
    NewRequest {
        entry: usize,
        method: String,
        path: String,
    },

    NewResponse {
        entry: usize,
        status: u16,
    },

    Detail {
        req: Vec<String>,
        res: Vec<String>,
    },

    Intercept {
        title: String,
        content: Vec<String>,
    },

    DismissDetail,
    DismissIntercept,
}

fn color_method(method: &str) -> &'static str {
    match method.to_uppercase().as_str() {
        "GET" => "PraxMethodGET",
        "HEAD" => "PraxMethodHEAD",
        "POST" => "PraxMethodPOST",
        "PUT" => "PraxMethodPUT",
        "DELETE" => "PraxMethodDELETE",
        "OPTIONS" => "PraxMethodOPTIONS",
        "TRACE" => "PraxMethodTRACE",
        "PATCH" => "PraxMethodPATCH",

        _ => "PraxMethodGET",
    }
}

fn color_status(status: u16) -> &'static str {
    match status {
        100..=199 => "PraxStatusInfo",
        200..=299 => "PraxStatusOk",
        300..=399 => "PraxStatusRedirect",
        400..=499 => "PraxStatusClientError",
        500..=599 => "PraxStatusServerError",

        _ => "PraxStatusServerError",
    }
}

fn get_id(win: &Window) -> u64 {
    let Value::Ext(1, bytes) = win.get_value() else {
        unreachable!();
    };

    let bytes = &bytes[1..];

    let mut buf = [0u8; 8];

    for (i, byte) in bytes.iter().rev().enumerate() {
        buf[7 - i] = *byte;
    }

    u64::from_be_bytes(buf)
}
