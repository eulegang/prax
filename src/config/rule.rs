use mlua::{FromLua, UserData};

use crate::proxy::{Elem as PElem, Rule as PRule};

#[derive(FromLua, Debug, Clone)]
pub enum Elem {
    Path,
    Body,
    Status,
    Method,
    Header(String),
    Query(String),
}

#[derive(FromLua, Clone)]
pub enum Rule {
    SetHeader(String, String),
    Log(Elem),
}

impl From<Rule> for PRule {
    fn from(value: Rule) -> Self {
        match value {
            Rule::SetHeader(k, v) => PRule::SetHeader(k, v),
            Rule::Log(elem) => PRule::Log(PElem::from(elem)),
        }
    }
}

impl From<Elem> for PElem {
    fn from(value: Elem) -> Self {
        match value {
            Elem::Path => PElem::Path,
            Elem::Body => PElem::Body,
            Elem::Status => PElem::Status,
            Elem::Method => PElem::Method,
            Elem::Header(h) => PElem::Header(h),
            Elem::Query(q) => PElem::Query(q),
        }
    }
}

impl UserData for Rule {
    fn add_fields<'lua, F: mlua::prelude::LuaUserDataFields<'lua, Self>>(_: &mut F) {}
    fn add_methods<'lua, M: mlua::prelude::LuaUserDataMethods<'lua, Self>>(_: &mut M) {}
}

impl UserData for Elem {
    fn add_fields<'lua, F: mlua::prelude::LuaUserDataFields<'lua, Self>>(_: &mut F) {}
    fn add_methods<'lua, M: mlua::prelude::LuaUserDataMethods<'lua, Self>>(_: &mut M) {}
}
