use std::sync::Arc;

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

pub async fn set_header(_: &Lua, (key, value): (String, String)) -> Result<Rule> {
    Ok(Rule::SetHeader(key, value))
}

pub async fn set(_: &Lua, (attr, value): (Attr, String)) -> Result<Rule> {
    Ok(Rule::Set(attr, value))
}

pub async fn sub<'a>(_: &'a Lua, (attr, value): (Attr, Value<'a>)) -> Result<Rule> {
    todo!();
    /*
    match value {
        Value::Function(func) => {}
        Value::UserData(data) => {
            if data.is::<Subst>() {
                Ok(Rule::Subst(attr, data.user_value()?))
            } else {
                Err(mlua::Error::UserDataTypeMismatch)
            }
        }

        _ => Err(mlua::Error::BadArgument {
            to: Some("sub".to_owned()),
            pos: 1,
            name: Some("strategy".to_owned()),
            cause: Arc::new(mlua::Error::BindError),
        }),
    }
    */
}

pub async fn header(_: &Lua, (key,): (String,)) -> Result<Attr> {
    Ok(Attr::Header(key))
}

pub async fn query(_: &Lua, (key,): (String,)) -> Result<Attr> {
    Ok(Attr::Query(key))
}

pub async fn system(_: &Lua, (cmd,): (String,)) -> Result<Subst> {
    Ok(Subst::System(cmd))
}
