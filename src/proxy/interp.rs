use std::{path::Path, sync::Arc};

use mlua::{AppDataRefMut, FromLua, Function, IntoLua, Lua};
use tokio::sync::mpsc::Sender;

use crate::proxy::Target;

use super::{Attr, Func, Proxy, Rule, Subst, TargetRef};

type Return = Val;
type Input = Val;

enum Val {
    Nil,
    Bool(bool),
    String(String),
}

type Chan<T> = tokio::sync::oneshot::Sender<T>;

struct Invocation {
    chan: Chan<mlua::Result<Return>>,
    selector: usize,
    input: Input,
}

#[derive(Clone)]
pub struct Interp {
    sender: Sender<Invocation>,
}

#[derive(Default)]
struct AppData {
    proxy: Proxy,
    funcs: Vec<Function<'static>>,
}

impl Interp {
    pub fn new(path: &Path, proxy: tokio::sync::oneshot::Sender<mlua::Result<Proxy>>) -> Self {
        let (sender, mut rx) = tokio::sync::mpsc::channel::<Invocation>(1);

        let path = path.to_path_buf();
        std::thread::spawn(move || {
            let lua = match Self::load(&path) {
                Ok(l) => l,

                Err(e) => {
                    let _ = proxy.send(Err(e));
                    return;
                }
            };

            let mut swap = Proxy::default();
            let mut funcs: Vec<Function<'static>> = Vec::new();
            match app_data_mut(&lua) {
                Ok(mut appdata) => {
                    std::mem::swap(&mut swap, &mut appdata.proxy);
                    std::mem::swap(&mut funcs, &mut appdata.funcs);

                    let _ = proxy.send(Ok(swap));
                }
                Err(e) => {
                    let _ = proxy.send(Err(e));
                    return;
                }
            }

            while let Some(s) = rx.blocking_recv() {
                let Some(func) = funcs.get(s.selector) else {
                    log::error!("invalid func dereferenced");
                    continue;
                };

                let r: Val = match func.call(s.input) {
                    Ok(r) => r,
                    Err(e) => {
                        s.chan.send(Err(e));
                        continue;
                    }
                };

                if let Err(_) = s.chan.send(Ok(r)) {
                    log::error!("error sending value back");
                    continue;
                }
            }
        });

        Interp { sender }
    }

    fn load(path: &Path) -> mlua::Result<Lua> {
        let lua = Lua::new();

        let appdata = AppData::default();

        lua.set_app_data(appdata);

        {
            let globals = lua.globals();

            globals.set("target", lua.create_function(target)?)?;
            globals.set("focus", lua.create_function(focus)?)?;
            globals.set("header", lua.create_function(header)?)?;
            globals.set("query", lua.create_function(query)?)?;

            globals.set("set", lua.create_function(set)?)?;
            globals.set("sub", lua.create_function(sub)?)?;

            globals.set("dump", lua.create_userdata(Rule::Dump)?)?;
            globals.set("intercept", lua.create_userdata(Rule::Intercept)?)?;

            globals.set("method", lua.create_userdata(Attr::Method)?)?;
            globals.set("status", lua.create_userdata(Attr::Status)?)?;
            globals.set("path", lua.create_userdata(Attr::Path)?)?;
            globals.set("body", lua.create_userdata(Attr::Body)?)?;
        }

        Ok(lua)
    }

    pub async fn invoke(&self, func: Func, arg: Val) -> mlua::Result<Val> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        let invok = Invocation {
            chan: tx,
            selector: func,
            input: arg,
        };

        if let Err(e) = self.sender.send(invok).await {
            return Err(mlua::Error::RuntimeError(format!(
                "lua thread exited {}",
                e
            )));
        }

        rx.await
            .map_err(|_| mlua::Error::RuntimeError("failed to receive ".to_string()))
    }
}

fn app_data_mut(lua: &Lua) -> mlua::Result<AppDataRefMut<AppData>> {
    lua.app_data_mut()
        .ok_or_else(|| mlua::Error::RuntimeError("app data not set".to_string()))
}

fn header(_: &Lua, (key,): (String,)) -> mlua::Result<Attr> {
    Ok(Attr::Header(key))
}

fn query(_: &Lua, (key,): (String,)) -> mlua::Result<Attr> {
    Ok(Attr::Query(key))
}

fn system(_: &Lua, (cmd,): (String,)) -> mlua::Result<Subst> {
    Ok(Subst::System(cmd))
}

fn focus(lua: &Lua, (): ()) -> mlua::Result<()> {
    let mut data = app_data_mut(lua)?;
    data.proxy.focus = true;
    Ok(())
}

fn target(lua: &Lua, (hostname,): (String,)) -> mlua::Result<TargetRef> {
    let mut data = app_data_mut(lua)?;
    log::info!("Targeting {}", &hostname);

    let r = TargetRef {
        hostname: hostname.clone(),
    };

    data.proxy.targets.push(Target {
        hostname,
        req: vec![],
        resp: vec![],
    });

    Ok(r)
}

fn set(_: &Lua, (attr, value): (Attr, String)) -> mlua::Result<Rule> {
    Ok(Rule::Set(attr, value))
}

fn sub<'a>(lua: &'a Lua, (attr, value): (Attr, mlua::Value<'a>)) -> mlua::Result<Rule> {
    match value {
        mlua::Value::Function(func) => {
            let mut data = app_data_mut(lua)?;

            let index = data.funcs.len();
            let func = unsafe {
                // funcs are held just as long as the lua interpreter is
                std::mem::transmute(func)
            };
            data.funcs.push(func);

            Ok(Rule::Subst(attr, Subst::Func(index)))
        }
        mlua::Value::UserData(data) => {
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
            cause: Arc::new(mlua::Error::RuntimeError(
                "invalid type given to sub".to_string(),
            )),
        }),
    }
}

impl<'lua> IntoLua<'lua> for Val {
    fn into_lua(self, lua: &'lua Lua) -> mlua::prelude::LuaResult<mlua::Value<'lua>> {
        match self {
            Val::Nil => Ok(mlua::Value::Nil),
            Val::Bool(b) => Ok(mlua::Value::Boolean(b)),
            Val::String(s) => Ok(mlua::Value::String(lua.create_string(s)?)),
        }
    }
}

impl<'lua> FromLua<'lua> for Val {
    fn from_lua(value: mlua::Value<'lua>, _: &'lua Lua) -> mlua::prelude::LuaResult<Self> {
        match value {
            mlua::Value::Nil => Ok(Val::Nil),
            mlua::Value::Boolean(b) => Ok(Val::Bool(b)),
            mlua::Value::String(s) => Ok(Val::String(s.to_str()?.to_string())),
            _ => Err(mlua::Error::RuntimeError(format!(
                "Invalid type to be coorced into Val [{}]",
                value.type_name()
            ))),
        }
    }
}
