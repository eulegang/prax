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

#[derive(Default)]
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
}

type UProxy = Arc<Mutex<Proxy>>;

#[derive(Default)]
pub struct Config {
    proxy: UProxy,
    lua: Arc<Mutex<Lua>>,
    nvim: Arc<Option<Mutex<NVim>>>,
}

// TODO: remove
// I probably should not have done this,
// but I'm not quite sure if this is nessecary
// and I'm not sure that wrapping fields in Arc allows Sync
unsafe impl Sync for Config {}
unsafe impl Send for Config {}

impl UserData for Rule {}
