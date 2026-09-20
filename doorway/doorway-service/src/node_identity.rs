//! The doorway's own Ed25519 node identity — persisted across boots.
//!
//! Story 5.1 (serving-edge campaign): "The doorway's node key persists
//! across boots." Before this module, `main.rs` generated a fresh
//! [`ed25519_dalek::SigningKey`] unconditionally on every boot. That key
//! signs EdDSA JWTs (`auth::jwt`), backs `GET /.well-known/doorway-keys`
//! (JWKS, `routes::federation`), and backs `GET /.well-known/did.json`
//! (`routes::identity`) — so an ephemeral key means a restart silently
//! rotates the doorway's federation identity: outstanding EdDSA tokens stop
//! verifying, and any sibling doorway that cached this doorway's JWKS entry
//! starts rejecting it until it refreshes.
//!
//! [`load_or_generate`] follows the pattern already proven at
//! `elohim-storage`'s `p2p_iroh::identity::load_or_generate`: read 32 raw
//! secret-key bytes if present; a present-but-wrong-length file is a hard
//! `InvalidData` error — **never** silently regenerated, because a silent
//! new key is a silent identity change. On `NotFound`, generate a fresh key
//! (reusing `custodial_keys::crypto::generate_keypair`, the same generator
//! `main.rs` used before this module existed) and publish it:
//!
//! - The key is written to an **unpredictable** temp sibling
//!   (`.{name}.{pid}.{random}.tmp`, random from `OsRng`) opened with
//!   `create_new` (`O_EXCL`) at mode 0600 from creation — it can never
//!   follow a pre-planted symlink and never truncates an existing file. The
//!   temp file is removed on every error path after it is created.
//! - Publish is **first-writer-wins**: the temp file is promoted to `path`
//!   via [`fs::hard_link`], not `rename`. Two doorway processes racing a
//!   cold first boot on the same path both attempt this; exactly one
//!   `hard_link` succeeds, and the loser discards its temp and loads the
//!   winner's key through the normal load path instead of quietly keeping a
//!   different key in memory. This is one-process-per-path by design (see
//!   `config::Args::node_key_file`); the race only converges correctly
//!   between processes that agree on the file, never sidesteps needing one.
//! - On a winning publish, the parent directory is fsynced (not
//!   best-effort — only a platform's "directory fsync unsupported" error is
//!   swallowed) before the temp name is dropped, mirroring
//!   `relay-addr-beacon`'s `pkarr` sink's insistence on real durability
//!   rather than an OS write-cache promise.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::{rngs::OsRng, RngCore};
use tracing::warn;

/// Raw Ed25519 secret key length in bytes.
const KEY_LEN: usize = 32;

/// Whether [`load_or_generate`] found the key already on disk or had to mint
/// a fresh one. Callers log this alongside the fingerprint at boot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyOrigin {
    /// Read from an existing, valid key file (including a first-boot race
    /// this process lost — the winner's key was loaded instead).
    Loaded,
    /// No file was present; a new key was generated and this process won
    /// the publish race.
    Generated,
}

impl KeyOrigin {
    pub fn as_str(&self) -> &'static str {
        match self {
            KeyOrigin::Loaded => "loaded",
            KeyOrigin::Generated => "generated",
        }
    }
}

/// Load a doorway node signing key from `path`, or generate a fresh one and
/// persist it if the file does not exist.
///
/// A present file of the wrong length (including empty) is a hard error and
/// is left untouched — regenerating over it would silently rotate the
/// node's identity. An existing file with permissions wider than 0600 is
/// tightened in place (logged at `warn!`) rather than rejected, since a
/// loose mode is a recoverable hygiene issue, not evidence of a different
/// key.
pub fn load_or_generate(path: &Path) -> io::Result<(SigningKey, KeyOrigin)> {
    match fs::read(path) {
        Ok(bytes) => {
            let key = parse_key_bytes(path, &bytes)?;
            #[cfg(unix)]
            tighten_permissions_if_needed(path)?;
            Ok((key, KeyOrigin::Loaded))
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    fs::create_dir_all(parent)?;
                }
            }
            let (signing_key, _verifying_key) = crate::custodial_keys::crypto::generate_keypair();
            let tmp_path = create_temp_key_file(path, signing_key.to_bytes().as_slice())?;
            publish(&tmp_path, path, signing_key)
        }
        Err(e) => Err(e),
    }
}

/// Parse raw key bytes, producing the wrong-length `InvalidData` error with
/// the file's path and observed length on mismatch.
fn parse_key_bytes(path: &Path, bytes: &[u8]) -> io::Result<SigningKey> {
    let arr: [u8; KEY_LEN] = bytes.try_into().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "doorway node key file {} has wrong length: expected {KEY_LEN}, got {}",
                path.display(),
                bytes.len()
            ),
        )
    })?;
    Ok(SigningKey::from_bytes(&arr))
}

