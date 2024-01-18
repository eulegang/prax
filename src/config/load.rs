use std::{path::Path, sync::Arc};

use mlua::Lua;
use notify::{recommended_watcher, Event, EventKind, Watcher};
use tokio::sync::Mutex;

use crate::nvim::NVim;

use super::{globals, Config, Proxy, Rule, UProxy};

impl Config {
    pub fn load(path: &Path, nvim: Arc<Option<Mutex<NVim>>>) -> eyre::Result<Self> {
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
        let config = Config { proxy, lua, nvim };
        config.watch(path)?;

        Ok(config)
    }

    pub fn watch(&self, path: &Path) -> eyre::Result<()> {
        let path = path.to_owned();
        let mut watcher = recommended_watcher(move |event| {
            let event: Event = match event {
                Ok(event) => event,
                Err(e) => {
                    log::error!("failed to watch config: {e}");
                    return;
                }
            };

            let mut proxy = self.proxy.lock();
            let mut lua = self.lua.lock();

            tokio::spawn(async move {
                if matches!(event.kind, EventKind::Modify(_)) && event.paths.len() == 1 {
                    log::trace!("config reload running");
                    let path = &event.paths[0];
                    let (new_proxy, new_lua) = match Self::eval_config(path) {
                        Ok(x) => x,

                        Err(e) => {
                            log::error!("error reevaluating config");
                            // should probably send something over nvim comms
                            return;
                        }
                    };

                    let mut proxy = proxy.await;
                    let mut lua = lua.await;

                    *proxy = new_proxy.lock().await.clone();
                    *lua = new_lua;
                }
            });
        })?;

        watcher.watch(&path, notify::RecursiveMode::NonRecursive)?;

        Ok(())
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

            globals.set(
                "set_header",
                lua.create_async_function(globals::set_header)?,
            )?;

            globals.set("dump", lua.create_userdata(Rule::Dump)?)?;
            globals.set("intercept", lua.create_userdata(Rule::Intercept)?)?;
        }

        let chunk = lua.load(content).set_name("atkpx-config");

        chunk.exec()?;

        Ok((proxy, lua))
    }
}
