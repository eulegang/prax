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
        let (send, mut recv) = tokio::sync::mpsc::channel(16);

        let handler = Handler::new(token.clone(), send);
        let single = conn_info.singleton();

        let (nvim, join) = io::IoConn::connect(&conn_info, handler).await?;
        let (view, action) = View::new(nvim, token.clone()).await?;
        let backlog = Arc::new(Mutex::new(VecDeque::<Arc<Notify>>::new()));

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

        let v = view.clone();
        let a = action.clone();
        let b = backlog.clone();
        tokio::spawn(async move {
            while let Some(event) = recv.recv().await {
                let view = v.lock().await;

                match event {
                    handler::Event::Detail => {
                        let Ok(line) = view.find_line().await else {
                            continue;
                        };

                        let index = line as usize - 1;

                        let Some(entry) = history.entry(index) else {
                            continue;
                        };

                        let Ok(req) = entry.request.to_lines() else {
                            continue;
                        };

                        let res = match &entry.response {
                            Some(response) => {
                                let Ok(res) = response.to_lines() else {
                                    continue;
                                };

                                res
                            }

                            None => {
                                vec![]
                            }
                        };

                        let Ok(_) = a.send(ViewOp::Detail { req, res }).await else {
                            return; // stop subtask if no reciever
                        };
                    }
                    handler::Event::SubmitIntercept => {
                        let mut backlog = b.lock().await;

                        if let Some(notify) = backlog.pop_front() {
                            notify.notify_one();
                        }
                    }
                }
            }
        });

        let a = action.clone();
        tokio::spawn(async move {
            let mut recv = history.listen();
            let action = a;

            loop {
                let Ok(event) = recv.recv().await else {
                    break;
                };

                match event {
                    crate::hist::HistoryEvent::Request { index } => {
                        let entry = index;
                        let Some(request) = history.request(index) else {
                            continue;
                        };

                        if action
                            .send(ViewOp::NewRequest {
                                entry,
                                method: request.method.clone(),
                                path: request.path.clone(),
                            })
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    crate::hist::HistoryEvent::Response { index } => {
                        let entry = index;
                        let Some(response) = history.response(index) else {
                            continue;
                        };

                        if action
                            .send(ViewOp::NewResponse {
                                entry,
                                status: response.status,
                            })
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                }
            }
        });

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
