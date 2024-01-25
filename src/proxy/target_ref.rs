use std::sync::Arc;

use mlua::{FromLua, Lua, Result, UserData, Variadic};

use super::{ConfError, Rule, UProxy};

#[derive(FromLua, Clone)]
pub struct TargetRef {
    pub hostname: String,
}

impl UserData for TargetRef {
    fn add_fields<'lua, F: mlua::prelude::LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("hostname", |_, this| Ok(this.hostname.clone()))
    }

    fn add_methods<'lua, M: mlua::prelude::LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_async_function("req", target_ref_req);
        methods.add_async_function("resp", target_ref_resp);
    }
}

async fn target_ref_req(
    lua: &Lua,
    (target, rules): (TargetRef, Variadic<Rule>),
) -> Result<TargetRef> {
    let lock = lua.app_data_ref::<UProxy>().unwrap();
    let mut proxy = lock.lock().await;

    let t = proxy
        .targets
        .iter_mut()
        .find(|name| name.hostname == target.hostname)
        .ok_or_else(|| {
            mlua::Error::ExternalError(Arc::new(ConfError::InvalidTargetRef(
                target.hostname.clone(),
            )))
        })?;

    for r in rules {
        t.req.push(r);
    }

    Ok(target)
}

async fn target_ref_resp(
    lua: &Lua,
    (target, rules): (TargetRef, Variadic<Rule>),
) -> Result<TargetRef> {
    let lock = lua.app_data_ref::<UProxy>().unwrap();
    let mut proxy = lock.lock().await;

    let t = proxy
        .targets
        .iter_mut()
        .find(|name| name.hostname == target.hostname)
        .ok_or_else(|| {
            mlua::Error::ExternalError(Arc::new(ConfError::InvalidTargetRef(
                target.hostname.clone(),
            )))
        })?;

    for r in rules {
        t.resp.push(r);
    }

    Ok(target)
}
