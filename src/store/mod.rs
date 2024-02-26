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

fn node_slot<'a, T>(node: *const Node<T>, index: usize) -> &'a AtomicPtr<T> {
    unsafe { &(*node).blk[index] }
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

    fn find_node(&self, iters: usize, alloced: &mut Option<*mut Node<T>>) -> *mut Node<T> {
        let mut cur = self.root.load(Ordering::SeqCst);
        if cur.is_null() {
            let node = Node::alloc();

            if self
                .root
                .compare_exchange(cur, node, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                cur = node;
            } else {
                cur = self.root.load(Ordering::SeqCst);
                *alloced = Some(node);
            }
        };

        for _ in 0..iters {
            let next = unsafe { &(*cur).next };

            let next = next.load(Ordering::SeqCst);

            if next.is_null() {
                let new = if let Some(node) = *alloced {
                    *alloced = None;
                    node
                } else {
                    Node::alloc()
                };

                let op = unsafe {
                    &(*cur).next.compare_exchange(
                        std::ptr::null_mut(),
                        new,
                        Ordering::AcqRel,
                        Ordering::Relaxed,
                    )
                };

                if op.is_err() {
                    *alloced = Some(new);
                }

                cur = new
            } else {
                cur = next
            };
        }

        cur
    }

    fn push_end(mut base: *mut Node<T>, new: *mut Node<T>) {
        loop {
            let next = unsafe { (*base).next.load(Ordering::SeqCst) };

            if next.is_null() {
                match unsafe {
                    (*base)
                        .next
                        .compare_exchange(next, new, Ordering::SeqCst, Ordering::Relaxed)
                } {
                    Ok(_) => break,
                    Err(_) => {
                        base = unsafe { (*base).next.load(Ordering::SeqCst) };
                    }
                };
            } else {
                base = next;
            }
        }
    }
}

impl<T> Store<T, Random> {
    pub fn insert(&self, index: usize, elem: T) -> bool {
        let iters = index / BLK_LEN;
        let off = index % BLK_LEN;
        let boxed_elem = Box::into_raw(Box::new(elem));

        let mut alloc = None::<*mut Node<T>>;
        let ptr = self.find_node(iters, &mut alloc);

        let slot_ptr = node_slot(ptr, off);

        let success = slot_ptr
            .compare_exchange(
                std::ptr::null_mut(),
                boxed_elem,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_ok();

        if let Some(node) = alloc {
            Self::push_end(ptr, node);
        }

        success
    }
}

impl<T> Store<T, Append> {
    pub fn push(&self, elem: T) -> usize {
        let slot = self.inserter.0.fetch_add(1, Ordering::AcqRel);

        let boxed_elem = Box::into_raw(Box::new(elem));

        let iters = slot / BLK_LEN;
        let off = slot % BLK_LEN;

        let mut alloc = None::<*mut Node<T>>;
        let ptr = self.find_node(iters, &mut alloc);
        let a = node_slot(ptr, off);

        //let a = unsafe { &*addr_of!((*ptr).blk[off]) };

        a.store(boxed_elem, Ordering::SeqCst);

        if let Some(node) = alloc {
            Self::push_end(ptr, node);
        }

        slot
    }
}

impl<T, I> std::fmt::Debug for Store<T, I>
where
    T: std::fmt::Debug,
    I: Inserter,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut li = f.debug_list();

        let mut cur = self.root.load(Ordering::SeqCst);

        while !cur.is_null() {
            let node = unsafe { cur.read_volatile() };

            for ent in &node.blk {
                let elem = ent.load(Ordering::SeqCst);
                if !elem.is_null() {
                    li.entry(unsafe { &*elem });
                    ent.store(std::ptr::null_mut(), Ordering::SeqCst);
                } else {
                    li.entry(&None::<()>);
                }
            }

            cur = node.next.load(Ordering::SeqCst);
        }

        li.finish()
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

impl<T, I> Drop for Store<T, I>
where
    I: Inserter,
{
    fn drop(&mut self) {
        let mut cur = self.root.load(Ordering::SeqCst);

        while !cur.is_null() {
            let node = unsafe { cur.read_volatile() };
            let base = node.blk.as_ptr();

            for i in 0..BLK_LEN {
                let ent = unsafe { &*base.add(i) };

                let elem = ent.load(Ordering::SeqCst);
                if !elem.is_null() {
                    drop(unsafe { Box::from_raw(elem) });
                    ent.store(std::ptr::null_mut(), Ordering::SeqCst);
                }
            }

            let next = node.next.load(Ordering::SeqCst);

            drop(unsafe { Box::from_raw(cur) });

            cur = next;
        }
    }
}

#[test]
fn node_size() {
    assert_eq!(std::mem::size_of::<Node<()>>(), 256);
}

#[test]
fn test_push() {
    let store = Store::<usize, Append>::default();

    store.push(1);
    store.push(2);
    store.push(3);

    assert_eq!(store.get(0), Some(&1));
    assert_eq!(store.get(1), Some(&2));
    assert_eq!(store.get(2), Some(&3));
}

#[test]
fn test_insert() {
    let store = Store::<usize, Random>::default();

    store.insert(1, 4);
    store.insert(2, 5);
    store.insert(3, 6);

    assert_eq!(store.get(1), Some(&4));
    assert_eq!(store.get(2), Some(&5));
    assert_eq!(store.get(3), Some(&6));
}

#[test]
fn test_multi_block_insert() {
    let store = Store::<usize, Random>::default();

    for i in 0..36 {
        store.insert(i, i);
    }

    assert_eq!(store.get(34), Some(&34));
}
