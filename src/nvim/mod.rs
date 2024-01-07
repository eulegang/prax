use std::{collections::VecDeque, sync::Arc};

use crate::{
    cli::NvimConnInfo,
    hist::Hist,
    srv::{self, Filter},
};
use tokio::sync::{mpsc, Mutex, Notify};
use tokio_util::sync::CancellationToken;

use self::{
    handler::Handler,
    lines::{LinesImprint, ToLines},
    view::{View, ViewOp},
};

mod handler;
mod io;
mod lines;
mod tasks;
mod view;

pub(crate) type Neovim = nvim_rs::Neovim<io::IoConn>;
pub(crate) type Buffer = nvim_rs::Buffer<io::IoConn>;
pub(crate) type Window = nvim_rs::Window<io::IoConn>;

pub struct NVim {
    view: Arc<Mutex<View>>,
    action: mpsc::Sender<ViewOp>,
    backlog: Arc<Mutex<VecDeque<Arc<Notify>>>>,
    //pub view: Arc<Mutex<AppState>>,
}

impl NVim {
    pub async fn connect(
        conn_info: NvimConnInfo,
        token: CancellationToken,
        history: &'static Hist,
    ) -> eyre::Result<NVim> {
        let (send, recv) = tokio::sync::mpsc::channel(16);

        let handler = Handler::new(token.clone(), send);
        let single = conn_info.singleton();

        let (nvim, join) = io::IoConn::connect(&conn_info, handler).await?;
        let (view, action) = View::new(nvim, token.clone()).await?;
        let backlog = Arc::new(Mutex::new(VecDeque::<Arc<Notify>>::new()));

        tasks::runloop(join, if single { Some(token) } else { None });
        tasks::ui_binding(recv, view.clone(), action.clone(), backlog.clone(), history);
        tasks::history_report(action.clone(), history);

        Ok(NVim {
            view,
            action,
            backlog,
        })
    }
}

impl Filter for NVim {
    async fn modify_request(&self, _: &str, req: &mut crate::srv::Req<Vec<u8>>) -> srv::Result<()> {
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

    async fn modify_response(
        &self,
        _: &str,
        res: &mut crate::srv::Res<Vec<u8>>,
    ) -> srv::Result<()> {
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
