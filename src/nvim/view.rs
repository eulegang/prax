use std::sync::Arc;

use nvim_rs::Value;
use tokio::sync::{oneshot::Receiver, Mutex};

use crate::{
    hist::{History, Request, Response},
    nvim::{
        lines::{imprint_lines, imprint_lines_resp},
        windower::DimTracker,
    },
    srv,
};

use super::{
    handler::Event,
    intercept::Backlog,
    lines::{req_to_lines, resp_to_lines},
    Buffer, Neovim, Window,
};

pub struct View {
    neovim: Neovim,
    history: Arc<Mutex<History>>,

    list: Buffer,
    detail: Buffer,
    intercept: Buffer,
    intercept_win: Option<Window>,

    backlog: Backlog,
    editing: bool,
    namespace: i64,
}

impl View {
    pub async fn handle_event(&mut self, event: Event) {
        match event {
            Event::Detail => {
                let (line, req, res) = {
                    let hist = self.history.lock().await;

                    match self.find_line().await {
                        Ok(line) => match hist.entry(line as usize) {
                            Some(entry) => {
                                (line.clone(), entry.request.clone(), entry.response.clone())
                            }
                            None => {
                                log::error!("No history line");
                                return;
                            }
                        },
                        Err(e) => {
                            log::error!("failed to find_line: {e}");
                            return;
                        }
                    }
                };

                if let Err(err) = self.show_detail(line as usize, &req, res.as_ref()).await {
                    log::error!("failed to show detail {err}")
                }
            }

            Event::SubmitIntercept => {
                if let Err(e) = self.submit_intercept().await {
                    log::error!("failed to submit intercept: {e}");
                };
            }
        }
    }

    pub async fn new(neovim: Neovim, history: Arc<Mutex<History>>) -> eyre::Result<Self> {
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

        let backlog = Backlog::default();
        let editing = false;

        Ok(Self {
            neovim,
            history,
            list,
            detail,
            intercept,
            intercept_win,
            backlog,
            editing,
            namespace,
        })
    }

    pub async fn report_req(&self, id: usize, req: &Request) -> eyre::Result<()> {
        self.list
            .set_lines(
                id as i64,
                id as i64,
                false,
                vec![format!("{} {}", req.method, req.path)],
            )
            .await?;

        let color = match req.method.as_str() {
            "GET" => "AtkpxMethodGET",
            "HEAD" => "AtkpxMethodHEAD",
            "POST" => "AtkpxMethodPOST",
            "PUT" => "AtkpxMethodPUT",
            "DELETE" => "AtkpxMethodDELETE",
            "OPTIONS" => "AtkpxMethodOPTIONS",
            "TRACE" => "AtkpxMethodTRACE",
            "PATCH" => "AtkpxMethodPATCH",

            _ => "AtkpxMethodGET",
        };

        self.list
            .add_highlight(self.namespace, color, id as i64, 0, req.method.len() as i64)
            .await?;

        Ok(())
    }

