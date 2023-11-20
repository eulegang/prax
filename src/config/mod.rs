use mlua::Lua;
use std::path::PathBuf;

mod err;
mod globals;
mod rule;
mod target_ref;

pub use err::ConfError;
pub use rule::{Elem, Rule};
pub use target_ref::TargetRef;

pub fn config(path: PathBuf) -> eyre::Result<()> {
    let content = std::fs::read_to_string(path)?;

    let lua = Lua::new();
    let globals = lua.globals();

    globals.set("target", lua.create_async_function(globals::target)?)?;
    globals.set("focus", lua.create_async_function(globals::focus)?)?;

    globals.set(
        "set_header",
        lua.create_async_function(globals::set_header)?,
    )?;
    globals.set("log", lua.create_async_function(globals::log)?)?;

    globals.set("path", lua.create_userdata(Elem::Path)?)?;
    globals.set("body", lua.create_userdata(Elem::Body)?)?;
    globals.set("status", lua.create_userdata(Elem::Status)?)?;
    globals.set("method", lua.create_userdata(Elem::Method)?)?;
    globals.set("dump", lua.create_userdata(Rule::Dump)?)?;
    globals.set("header", lua.create_async_function(globals::header)?)?;
    globals.set("query", lua.create_async_function(globals::query)?)?;

    let chunk = lua.load(content).set_name("atkpx-config");

    chunk.exec()?;

    Ok(())
}
