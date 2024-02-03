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
            let mut notify = INotify::new().unwrap();
            let i = intercept.clone();

            notify
                .add(
                    &watch,
                    Mask::CREATE | Mask::MODIFY | Mask::CLOSE_WRITE | Mask::DELETE_SELF,
                )
                .unwrap();

            loop {
                let event = match notify.watch().await {
                    Ok(e) => e,
                    Err(e) => {
                        log::error!("notify error: {e}");
                        continue;
                    }
                };

                log::debug!("event = {event:?}");

                if event.mask.contains(Mask::IGNORED) {
                    notify
                        .add(
                            &watch,
                            Mask::CREATE | Mask::MODIFY | Mask::CLOSE_WRITE | Mask::DELETE_SELF,
                        )
                        .unwrap();
                    continue;
                }

                match Config::load(&path, i.clone()).await {
                    Ok(config) => {
                        let _ = tx.send(config).await;
                    }
                    Err(err) => {
                        log::error!("failed to load config {err}");
                    }
                }
            }
        });

        rx
    }
}

impl Config<()> {
    #[cfg(test)]
    pub async fn test(content: &'static str) -> eyre::Result<Self> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        let interp = Interp::test(content, tx);

        let proxy = rx.await??;

        let config = Config {
            proxy,
            intercept: (),
            interp,
        };

        Ok(config)
    }
}
