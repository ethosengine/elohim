//! Persisted iroh secret key. Load if present, generate-and-write if not.
//!
//! Stored as 32 raw bytes (the ed25519 secret) at the configured path, mode
//! 0600 on Unix. Distinct from any libp2p keypair file; the two stacks have
//! separate identities throughout cutover. Cross-stack identity unification
//! is a Phase 10 concern (see plan).

use std::path::Path;
use std::{fs, io};

use iroh::SecretKey;

/// Load a secret key from `path`, or generate a fresh one and persist it
/// if the file does not exist. Existing files of the wrong length are
/// rejected — a corrupt key file should fail loud, not silently regenerate
/// (regeneration would change the node's [`iroh::NodeId`] and break peer
/// expectations).
pub fn load_or_generate(path: &Path) -> io::Result<SecretKey> {
    match fs::read(path) {
        Ok(bytes) => {
            let arr: [u8; 32] = bytes.as_slice().try_into().map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "iroh key file {} has wrong length: expected 32, got {}",
                        path.display(),
                        bytes.len()
                    ),
                )
            })?;
            Ok(SecretKey::from_bytes(&arr))
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let key = SecretKey::generate(&mut rand::rngs::OsRng);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, key.to_bytes())?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
            }
            Ok(key)
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn generates_when_missing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("iroh.key");
        let key = load_or_generate(&path).unwrap();
        assert!(path.exists());
        assert_eq!(fs::read(&path).unwrap().len(), 32);
        // sanity: derived public key works
        let _public = key.public();
    }

    #[test]
    fn round_trips_existing_key() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("iroh.key");
        let k1 = load_or_generate(&path).unwrap();
        let k2 = load_or_generate(&path).unwrap();
        assert_eq!(k1.to_bytes(), k2.to_bytes());
    }

    #[test]
    fn rejects_wrong_length_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("iroh.key");
        fs::write(&path, b"too short").unwrap();
        let err = load_or_generate(&path).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[cfg(unix)]
    #[test]
    fn generated_key_is_mode_0600() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let path = dir.path().join("iroh.key");
        let _ = load_or_generate(&path).unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn creates_parent_dir_if_missing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nested/sub/iroh.key");
        let _ = load_or_generate(&path).unwrap();
        assert!(path.exists());
    }
}