    pub async fn report_res(&self, id: usize, res: &Response) -> eyre::Result<()> {
        let group: Value = match res.status {
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
                        Value::Array(vec![Value::Array(vec![
                            format!("{}", res.status).into(),
                            group,
                        ])]),
                    ),
                    ("virt_text_pos".into(), "eol".into()),
                ],
            )
            .await?;

        Ok(())
    }

    pub async fn find_line(&self) -> eyre::Result<i64> {
        let win = self.neovim.get_current_win().await?;
        let buf = win.get_buf().await?;

        if buf == self.list {
            let (line, _) = win.get_cursor().await?;

            Ok(line - 1)
        } else {
            eyre::bail!("list is not the current window")
        }
    }

    pub async fn show_detail(
        &mut self,
        _id: usize,
        req: &Request,
        res: Option<&Response>,
    ) -> eyre::Result<()> {
        let mut dim = DimTracker::default();

        dim.push(format!("{} {} {}", req.method, req.path, req.version));
        for (key, value) in &req.headers {
            dim.push(format!("{}: {}", key, value));
        }

        dim.blank();

        if let Some(body) = req.body.lines() {
            for line in body {
                dim.push(line.to_string());
            }
        }

        dim.blank();

        if let Some(res) = res {
            dim.push(format!("HTTP/1.1 {}", res.status));

            for (key, value) in &res.headers {
                dim.push(format!("{}: {}", key, value));
            }

            dim.blank();

            if let Some(body) = res.body.lines() {
                for line in body {
                    dim.push(line.to_string())
                }
            }
        }

        let (width, height, lines) = dim.take();
        self.detail.set_lines(0, -1, false, lines).await?;

        self.intercept_win = Some(
            self.neovim
                .open_win(
                    &self.detail,
                    true,
                    vec![
                        ("relative".into(), "cursor".into()),
                        ("style".into(), "minimal".into()),
                        ("row".into(), 0.into()),
                        ("col".into(), 0.into()),
                        ("height".into(), height.into()),
                        ("width".into(), width.into()),
                        ("border".into(), "rounded".into()),
                    ],
                )
                .await?,
        );

        Ok(())
    }

    pub async fn intercept_request(
        &mut self,
        req: &mut hyper::Request<Vec<u8>>,
    ) -> srv::Result<Receiver<Vec<String>>> {
        let lines = req_to_lines(req)?;

        if self.editing {
            log::debug!("pushing intercept backlog");
            Ok(self.backlog.push_backlog(lines))
        } else {
            let recv = self.backlog.push_current();
            log::debug!("displaying intercept");
            self.intercept.set_lines(0, -1, false, lines).await?;

            let width = self.neovim.get_current_win().await?.get_width().await?;
            let height = self.neovim.get_current_win().await?.get_height().await?;

            self.intercept_win = Some(
                self.neovim
                    .open_win(
                        &self.intercept,
                        true,
                        vec![
                            ("relative".into(), "editor".into()),
                            ("title".into(), "Intercept Request".into()),
                            ("row".into(), 5.into()),
                            ("col".into(), 5.into()),
                            ("height".into(), (height - 10).into()),
                            ("width".into(), (width - 10).into()),
                            ("border".into(), "rounded".into()),
                        ],
                    )
                    .await?,
            );

            self.editing = true;

            Ok(recv)
        }
    }

    pub async fn intercept_response(
        &mut self,
        resp: &mut hyper::Response<Vec<u8>>,
    ) -> srv::Result<Receiver<Vec<String>>> {
        let lines = resp_to_lines(resp)?;

        if self.editing {
            log::debug!("pushing intercept backlog");
            Ok(self.backlog.push_backlog(lines))
        } else {
            let recv = self.backlog.push_current();
            log::debug!("displaying intercept");
            self.intercept.set_lines(0, -1, false, lines).await?;

            let width = self.neovim.get_current_win().await?.get_width().await?;
            let height = self.neovim.get_current_win().await?.get_height().await?;

            let win = self
                .neovim
                .open_win(
                    &self.intercept,
                    true,
                    vec![
                        ("relative".into(), "editor".into()),
                        ("title".into(), "Intercept Response".into()),
                        ("row".into(), 5.into()),
                        ("col".into(), 5.into()),
                        ("height".into(), (height - 10).into()),
                        ("width".into(), (width - 10).into()),
                        ("border".into(), "rounded".into()),
                    ],
                )
                .await?;

            self.intercept_win = Some(win);

            self.editing = true;

            Ok(recv)
        }
    }

    pub async fn retrieve_intercept_request(
        &mut self,
        lines: Vec<String>,
        req: &mut hyper::Request<Vec<u8>>,
    ) -> srv::Result<bool> {
        log::debug!("filling in response");

        imprint_lines(req, lines)?;

        if let Some(backlog) = self.backlog.pop() {
            self.intercept.set_lines(0, -1, false, backlog).await?;
        } else {
            self.editing = false;
            if let Some(win) = &self.intercept_win {
                win.close(true).await?;
            }

            self.intercept_win = None;
        }

        Ok(true)
    }

    pub async fn retrieve_intercept_response(
        &mut self,
        lines: Vec<String>,
        resp: &mut hyper::Response<Vec<u8>>,
    ) -> srv::Result<bool> {
        log::debug!("filling in response");

        imprint_lines_resp(resp, lines)?;

        if let Some(backlog) = self.backlog.pop() {
            self.intercept.set_lines(0, -1, false, backlog).await?;
        } else {
            self.editing = false;
            if let Some(win) = &self.intercept_win {
                win.close(true).await?;
            }

            self.intercept_win = None;
        }

        Ok(true)
    }

    pub async fn submit_intercept(&mut self) -> srv::Result<()> {
        log::debug!("notifying waiting green threads");

        let lines = self.intercept.get_lines(0, -1, false).await?;

        self.backlog.notify(lines);

        Ok(())
    }
}
