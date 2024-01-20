pub trait Report {
    fn info(&self, msg: String);
    fn error(&self, msg: String);
}
