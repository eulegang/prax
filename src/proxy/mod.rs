mod service;

#[derive(Default, Debug)]
pub struct Proxy {
    pub targets: Vec<Target>,
    pub focus: bool,
}

#[derive(Debug)]
pub struct Target {
    pub hostname: String,
    pub rules: Vec<Rule>,
}

#[derive(Debug)]
pub enum Rule {
    SetHeader(String, String),
}

impl Proxy {
    fn find_target<'a>(&'a self, name: &str) -> Option<&'a Target> {
        for t in &self.targets {
            if t.hostname == name {
                return Some(t);
            }
        }

        None
    }
}
