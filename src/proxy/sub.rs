use mlua::FromLua;

use super::interp::{Interp, Val};

pub type Func = usize;

#[derive(FromLua, Debug, Clone)]
pub enum Subst {
    Func(Func),
    System(String),
}

#[derive(thiserror::Error, Debug)]
pub enum SubstError {
    #[error("lua subst error {0}")]
    Lua(#[from] mlua::Error),

    #[error("lua subst error {0}")]
    Io(#[from] tokio::io::Error),

    #[error("lua invoked and expected a string but got {0}")]
    TypeMismatch(Val),
}

impl Subst {
    pub async fn subst(&self, interp: &Interp, content: String) -> Result<String, SubstError> {
        match self {
            Subst::Func(slot) => {
                let res = interp.invoke(*slot, content.into()).await?;

                match res {
                    Val::String(s) => Ok(s),
                    Val::Nil => Err(SubstError::TypeMismatch(res)),
                    Val::Bool(_) => Err(SubstError::TypeMismatch(res)),
                }
            }

            Subst::System(_) => todo!("implement running system command"),
        }
    }
}
