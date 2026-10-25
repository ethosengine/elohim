use super::{Refusal, Result};
use eprfs_core::BlobCid;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    collections::BTreeMap,
    ffi::CString,
    fs::{self, File, OpenOptions},
    io::Write,
    os::{
        fd::AsRawFd,
        unix::fs::{symlink, PermissionsExt},
    },
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub(super) fn env(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
}
pub(super) fn absolute(path: &Path) -> Result<PathBuf> {
    let text = path.to_string_lossy();
    let path = if let Some(rest) = text.strip_prefix("~/") {
        PathBuf::from(env("HOME").ok_or_else(|| Refusal::new("path-invalid", "HOME unavailable"))?)
            .join(rest)
    } else {
        path.to_owned()
    };
    let path = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut result = PathBuf::new();
    for c in path.components() {
        match c {
            std::path::Component::ParentDir => {
                result.pop();
            }
            std::path::Component::CurDir => {}
            _ => result.push(c),
        }
    }
    Ok(result)
}
pub(crate) fn require_cid(text: &str) -> Result<BlobCid> {
    let cid = BlobCid::parse(text).map_err(|_| Refusal::new("invalid-cid", text))?;
    if cid.to_string() != text || !(text.starts_with("bafkrei") || text.starts_with("bafyrei")) {
        return Err(Refusal::new("invalid-cid", text));
    }
    Ok(cid)
}
/// Compute the canonical raw CID of a file's exact bytes.
pub fn file_cid(path: &Path) -> Result<String> {
    Ok(BlobCid::compute_raw(&fs::read(path)?).to_string())
}
/// Canonical DAG-CBOR mapping from release file names to their raw CID strings.
pub fn sums_manifest(sums: &BTreeMap<String, String>) -> Result<Vec<u8>> {
    for cid in sums.values() {
        require_cid(cid)?;
        if !cid.starts_with("bafkrei") {
            return Err(Refusal::new("invalid-cid", "file sums require raw CIDs"));
        }
    }
    BlobCid::encode_manifest(sums)
        .map(|(_, bytes)| bytes)
        .map_err(|e| Refusal::new("manifest-invalid", e.to_string()))
}
pub(super) fn executable(path: &Path) -> bool {
    path.metadata()
        .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}
pub(super) fn read_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}
pub(super) fn children(path: &Path) -> Vec<PathBuf> {
    fs::read_dir(path)
        .map(|it| it.flatten().map(|e| e.path()).collect())
        .unwrap_or_default()
}
pub(super) fn read_link(path: &Path) -> Option<String> {
    let link = fs::read_link(path).ok()?;
    // Store links must be relative CID names, never paths escaping the store.
    let text = link.to_str()?;
    require_cid(text).ok().map(|_| text.to_owned())
}
pub(super) fn temporary(path: &Path) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    path.with_file_name(format!(
        ".{}.{}.{}",
        path.file_name().unwrap_or_default().to_string_lossy(),
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}
pub(super) fn sync_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        File::open(parent)?.sync_all()?;
    }
    Ok(())
}
pub(crate) fn atomic_bytes(path: &Path, bytes: &[u8], mode: u32) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp = temporary(path);
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp)?;
        file.write_all(bytes)?;
        file.set_permissions(fs::Permissions::from_mode(mode))?;
        file.sync_all()?;
        fs::rename(&temp, path)?;
        sync_parent(path)
    })();
    let _ = fs::remove_file(temp);
    result
}
pub(crate) fn atomic_json(path: &Path, value: &impl Serialize) -> Result<()> {
    atomic_bytes(path, &serde_json::to_vec(value)?, 0o600)
}
pub(super) fn atomic_link(path: &Path, target: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp = temporary(path);
    let result = (|| {
        symlink(target, &temp)?;
        fs::rename(&temp, path)?;
        sync_parent(path)
    })();
    let _ = fs::remove_file(temp);
    result
}
pub(crate) struct Lock(File);
impl Lock {
    pub(crate) fn acquire(root: &Path, timeout: Duration) -> Result<Self> {
        Self::acquire_mode(root, timeout, libc::LOCK_EX)
    }
    pub(crate) fn shared(root: &Path, timeout: Duration) -> Result<Self> {
        Self::acquire_mode(root, timeout, libc::LOCK_SH)
    }
    fn acquire_mode(root: &Path, timeout: Duration, mode: libc::c_int) -> Result<Self> {
        fs::create_dir_all(root)?;
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(root.join(".lock"))?;
        let start = Instant::now();
        loop {
            // SAFETY: the fd is open and owned for the lifetime of Lock; flock doesn't retain pointers.
            if unsafe { libc::flock(file.as_raw_fd(), mode | libc::LOCK_NB) } == 0 {
                return Ok(Self(file));
            }
            let error = std::io::Error::last_os_error();
            if error.kind() != std::io::ErrorKind::WouldBlock {
                return Err(error.into());
            }
            if start.elapsed() >= timeout {
                return Err(Refusal::new(
                    "lock-timeout",
                    format!("store lock remained busy for {}s", timeout.as_secs_f64()),
                ));
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}
impl Drop for Lock {
    fn drop(&mut self) {
        // SAFETY: the owned file is still open here; unlocking uses no pointer arguments.
        unsafe {
            libc::flock(self.0.as_raw_fd(), libc::LOCK_UN);
        }
    }
}
pub(super) fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
pub(crate) fn now() -> String {
    let seconds = now_seconds() as libc::time_t;
    let mut tm = std::mem::MaybeUninit::<libc::tm>::uninit();
    // SAFETY: gmtime_r writes to tm and reads a valid time_t. Check success before reading tm.
    if unsafe { libc::gmtime_r(&seconds, tm.as_mut_ptr()) }.is_null() {
        return "1970-01-01T00:00:00Z".into();
    }
    // SAFETY: gmtime_r initialized all fields.
    let tm = unsafe { tm.assume_init() };
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        tm.tm_year + 1900,
        tm.tm_mon + 1,
        tm.tm_mday,
        tm.tm_hour,
        tm.tm_min,
        tm.tm_sec
    )
}
pub(super) fn parse_time(text: &str) -> Option<u64> {
    let input = CString::new(text).ok()?;
    // SAFETY: zero is valid for every field of the C tm structure.
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    // SAFETY: input and format are terminated C strings; tm is writable and lives through the call.
    let end = unsafe { libc::strptime(input.as_ptr(), c"%Y-%m-%dT%H:%M:%S".as_ptr(), &mut tm) };
    if end.is_null() {
        return None;
    }
    // Old records can include fractional seconds; they are irrelevant to the 14-day window.
    // SAFETY: tm was initialized and populated by strptime, timegm uses UTC and no shared buffer.
    u64::try_from(unsafe { libc::timegm(&mut tm) }).ok()
}
pub(super) fn touch(path: &Path) -> Result<()> {
    let file = OpenOptions::new().create(true).append(true).open(path)?;
    file.set_modified(SystemTime::now())?;
    Ok(())
}
pub(super) const MUTATION_WAIT: Duration = Duration::from_secs(60);
pub(super) const EXEC_WAIT: Duration = Duration::from_secs(5);
pub(super) const GRACE: u64 = 14 * 86400;
