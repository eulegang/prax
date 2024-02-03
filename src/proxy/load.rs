use std::{
    path::{Path, PathBuf},
    thread::sleep,
    time::Duration,
};

use notify::{Event, EventKind, RecursiveMode, Watcher};

use crate::{
    notify::linux::{INotify, Mask},
    Filter,
};

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
                    Mask::IN_CREATE | Mask::IN_MODIFY | Mask::IN_CLOSE_WRITE | Mask::IN_DELETE_SELF,
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

                if event.mask.contains(Mask::IN_IGNORED) {
                    notify
                        .add(
                            &watch,
                            Mask::IN_CREATE
                                | Mask::IN_MODIFY
                                | Mask::IN_CLOSE_WRITE
                                | Mask::IN_DELETE_SELF,
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

        /*
        tokio::spawn(async move {
            let i = intercept.clone();
            while let Some(path) = path_rx.recv().await {
                tokio::time::sleep(Duration::from_millis(100)).await;
                log::info!("found path loading config");
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

        std::thread::spawn(move || {
            log::debug!("spawning watcher thread");
            let original_path = path.clone();
            let mut watcher = match notify::recommended_watcher(move |res| {
                log::debug!("watcher invoked");
                let event: Event = match res {
                    Ok(e) => e,
                    Err(e) => {
                        log::error!("{e}");
                        return;
                    }
                };

                log::debug!("kind: {:?}", event.kind);
                if !matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_)) {
                    return;
                }

                let _ = path_tx.blocking_send(path.to_owned());
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

            loop {
                sleep(Duration::from_secs(1));
            }

            log::debug!("ended watcher thread");
            drop(watcher);
        });
        */

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
