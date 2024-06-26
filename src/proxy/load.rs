use std::path::{Path, PathBuf};

use tokinotify::{INotify, Mask};

use crate::Filter;

use super::{interp::Interp, Config};

impl<F> Config<F>
where
    F: Filter + Send + Sync + Clone + 'static,
{
    pub async fn load(path: &Path, intercept: F) -> eyre::Result<Self> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        let interp = Interp::new(path, tx);

        let proxy = rx.await??;

        let config = Config {
            proxy,
            intercept,
            interp,
        };

        Ok(config)
    }

    pub fn watch(&self, path: PathBuf) -> tokio::sync::mpsc::Receiver<Self> {
        let (tx, rx) = tokio::sync::mpsc::channel(1);

        let intercept = self.intercept.clone();

        let watch = path.clone();
        tokio::spawn(async move {
            let interest = Mask::CREATE | Mask::MODIFY | Mask::CLOSE_WRITE | Mask::DELETE_SELF;

            let mut notify = match INotify::new() {
                Ok(i) => i,
                Err(e) => {
                    tracing::error!("failed to start inotify: {e}");
                    return;
                }
            };

            let i = intercept.clone();

            if let Err(e) = notify.add(&watch, interest) {
                tracing::error!("failed to start watch: {e}");
                return;
            }

            loop {
                let event = match notify.watch().await {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::error!("notify error: {e}");
                        continue;
                    }
                };

                tracing::debug!("event = {event:?}");

                if event.mask.contains(Mask::IGNORED) {
                    if let Err(e) = notify.add(&watch, interest) {
                        tracing::error!("failed to readd watch: {e}");
                    }
                    continue;
                }

                match Config::load(&path, i.clone()).await {
                    Ok(config) => {
                        if tx.send(config).await.is_err() {
                            tracing::error!("failed to send config");
                        }
                    }
                    Err(err) => {
                        tracing::error!("failed to load config {err}");
                    }
                }
            }
        });

        rx
    }
}

impl<F> Config<F>
where
    F: Filter + Clone + Send + Sync + 'static,
{
    pub async fn test(content: &'static str, intercept: F) -> eyre::Result<Self> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        let interp = Interp::test(content, tx);

        let proxy = rx.await??;

        let config = Config {
            proxy,
            intercept,
            interp,
        };

        Ok(config)
    }
}
