use super::{io::IoConn, submit_intercept};
use tokio_util::sync::CancellationToken;

use super::COMMS;
//use crate::HIST;

use super::Neovim;
use nvim_rs::Value;

#[derive(Clone)]
pub struct Handler {
    token: CancellationToken,
}

impl Handler {
    pub fn init(token: CancellationToken) -> Self {
        Handler { token }
    }
}

#[async_trait::async_trait]
impl nvim_rs::Handler for Handler {
    type Writer = IoConn;

    async fn handle_notify(&self, name: String, _: Vec<Value>, _: Neovim) {
        /*
        match name.as_str() {
            "shutdown" => self.token.cancel(),
            "detail" => {
                let hist = HIST.read().await;
                let Some(ref mut comms) = *COMMS.lock().await else {
                    return;
                };

                let (line, req, res) = match comms.find_line().await {
                    Ok(line) => match hist.entry(line as usize) {
                        Some(entry) => (line, &entry.request, &entry.response),
                        None => {
                            log::error!("No history line");
                            return;
                        }
                    },
                    Err(e) => {
                        log::error!("failed to find_line: {e}");
                        return;
                    }
                };

                if let Err(err) = comms.show_detail(line as usize, req, res.as_ref()).await {
                    log::error!("failed to show detail {err}")
                }
            }

            "submit_intercept" => {
                if let Err(e) = submit_intercept().await {
                    log::error!("failed to submit intercept: {e}");
                };
            }
            _ => (),
        }
        */
    }
}
