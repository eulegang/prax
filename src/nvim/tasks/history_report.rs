use tokio::sync::mpsc::Sender;

use crate::{hist::Hist, nvim::view::ViewOp};

pub fn history_report(actions: Sender<ViewOp>, history: &'static Hist) {
    tokio::spawn(async move {
        let mut recv = history.listen();

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

                    if actions
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

                    if actions
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
}
