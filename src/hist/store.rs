use std::{
    sync::atomic::Ordering,
    sync::atomic::{AtomicPtr, AtomicUsize},
};

const BLK_LEN: usize = 31;

struct Node<T> {
    blk: [AtomicPtr<T>; BLK_LEN],
    next: AtomicPtr<Node<T>>,
}

pub trait Inserter {}

#[derive(Default)]
pub struct Append(AtomicUsize);

#[derive(Default)]
pub struct Random;

impl Inserter for Append {}
impl Inserter for Random {}

impl<T> Node<T> {
    fn alloc() -> *mut Self {
        let b = Box::new(Node {
            blk: [
                AtomicPtr::new(std::ptr::null_mut()),
                AtomicPtr::new(std::ptr::null_mut()),
                AtomicPtr::new(std::ptr::null_mut()),
                AtomicPtr::new(std::ptr::null_mut()),
                AtomicPtr::new(std::ptr::null_mut()),
                AtomicPtr::new(std::ptr::null_mut()),
                AtomicPtr::new(std::ptr::null_mut()),
                AtomicPtr::new(std::ptr::null_mut()),
                AtomicPtr::new(std::ptr::null_mut()),
                AtomicPtr::new(std::ptr::null_mut()),
                AtomicPtr::new(std::ptr::null_mut()),
                AtomicPtr::new(std::ptr::null_mut()),
                AtomicPtr::new(std::ptr::null_mut()),
                AtomicPtr::new(std::ptr::null_mut()),
                AtomicPtr::new(std::ptr::null_mut()),
                AtomicPtr::new(std::ptr::null_mut()),
                AtomicPtr::new(std::ptr::null_mut()),
                AtomicPtr::new(std::ptr::null_mut()),
                AtomicPtr::new(std::ptr::null_mut()),
                AtomicPtr::new(std::ptr::null_mut()),
                AtomicPtr::new(std::ptr::null_mut()),
                AtomicPtr::new(std::ptr::null_mut()),
                AtomicPtr::new(std::ptr::null_mut()),
                AtomicPtr::new(std::ptr::null_mut()),
                AtomicPtr::new(std::ptr::null_mut()),
                AtomicPtr::new(std::ptr::null_mut()),
                AtomicPtr::new(std::ptr::null_mut()),
                AtomicPtr::new(std::ptr::null_mut()),
                AtomicPtr::new(std::ptr::null_mut()),
                AtomicPtr::new(std::ptr::null_mut()),
                AtomicPtr::new(std::ptr::null_mut()),
            ],
            next: AtomicPtr::new(std::ptr::null_mut()),
        });

        Box::into_raw(b)
    }
}

pub struct Store<T, I: Inserter> {
    inserter: I,
    root: AtomicPtr<Node<T>>,
}

impl<T, I> Store<T, I>
where
    I: Inserter,
{
    pub fn get(&self, index: usize) -> Option<&T> {
        let iters = index / BLK_LEN;
        let off = index % BLK_LEN;

        let mut cur = self.root.load(Ordering::SeqCst);

        for _ in 0..iters {
            if cur.is_null() {
                return None;
            }

            let node = unsafe { cur.read() };
            cur = node.next.load(Ordering::SeqCst);
        }

        if cur.is_null() {
            return None;
        }

        let node = unsafe { cur.read() };

        let ptr = &node.blk[off];
        let elem = ptr.load(Ordering::SeqCst);

        if elem.is_null() {
            None
        } else {
            Some(unsafe { &*elem })
        }
    }
}

