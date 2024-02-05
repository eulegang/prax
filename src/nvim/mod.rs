use std::{collections::VecDeque, sync::Arc};

use crate::cli::NvimConnInfo;
use prax::hist::Hist;
use tokio::sync::{mpsc, Mutex, Notify};
use tokio_util::sync::CancellationToken;

use self::{
    handler::Handler,
    view::{View, ViewOp},
};

mod filter;
mod handler;
mod io;
mod tasks;
mod view;

pub use filter::Intercept;

pub(crate) type Neovim = nvim_rs::Neovim<io::IoConn>;
pub(crate) type Buffer = nvim_rs::Buffer<io::IoConn>;
pub(crate) type Window = nvim_rs::Window<io::IoConn>;

pub struct NVim {
    view: Arc<Mutex<View>>,
    action: mpsc::Sender<ViewOp>,
    backlog: Arc<Mutex<VecDeque<Arc<Notify>>>>,
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

    pub fn intercept(self) -> Intercept {
        self.into()
    }
}
