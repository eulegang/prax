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

    pub async fn report_req(&self, id: usize, path: &str) -> eyre::Result<()> {
        self.list
            .set_lines(id as i64, id as i64, false, vec![format!("{}", path)])
            .await?;

        Ok(())
    }

    pub async fn report_res(&self, id: usize, status: u16) -> eyre::Result<()> {
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

    pub async fn show_detail(&self, req: &Request, res: Option<&Response>) -> eyre::Result<()> {
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
