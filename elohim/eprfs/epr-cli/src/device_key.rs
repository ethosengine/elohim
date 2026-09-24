//! The device key — one ed25519 key per OS user per device, living outside every checkout.
//!
//! WHY A DEVICE KEY AND NOT A CHECKOUT KEY. A participant's claim has to travel with the
//! participant, not with the working tree: the same person works in an Eclipse Che workspace, in
//! VS Code on a laptop, in a fresh worktree of either. A key stored in a clone (brit's
//! `.git/brit/agent-key`) is a new identity per clone; a key stored on the host a service runs on
//! is the host's identity, not the person's. So the key lives in the OS user's config home
//! (`${XDG_CONFIG_HOME:-$HOME/.config}/elohim/device/ed25519.seed`), and every worktree — and
//! every restart of a Che workspace that keeps its home volume — sees the same key. A wiped home
//! is a new device, bound again, honestly; nothing here pretends otherwise.
//!
//! **A device key makes a claim portable and self-verifying. It never makes it a credential.**
//! Nothing in the repository refuses an act for want of a signature — the attribution floor stays
//! honor-system (ruling R-P1 of the post-station-4 sprint). The signature is evidence a later
//! reader can check, never a gate a writer must pass.
//!
//! **The key never crosses devices.** A second device mints its own key and is bound to the first
//! through a two-signature roster row (`elohim_epr_rea::participants`); a copied seed would make two
//! devices indistinguishable, which is the one thing a device identity exists to prevent.
//!
//! Load-or-generate follows `doorway/doorway-service/src/node_identity.rs`, the most careful key
//! persistence in the repository: a present file of the wrong length is a hard error and is never
//! regenerated over (a silent new key is a silent identity change); a fresh key is written to an
//! unpredictable `O_EXCL` temp sibling at mode 0600 from creation and published with a hard link,
//! so two processes racing a first run converge on ONE key — the loser discards its own and loads
//! the winner's. A filesystem that refuses hard links (some FUSE and network mounts) falls back
//! to a `rename` of the same temp file onto an ABSENT path, then re-reads what is on disk, so the
//! key a process returns is always the key the file holds.
//!
//! **Where the key may live** (ruling R-P19). The override `ELOHIM_DEVICE_KEY_FILE` must be an
//! absolute path outside every git repository — a relative path resolves differently in every
//! checkout, and a path inside a repository is one commit away from publishing the seed. The
//! directory holding the key must not be group- or world-writable: whoever can write there can
//! swap the key. Both refusals are errors, never a silent fallback to the config home.
//!
//! **Reading never widens anything.** [`DeviceKey::load`] — the read every standing lookup makes —
//! never mints a key, and touches the file's mode only when it MUST: when the seed is readable or
//! writable by group or others. A seed stricter than 0600 (0400, say) is left exactly as it is.
//!
//! **The roster pins live beside the key.** The chain root a device first verified for a handle
//! is recorded at `<key dir>/rosters/<handle>.root` — for the default key path that is
//! `<config home>/elohim/device/rosters/<handle>.root` (ruling R-P15) — so a device's pins travel
//! with its key and a temp test key gets temp pins.

use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use elohim_epr::proof::{self, AgentKeypair};
use rand_core::{OsRng, RngCore};

/// Test-and-tooling override for the key's path. Its only intended uses are tests and the
/// two-device enrolment rehearsal; an ordinary session resolves the config-home path.
pub const DEVICE_KEY_ENV: &str = "ELOHIM_DEVICE_KEY_FILE";

/// Raw ed25519 seed length in bytes.
const SEED_LEN: usize = 32;

/// Whether [`DeviceKey::load_or_generate`] found the key on disk or minted it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyOrigin {
    /// Read from an existing, valid seed file — including a first-run race this process lost,
    /// where the winner's key was loaded instead of the one this process generated.
    Loaded,
    /// No file was present; a new key was generated and this process won the publish.
    Generated,
}

