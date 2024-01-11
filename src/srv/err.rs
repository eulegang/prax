pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("no host in request")]
    NoHost,

    #[error("io error \"{0}\"")]
    IO(#[from] tokio::io::Error),

    #[error("hyper error \"{0}\"")]
    Hyper(#[from] hyper::Error),

    #[error("hyper error \"{0}\"")]
    HttpHyper(#[from] hyper::http::Error),

    #[error("failed to call nvim funciton \"{0}\"")]
    Nvim(#[from] Box<nvim_rs::error::CallError>),

    #[error("Failed to recieve channel item error")]
    TokioRecv(#[from] tokio::sync::oneshot::error::RecvError),

    #[error("Intercept does not conform to format")]
    InterceptMalformed,

    #[error("Invalid status code")]
    StatusCode(#[from] hyper::http::status::InvalidStatusCode),

    #[error("Invalid invalid header name")]
    HeaderName(#[from] hyper::header::InvalidHeaderName),

    #[error("Invalid invalid header value")]
    HeaderValue(#[from] hyper::header::InvalidHeaderValue),

    #[error("Invalid invalid method")]
    Method(#[from] hyper::http::method::InvalidMethod),

    #[error("Failed to parse utf-8")]
    Body(#[from] std::str::Utf8Error),

    #[error("Failed to marshal header")]
    HeaderMarshal(#[from] hyper::header::ToStrError),

    #[error("No tls configuration when upgrading")]
    NoTlsConfig,
}
