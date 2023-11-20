use mlua::{Lua, Result};

use crate::{proxy::Target, PROXY};

use super::{Elem, Rule, TargetRef};

pub async fn target(_: &Lua, (hostname,): (String,)) -> Result<TargetRef> {
    let mut proxy = PROXY.write().await;
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

pub async fn focus(_: &Lua, (): ()) -> Result<()> {
    let mut proxy = PROXY.write().await;
    proxy.focus = true;
    Ok(())
}

pub async fn set_header(_: &Lua, (key, value): (String, String)) -> Result<Rule> {
    Ok(Rule::SetHeader(key, value))
}

pub async fn log(_: &Lua, elem: Elem) -> Result<Rule> {
    Ok(Rule::Log(elem))
}

pub async fn header(_: &Lua, name: String) -> Result<Elem> {
    Ok(Elem::Header(name))
}

pub async fn query(_: &Lua, name: String) -> Result<Elem> {
    Ok(Elem::Query(name))
}
