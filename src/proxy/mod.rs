use mlua::{FromLua, UserData};

mod err;
mod filter;
mod load;
mod query;

mod interp;

#[cfg(test)]
mod test;

pub use err::ConfError;

use crate::Filter;

use self::interp::Interp;

#[derive(Default, Clone, Debug)]
pub struct Proxy {
    pub targets: Vec<Target>,
    pub focus: bool,
}

#[derive(FromLua, Debug, Clone)]
pub struct Target {
    pub hostname: String,
    pub req: Vec<Rule>,
    pub resp: Vec<Rule>,
}

#[derive(FromLua, Debug, Clone)]
pub enum Rule {
    Intercept,
    Dump,
    Set(Attr, String),
    Subst(Attr, Subst),
}

pub type Func = usize;

#[derive(FromLua, Debug, Clone)]
pub enum Subst {
    Func(Func),
    System(String),
}

#[derive(FromLua, Debug, Clone)]
pub enum Attr {
    Method,
    Status,
    Path,
    Query(String),
    Header(String),
    Body,
}

#[derive(Default)]
pub struct Config<F: Filter + Sync> {
    proxy: Proxy,
    interp: Interp,
    intercept: F,
}

impl UserData for Rule {}
impl UserData for Attr {}
