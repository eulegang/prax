use nvim_rs::Value;
use tokio_util::sync::CancellationToken;

use super::Neovim;

#[derive(Debug)]
pub enum Event {
    Detail,
    SubmitIntercept,
    DismissDetail,
    Chan(u64),
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

    async fn handle_notify(&self, name: String, args: Vec<Value>, _: Neovim) {
        log::debug!("notify was sent: {name}");
        match name.as_str() {
            "shutdown" => self.cancel.cancel(),
            "detail" => {
                let _ = self.chan.send(Event::Detail).await;
            }

            "job_id" => {
                let Some(Value::Integer(i)) = args.first() else {
                    return;
                };

                let Some(i) = i.as_u64() else { return };

                let _ = self.chan.send(Event::Chan(i)).await;
            }

            "dismiss_detail" => {
                let _ = self.chan.send(Event::DismissDetail).await;
            }

            "submit_intercept" => {
                let _ = self.chan.send(Event::SubmitIntercept).await;
            }

            _ => (),
        }
    }
}
