pub mod service;

#[derive(Default, Debug)]
pub struct Proxy {
    pub targets: Vec<Target>,
    pub focus: bool,
}

#[derive(Debug)]
pub struct Target {
    pub hostname: String,
    pub req: Vec<Rule>,
    pub resp: Vec<Rule>,
}

#[derive(Debug, Clone)]
pub enum Rule {
    SetHeader(String, String),
    Dump,
}

impl Proxy {
    pub fn find_target<'a>(&'a self, name: &str) -> Option<&'a Target> {
        self.targets.iter().find(|t| t.hostname == name)
    }
}