/// This device's ed25519 key, and where it lives.
pub struct DeviceKey {
    keypair: AgentKeypair,
    path: PathBuf,
    origin: KeyOrigin,
}

impl std::fmt::Debug for DeviceKey {
    /// Never prints the seed: a debug line is a log line, and a log line is not a secret store.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeviceKey")
            .field("did_key", &self.did_key())
            .field("path", &self.path)
            .field("origin", &self.origin)
            .finish()
    }
}

/// Resolve the seed path from the process environment. See [`resolve_path_with`].
pub fn resolve_path() -> io::Result<PathBuf> {
    resolve_path_with(|name| std::env::var_os(name))
}

/// Resolve the seed path from an environment lookup, in order:
///
/// 1. `ELOHIM_DEVICE_KEY_FILE`, when set and non-empty — it must be ABSOLUTE and lie outside
///    every git repository (see [`check_override_path`]); anything else is refused, never used.
/// 2. `$XDG_CONFIG_HOME/elohim/device/ed25519.seed`, when `XDG_CONFIG_HOME` is set to an
///    ABSOLUTE path (the XDG base-directory spec says a relative value is invalid and ignored).
/// 3. `$HOME/.config/elohim/device/ed25519.seed`.
///
/// No home at all is an error, never a fallback into the working directory: a key written into a
/// checkout is exactly the per-clone identity this module exists to avoid.
///
/// Takes the lookup as a function so the precedence is testable without mutating the process
/// environment, which parallel tests share.
pub fn resolve_path_with<F>(lookup: F) -> io::Result<PathBuf>
where
    F: Fn(&str) -> Option<OsString>,
{
    let non_empty = |name: &str| lookup(name).filter(|value| !value.is_empty());

    if let Some(explicit) = non_empty(DEVICE_KEY_ENV) {
        let explicit = PathBuf::from(explicit);
        check_override_path(&explicit)?;
        return Ok(explicit);
    }
    let config_home = match non_empty("XDG_CONFIG_HOME").map(PathBuf::from) {
        Some(xdg) if xdg.is_absolute() => xdg,
        _ => match non_empty("HOME") {
            Some(home) => PathBuf::from(home).join(".config"),
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!(
                        "no device key path: {DEVICE_KEY_ENV}, XDG_CONFIG_HOME and HOME are all \
                         unset — the device key lives in the OS user's config home, never in a \
                         checkout"
                    ),
                ))
            }
        },
    };
    Ok(config_home
        .join("elohim")
        .join("device")
        .join("ed25519.seed"))
}

/// Refuse an `ELOHIM_DEVICE_KEY_FILE` that is relative, or that lies inside a git repository.
///
/// The repository test walks up from the path's parent looking for a `.git` entry (a directory in
/// a clone, a file in a worktree or submodule); the first one found refuses.
pub fn check_override_path(path: &Path) -> io::Result<()> {
    if !path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "{DEVICE_KEY_ENV}=`{}` is relative — a relative key path names a different key in \
                 every checkout; give an absolute path outside any repository",
                path.display()
            ),
        ));
    }
    let mut dir = path.parent();
    while let Some(current) = dir {
        if current.join(".git").exists() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "{DEVICE_KEY_ENV}=`{}` lies inside the git repository at `{}` — a seed in a \
                     repository is one commit from being published; keep it outside every \
                     checkout",
                    path.display(),
                    current.display()
                ),
            ));
        }
        dir = current.parent();
    }
    Ok(())
}

/// Refuse a key directory that group or others may write: whoever can write it can replace the
/// key. Only an EXISTING directory is checked — one this module creates is 0700 from birth.
#[cfg(unix)]
fn check_key_dir(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) else {
        return Ok(());
    };
    match fs::metadata(parent) {
        Ok(meta) if meta.permissions().mode() & 0o022 != 0 => Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "the device key directory `{}` is group- or world-writable (mode {:o}) — anyone \
                 who can write there can swap the key; `chmod go-w` it first",
                parent.display(),
                meta.permissions().mode() & 0o777
            ),
        )),
        Ok(_) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

