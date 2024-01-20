use mlua::{FromLua, Lua, UserData};
use std::sync::Arc;
use tokio::sync::Mutex;

mod err;
mod globals;
mod load;
mod target_ref;

mod filter;

pub use err::ConfError;
pub use target_ref::TargetRef;

use crate::nvim::NVim;

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
    SetHeader(String, String),
    Intercept,
    Dump,
    Set(Attr, String),
    //Subst(Attr, Subst),
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
pub struct Config {
    proxy: UProxy,
    nvim: Arc<Option<Mutex<NVim>>>,

    // May need in future and need to ensure lua interp stays around
    #[allow(dead_code)]
    lua: Arc<Mutex<Lua>>,
}

impl UserData for Rule {}
impl UserData for Attr {}
