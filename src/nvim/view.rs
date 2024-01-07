use nvim_rs::{error::CallError, Value};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio_util::sync::CancellationToken;

use super::{Buffer, Neovim, Window};

pub struct View {
    neovim: Neovim,
    list: Buffer,
    detail: Buffer,
    intercept: Buffer,

    intercept_win: Option<Window>,

    namespace: i64,
}

impl View {
    pub async fn new(
        neovim: Neovim,
        cancel: CancellationToken,
    ) -> eyre::Result<(Arc<Mutex<Self>>, mpsc::Sender<ViewOp>)> {
        let list = neovim.create_buf(true, true).await?;
        let detail = neovim.create_buf(false, true).await?;
        let intercept = neovim.create_buf(false, true).await?;
        list.set_name("atkpx").await?;
        let intercept_win = None;
        let namespace = neovim.create_namespace("atkpx").await?;

        intercept
            .set_keymap(
                "n",
                "q",
                ":lua require(\"atkpx\").submit_intercept()<cr>",
                vec![],
            )
            .await?;

        let win = neovim.get_current_win().await?;
        win.set_buf(&list).await?;

        list.set_keymap("n", "<cr>", ":lua require(\"atkpx\").detail()<cr>", vec![])
            .await?;

        let s = Self {
            neovim,
            list,
            detail,
            intercept,
            intercept_win,
            namespace,
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
        log::trace!("new reponse found");
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

    async fn handle_detail(&mut self, mut req: Vec<String>, res: Vec<String>) -> eyre::Result<()> {
        req.push(String::new());
        req.extend(res);

        let lines = req;

        self.detail.set_lines(0, -1, false, lines).await?;

        let win = self.neovim.get_current_win().await?;
        let height = win.get_height().await?;
        let width = win.get_width().await?;

        let height = height.saturating_sub(10);
        let width = width.saturating_sub(10);

        self.neovim
            .open_win(
                &self.detail,
                true,
                vec![
                    ("relative".into(), "win".into()),
                    ("style".into(), "minimal".into()),
                    ("row".into(), 5.into()),
                    ("col".into(), 5.into()),
                    ("height".into(), height.into()),
                    ("width".into(), width.into()),
                    ("border".into(), "rounded".into()),
                ],
            )
            .await?;

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

        self.intercept_win = Some(win);

        Ok(())
    }

    async fn handle_dismiss_intercept(&mut self) -> eyre::Result<()> {
        if let Some(win) = &self.intercept_win {
            win.close(true).await?;

            self.intercept_win = None;
        }

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

    DismissIntercept,
}

fn color_method(method: &str) -> &'static str {
    match method.to_uppercase().as_str() {
        "GET" => "AtkpxMethodGET",
        "HEAD" => "AtkpxMethodHEAD",
        "POST" => "AtkpxMethodPOST",
        "PUT" => "AtkpxMethodPUT",
        "DELETE" => "AtkpxMethodDELETE",
        "OPTIONS" => "AtkpxMethodOPTIONS",
        "TRACE" => "AtkpxMethodTRACE",
        "PATCH" => "AtkpxMethodPATCH",

        _ => "AtkpxMethodGET",
    }
}

fn color_status(status: u16) -> &'static str {
    match status {
        100..=199 => "AtkpxStatusInfo",
        200..=299 => "AtkpxStatusOk",
        300..=399 => "AtkpxStatusRedirect",
        400..=499 => "AtkpxStatusClientError",
        500..=599 => "AtkpxStatusServerError",

        _ => "AtkpxStatusServerError",
    }
}
