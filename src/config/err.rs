#[derive(Debug)]
pub enum ConfError {
    InvalidTargetRef(String),
}

impl std::error::Error for ConfError {}

impl std::fmt::Display for ConfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfError::InvalidTargetRef(hostname) => write!(f, "invalid target {}", hostname)?,
        }

        Ok(())
    }
}