#[cfg(not(unix))]
fn check_key_dir(_path: &Path) -> io::Result<()> {
    Ok(())
}

impl DeviceKey {
    /// Open this device's key at the resolved path, generating it on first use.
    pub fn open() -> io::Result<Self> {
        Self::load_or_generate(&resolve_path()?)
    }

    /// Load the EXISTING seed at `path` — the read path: never mints a key (a missing file is
    /// `NotFound`), and changes the file's mode only when it must (group/other bits set).
    pub fn load(path: &Path) -> io::Result<Self> {
        check_key_dir(path)?;
        let keypair = load_existing(path)?;
        Ok(Self::new(keypair, path, KeyOrigin::Loaded))
    }

    /// Load the seed at `path`, or generate one and publish it there if the file does not exist.
    ///
    /// A present file of the wrong length (including empty) is a hard `InvalidData` error and is
    /// left untouched. An existing file readable or writable by group or others is tightened in
    /// place (owner bits kept) rather than refused: a loose mode is a hygiene fault, not evidence
    /// of a different key. A group- or world-writable key directory is refused.
    pub fn load_or_generate(path: &Path) -> io::Result<Self> {
        check_key_dir(path)?;
        match fs::read(path) {
            Ok(bytes) => {
                let keypair = parse_seed(path, &bytes)?;
                #[cfg(unix)]
                tighten_permissions_if_needed(path)?;
                Ok(Self::new(keypair, path, KeyOrigin::Loaded))
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                if let Some(parent) = path.parent() {
                    if !parent.as_os_str().is_empty() {
                        create_private_dir_all(parent)?;
                    }
                }
                let generated = AgentKeypair::generate(&mut OsRng);
                let tmp_path = create_temp_seed_file(path, &generated.secret_bytes())?;
                publish(&tmp_path, path, generated)
            }
            Err(e) => Err(e),
        }
    }

    fn new(keypair: AgentKeypair, path: &Path, origin: KeyOrigin) -> Self {
        Self {
            keypair,
            path: path.to_path_buf(),
            origin,
        }
    }

    /// Where the seed lives.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Whether this handle loaded an existing key or minted it.
    pub fn origin(&self) -> KeyOrigin {
        self.origin
    }

    /// The raw 32-byte ed25519 public key.
    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.keypair.public_key_bytes()
    }

    /// This device's public name: `did:key:z6Mk…`, encoded by the repository's one did:key codec
    /// (`did_bridge::codec::core32_to_did_key`), so it resolves through `DidKeyResolver` unchanged.
    pub fn did_key(&self) -> String {
        did_bridge::codec::core32_to_did_key(&self.public_key_bytes())
    }

    /// Sign `message`; returns the 64 raw ed25519 signature bytes.
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        proof::sign(&self.keypair, message)
    }
}

/// Verify a detached ed25519 signature against a signer named by its `did:key`.
///
/// `false` — never an error — for a did:key that does not decode to an ed25519 key, so a caller
/// can pass this straight in as the verification closure `elohim_epr_rea::participants` takes.
pub fn verify(signer_did_key: &str, message: &[u8], signature: &[u8]) -> bool {
    match did_bridge::codec::did_key_to_core32(signer_did_key) {
        Ok(core) => proof::verify(&core, message, signature),
        Err(_) => false,
    }
}

/// Parse raw seed bytes; a wrong length is `InvalidData` naming the file and what was found.
fn parse_seed(path: &Path, bytes: &[u8]) -> io::Result<AgentKeypair> {
    if bytes.len() != SEED_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "device key file {} has wrong length: expected {SEED_LEN}, got {} — refusing to \
                 regenerate over it, because a silent new key is a silent identity change",
                path.display(),
                bytes.len()
            ),
        ));
    }
    AgentKeypair::from_secret(bytes)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
}

