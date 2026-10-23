//! The device's iroh peer key: generated once on the device, private from the moment its bytes
//! exist, and never copied, sent, printed or logged.

use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use iroh::{NodeId, SecretKey};

use crate::error::PeerError;

/// The length of an ed25519 secret key, which is all the key file holds.
const SECRET_LEN: usize = 32;

/// This device's iroh secret key. Its [`NodeId`] is the public half the roster's
/// `PeerEndpoint` row names.
///
/// `Debug` shows the node id only; there is no `Display`, `Clone` or serialization, and the secret
/// leaves this crate only into the endpoint [`crate::bind`] builds.
pub struct PeerKey {
    secret: SecretKey,
}

impl PeerKey {
    /// The key file's name inside the directory given to [`Self::load_or_generate`]. On a
    /// workspace device that directory is `$XDG_CONFIG_HOME/epr/peer` (spec §3.1).
    pub const FILE_NAME: &'static str = "iroh.key";

    /// Load the key from `dir/iroh.key`, or generate and persist one when the file does not
    /// exist.
    ///
    /// A new key is written to a fresh `O_EXCL` sibling created at mode 0600 — no chmod ever
    /// follows the secret bytes — and published with a hard link, so of two concurrent first
    /// starts the first wins and the second loads the winner's key. A directory this call
    /// creates is created 0700.
    ///
    /// # Errors
    ///
    /// - [`PeerError::KeyFileExposed`] when the existing file grants group or others any
    ///   permission; the file is left as found.
    /// - [`PeerError::KeyFileCorrupt`] when it does not hold exactly 32 bytes; it is never
    ///   regenerated, because a new key would silently change this device's node id.
    /// - [`PeerError::KeyIo`] for any filesystem failure.
    pub fn load_or_generate(dir: &Path) -> Result<PeerKey, PeerError> {
        let path = dir.join(Self::FILE_NAME);
        match load(&path) {
            Err(PeerError::KeyIo { source, .. }) if source.kind() == io::ErrorKind::NotFound => {
                generate(dir, &path)
            }
            loaded => loaded,
        }
    }

    /// The public node id: what the roster names and what peers authenticate this device as.
    pub fn node_id(&self) -> NodeId {
        self.secret.public()
    }

    /// The secret, for the endpoint builder only.
    pub(crate) fn secret(&self) -> &SecretKey {
        &self.secret
    }
}

impl fmt::Debug for PeerKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PeerKey")
            .field("node_id", &self.node_id().to_string())
            .finish_non_exhaustive()
    }
}

fn key_io(path: &Path) -> impl FnOnce(io::Error) -> PeerError + '_ {
    move |source| PeerError::KeyIo {
        path: path.to_path_buf(),
        source,
    }
}

/// Open, check the mode on the open handle (so the file checked is the file read), then read.
fn load(path: &Path) -> Result<PeerKey, PeerError> {
    let mut file = File::open(path).map_err(key_io(path))?;
    let meta = file.metadata().map_err(key_io(path))?;
    let mode = meta.permissions().mode() & 0o777;
    if mode & 0o077 != 0 {
        return Err(PeerError::KeyFileExposed {
            path: path.to_path_buf(),
            mode,
        });
    }
    if meta.len() != SECRET_LEN as u64 {
        return Err(PeerError::KeyFileCorrupt {
            path: path.to_path_buf(),
            len: meta.len(),
        });
    }
    let mut bytes = [0u8; SECRET_LEN];
    file.read_exact(&mut bytes).map_err(key_io(path))?;
    let secret = SecretKey::from_bytes(&bytes);
    bytes.fill(0);
    Ok(PeerKey { secret })
}

fn generate(dir: &Path, path: &Path) -> Result<PeerKey, PeerError> {
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)
        .map_err(key_io(dir))?;
    let secret = SecretKey::generate(rand::rngs::OsRng);
    let tmp = temp_sibling(path);
    write_private(&tmp, &secret.to_bytes()).map_err(key_io(&tmp))?;
    let published = fs::hard_link(&tmp, path);
    // The temp name is only a staging name; the published link (or the winner's file) remains.
    let removed = fs::remove_file(&tmp);
    match published {
        Ok(()) => {
            removed.map_err(key_io(&tmp))?;
            File::open(dir)
                .and_then(|d| d.sync_all())
                .map_err(key_io(dir))?;
            Ok(PeerKey { secret })
        }
        // A concurrent first start published first: its key is this device's key.
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => load(path),
        Err(e) => Err(key_io(path)(e)),
    }
}

/// Create `path` exclusively at mode 0600 and write `bytes` through to disk. On failure after
/// creation the partial file is removed.
fn write_private(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    let written = file.write_all(bytes).and_then(|()| file.sync_all());
    if written.is_err() {
        // Best effort: the write error is the one worth reporting.
        let _ = fs::remove_file(path);
    }
    written
}

/// An unpredictable staging name beside `path`, so concurrent starts never share one.
fn temp_sibling(path: &Path) -> PathBuf {
    let nonce: u64 = rand::random();
    path.with_file_name(format!(
        ".{}.{}.{nonce:016x}.tmp",
        PeerKey::FILE_NAME,
        std::process::id()
    ))
}
