use super::{Buffer, Neovim, Value};

use crate::hist::{Request, Response};

#[allow(dead_code)]
pub struct NvimComms {
    nvim: Neovim,
    list: Buffer,
    detail: Buffer,
    namespace: i64,
}

impl NvimComms {
    pub fn init(nvim: Neovim, list: Buffer, detail: Buffer, namespace: i64) -> Self {
        Self {
            nvim,
            list,
            detail,
            namespace,
        }
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

        self.list
            .add_highlight(
                self.namespace,
                "AtkpxMethodGET",
                id as i64,
                0,
                req.method.len() as i64,
            )
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
        let win = self.nvim.get_current_win().await?;
        let buf = win.get_buf().await?;

        if buf == self.list {
            let (line, _) = win.get_cursor().await?;

            Ok(line - 1)
        } else {
            eyre::bail!("list is not the current window")
        }
    }

    pub async fn show_detail(
        &self,
        id: usize,
        req: &Request,
        res: Option<&Response>,
    ) -> eyre::Result<()> {
        let mut lines: Vec<String> = Vec::new();
        let mut width = 0;

        let line = format!("{} {} {}", req.method, req.path, req.version);
        width = width.max(line.len());
        lines.push(line);
        for (key, value) in &req.headers {
            let line = format!("{}: {}", key, value);
            width = width.max(line.len());
            lines.push(line);
        }

        lines.push(String::new());

        if let Some(body) = req.body.lines() {
            for line in body {
                width = width.max(line.len());
                lines.push(line.to_string());
            }
        }

        lines.push(String::new());

        if let Some(res) = res {
            let line = format!("HTTP/1.1 {}", res.status);
            width = width.max(line.len());
            lines.push(line);

            for (key, value) in &res.headers {
                let line = format!("{}: {}", key, value);
                width = width.max(line.len());
                lines.push(line);
            }

            lines.push(String::new());

            if let Some(body) = res.body.lines() {
                for line in body {
                    width = width.max(line.len());
                    lines.push(line.to_string());
                }
            }
        }

        let height = lines.len();
        self.detail.set_lines(0, -1, false, lines).await?;

        self.nvim
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
            .await?;

        Ok(())
    }
}