/// Load an existing seed through the validated path. Used when this process lost the publish race.
fn load_existing(path: &Path) -> io::Result<AgentKeypair> {
    let bytes = fs::read(path)?;
    let keypair = parse_seed(path, &bytes)?;
    #[cfg(unix)]
    tighten_permissions_if_needed(path)?;
    Ok(keypair)
}

/// Create the key's directory chain, private (0700) on Unix where this call creates it.
fn create_private_dir_all(dir: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)
    }
    #[cfg(not(unix))]
    {
        fs::create_dir_all(dir)
    }
}

/// Write `bytes` to an unpredictable `O_EXCL` temp sibling of `path` at mode 0600 and fsync it,
/// WITHOUT publishing. The temp file is removed on every error path after it is created.
fn create_temp_seed_file(path: &Path, bytes: &[u8]) -> io::Result<PathBuf> {
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

        // Force 0600 regardless of the writer's umask.
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

/// Publish by hard-linking the temp file to `path` — first writer wins, atomically. On a lost
/// race (`AlreadyExists`) the temp is discarded and the WINNER's key is loaded, so two processes
/// never hold two different keys for one path. A filesystem that refuses links falls back to
/// [`publish_by_rename`].
fn publish(tmp_path: &Path, path: &Path, generated: AgentKeypair) -> io::Result<DeviceKey> {
    match fs::hard_link(tmp_path, path) {
        Ok(()) => {
            let fsync_result = fsync_parent_dir(path);
            let _ = fs::remove_file(tmp_path);
            fsync_result?;
            Ok(DeviceKey::new(generated, path, KeyOrigin::Generated))
        }
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
            let _ = fs::remove_file(tmp_path);
            let winner = load_existing(path)?;
            Ok(DeviceKey::new(winner, path, KeyOrigin::Loaded))
        }
        Err(e) if links_refused(&e) => publish_by_rename(tmp_path, path, generated),
        Err(e) => {
            let _ = fs::remove_file(tmp_path);
            Err(e)
        }
    }
}

/// Whether a `hard_link` error means "this filesystem does not do links" rather than a real
/// failure: `EPERM` (1), `EMLINK` (31), `EOPNOTSUPP` (95), or an `Unsupported` kind.
fn links_refused(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::Unsupported || matches!(error.raw_os_error(), Some(1 | 31 | 95))
}

/// The link-less fallback: rename the temp file onto `path` only while `path` is still absent,
/// then load whatever the file holds. `rename` replaces, so it cannot be first-writer-wins on its
/// own; re-reading after the rename means this process returns the key on DISK, and a racer that
/// lands later is loaded, never shadowed by a key held only in memory.
fn publish_by_rename(
    tmp_path: &Path,
    path: &Path,
    generated: AgentKeypair,
) -> io::Result<DeviceKey> {
    if path.exists() {
        let _ = fs::remove_file(tmp_path);
        let winner = load_existing(path)?;
        return Ok(DeviceKey::new(winner, path, KeyOrigin::Loaded));
    }
    if let Err(e) = fs::rename(tmp_path, path) {
        let _ = fs::remove_file(tmp_path);
        return Err(e);
    }
    fsync_parent_dir(path)?;
    let on_disk = load_existing(path)?;
    let origin = if on_disk.public_key_bytes() == generated.public_key_bytes() {
        KeyOrigin::Generated
    } else {
        KeyOrigin::Loaded
    };
    Ok(DeviceKey::new(on_disk, path, origin))
}

/// `.{name}.{pid}.{16 random hex}.tmp` beside `path` — same filesystem for the hard link, and
/// unguessable, so paired with `O_EXCL` nobody can pre-plant it.
fn random_tmp_sibling_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "ed25519.seed".to_string());
    let mut rand_bytes = [0u8; 8];
    OsRng.fill_bytes(&mut rand_bytes);
    let rand_hex: String = rand_bytes.iter().map(|b| format!("{b:02x}")).collect();
    path.with_file_name(format!(
        ".{file_name}.{}.{rand_hex}.tmp",
        std::process::id()
    ))
}

