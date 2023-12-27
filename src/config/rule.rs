use mlua::{FromLua, UserData};

use crate::proxy::Rule as PRule;

#[derive(FromLua, Clone)]
pub enum Rule {
    SetHeader(String, String),
    Dump,
    Intercept,
}

impl From<Rule> for PRule {
    fn from(value: Rule) -> Self {
        match value {
            Rule::SetHeader(k, v) => PRule::SetHeader(k, v),
            Rule::Dump => PRule::Dump,
            Rule::Intercept => PRule::Intercept,
        }
    }
}

impl UserData for Rule {
    fn add_fields<'lua, F: mlua::prelude::LuaUserDataFields<'lua, Self>>(_: &mut F) {}
    fn add_methods<'lua, M: mlua::prelude::LuaUserDataMethods<'lua, Self>>(_: &mut M) {}
}
