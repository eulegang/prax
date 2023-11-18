use std::{path::PathBuf, sync::Arc};

use mlua::Lua;
use tokio::{runtime::Runtime, sync::RwLock};

use crate::proxy::{Proxy, Target};

pub fn config(path: PathBuf, rt: Arc<Runtime>, proxy: Arc<RwLock<Proxy>>) -> eyre::Result<()> {
    let content = std::fs::read_to_string(path)?;

    let lua = Lua::new();
    let proxy = proxy.clone();

    let globals = lua.globals();

    let target_proxy = proxy.clone();
    let xrt = rt.clone();
    let target = lua.create_function_mut(move |_, (hostname,): (String,)| {
        xrt.block_on(async {
            let mut proxy = target_proxy.write().await;

            log::info!("Targeting {}", &hostname);
            proxy.targets.push(Target {
                hostname,
                rules: vec![],
            });

            Ok(())
        })
    })?;

    let focus_proxy = proxy.clone();

    let focus = lua.create_function_mut(move |_, (): ()| {
        rt.block_on(async {
            let mut p = focus_proxy.write().await;
            p.focus = true;
            Ok(())
        })
    })?;

    globals.set("target", target)?;
    globals.set("focus", focus)?;

    let chunk = lua.load(content).set_name("atkpx-config");

    chunk.exec()?;

    Ok(())
}
