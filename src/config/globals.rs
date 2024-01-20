use mlua::{Lua, Result, Value};

use super::{Attr, Rule, Subst, Target, TargetRef, UProxy};

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

pub async fn set(_: &Lua, (attr, value): (Attr, String)) -> Result<Rule> {
    Ok(Rule::Set(attr, value))
}

pub async fn sub<'a>(_: &'a Lua, (_, _): (Attr, Value<'a>)) -> Result<Rule> {
    todo!();
}

pub async fn header(_: &Lua, (key,): (String,)) -> Result<Attr> {
    Ok(Attr::Header(key))
}

pub async fn query(_: &Lua, (key,): (String,)) -> Result<Attr> {
    Ok(Attr::Query(key))
}
