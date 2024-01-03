use std::os::fd::FromRawFd;

use nvim_rs::{compat::tokio::Compat, error::LoopError, Neovim};
use parity_tokio_ipc::{Connection, Endpoint};
use pin_project_lite::pin_project;
use tokio::{
    fs::File,
    io::{split, WriteHalf},
    spawn,
    task::JoinHandle,
};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use crate::cli::NvimConnInfo;

pin_project! {
    #[project = IoConnProj]
    pub enum IoConn {
        Std {
            #[pin]
            stdout: Compat<tokio::fs::File>
        },

        Unix {
            #[pin]
            unix: Compat<WriteHalf<Connection>>,
        }
    }
}

impl IoConn {
    pub async fn connect<Handler>(
        info: &NvimConnInfo,
        handler: Handler,
    ) -> std::io::Result<(Neovim<Self>, JoinHandle<Result<(), Box<LoopError>>>)>
    where
        Handler: nvim_rs::Handler<Writer = Self> + Send + 'static,
    {
        match info {
            NvimConnInfo::Stdin => {
                let stdin = unsafe { File::from_raw_fd(0) };
                let stdout = unsafe { File::from_raw_fd(1) };

                let (neovim, io) = Neovim::<Self>::new(
                    stdin.compat(),
                    IoConn::Std {
                        stdout: stdout.compat_write(),
                    },
                    handler,
                );
                let io_handle = spawn(io);

                Ok((neovim, io_handle))
            }
            NvimConnInfo::Unix(path) => {
                let stream = Endpoint::connect(path).await?;
                let (reader, writer) = split(stream);
                let (neovim, io) = Neovim::<Self>::new(
                    reader.compat(),
                    IoConn::Unix {
                        unix: writer.compat_write(),
                    },
                    handler,
                );
                let io_handle = spawn(io);

                Ok((neovim, io_handle))
            }
        }
    }
}

#[async_trait::async_trait]
impl futures::AsyncWrite for IoConn {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        match self.project() {
            IoConnProj::Std { stdout } => stdout.poll_write(cx, buf),
            IoConnProj::Unix { unix } => unix.poll_write(cx, buf),
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.project() {
            IoConnProj::Std { stdout } => stdout.poll_flush(cx),
            IoConnProj::Unix { unix } => unix.poll_flush(cx),
        }
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.project() {
            IoConnProj::Std { stdout } => stdout.poll_close(cx),
            IoConnProj::Unix { unix } => unix.poll_close(cx),
        }
    }
}
