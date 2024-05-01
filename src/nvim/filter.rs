use std::sync::Arc;

use prax::Filter;

use super::{view::ViewOp, NVim};

use prax::{Req, Res};
use tokio::sync::{Mutex, Notify};

use prax::lines::{LinesImprint, ToLines};

#[derive(Clone)]
pub struct Intercept(Arc<Mutex<NVim>>);

impl From<NVim> for Intercept {
    fn from(value: NVim) -> Self {
        Intercept(Arc::new(Mutex::new(value)))
    }
}

impl Filter for Intercept {
    async fn modify_request(&self, hostname: &mut str, req: &mut Req<Vec<u8>>) -> prax::Result<()> {
        let nvim = self.0.lock().await;
        nvim.modify_request(hostname, req).await
    }

    async fn modify_response(
        &self,
        hostname: &mut str,
        req: &mut Res<Vec<u8>>,
    ) -> prax::Result<()> {
        let nvim = self.0.lock().await;
        nvim.modify_response(hostname, req).await
    }
}

impl Filter for NVim {
    async fn modify_request(&self, _: &mut str, req: &mut Req<Vec<u8>>) -> prax::Result<()> {
        let content = req.to_lines()?;

        let notify = {
            let mut backlog = self.backlog.lock().await;

            if self
                .action
                .send(ViewOp::Intercept {
                    title: "Intercept Request".into(),
                    content,
                })
                .await
                .is_err()
            {
                return Ok(());
            };

            let notify = Arc::new(Notify::new());
            backlog.push_back(notify.clone());
            notify
        };

        notify.notified().await;

        let view = self.view.lock().await;
        let content = view.intercept_buffer().await?;

        req.imprint(content)?;

        Ok(())
    }

    async fn modify_response(&self, _: &mut str, res: &mut Res<Vec<u8>>) -> prax::Result<()> {
        let content = res.to_lines()?;

        let notify = {
            let mut backlog = self.backlog.lock().await;

            if self
                .action
                .send(ViewOp::Intercept {
                    title: "Intercept Response".into(),
                    content,
                })
                .await
                .is_err()
            {
                return Ok(());
            }

            let notify = Arc::new(Notify::new());
            backlog.push_back(notify.clone());
            notify
        };

        notify.notified().await;

        let view = self.view.lock().await;
        let content = view.intercept_buffer().await?;

        res.imprint(content)?;

        Ok(())
    }
}
