use std::path::{Path, PathBuf};

use notify::{RecursiveMode, Watcher};

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

        std::thread::spawn(move || {
            let original_path = path.clone();
            let mut watcher = match notify::recommended_watcher(move |res| {
                if let Err(e) = res {
                    log::error!("{e}");
                    return;
                };

                let i = intercept.clone();
                let t = tx.clone();
                let p = path.clone();

                tokio::spawn(async move {
                    match Config::load(p.as_ref(), i.clone()).await {
                        Ok(config) => {
                            let _ = t.send(config).await;
                        }
                        Err(err) => {
                            log::error!("failed to load config {err}");
                        }
                    }
                });
            }) {
                Ok(w) => w,
                Err(e) => {
                    log::error!("failed to build watcher {e}");
                    return;
                }
            };

            if let Err(e) = watcher.watch(&original_path, RecursiveMode::NonRecursive) {
                log::error!("failed to watch {} {e}", original_path.display());
            };
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
