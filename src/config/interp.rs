use std::{
    path::{Path, PathBuf},
    sync::{atomic::AtomicPtr, Arc, Mutex},
};

use mlua::{AppDataRefMut, FromLua, Function, IntoLua, Lua, Value};
use tokio::sync::mpsc;

use crate::config::Target;

use super::{Attr, Proxy, Rule, Subst, TargetRef};

enum Inst {
    Reload,
    CallFunc(usize, Val),
}

enum Val {
    Nil,
    Bool(bool),
    String(String),
}

type ResChan<T> = tokio::sync::oneshot::Sender<T>;

#[derive(Default)]
struct AppData {
    proxy: Proxy,
    funcs: Vec<Function<'static>>,
}

#[derive(Clone)]
pub struct Interp {
    proxy: &'static AtomicPtr<Arc<Proxy>>,
    tx: mpsc::Sender<(ResChan<mlua::Result<Val>>, Inst)>,
}

impl Interp {
    pub fn new(path: PathBuf, proxy: &'static AtomicPtr<Arc<Proxy>>) -> Interp {
        let (tx, mut rx) = mpsc::channel::<(ResChan<mlua::Result<Val>>, Inst)>(1);

        std::thread::spawn(move || {
            let mut lua = match Interp::load(&path) {
                Ok(lua) => Some(lua),
                Err(e) => {
                    log::error!("failed to load config: {e}");
                    None
                }
            };

            if let Some(lua) = &lua {
                let mut data = lua.app_data_mut::<AppData>().unwrap();

                Arc::new(data.proxy.clone());

                let mut default = Proxy::default();

                std::mem::swap(&mut data.proxy, &mut default);

                let mut res = Box::new(Arc::new(default));

                proxy.store(res.as_mut(), std::sync::atomic::Ordering::SeqCst);
            }

            while let Some((tx, inst)) = rx.blocking_recv() {
                match inst {
                    Inst::Reload => {
                        let mut lua = match Interp::load(&path) {
                            Ok(lua) => Some(lua),
                            Err(e) => {
                                log::error!("failed to load config: {e}");
                                None
                            }
                        };

                        if let Some(lua) = &lua {
                            let mut data = lua.app_data_mut::<AppData>().unwrap();

                            Arc::new(data.proxy.clone());

                            let mut default = Proxy::default();

                            std::mem::swap(&mut data.proxy, &mut default);

                            let mut res = Box::new(Arc::new(default));

                            proxy.store(res.as_mut(), std::sync::atomic::Ordering::SeqCst);
                        }
                    }

                    Inst::CallFunc(pos, val) => {
                        if let Some(ref lua) = lua {
                            let data = lua.app_data_ref::<AppData>().unwrap();
                            if let Some(func) = data.funcs.get(pos) {
                                let res = func.call::<_, Val>(val);

                                let _ = tx.send(res);
                            }
                        } else {
                            log::error!("lua interpreter is not ready some how");
                        }
                    }
                }
            }
        });

        Interp { proxy, tx }
    }

    pub fn load(path: &Path) -> eyre::Result<Lua> {
        let content = std::fs::read_to_string(path)?;

        let lua = Lua::new();

        let appdata = Arc::new(Mutex::new(AppData::default()));

        lua.set_app_data(appdata.clone());

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

        let chunk = lua.load(content).set_name("atkpx-config");

        chunk.exec()?;

        Ok(lua)
    }
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

fn sub<'a>(lua: &'a Lua, (attr, value): (Attr, Value<'a>)) -> mlua::Result<Rule> {
    match value {
        Value::Function(func) => {
            let mut data = app_data_mut(lua)?;

            let index = data.funcs.len();
            let func = unsafe {
                // funcs are held just as long as the lua interpreter is
                std::mem::transmute(func)
            };
            data.funcs.push(func);

            Ok(Rule::Subst(attr, Subst::Func(index)))
        }
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
            cause: Arc::new(mlua::Error::RuntimeError(
                "invalid type given to sub".to_string(),
            )),
        }),
    }
}

fn app_data_mut(lua: &Lua) -> mlua::Result<AppDataRefMut<AppData>> {
    lua.app_data_mut()
        .ok_or_else(|| mlua::Error::RuntimeError("app data not set".to_string()))
}

impl<'lua> IntoLua<'lua> for Val {
    fn into_lua(self, lua: &'lua Lua) -> mlua::prelude::LuaResult<Value<'lua>> {
        match self {
            Val::Nil => Ok(Value::Nil),
            Val::Bool(b) => Ok(Value::Boolean(b)),
            Val::String(s) => Ok(Value::String(lua.create_string(s)?)),
        }
    }
}

impl<'lua> FromLua<'lua> for Val {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> mlua::prelude::LuaResult<Self> {
        match value {
            Value::Nil => Ok(Val::Nil),
            Value::Boolean(b) => Ok(Val::Bool(b)),
            Value::String(s) => Ok(Val::String(s.to_str()?.to_string())),
            _ => Err(mlua::Error::RuntimeError(format!(
                "Invalid type to be coorced into Val [{}]",
                value.type_name()
            ))),
        }
    }
}