impl<T> Store<T, Random> {
    pub fn insert(&self, index: usize, elem: T) -> bool {
        let ptr = Box::into_raw(Box::new(elem));

        let iters = index / BLK_LEN;
        let off = index % BLK_LEN;
        let mut cur = self.root.load(Ordering::SeqCst);

        let mut left_over = None::<*mut Node<T>>;

        if cur.is_null() {
            let node = Node::alloc();

            if self
                .root
                .compare_exchange(cur, node, Ordering::AcqRel, Ordering::Relaxed)
                .is_err()
            {
                left_over = Some(node);
            }
        };

        for _ in 0..iters {
            let node = unsafe { cur.read() };

            let next = node.next.load(Ordering::SeqCst);

            if next.is_null() {
                let alloced = if let Some(node) = left_over {
                    left_over = None;
                    node
                } else {
                    Node::alloc()
                };
                let node = unsafe { next.read() };

                if node
                    .next
                    .compare_exchange(
                        std::ptr::null_mut(),
                        alloced,
                        Ordering::AcqRel,
                        Ordering::Relaxed,
                    )
                    .is_err()
                {
                    left_over = Some(alloced);
                }

                cur = alloced
            } else {
                cur = next
            };
        }

        let node = unsafe { cur.read() };

        let success = node.blk[off]
            .compare_exchange(
                std::ptr::null_mut(),
                ptr,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_ok();

        if let Some(node) = left_over {
            loop {
                let n = unsafe { cur.read() };
                let next = n.next.load(Ordering::SeqCst);

                if next.is_null() {
                    match n.next.compare_exchange(
                        std::ptr::null_mut(),
                        node,
                        Ordering::SeqCst,
                        Ordering::Relaxed,
                    ) {
                        Ok(_) => break,
                        Err(_) => {
                            cur = n.next.load(Ordering::SeqCst);
                        }
                    };
                } else {
                    cur = next;
                }
            }
        }

        success
    }
}

impl<T> Store<T, Append> {
    pub fn push(&self, elem: T) -> usize {
        let slot = self.inserter.0.fetch_add(1, Ordering::AcqRel);

        let ptr = Box::into_raw(Box::new(elem));

        let iters = slot / BLK_LEN;
        let off = slot % BLK_LEN;
        let mut cur = self.root.load(Ordering::SeqCst);

        let mut left_over = None::<*mut Node<T>>;

        if cur.is_null() {
            let node = Node::alloc();

            if self
                .root
                .compare_exchange(cur, node, Ordering::AcqRel, Ordering::Relaxed)
                .is_err()
            {
                left_over = Some(node);
            }
        };

        for _ in 0..iters {
            let node = unsafe { cur.read() };

            let next = node.next.load(Ordering::SeqCst);

            if next.is_null() {
                let alloced = if let Some(node) = left_over {
                    left_over = None;
                    node
                } else {
                    Node::alloc()
                };
                let node = unsafe { next.read() };

                if node
                    .next
                    .compare_exchange(
                        std::ptr::null_mut(),
                        alloced,
                        Ordering::AcqRel,
                        Ordering::Relaxed,
                    )
                    .is_err()
                {
                    left_over = Some(alloced);
                }

                cur = alloced
            } else {
                cur = next
            };
        }

        let node = unsafe { cur.read() };

        // should have no conflicts slots can't overlap and slots are guaranteed to be unique
        node.blk[off].store(ptr, Ordering::SeqCst);

        if let Some(node) = left_over {
            loop {
                let n = unsafe { cur.read() };
                let next = n.next.load(Ordering::SeqCst);

                if next.is_null() {
                    match n.next.compare_exchange(
                        std::ptr::null_mut(),
                        node,
                        Ordering::SeqCst,
                        Ordering::Relaxed,
                    ) {
                        Ok(_) => break,
                        Err(_) => {
                            cur = n.next.load(Ordering::SeqCst);
                        }
                    };
                } else {
                    cur = next;
                }
            }
        }

        slot
    }
}

impl<T, I> Default for Store<T, I>
where
    I: Default + Inserter,
{
    fn default() -> Self {
        let inserter = I::default();
        let root = AtomicPtr::new(std::ptr::null_mut());

        Store { inserter, root }
    }
}

#[test]
fn node_size() {
    assert_eq!(std::mem::size_of::<Node<()>>(), 256);
}
