use std::sync::Arc;

use tokio::sync::{oneshot::Receiver, Mutex};

use crate::hist::{History, Request, Response};

use super::{handler::Event, Neovim};

pub struct View {
    neovim: Neovim,
    history: Arc<Mutex<History>>,
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
        Ok(Self { neovim, history })
    }

    pub async fn report_req(&self, id: usize, req: &Request) -> eyre::Result<()> {
        todo!()
    }

    pub async fn report_res(&self, id: usize, res: &Response) -> eyre::Result<()> {
        todo!()
    }

    pub async fn find_line(&self) -> eyre::Result<i64> {
        todo!()
    }

    pub async fn show_detail(
        &mut self,
        _id: usize,
        req: &Request,
        res: Option<&Response>,
    ) -> eyre::Result<()> {
        todo!()
    }

    pub async fn intercept_request(
        &mut self,
        req: &mut hyper::Request<Vec<u8>>,
    ) -> eyre::Result<Receiver<Vec<String>>> {
        todo!()
    }

    pub async fn intercept_response(
        &mut self,
        resp: &mut hyper::Response<Vec<u8>>,
    ) -> eyre::Result<Receiver<Vec<String>>> {
        todo!()
    }

    pub async fn retrieve_intercept_request(
        &mut self,
        lines: Vec<String>,
        req: &mut hyper::Request<Vec<u8>>,
    ) -> eyre::Result<bool> {
        todo!()
    }

    pub async fn retrieve_intercept_response(
        &mut self,
        lines: Vec<String>,
        resp: &mut hyper::Response<Vec<u8>>,
    ) -> eyre::Result<bool> {
        todo!()
    }

    pub async fn submit_intercept(&mut self) -> eyre::Result<()> {
        todo!()
    }
}
