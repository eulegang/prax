use std::path::Path;

use crate::Filter;

use super::{interp::Interp, Config};

impl<F> Config<F>
where
    F: Filter + Sync,
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
}
