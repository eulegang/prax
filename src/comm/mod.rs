use std::sync::Arc;

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tokio::io::{self, stdout, AsyncWriteExt, BufReader};
use tokio::io::{AsyncReadExt, Stdout};
use tokio::sync::Mutex;

use crate::hist::{Request, Response};
use crate::HIST;

#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "type")]
pub enum Req {
    #[serde(rename = "hello")]
    Hello { message: String },

    #[serde(rename = "show_detail")]
    ShowDetail { id: usize },
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "type")]
pub enum Res {
    MissingRecord {},
    ShowDetail {
        req: Box<Request>,
        res: Option<Box<Response>>,
    },
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "type")]
pub enum Note {
    #[serde(rename = "request_declare")]
    ReqDecl { id: usize, path: String },

    #[serde(rename = "response_declare")]
    ResDecl { id: usize, status: u16 },
}

static STDOUT: Lazy<Arc<Mutex<Stdout>>> = Lazy::new(|| Arc::new(Mutex::new(stdout())));

pub async fn main() -> eyre::Result<()> {
    let mut stdin = BufReader::new(io::stdin());
    let mut buf = Vec::<u8>::with_capacity(4096);

    loop {
        let mut b = [0u8; 4096];
        let size = stdin.read(&mut b).await?;

        buf.extend(&b[0..size]);

        let req: Req = match serde_json::from_slice(&buf) {
            Ok(msg) => {
                buf.clear();

                msg
            }
            Err(e) => {
                log::error!("error found: {}", e);
                continue;
            }
        };

        match req {
            Req::Hello { message } => log::info!("hello: {message}"),
            Req::ShowDetail { id } => {
                let hist = HIST.read().await;
                let entry = hist.get(id);

                if let Some(entry) = entry {
                    send_res(Res::ShowDetail {
                        req: Box::new(entry.request.clone()),
                        res: entry.response.as_ref().map(|r| Box::new(r.clone())),
                    })
                    .await?;
                } else {
                    send_res(Res::MissingRecord {}).await?;
                }
            }
        }
    }

    //Ok(())
}

async fn send_res(res: Res) -> eyre::Result<()> {
    let buf = serde_json::to_vec(&res)?;

    let mut stdout = STDOUT.lock().await;

    stdout.write_all(&buf).await?;
    stdout.write_all(&[b'\n']).await?;
    stdout.flush().await?;

    Ok(())
}

pub async fn send_note(note: Note) -> eyre::Result<()> {
    log::info!("sending note: {:?}", note);

    let buf = serde_json::to_vec(&note)?;

    let mut stdout = STDOUT.lock().await;

    stdout.write_all(&buf).await?;
    stdout.write_all(&[b'\n']).await?;
    stdout.flush().await?;

    Ok(())
}
