pub struct Windower<I, F> {
    width: usize,
    height: usize,
    iter: I,
    handle: F,
}

impl<I, F> Iterator for Windower<I, F>
where
    I: Iterator,
    F: Fn(&I::Item) -> usize,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let sub = self.iter.next()?;
        self.height += 1;
        self.width = self.width.max((self.handle)(&sub));

        Some(sub)
    }
}

impl<I, F> Windower<I, F> {
    pub fn take(self) -> (usize, usize) {
        (self.width, self.height)
    }

    pub fn init(iter: I, handle: F) -> Self {
        Windower {
            width: 0,
            height: 0,
            iter,
            handle,
        }
    }
}
