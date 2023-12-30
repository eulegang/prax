use mlua::{Lua, Result};

use super::{Rule, Target, TargetRef, UProxy};

pub async fn target(lua: &Lua, (hostname,): (String,)) -> Result<TargetRef> {
    let lock = lua.app_data_ref::<UProxy>().unwrap();
    let mut proxy = lock.lock().await;
    log::info!("Targeting {}", &hostname);

    let r = TargetRef {
        hostname: hostname.clone(),
    };

    proxy.targets.push(Target {
        hostname,
        req: vec![],
        resp: vec![],
    });

    Ok(r)
}

pub async fn focus(lua: &Lua, (): ()) -> Result<()> {
    let lock = lua.app_data_ref::<UProxy>().unwrap();
    let mut proxy = lock.lock().await;
    proxy.focus = true;
    Ok(())
}

pub async fn set_header(_: &Lua, (key, value): (String, String)) -> Result<Rule> {
    Ok(Rule::SetHeader(key, value))
}
