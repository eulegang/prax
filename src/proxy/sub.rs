use std::process::Stdio;

use mlua::FromLua;
use tokio::io::AsyncWriteExt;

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

    #[error("failed to run command: {0}")]
    SystemFailure(i32),

    #[error("system output not utf8: {0}")]
    NonUTFOutput(#[from] std::string::FromUtf8Error),

    #[error("lua invoked and expected a string but got {0}")]
    TypeMismatch(Val),
}

impl Subst {
    pub async fn sub_num(&self, interp: &Interp, num: i64) -> Result<i64, SubstError> {
        match self {
            Subst::Func(slot) => {
                let res = interp.invoke(*slot, num.into()).await?;

                match res {
                    Val::Number(n) => Ok(n),
                    _ => Err(SubstError::TypeMismatch(res)),
                }
            }

            Subst::System(_) => todo!("implement running system command"),
        }
    }

    pub async fn subst(&self, interp: &Interp, content: String) -> Result<String, SubstError> {
        match self {
            Subst::Func(slot) => {
                let res = interp.invoke(*slot, content.into()).await?;

                match res {
                    Val::String(s) => Ok(s),
                    _ => Err(SubstError::TypeMismatch(res)),
                }
            }

            Subst::System(sh) => {
                let mut proc = tokio::process::Command::new("/bin/sh")
                    .arg("-c")
                    .arg(sh)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()?;

                proc.stdin
                    .take()
                    .unwrap()
                    .write_all(content.as_bytes())
                    .await?;
                let out = proc.wait_with_output().await?;

                if !out.status.success() {
                    return Err(SubstError::SystemFailure(out.status.code().unwrap_or(-1)));
                }

                let mut out = String::from_utf8(out.stdout)?;

                if out.ends_with('\n') {
                    out.pop();
                }

                Ok(out)
            }
        }
    }
}
