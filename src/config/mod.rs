use mlua::Lua;
use std::path::PathBuf;

mod err;
mod globals;
mod rule;
mod target_ref;

pub use err::ConfError;
pub use rule::Rule;
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

    globals.set("dump", lua.create_userdata(Rule::Dump)?)?;

    let chunk = lua.load(content).set_name("atkpx-config");

    chunk.exec()?;

    Ok(())
}
