use std::collections::VecDeque;

use tokio::sync::oneshot::{channel, Receiver, Sender};

#[derive(Default, Debug)]
pub struct Backlog {
    current: Option<Sender<Vec<String>>>,
    backlog: VecDeque<(Sender<Vec<String>>, Vec<String>)>,
}

impl Backlog {
    pub fn push_backlog(&mut self, lines: Vec<String>) -> Receiver<Vec<String>> {
        let (send, recv) = channel();
        self.backlog.push_back((send, lines));

        recv
    }

    pub fn push_current(&mut self) -> Receiver<Vec<String>> {
        let (send, recv) = channel();
        if self.current.is_some() {
            panic!("Failed to check state");
        }

        self.current = Some(send);

        recv
    }

    pub fn notify(&mut self, lines: Vec<String>) {
        if let Some(s) = self.current.take() {
            s.send(lines).unwrap();
        }
    }

    pub fn pop(&mut self) -> Option<Vec<String>> {
        if let Some((send, lines)) = self.backlog.pop_front() {
            self.current = Some(send);

            Some(lines)
        } else {
            self.current = None;

            None
        }
    }
}