/// Load an existing key file through the normal validated path (length
/// check + permission tightening). Used both by [`load_or_generate`]'s
/// happy path and by [`publish`] when this process lost a first-boot
/// publish race and must load the winner's key.
fn load_existing(path: &Path) -> io::Result<SigningKey> {
    let bytes = fs::read(path)?;
    let key = parse_key_bytes(path, &bytes)?;
    #[cfg(unix)]
    tighten_permissions_if_needed(path)?;
    Ok(key)
}

/// Create an unpredictable temp sibling of `path`, write `bytes` to it, and
/// fsync — WITHOUT publishing it to `path` yet. The file is opened with
/// `create_new` (`O_EXCL`) so it can never follow a pre-planted symlink or
/// clobber something already there, at mode 0600 from creation on Unix so
/// the secret is never briefly world-readable. On any failure after the
/// temp path is chosen, the temp file (if created) is removed before the
/// error is returned.
fn create_temp_key_file(path: &Path, bytes: &[u8]) -> io::Result<PathBuf> {
    let tmp_path = random_tmp_sibling_path(path);

    let result: io::Result<()> = (|| {
        #[cfg(unix)]
        let mut file = {
            use std::os::unix::fs::OpenOptionsExt;
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&tmp_path)?
        };
        #[cfg(not(unix))]
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp_path)?;

        file.write_all(bytes)?;
        file.sync_all()?;

        // Belt-and-suspenders: force 0600 regardless of the writer's umask,
        // mirroring relay-addr-beacon's pkarr sink (sinks/pkarr.rs).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o600))?;
        }

        Ok(())
    })();

    match result {
        Ok(()) => Ok(tmp_path),
        Err(e) => {
            let _ = fs::remove_file(&tmp_path);
            Err(e)
        }
    }
}

/// Publish a generated key by hard-linking `tmp_path` to `path`
/// (first-writer-wins). Two processes racing to generate a key for the same
/// path on a cold first boot both reach this call; exactly one `hard_link`
/// succeeds — the OS makes that atomic — so they converge on the SAME key
/// rather than each holding a different one in memory.
///
/// - On success: fsync the parent directory (durability, not a best-effort
///   write-cache promise), then drop the temp name — the file survives
///   under `path`, its other hard-link name.
/// - On `AlreadyExists` (lost the race): discard the temp and load the
///   winner's key through [`load_existing`], reporting [`KeyOrigin::Loaded`].
/// - Any other error: discard the temp and propagate.
fn publish(
    tmp_path: &Path,
    path: &Path,
    generated_key: SigningKey,
) -> io::Result<(SigningKey, KeyOrigin)> {
    match fs::hard_link(tmp_path, path) {
        Ok(()) => {
            let fsync_result = fsync_parent_dir(path);
            let _ = fs::remove_file(tmp_path);
            fsync_result?;
            Ok((generated_key, KeyOrigin::Generated))
        }
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
            let _ = fs::remove_file(tmp_path);
            let key = load_existing(path)?;
            Ok((key, KeyOrigin::Loaded))
        }
        Err(e) => {
            let _ = fs::remove_file(tmp_path);
            Err(e)
        }
    }
}

/// An unpredictable temp sibling path (`.{name}.{pid}.{random}.tmp`) in the
/// same directory as `path`, so the eventual `hard_link` is same-filesystem.
/// The PID + 8 random bytes (hex) make the name unguessable and collisions
/// between concurrent boots effectively impossible; paired with
/// `create_new` (`O_EXCL`) at open time, a caller can neither predict nor
/// pre-plant this path.
fn random_tmp_sibling_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "node".to_string());
    let pid = std::process::id();
    let mut rand_bytes = [0u8; 8];
    OsRng.fill_bytes(&mut rand_bytes);
    let rand_hex = hex::encode(rand_bytes);
    path.with_file_name(format!(".{file_name}.{pid}.{rand_hex}.tmp"))
}

/// Fsync `path`'s parent directory so a `hard_link` publish is durable
/// (survives a crash, not just an OS write cache). This is NOT a
/// best-effort operation — the only error swallowed is the specific case
/// where the platform/filesystem doesn't support directory fsync at all;
/// every other error propagates.
#[cfg(unix)]
fn fsync_parent_dir(path: &Path) -> io::Result<()> {
    let parent = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => return Ok(()),
    };
    let dir = File::open(parent)?;
    match dir.sync_all() {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::Unsupported => Ok(()),
        Err(e) => Err(e),
    }
}

#[cfg(not(unix))]
fn fsync_parent_dir(_path: &Path) -> io::Result<()> {
    Ok(())
}

