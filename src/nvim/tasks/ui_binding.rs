use std::{collections::VecDeque, sync::Arc};

use tokio::sync::{
    mpsc::{Receiver, Sender},
    Mutex, Notify,
};

use crate::{
    hist::Hist,
    nvim::{
        handler::Event,
        lines::ToLines,
        view::{View, ViewOp},
    },
};

pub fn ui_binding(
    mut recv: Receiver<Event>,
    view: Arc<Mutex<View>>,
    actions: Sender<ViewOp>,
    backlog: Arc<Mutex<VecDeque<Arc<Notify>>>>,
    history: &'static Hist,
) {
    tokio::spawn(async move {
        while let Some(event) = recv.recv().await {
            log::trace!("handling ui event: {event:?}");
            let mut view = view.lock().await;

            match event {
                Event::Detail => {
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

                    let Ok(_) = actions.send(ViewOp::Detail { req, res }).await else {
                        return; // stop subtask if no reciever
                    };
                }
                Event::SubmitIntercept => {
                    let mut backlog = backlog.lock().await;

                    if let Some(notify) = backlog.pop_front() {
                        if backlog.is_empty() {
                            let Ok(_) = actions.send(ViewOp::DismissIntercept).await else {
                                return;
                            };
                        }
                        notify.notify_one();
                    }
                }

                Event::DismissDetail => {
                    let _ = actions.send(ViewOp::DismissDetail).await;
                }

                Event::Chan(chan) => {
                    view.chan = chan;
                }
            }
        }
    });
}
