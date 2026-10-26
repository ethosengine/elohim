//! The runtime directory (contract C3), the device lock, the hello line (C4) and the socket ends.
//!
//! One device-wide mux serves every `(lang, pin, root)` on `<runtime dir>/device.sock`. It holds
//! an exclusive `flock` on `device.lock` for its whole life, so a second resident refuses; the
//! kernel drops the lock however the holder dies, so a stale socket file is removed by the next
//! holder before it binds. Per-root files (`<lang>-<root12>.json`, `attrib-<lang>-<root12>.json`)
//! keep their C3 names.

use std::fs::File;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

use crate::attrib::SessionKey;

/// The first token of the hello line.
pub const HELLO_PREFIX: &str = "EPR-LSP-HELLO ";

/// The longest hello line accepted.
const MAX_HELLO: usize = 4096;

/// `root12`: the first 12 hex digits of sha256 over the canonical absolute checkout root.
#[must_use]
pub fn root12(root: &Path) -> String {
    let canonical = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let digest = Sha256::digest(canonical.to_string_lossy().as_bytes());
    digest
        .iter()
        .take(6)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The device-wide socket in the runtime directory.
#[must_use]
pub fn device_socket(dir: &Path) -> PathBuf {
    dir.join("device.sock")
}

/// The device mux's lock in the runtime directory.
#[must_use]
pub fn device_lock(dir: &Path) -> PathBuf {
    dir.join("device.lock")
}

/// The C2 edit log shared by every root of the checkout.
#[must_use]
pub fn edits_log(dir: &Path) -> PathBuf {
    dir.join("edits.jsonl")
}

/// The per-root files of one server in `<main checkout>/.eprfs/status/lsp/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeDir {
    /// The runtime directory.
    pub dir: PathBuf,
    /// The language.
    pub lang: String,
    /// The root's 12-hex key.
    pub root12: String,
}

impl RuntimeDir {
    /// The runtime files of `lang` over `root`, in `dir`.
    #[must_use]
    pub fn new(dir: PathBuf, lang: &str, root: &Path) -> RuntimeDir {
        RuntimeDir {
            dir,
            lang: lang.to_owned(),
            root12: root12(root),
        }
    }

    /// `<lang>-<root12>`.
    #[must_use]
    pub fn name(&self) -> String {
        format!("{}-{}", self.lang, self.root12)
    }

    /// The server record, present while that root's server runs.
    #[must_use]
    pub fn json(&self) -> PathBuf {
        self.dir.join(format!("{}.json", self.name()))
    }

    /// The attribution snapshot.
    #[must_use]
    pub fn attrib(&self) -> PathBuf {
        self.dir.join(format!("attrib-{}.json", self.name()))
    }
}

/// One root's server record (`<lang>-<root12>.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DaemonRecord {
    /// The device mux's pid.
    pub pid: u32,
    /// Its start time in clock ticks, as decimal text.
    pub start_time: Option<String>,
    /// The checkout root.
    pub root: String,
    /// The language.
    pub lang: String,
    /// The pinned toolchain.
    pub toolchain: String,
    /// The `CARGO_TARGET_DIR` slot, when any.
    pub target_dir: Option<String>,
    /// When this root's server started (RFC 3339).
    pub started_at: String,
    /// The device socket.
    pub sock: String,
}

/// The exclusive device lock, held for the mux's life and released when dropped.
#[derive(Debug)]
pub struct ServeLock {
    _file: File,
}

impl ServeLock {
    /// Take the lock without waiting: `None` when another process holds it.
    ///
    /// # Errors
    ///
    /// The lock file could not be created or locked for another reason.
    pub fn try_acquire(path: &Path) -> std::io::Result<Option<ServeLock>> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        match rustix::fs::flock(&file, rustix::fs::FlockOperation::NonBlockingLockExclusive) {
            Ok(()) => Ok(Some(ServeLock { _file: file })),
            Err(errno) if errno == rustix::io::Errno::WOULDBLOCK => Ok(None),
            Err(errno) => Err(errno.into()),
        }
    }
}

/// Why a hello line was refused.
#[derive(Debug, thiserror::Error)]
pub enum HelloError {
    /// The line does not start with [`HELLO_PREFIX`].
    #[error("connection did not open with EPR-LSP-HELLO")]
    Prefix,
    /// The JSON is malformed or names no session, root or language.
    #[error("hello is malformed: {0}")]
    Body(String),
    /// The reader failed or the line is too long.
    #[error("hello io: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Deserialize)]
struct HelloWire {
    v: u32,
    root: PathBuf,
    lang: String,
    #[serde(flatten)]
    key: SessionKey,
}

/// What a client says first on the device socket: who it is, and which server it wants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hello {
    /// The client's session (C1).
    pub key: SessionKey,
    /// The checkout root it wants diagnostics for (absolute).
    pub root: PathBuf,
    /// The language.
    pub lang: String,
}

/// The hello line: `EPR-LSP-HELLO {"v":1,"session",…,"root","lang"}\n`.
#[must_use]
pub fn hello_line(hello: &Hello) -> String {
    let body = json!({
        "v": 1,
        "session": hello.key.session,
        "ownerPid": hello.key.owner_pid,
        "ownerStartTime": hello.key.owner_start_time,
        "root": hello.root,
        "lang": hello.lang,
    });
    format!("{HELLO_PREFIX}{body}\n")
}

