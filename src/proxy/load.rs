use std::{path::Path, sync::Arc};

use mlua::Lua;
use tokio::sync::Mutex;

use crate::Filter;

use super::{globals, Attr, Config, Proxy, Rule, UProxy};

impl<F> Config<F>
where
    F: Filter + Sync,
{
    pub fn load(path: &Path, intercept: F) -> eyre::Result<Self> {
        let (proxy, _) = Self::eval_config(path)?;

        let config = Config { proxy, intercept };

        Ok(config)
    }

    fn eval_config(path: &Path) -> eyre::Result<(UProxy, Lua)> {
        let content = std::fs::read_to_string(path)?;

        let proxy = Arc::new(Mutex::new(Proxy::default()));

        let lua = Lua::new();
        lua.set_app_data(proxy.clone());
        {
            let globals = lua.globals();

            globals.set("target", lua.create_async_function(globals::target)?)?;
            globals.set("focus", lua.create_async_function(globals::focus)?)?;
            globals.set("header", lua.create_async_function(globals::header)?)?;
            globals.set("query", lua.create_async_function(globals::query)?)?;

            globals.set("set", lua.create_async_function(globals::set)?)?;
            globals.set("sub", lua.create_async_function(globals::sub)?)?;

            globals.set("dump", lua.create_userdata(Rule::Dump)?)?;
            globals.set("intercept", lua.create_userdata(Rule::Intercept)?)?;

            globals.set("method", lua.create_userdata(Attr::Method)?)?;
            globals.set("status", lua.create_userdata(Attr::Status)?)?;
            globals.set("path", lua.create_userdata(Attr::Path)?)?;
            globals.set("body", lua.create_userdata(Attr::Body)?)?;
        }

        let chunk = lua.load(content).set_name("atkpx-config");

        chunk.exec()?;

        Ok((proxy, lua))
    }
}
