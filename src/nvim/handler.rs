use nvim_rs::Value;
use tokio_util::sync::CancellationToken;

use super::Neovim;

pub enum Event {
    Detail,
    SubmitIntercept,
}

#[derive(Clone)]
pub struct Handler {
    cancel: CancellationToken,
    chan: tokio::sync::mpsc::Sender<Event>,
}

impl Handler {
    pub fn new(cancel: CancellationToken, chan: tokio::sync::mpsc::Sender<Event>) -> Self {
        Handler { cancel, chan }
    }
}

#[async_trait::async_trait]
impl nvim_rs::Handler for Handler {
    type Writer = super::io::IoConn;

    async fn handle_notify(&self, name: String, _: Vec<Value>, _: Neovim) {
        match name.as_str() {
            "shutdown" => self.cancel.cancel(),
            "detail" => {
                let _ = self.chan.send(Event::Detail).await;
            }
            "submit_intercept" => {
                let _ = self.chan.send(Event::SubmitIntercept).await;
            }

            _ => (),
        }
    }
}