/// Tighten an existing key file's permissions to 0600 if wider, logging a
/// warning. Never rejects the file — a loose mode is a hygiene fix, not an
/// identity change.
#[cfg(unix)]
fn tighten_permissions_if_needed(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let meta = fs::metadata(path)?;
    let mode = meta.permissions().mode() & 0o777;
    if mode != 0o600 {
        warn!(
            path = %path.display(),
            mode = format!("{mode:o}"),
            "doorway node key file has overly permissive mode; tightening to 0600"
        );
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

/// A short, non-secret hex fingerprint of a public key, safe to log.
/// Never call this with secret-key bytes.
pub fn fingerprint(verifying_key: &VerifyingKey) -> String {
    hex::encode(&verifying_key.to_bytes()[..8])
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn generates_and_persists_when_absent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("node.key");
        assert!(!path.exists());

        let (key, origin) = load_or_generate(&path).unwrap();

        assert_eq!(origin, KeyOrigin::Generated);
        assert!(path.exists());
        assert_eq!(fs::read(&path).unwrap().len(), KEY_LEN);
        // Sanity: derived public key works.
        let _public = key.verifying_key();
    }

    #[test]
    fn second_call_returns_same_key_and_reports_loaded() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("node.key");

        let (k1, origin1) = load_or_generate(&path).unwrap();
        let (k2, origin2) = load_or_generate(&path).unwrap();

        assert_eq!(origin1, KeyOrigin::Generated);
        assert_eq!(origin2, KeyOrigin::Loaded);
        assert_eq!(k1.verifying_key(), k2.verifying_key());
        assert_eq!(k1.to_bytes(), k2.to_bytes());
    }

    #[cfg(unix)]
    #[test]
    fn generated_key_file_is_mode_0600() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let path = dir.path().join("node.key");

        let _ = load_or_generate(&path).unwrap();

        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn wrong_length_file_is_rejected_and_left_untouched() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("node.key");
        fs::write(&path, b"too short").unwrap();

        let err = load_or_generate(&path).unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        // Untouched: still the original bad bytes, not regenerated.
        assert_eq!(fs::read(&path).unwrap(), b"too short");
    }

    #[test]
    fn an_empty_key_file_is_rejected_and_left_untouched() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("node.key");
        fs::write(&path, b"").unwrap();

        let err = load_or_generate(&path).unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert_eq!(fs::read(&path).unwrap().len(), 0);
    }

    #[cfg(unix)]
    #[test]
    fn over_permissive_existing_file_is_tightened() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let path = dir.path().join("node.key");

        let (original, _) = load_or_generate(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();

        let (reloaded, origin) = load_or_generate(&path).unwrap();

        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        assert_eq!(origin, KeyOrigin::Loaded);
        // Tightening must not have touched the key material.
        assert_eq!(original.to_bytes(), reloaded.to_bytes());
    }

    #[test]
    fn creates_parent_dir_if_missing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nested/sub/node.key");
        assert!(!path.parent().unwrap().exists());

        let _ = load_or_generate(&path).unwrap();

        assert!(path.exists());
    }

    #[test]
    fn fingerprint_is_stable_and_short() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("node.key");
        let (key, _) = load_or_generate(&path).unwrap();

        let fp1 = fingerprint(&key.verifying_key());
        let fp2 = fingerprint(&key.verifying_key());

        assert_eq!(fp1, fp2);
        assert_eq!(fp1.len(), 16); // 8 bytes, hex-encoded
    }

    /// Two processes race a cold first boot: both generate a key and try to
    /// publish. This process's temp file loses the `hard_link` race because
    /// the target already exists (the other process's key). It must
    /// converge on the WINNER's key, report `Loaded`, and leave no temp
    /// file behind — never keep its own generated key in memory.
    #[test]
    fn a_lost_publish_race_loads_the_winner_s_key() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("node.key");

        let (loser_key, _) = crate::custodial_keys::crypto::generate_keypair();
        let (winner_key, _) = crate::custodial_keys::crypto::generate_keypair();
        assert_ne!(loser_key.to_bytes(), winner_key.to_bytes());

        // Simulate the winner having already published between this
        // process's generate and publish steps.
        fs::write(&path, winner_key.to_bytes().as_slice()).unwrap();

        let tmp_path = create_temp_key_file(&path, loser_key.to_bytes().as_slice()).unwrap();
        assert!(tmp_path.exists());

        let (key, origin) = publish(&tmp_path, &path, loser_key).unwrap();

        assert_eq!(origin, KeyOrigin::Loaded);
        assert_eq!(key.to_bytes(), winner_key.to_bytes());
        assert!(
            !tmp_path.exists(),
            "temp file must be removed after a lost publish race"
        );
    }

    // `an_unreadable_key_file_is_a_hard_error_not_a_regenerate` is
    // intentionally omitted: this suite runs as root in CI/dev containers
    // (verified: chmod 0000 + read still succeeds), so a permission-denied
    // path can't be exercised here without a vacuous test that doesn't
    // actually deny access. The behavior it would cover (a permission
    // error other than NotFound propagates via `Err(e) => Err(e)` rather
    // than falling into the generate branch) is structurally guaranteed by
    // `load_or_generate`'s match arm, which only special-cases
    // `ErrorKind::NotFound`.
}
