use std::sync::Arc;

use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::{cli::NvimConnInfo, hist::History};

use self::{handler::Handler, view::View};

mod handler;
mod io;
mod view;

pub(crate) type Neovim = nvim_rs::Neovim<io::IoConn>;
pub(crate) type Buffer = nvim_rs::Buffer<io::IoConn>;
pub(crate) type Window = nvim_rs::Window<io::IoConn>;

pub struct NVim {
    view: Arc<Mutex<View>>,
}

impl NVim {
    pub async fn connect(
        conn_info: NvimConnInfo,
        token: CancellationToken,
        history: Arc<Mutex<History>>,
    ) -> eyre::Result<NVim> {
        let (send, mut recv) = tokio::sync::mpsc::channel(16);

        let handler = Handler::new(token.clone(), send);
        let single = conn_info.singleton();

        let (nvim, join) = io::IoConn::connect(&conn_info, handler).await?;
        let view = View::new(nvim, history).await?;

        let v = Arc::new(Mutex::new(view));

        tokio::spawn(async move {
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
        });

        let background_view = v.clone();
        tokio::spawn(async move {
            while let Some(event) = recv.recv().await {
                let mut view = background_view.lock().await;
                view.handle_event(event).await;
            }
        });

        Ok(NVim { view: v })
    }
}
