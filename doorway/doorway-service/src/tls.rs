//! TLS listener config loading — story 5.2 ("the doorway terminates its own
//! TLS"). A rustls listener runs beside the plain HTTP listener, serving the
//! exact same router (see `server::http::run` / `serve_connection`), so a
//! household mesh can be reached at `https://…elohim.local` with a locally
//! minted CA.
//!
//! Boot contract mirrors `node_identity` (story 5.1): the three env knobs
//! (`DOORWAY_TLS_PORT`, `DOORWAY_TLS_CERT_FILE`, `DOORWAY_TLS_KEY_FILE`) are
//! all-or-none (`config::Args::validate`); when all three are set, a
//! present-but-invalid cert/key pair aborts boot loudly here rather than
//! silently falling back to plaintext-only. Absent config leaves today's
//! behavior unchanged.
//!
//! No certificate issuance or rotation lives here — that is story 5.3
//! (gated). This module only loads a file pair the operator, or the
//! household mesh's local-CA opt-in (`MESH_DOORWAY_TLS=1` in
//! `hc-mesh.sh`), already minted.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;

/// Load a rustls server config from a PEM certificate chain file and a PEM
/// private key file. Every failure mode — missing file, unparseable PEM, a
/// key that does not match the certificate — is surfaced as a descriptive
/// `Err` naming the file and reason, so the boot-time caller can log exactly
/// what to fix rather than a bare rustls error.
pub fn load_server_config(
    cert_file: &Path,
    key_file: &Path,
) -> Result<Arc<rustls::ServerConfig>, String> {
    let cert_chain = load_certs(cert_file)?;
    let key = load_key(key_file)?;

    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, key)
        .map_err(|e| {
            format!(
                "TLS certificate ({}) and key ({}) do not form a valid pair: {e}",
                cert_file.display(),
                key_file.display()
            )
        })?;

    Ok(Arc::new(config))
}

fn load_certs(path: &Path) -> Result<Vec<rustls::pki_types::CertificateDer<'static>>, String> {
    let file =
        File::open(path).map_err(|e| format!("failed to open TLS cert file {path:?}: {e}"))?;
    let mut reader = BufReader::new(file);
    let certs: Vec<_> = rustls_pemfile::certs(&mut reader)
        .collect::<Result<_, _>>()
        .map_err(|e| format!("failed to parse TLS cert file {path:?} as PEM: {e}"))?;
    if certs.is_empty() {
        return Err(format!(
            "TLS cert file {path:?} was readable but contained no PEM certificates"
        ));
    }
    Ok(certs)
}

fn load_key(path: &Path) -> Result<rustls::pki_types::PrivateKeyDer<'static>, String> {
    let file =
        File::open(path).map_err(|e| format!("failed to open TLS key file {path:?}: {e}"))?;
    let mut reader = BufReader::new(file);
    rustls_pemfile::private_key(&mut reader)
        .map_err(|e| format!("failed to parse TLS key file {path:?} as PEM: {e}"))?
        .ok_or_else(|| {
            format!("TLS key file {path:?} was readable but contained no PEM private key")
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// A fresh self-signed cert/key PEM pair, generated per-test so no
    /// checked-in fixture can silently expire.
    fn self_signed_pair() -> (Vec<u8>, Vec<u8>) {
        let rcgen::CertifiedKey { cert, signing_key } =
            rcgen::generate_simple_self_signed(vec!["doorway.elohim.local".to_string()])
                .expect("self-signed cert generation");
        (
            cert.pem().into_bytes(),
            signing_key.serialize_pem().into_bytes(),
        )
    }

    fn write_temp(bytes: &[u8]) -> tempfile::NamedTempFile {
        let mut f = tempfile::Builder::new()
            .suffix(".pem")
            .tempfile()
            .expect("tempfile");
        f.write_all(bytes).expect("write tempfile");
        f
    }

    #[test]
    fn loads_a_valid_self_signed_pair() {
        let (cert_pem, key_pem) = self_signed_pair();
        let cert_file = write_temp(&cert_pem);
        let key_file = write_temp(&key_pem);

        let result = load_server_config(cert_file.path(), key_file.path());
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
    }

    #[test]
    fn rejects_a_missing_cert_file() {
        let (_cert_pem, key_pem) = self_signed_pair();
        let key_file = write_temp(&key_pem);

        let err = load_server_config(
            Path::new("/nonexistent/tls-5-2-test-cert.pem"),
            key_file.path(),
        )
        .unwrap_err();
        assert!(err.contains("failed to open TLS cert file"), "{err}");
    }

    #[test]
    fn rejects_a_missing_key_file() {
        let (cert_pem, _key_pem) = self_signed_pair();
        let cert_file = write_temp(&cert_pem);

        let err = load_server_config(
            cert_file.path(),
            Path::new("/nonexistent/tls-5-2-test-key.pem"),
        )
        .unwrap_err();
        assert!(err.contains("failed to open TLS key file"), "{err}");
    }

    #[test]
    fn rejects_garbage_cert_content() {
        let (_cert_pem, key_pem) = self_signed_pair();
        let cert_file = write_temp(b"this is not a certificate");
        let key_file = write_temp(&key_pem);

        let err = load_server_config(cert_file.path(), key_file.path()).unwrap_err();
        assert!(
            err.contains("contained no PEM certificates") || err.contains("failed to parse"),
            "{err}"
        );
    }

    #[test]
    fn rejects_garbage_key_content() {
        let (cert_pem, _key_pem) = self_signed_pair();
        let cert_file = write_temp(&cert_pem);
        let key_file = write_temp(b"this is not a private key");

        let err = load_server_config(cert_file.path(), key_file.path()).unwrap_err();
        assert!(
            err.contains("contained no PEM private key") || err.contains("failed to parse"),
            "{err}"
        );
    }

    #[test]
    fn rejects_a_key_that_does_not_match_the_certificate() {
        let (cert_pem, _matching_key_pem) = self_signed_pair();
        let (_other_cert_pem, other_key_pem) = self_signed_pair();
        let cert_file = write_temp(&cert_pem);
        let key_file = write_temp(&other_key_pem);

        let err = load_server_config(cert_file.path(), key_file.path()).unwrap_err();
        assert!(err.contains("do not form a valid pair"), "{err}");
    }
}