/// Fsync the parent directory so a published link survives a crash. Only a platform's
/// "directory fsync unsupported" is swallowed.
#[cfg(unix)]
fn fsync_parent_dir(path: &Path) -> io::Result<()> {
    let parent = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => return Ok(()),
    };
    match File::open(parent)?.sync_all() {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::Unsupported => Ok(()),
        Err(e) => Err(e),
    }
}

#[cfg(not(unix))]
fn fsync_parent_dir(_path: &Path) -> io::Result<()> {
    Ok(())
}

/// Tighten an existing seed file only when it MUST be: when group or others hold any bit. The
/// owner's bits are kept (a 0400 seed stays 0400; a 0644 seed becomes 0600), so a read never
/// widens a mode and never rewrites a mode that is already private.
#[cfg(unix)]
fn tighten_permissions_if_needed(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mode = fs::metadata(path)?.permissions().mode() & 0o777;
    if mode & 0o077 != 0 {
        fs::set_permissions(path, fs::Permissions::from_mode(mode & 0o700))?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// roster root pins (ruling R-P15)
// ---------------------------------------------------------------------------

/// Where this device pins `handle`'s chain root: `<key dir>/rosters/<handle>.root`.
pub fn root_pin_path(key_file: &Path, handle: &str) -> PathBuf {
    key_file
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join("rosters")
        .join(format!("{handle}.root"))
}

/// The chain root this device pinned for `handle`, or `None` when it has never pinned one. An
/// empty pin file is an error, never "unpinned": unpinned would let the next read pin anew.
pub fn read_root_pin(key_file: &Path, handle: &str) -> io::Result<Option<String>> {
    let path = root_pin_path(key_file, handle);
    match fs::read_to_string(&path) {
        Ok(text) if text.trim().is_empty() => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "the roster pin {} is empty — refusing to read it as unpinned",
                path.display()
            ),
        )),
        Ok(text) => Ok(Some(text.trim().to_string())),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// Pin `root` for `handle` on this device, once. First writer wins (the key's own temp-and-link
/// publish, with the same rename fallback), and the pin that ends up on disk is returned — a
/// racer's pin is read back, never overwritten.
pub fn write_root_pin(key_file: &Path, handle: &str, root: &str) -> io::Result<String> {
    if let Some(existing) = read_root_pin(key_file, handle)? {
        return Ok(existing);
    }
    let path = root_pin_path(key_file, handle);
    if let Some(parent) = path.parent() {
        create_private_dir_all(parent)?;
    }
    let tmp = create_temp_seed_file(&path, format!("{root}\n").as_bytes())?;
    let published = match fs::hard_link(&tmp, &path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(e) if links_refused(&e) && !path.exists() => fs::rename(&tmp, &path),
        Err(e) => Err(e),
    };
    let _ = fs::remove_file(&tmp);
    published?;
    fsync_parent_dir(&path)?;
    read_root_pin(key_file, handle)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("the roster pin {} vanished after writing", path.display()),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Barrier};

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
        let map: HashMap<String, OsString> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), OsString::from(v)))
            .collect();
        move |name| map.get(name).cloned()
    }

    #[test]
    fn generate_then_load_is_stable() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("device").join("ed25519.seed");

        let first = DeviceKey::load_or_generate(&path).unwrap();
        let second = DeviceKey::load_or_generate(&path).unwrap();

        assert_eq!(first.origin(), KeyOrigin::Generated);
        assert_eq!(second.origin(), KeyOrigin::Loaded);
        assert_eq!(first.public_key_bytes(), second.public_key_bytes());
        assert_eq!(first.did_key(), second.did_key());
        assert_eq!(fs::read(&path).unwrap().len(), SEED_LEN);
    }

    #[test]
    fn a_wrong_length_seed_is_refused_and_left_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ed25519.seed");
        fs::write(&path, b"short").unwrap();

        let err = DeviceKey::load_or_generate(&path).expect_err("never regenerated over");
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert_eq!(fs::read(&path).unwrap(), b"short");
    }

    #[cfg(unix)]
    #[test]
    fn seed_file_is_0600() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir
            .path()
            .join("elohim")
            .join("device")
            .join("ed25519.seed");

        DeviceKey::load_or_generate(&path).unwrap();

        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "the seed is private from creation");
        let dir_mode = fs::metadata(path.parent().unwrap())
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(dir_mode, 0o700, "and so is the directory this call created");

        // A loosened existing file is tightened, not refused.
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        DeviceKey::load_or_generate(&path).unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn concurrent_first_writer_yields_one_key() {
        for _ in 0..8 {
            let dir = tempfile::tempdir().unwrap();
            let path = Arc::new(dir.path().join("ed25519.seed"));
            let barrier = Arc::new(Barrier::new(2));

            let handles: Vec<_> = (0..2)
                .map(|_| {
                    let path = Arc::clone(&path);
                    let barrier = Arc::clone(&barrier);
                    std::thread::spawn(move || {
                        barrier.wait();
                        DeviceKey::load_or_generate(&path).unwrap()
                    })
                })
                .collect();
            let keys: Vec<DeviceKey> = handles.into_iter().map(|h| h.join().unwrap()).collect();

            assert_eq!(
                keys[0].public_key_bytes(),
                keys[1].public_key_bytes(),
                "two racing first runs converge on ONE key"
            );
            let generated = keys
                .iter()
                .filter(|k| k.origin() == KeyOrigin::Generated)
                .count();
            assert_eq!(generated, 1, "exactly one writer wins the publish");
            let on_disk = DeviceKey::load_or_generate(&path).unwrap();
            assert_eq!(on_disk.public_key_bytes(), keys[0].public_key_bytes());
            let leftovers: Vec<_> = fs::read_dir(dir.path())
                .unwrap()
                .map(|e| e.unwrap().file_name())
                .collect();
            assert_eq!(leftovers.len(), 1, "no temp file survives: {leftovers:?}");
        }
    }

    #[test]
    fn did_key_round_trips_through_did_key_to_core32() {
        let dir = tempfile::tempdir().unwrap();
        let key = DeviceKey::load_or_generate(&dir.path().join("ed25519.seed")).unwrap();

        let did = key.did_key();
        assert!(did.starts_with("did:key:z6Mk"), "an ed25519 did:key: {did}");
        let core = did_bridge::codec::did_key_to_core32(&did).unwrap();
        assert_eq!(core, key.public_key_bytes());
    }

    #[test]
    fn env_override_wins() {
        let explicit = resolve_path_with(env(&[
            (DEVICE_KEY_ENV, "/tmp/elsewhere/key.seed"),
            ("XDG_CONFIG_HOME", "/xdg"),
            ("HOME", "/home/u"),
        ]))
        .unwrap();
        assert_eq!(explicit, PathBuf::from("/tmp/elsewhere/key.seed"));

        let xdg =
            resolve_path_with(env(&[("XDG_CONFIG_HOME", "/xdg"), ("HOME", "/home/u")])).unwrap();
        assert_eq!(xdg, PathBuf::from("/xdg/elohim/device/ed25519.seed"));

        let home = resolve_path_with(env(&[
            (DEVICE_KEY_ENV, ""),
            ("XDG_CONFIG_HOME", "relative/ignored"),
            ("HOME", "/home/u"),
        ]))
        .unwrap();
        assert_eq!(
            home,
            PathBuf::from("/home/u/.config/elohim/device/ed25519.seed"),
            "an empty override and a relative XDG_CONFIG_HOME both fall through"
        );

        assert!(
            resolve_path_with(env(&[])).is_err(),
            "no home is an error, never the working directory"
        );
    }

    #[test]
    fn m4_relative_or_in_repo_key_path_refused() {
        let relative = resolve_path_with(env(&[
            (DEVICE_KEY_ENV, "keys/ed25519.seed"),
            ("HOME", "/home/u"),
        ]))
        .expect_err("a relative override is refused, never resolved against the cwd");
        assert_eq!(relative.kind(), io::ErrorKind::InvalidInput);
        assert!(relative.to_string().contains("relative"), "{relative}");

        // A path anywhere under a directory holding `.git` (a clone's dir or a worktree's file).
        let repo = tempfile::tempdir().unwrap();
        std::fs::create_dir(repo.path().join(".git")).unwrap();
        let inside = repo.path().join("deep/er/ed25519.seed");
        let err = resolve_path_with(env(&[(DEVICE_KEY_ENV, inside.to_str().unwrap())]))
            .expect_err("a seed inside a repository is refused");
        assert!(err.to_string().contains("git repository"), "{err}");
        let worktree = tempfile::tempdir().unwrap();
        std::fs::write(worktree.path().join(".git"), "gitdir: /elsewhere\n").unwrap();
        assert!(check_override_path(&worktree.path().join("k.seed")).is_err());

        // Outside every repository: accepted as given.
        let outside = tempfile::tempdir().unwrap();
        let fine = outside.path().join("ed25519.seed");
        assert_eq!(
            resolve_path_with(env(&[(DEVICE_KEY_ENV, fine.to_str().unwrap())])).unwrap(),
            fine
        );
    }

    #[cfg(unix)]
    #[test]
    fn m4_a_writable_key_dir_is_refused_and_a_read_leaves_a_private_mode_alone() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let shared = dir.path().join("shared");
        fs::create_dir(&shared).unwrap();
        fs::set_permissions(&shared, fs::Permissions::from_mode(0o777)).unwrap();
        let err = DeviceKey::load_or_generate(&shared.join("ed25519.seed"))
            .expect_err("anyone who can write the directory can swap the key");
        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
        assert!(
            !shared.join("ed25519.seed").exists(),
            "nothing minted there"
        );

        // A 0400 seed is stricter than 0600: the read path leaves it exactly as it is.
        let path = dir.path().join("private").join("ed25519.seed");
        DeviceKey::load_or_generate(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
        DeviceKey::load(&path).unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o400, "a read never rewrites a private mode");
        // And the read never mints.
        let missing = dir.path().join("private").join("absent.seed");
        assert_eq!(
            DeviceKey::load(&missing).unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
        assert!(!missing.exists());
    }

    #[test]
    fn root_pin_is_written_once_and_read_back() {
        let dir = tempfile::tempdir().unwrap();
        let key_file = dir.path().join("device").join("ed25519.seed");
        assert_eq!(read_root_pin(&key_file, "matthew").unwrap(), None);
        assert_eq!(
            write_root_pin(&key_file, "matthew", "did:key:zFirst").unwrap(),
            "did:key:zFirst"
        );
        assert_eq!(
            write_root_pin(&key_file, "matthew", "did:key:zSecond").unwrap(),
            "did:key:zFirst",
            "first writer wins; a later root never overwrites the pin"
        );
        assert_eq!(
            root_pin_path(&key_file, "matthew"),
            dir.path().join("device/rosters/matthew.root")
        );
        fs::write(root_pin_path(&key_file, "matthew"), "  \n").unwrap();
        assert!(
            read_root_pin(&key_file, "matthew").is_err(),
            "an empty pin is an error, never unpinned"
        );
    }

    #[test]
    fn sign_verify_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let key = DeviceKey::load_or_generate(&dir.path().join("a.seed")).unwrap();
        let other = DeviceKey::load_or_generate(&dir.path().join("b.seed")).unwrap();
        let message = b"bafyrei-some-record-cid";

        let signature = key.sign(message);
        assert_eq!(signature.len(), 64);
        assert!(verify(&key.did_key(), message, &signature));
        assert!(!verify(&key.did_key(), b"another message", &signature));
        assert!(
            !verify(&other.did_key(), message, &signature),
            "a signature verifies only against its own signer"
        );
        assert!(!verify("did:key:not-a-key", message, &signature));
    }
}
