#[derive(Default, Debug)]
pub struct DimTracker {
    width: usize,
    lines: Vec<String>,
}

impl DimTracker {
    pub fn push(&mut self, line: String) {
        self.width = self.width.max(line.len());
        self.lines.push(line);
    }

    pub fn blank(&mut self) {
        self.lines.push(String::new());
    }

    pub fn take(self) -> (usize, usize, Vec<String>) {
        (self.width, self.lines.len(), self.lines)
    }
}
