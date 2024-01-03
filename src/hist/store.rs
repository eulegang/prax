use std::{collections::LinkedList, sync::atomic::AtomicUsize, sync::atomic::Ordering};

use super::{Entry, Request};

const BLK_SIZE: usize = 0x20;

pub struct Store {
    len: AtomicUsize,
    blks: LinkedList<Vec<Entry>>,
}

impl Store {
    pub fn request(&mut self, request: Request) -> usize {
        let idx = self.len.fetch_add(1, Ordering::SeqCst);
        let blk = (idx & !(BLK_SIZE - 1)) >> BLK_SIZE.trailing_zeros();
        let off = idx & (BLK_SIZE - 1);

        let response = None;
        let entry = Entry { request, response };

        if off == 0 {
            let mut block = Vec::with_capacity(BLK_SIZE);

            block.push(entry);

            self.blks.push_back(block)
        } else {
            let Some(block) = self.blks.iter_mut().nth(blk) else {
                panic!("foobar");
            };

            if off == block.len() {
                block.push(entry);
            }
        }

        idx
    }
}
