use mlua::{FromLua, UserData};
use std::sync::Arc;
use tokio::sync::Mutex;

mod err;
mod filter;
mod globals;
mod load;
mod target_ref;

#[cfg(test)]
mod test;

pub use err::ConfError;
pub use target_ref::TargetRef;

use crate::Filter;

#[derive(Default, Clone)]
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

#[derive(FromLua, Debug, Clone)]
pub enum Subst {
    Func(usize),
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

type UProxy = Arc<Mutex<Proxy>>;

#[derive(Default)]
pub struct Config<F: Filter + Sync> {
    proxy: UProxy,
    intercept: F,
}

impl UserData for Rule {}
impl UserData for Attr {}
