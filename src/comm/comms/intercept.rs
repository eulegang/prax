use std::{collections::VecDeque, sync::Arc};

use tokio::sync::Notify;

#[derive(Default)]
pub struct Backlog {
    req_index: usize,
    submit_index: usize,
    backlog: VecDeque<Vec<String>>,
    notify: Arc<Notify>,
}

impl Backlog {
    pub fn push_backlog(&mut self, lines: Vec<String>) {
        self.backlog.push_back(lines);

        self.req_index += 1;
    }

    pub fn notify(&self) -> Arc<Notify> {
        self.notify.clone()
    }

    pub fn req_index(&self) -> usize {
        self.req_index
    }

    pub fn submit_tick(&mut self, tick: usize) -> bool {
        if self.submit_index == tick {
            self.submit_index += 1;

            true
        } else {
            false
        }
    }
}
