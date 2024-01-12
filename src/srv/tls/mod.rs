use std::{fs::File, io::BufReader, path::Path, sync::Arc};

use rustls::{
    pki_types::{CertificateDer, PrivateKeyDer},
    ClientConfig, RootCertStore, ServerConfig,
};
use rustls_pemfile::Item;

use crate::cli::CertOpts;

#[derive(Clone)]
pub struct Tls {
    pub client: Arc<ClientConfig>,
    pub server: Arc<ServerConfig>,
}

#[derive(Debug, thiserror::Error)]
pub enum TlsLoadError {
    #[error("failed to load key: {0}")]
    Key(LoadError),
    #[error("failed to load cert: {0}")]
    Cert(LoadError),

    #[error("failed to construct context: {0}")]
    Tls(#[from] rustls::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("no cryptographical content")]
    NoContent,

    #[error("content mismatch")]
    ContentMismatch,

    #[error("io error {0}")]
    IO(#[from] std::io::Error),
}

impl Tls {
    pub fn load(opts: CertOpts) -> Result<Option<Self>, TlsLoadError> {
        let Some(key) = opts.key else { return Ok(None) };
        let Some(cert) = opts.cert else {
            return Ok(None);
        };

        let root_store = RootCertStore {
            roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
        };

        let key = load_key(&key).map_err(TlsLoadError::Key)?;
        let certs = load_certs(&cert).map_err(TlsLoadError::Cert)?;

        let client = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        let server = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)?;

        let client = Arc::new(client);
        let server = Arc::new(server);

        Ok(Some(Tls { client, server }))
    }
}

fn load_key(path: &Path) -> Result<PrivateKeyDer<'static>, LoadError> {
    let file = File::open(path)?;

    let mut buf = BufReader::new(file);

    let Some(item) = rustls_pemfile::read_one(&mut buf)? else {
        return Err(LoadError::NoContent);
    };

    match item {
        Item::Pkcs1Key(key) => Ok(PrivateKeyDer::Pkcs1(key)),
        Item::Pkcs8Key(key) => Ok(PrivateKeyDer::Pkcs8(key)),
        Item::Sec1Key(key) => Ok(PrivateKeyDer::Sec1(key)),
        _ => Err(LoadError::ContentMismatch),
    }
}

fn load_certs(path: &Path) -> Result<Vec<CertificateDer<'static>>, LoadError> {
    let file = File::open(path)?;
    let mut certs = Vec::new();

    let mut buf = BufReader::new(file);

    for item in rustls_pemfile::read_all(&mut buf) {
        let item = item?;

        match item {
            Item::X509Certificate(cert) => certs.push(cert),
            _ => continue,
        }
    }

    Ok(certs)
}
