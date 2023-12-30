use std::{path::PathBuf, sync::Arc};

use mlua::Lua;
use tokio::sync::Mutex;

use super::{globals, Config, Proxy, Rule};

impl Config {
    pub fn load(path: PathBuf) -> eyre::Result<Self> {
        let content = std::fs::read_to_string(path)?;

        let proxy = Arc::new(Mutex::new(Proxy::default()));

        let lua = Lua::new();
        lua.set_app_data(proxy.clone());
        {
            let globals = lua.globals();

            globals.set("target", lua.create_async_function(globals::target)?)?;
            globals.set("focus", lua.create_async_function(globals::focus)?)?;

            globals.set(
                "set_header",
                lua.create_async_function(globals::set_header)?,
            )?;

            globals.set("dump", lua.create_userdata(Rule::Dump)?)?;
            globals.set("intercept", lua.create_userdata(Rule::Intercept)?)?;
        }

        let chunk = lua.load(content).set_name("atkpx-config");

        chunk.exec()?;

        let lua = Arc::new(Mutex::new(lua));
        Ok(Config { proxy, lua })
    }
}
