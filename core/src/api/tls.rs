//! TLS configuration for RPC/WebSocket endpoints.
//!
//! Provides self-signed certificate generation via [`rcgen`] and a
//! [`rustls`] server config wrapped in a [`tokio_rustls::TlsAcceptor`].
//! Certificates live under `/etc/atoll/tls/`.

use rcgen::{generate_simple_self_signed, CertifiedKey};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio_rustls::TlsAcceptor;

/// Directory holding TLS certificates and keys.
const TLS_DIR: &str = "/etc/atoll/tls";
/// Path to the generated certificate file.
const CERT_FILE: &str = "/etc/atoll/tls/cert.pem";
/// Path to the generated private key file.
const KEY_FILE: &str = "/etc/atoll/tls/key.pem";

/// Result of self-signed certificate generation: the server config plus the
/// paths where the PEM cert/key were written.
pub struct TlsCredentials {
    /// Rustls server config presenting the self-signed certificate.
    pub config: Arc<ServerConfig>,
    /// Path to the written certificate file (`/etc/atoll/tls/cert.pem`).
    pub cert_path: PathBuf,
    /// Path to the written private key file (`/etc/atoll/tls/key.pem`).
    pub key_path: PathBuf,
}

/// Generates a self-signed certificate and key, writes PEM files to
/// `/etc/atoll/tls/`, and builds a [`rustls::ServerConfig`] in-memory from the
/// freshly generated material (no need to re-parse the files).
///
/// If both PEM files already exist they are left untouched and a config is
/// rebuilt from the regenerated material.
///
/// # Errors
///
/// Returns an error if the directory cannot be created, the cert/key cannot
/// be generated, the files cannot be written, or the config is invalid.
pub fn generate_self_signed() -> Result<TlsCredentials, Box<dyn std::error::Error>> {
    fs::create_dir_all(TLS_DIR)?;

    let CertifiedKey { cert, signing_key } =
        generate_simple_self_signed(vec!["localhost".to_string(), "atoll.protocol".to_string()])?;

    // Persist PEM files under /etc/atoll/tls/.
    let cert_pem = cert.pem();
    let key_pem = signing_key.serialize_pem();
    fs::write(CERT_FILE, cert_pem.as_bytes())?;
    fs::write(KEY_FILE, key_pem.as_bytes())?;

    // Build the server config in-memory (no need to re-parse the files).
    let cert_der = CertificateDer::from(cert.der().to_vec());
    let key_der = PrivateKeyDer::try_from(signing_key.serialize_der())?;
    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert_der], key_der)?;

    Ok(TlsCredentials {
        config: Arc::new(config),
        cert_path: PathBuf::from(CERT_FILE),
        key_path: PathBuf::from(KEY_FILE),
    })
}

/// Wraps a [`rustls::ServerConfig`] in a [`tokio_rustls::TlsAcceptor`] for
/// accepting TLS connections on a TCP stream.
pub fn acceptor(config: Arc<ServerConfig>) -> TlsAcceptor {
    TlsAcceptor::from(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tls_dir_constant_points_to_etc() {
        assert_eq!(TLS_DIR, "/etc/atoll/tls");
        assert_eq!(CERT_FILE, "/etc/atoll/tls/cert.pem");
        assert_eq!(KEY_FILE, "/etc/atoll/tls/key.pem");
    }

    #[test]
    fn generate_simple_self_signed_produces_pem() {
        let CertifiedKey { cert, signing_key } =
            generate_simple_self_signed(vec!["localhost".to_string()])
                .expect("self-signed generation");
        let cert_pem = cert.pem();
        let key_pem = signing_key.serialize_pem();
        assert!(!cert_pem.is_empty());
        assert!(!key_pem.is_empty());
        assert!(cert_pem.contains("BEGIN CERTIFICATE"));
        assert!(key_pem.contains("BEGIN PRIVATE KEY"));
        assert!(!cert.der().is_empty());
    }
}
