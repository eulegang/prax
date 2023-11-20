use std::sync::Arc;

use mlua::{FromLua, Lua, Result, UserData, Variadic};

use crate::PROXY;

use super::{ConfError, Elem, Rule};

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
    _: &Lua,
    (target, rules): (TargetRef, Variadic<Rule>),
) -> Result<TargetRef> {
    let mut proxy = PROXY.write().await;

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
        if matches!(r, Rule::Log(Elem::Status)) {
            log::warn!("status is a response oriented item")
        }

        t.req.push(r.into());
    }

    Ok(target)
}

async fn target_ref_resp(
    _: &Lua,
    (target, rules): (TargetRef, Variadic<Rule>),
) -> Result<TargetRef> {
    let mut proxy = PROXY.write().await;

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
        if matches!(r, Rule::Log(Elem::Method)) {
            log::warn!("method is a response oriented item")
        }

        if matches!(r, Rule::Log(Elem::Path)) {
            log::warn!("path is a response oriented item")
        }

        if matches!(r, Rule::Log(Elem::Query(_))) {
            log::warn!("query is a response oriented item")
        }

        t.resp.push(r.into());
    }

    Ok(target)
}