/// Parse a hello line (with or without its newline).
///
/// # Errors
///
/// [`HelloError::Prefix`] or [`HelloError::Body`].
pub fn parse_hello(line: &str) -> Result<Hello, HelloError> {
    let body = line
        .trim_end_matches(['\r', '\n'])
        .strip_prefix(HELLO_PREFIX)
        .ok_or(HelloError::Prefix)?;
    let hello: HelloWire =
        serde_json::from_str(body).map_err(|e| HelloError::Body(e.to_string()))?;
    if hello.v != 1 || hello.key.session.is_empty() || hello.lang.is_empty() {
        return Err(HelloError::Body(format!(
            "version {} session `{}` lang `{}`",
            hello.v, hello.key.session, hello.lang
        )));
    }
    if !hello.root.is_absolute() {
        return Err(HelloError::Body(format!(
            "root `{}` is not absolute",
            hello.root.display()
        )));
    }
    Ok(Hello {
        key: hello.key,
        root: hello.root,
        lang: hello.lang,
    })
}

/// Read the hello line byte by byte, so no LSP byte after it is consumed.
///
/// # Errors
///
/// An I/O error, an end of stream, an overlong line or a malformed hello.
pub async fn read_hello<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Hello, HelloError> {
    let mut line = Vec::new();
    loop {
        let byte = reader.read_u8().await?;
        if byte == b'\n' {
            break;
        }
        line.push(byte);
        if line.len() > MAX_HELLO {
            return Err(HelloError::Body("hello line too long".into()));
        }
    }
    parse_hello(&String::from_utf8_lossy(&line))
}

/// Connect to the device mux and say hello.
///
/// # Errors
///
/// The socket is absent, refuses, or the hello could not be written.
pub async fn connect(sock: &Path, hello: &Hello) -> std::io::Result<UnixStream> {
    let mut stream = UnixStream::connect(sock).await?;
    stream.write_all(hello_line(hello).as_bytes()).await?;
    Ok(stream)
}

/// Replace `path` atomically: write a sibling temp file, then rename over it.
///
/// # Errors
///
/// The write or the rename failed.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension(format!("tmp.{}", std::process::id()));
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hello(session: &str) -> Hello {
        Hello {
            key: SessionKey {
                session: session.into(),
                owner_pid: Some(12),
                owner_start_time: Some("345".into()),
            },
            root: PathBuf::from("/r"),
            lang: "rust".into(),
        }
    }

    #[test]
    fn hello_round_trips_and_refuses_strangers() {
        let line = hello_line(&hello("s-1"));
        assert!(line.ends_with('\n'));
        assert_eq!(parse_hello(&line).unwrap(), hello("s-1"));
        assert!(matches!(
            parse_hello("GET / HTTP/1.1"),
            Err(HelloError::Prefix)
        ));
        let body = |json: &str| format!("{HELLO_PREFIX}{json}");
        let refused = [
            r#"{"v":1,"session":"","root":"/r","lang":"rust"}"#,
            r#"{"v":2,"session":"x","root":"/r","lang":"rust"}"#,
            r#"{"v":1,"session":"x","root":"rel","lang":"rust"}"#,
            r#"{"v":1,"session":"x","lang":"rust"}"#,
            r#"{"v":1,"session":"x","root":"/r","lang":""}"#,
        ];
        for json in refused {
            assert!(parse_hello(&body(json)).is_err(), "{json}");
        }
        let numeric = parse_hello(&body(
            r#"{"v":1,"session":"x","ownerStartTime":9,"root":"/r","lang":"rust"}"#,
        ));
        assert_eq!(numeric.unwrap().key.owner_start_time.as_deref(), Some("9"));
    }

    #[tokio::test]
    async fn read_hello_stops_at_the_newline() {
        let mut bytes = hello_line(&hello("s")).into_bytes();
        bytes.extend_from_slice(b"Content-Length: 2\r\n\r\n{}");
        let mut reader = &bytes[..];
        assert_eq!(read_hello(&mut reader).await.unwrap().key.session, "s");
        assert_eq!(
            reader, b"Content-Length: 2\r\n\r\n{}",
            "LSP bytes left unread"
        );
    }

    #[test]
    fn root12_is_twelve_hex_and_distinguishes_roots() {
        let a = root12(Path::new("/projects/elohim"));
        let b = root12(Path::new("/projects/elohim/.worktrees/x"));
        assert_eq!(a.len(), 12);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
    }

    #[test]
    fn the_device_lock_admits_one_holder_until_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let lock = device_lock(dir.path());
        let first = ServeLock::try_acquire(&lock).unwrap();
        assert!(first.is_some());
        assert!(ServeLock::try_acquire(&lock).unwrap().is_none());
        drop(first);
        assert!(ServeLock::try_acquire(&lock).unwrap().is_some());
        assert!(device_socket(dir.path()).ends_with("device.sock"));
        let runtime = RuntimeDir::new(dir.path().to_path_buf(), "rust", dir.path());
        let attrib = runtime.attrib();
        assert!(attrib
            .to_string_lossy()
            .ends_with(&format!("attrib-rust-{}.json", runtime.root12)));
    }
}
