//! Expiring compute attachments, served by the existing native blob plane.
//! Source of truth: local operational custody leases (C), reconstructable from
//! the task/receipt retention policy and fetched bytes. Never a content head.
//! A separate directory prevents cleanup deleting an independently retained blob.
use crate::{blob_store::BlobStore, error::StorageError};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::OnceLock,
};
use tokio::{fs, io::AsyncWriteExt, sync::Mutex};

pub const CHUNK_LIMIT: usize = 1024 * 1024;
const TOMBSTONE_SECONDS: u64 = 30 * 86400;
static WRITES: OnceLock<Mutex<()>> = OnceLock::new();

#[derive(Default, Deserialize, Serialize)]
struct Leases {
    owners: BTreeMap<String, u64>,
}

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn paths(root: &Path, address: &str) -> Result<(PathBuf, PathBuf), StorageError> {
    let hex = BlobStore::parse_content_address(address)?;
    if hex.len() != 64 || !hex.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(StorageError::InvalidInput(
            "invalid compute chunk address".into(),
        ));
    }
    let base = root.join("compute-payloads");
    Ok((
        base.join(format!("{hex}.blob")),
        base.join(format!("{hex}.json")),
    ))
}

async fn leases(path: &Path) -> Result<Leases, StorageError> {
    match fs::read(path).await {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Leases::default()),
        Err(e) => Err(e.into()),
    }
}

async fn atomic(path: &Path, bytes: &[u8]) -> Result<(), StorageError> {
    let tmp = path.with_extension("tmp");
    let mut file = fs::File::create(&tmp).await?;
    file.write_all(bytes).await?;
    file.sync_all().await?;
    fs::rename(tmp, path).await?;
    Ok(())
}

pub async fn contains(root: &Path, address: &str) -> bool {
    let Ok((data, meta)) = paths(root, address) else {
        return false;
    };
    leases(&meta)
        .await
        .is_ok_and(|l| l.owners.values().any(|expiry| *expiry > now()))
        && fs::metadata(data).await.is_ok()
}

pub async fn expired(root: &Path, address: &str) -> bool {
    let Ok((_, meta)) = paths(root, address) else {
        return false;
    };
    fs::metadata(&meta).await.is_ok()
        && leases(&meta)
            .await
            .is_ok_and(|l| !l.owners.values().any(|expiry| *expiry > now()))
}

/// Reading one task's result must not extend its lifetime because another task
/// happens to reference identical bytes. None means this task has not fetched it.
pub async fn owner_expiry(root: &Path, address: &str, owner: &str) -> Option<u64> {
    let (_, meta) = paths(root, address).ok()?;
    leases(&meta).await.ok()?.owners.get(owner).copied()
}

pub async fn get(root: &Path, address: &str) -> Result<Vec<u8>, StorageError> {
    if !contains(root, address).await {
        return Err(StorageError::NotFound(address.into()));
    }
    let (data, _) = paths(root, address)?;
    let bytes = fs::read(data).await?;
    if bytes.len() > CHUNK_LIMIT
        || BlobStore::parse_content_address(address)?
            != BlobStore::compute_hash(&bytes).trim_start_matches("sha256-")
    {
        return Err(StorageError::InvalidInput("compute chunk corrupt".into()));
    }
    Ok(bytes)
}

/// Expire just one task's custody lease; overlapping tasks retain their copies.
pub async fn release(root: &Path, address: &str, owner: &str) -> Result<(), StorageError> {
    let _lock = WRITES.get_or_init(|| Mutex::new(())).lock().await;
    let (data, meta) = paths(root, address)?;
    let mut state = leases(&meta).await?;
    if let Some(expiry) = state.owners.get_mut(owner) {
        *expiry = (*expiry).min(now());
    }
    if fs::metadata(&meta).await.is_ok() {
        atomic(&meta, &serde_json::to_vec(&state)?).await?;
    }
    if !state.owners.values().any(|expiry| *expiry > now()) && fs::metadata(&data).await.is_ok() {
        fs::remove_file(data).await?;
    }
    Ok(())
}

pub async fn put(
    root: &Path,
    address: &str,
    owner: &str,
    ttl: u64,
    data: &[u8],
) -> Result<(), StorageError> {
    if owner.is_empty() || owner.len() > 256 || data.is_empty() || data.len() > CHUNK_LIMIT {
        return Err(StorageError::InvalidInput(
            "invalid compute payload lease or chunk size".into(),
        ));
    }
    if BlobStore::compute_cid(data).to_string() != address {
        return Err(StorageError::InvalidInput(
            "compute payload CID mismatch".into(),
        ));
    }
    let _lock = WRITES.get_or_init(|| Mutex::new(())).lock().await;
    cleanup_inner(root).await?;
    let (blob, meta) = paths(root, address)?;
    fs::create_dir_all(blob.parent().expect("payload parent")).await?;
    let mut state = leases(&meta).await?;
    state
        .owners
        .retain(|_, expiry| expiry.saturating_add(TOMBSTONE_SECONDS) > now());
    if state
        .owners
        .get(owner)
        .is_some_and(|expiry| *expiry <= now())
    {
        return Err(StorageError::InvalidInput(
            "compute owner lease expired".into(),
        ));
    }
    if state.owners.len() >= 128 && !state.owners.contains_key(owner) {
        return Err(StorageError::InvalidInput(
            "compute chunk lease limit".into(),
        ));
    }
    if !fs::try_exists(&blob).await? {
        let budget = std::env::var("ELOHIM_COMPUTE_PAYLOAD_BYTES")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(10 * 1024 * 1024 * 1024);
        let mut used = 0u64;
        let mut entries = fs::read_dir(blob.parent().expect("payload parent")).await?;
        while let Some(entry) = entries.next_entry().await? {
            if entry.path().extension().is_some_and(|e| e == "blob") {
                used = used.saturating_add(entry.metadata().await?.len());
            }
        }
        if used.saturating_add(data.len() as u64) > budget {
            return Err(StorageError::InvalidInput(
                "compute payload budget exhausted".into(),
            ));
        }
        atomic(&blob, data).await?;
    }
    state.owners.entry(owner.into()).or_insert_with(|| {
        if ttl == 0 {
            u64::MAX
        } else {
            now().saturating_add(ttl)
        }
    });
    atomic(&meta, &serde_json::to_vec(&state)?).await
}

pub async fn cleanup(root: &Path) -> Result<(), StorageError> {
    let _lock = WRITES.get_or_init(|| Mutex::new(())).lock().await;
    cleanup_inner(root).await
}

async fn cleanup_inner(root: &Path) -> Result<(), StorageError> {
    let mut entries = match fs::read_dir(root.join("compute-payloads")).await {
        Ok(dir) => dir,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "blob") {
            let meta = path.with_extension("json");
            if !leases(&meta)
                .await?
                .owners
                .values()
                .any(|expiry| *expiry > now())
            {
                fs::remove_file(path).await?;
            }
        } else if path.extension().is_some_and(|e| e == "json") {
            // Keep a short tombstone to distinguish expiry from unknown data,
            // then discard operational metadata too; the REA receipt survives.
            if !leases(&path)
                .await?
                .owners
                .values()
                .any(|expiry| expiry.saturating_add(TOMBSTONE_SECONDS) > now())
            {
                fs::remove_file(path).await?;
            }
        } else if path.extension().is_some_and(|e| e == "tmp") {
            fs::remove_file(path).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn expiry_never_deletes_independently_retained_content() {
        let dir = tempfile::tempdir().unwrap();
        let store = BlobStore::new(dir.path()).await.unwrap();
        let retained = store.store(b"evidence").await.unwrap();
        put(dir.path(), &retained.cid, "one", 60, b"evidence")
            .await
            .unwrap();
        put(dir.path(), &retained.cid, "two", 60, b"evidence")
            .await
            .unwrap();
        release(dir.path(), &retained.cid, "one").await.unwrap();
        assert_eq!(get(dir.path(), &retained.cid).await.unwrap(), b"evidence");
        release(dir.path(), &retained.cid, "two").await.unwrap();
        assert!(get(dir.path(), &retained.cid).await.is_err());
        assert_eq!(store.get(&retained.hash).await.unwrap(), b"evidence");
    }
    #[tokio::test]
    async fn expired_owner_stays_expired_when_identical_bytes_are_reused() {
        let dir = tempfile::tempdir().unwrap();
        let cid = BlobStore::compute_cid(b"shared").to_string();
        put(dir.path(), &cid, "old", 60, b"shared").await.unwrap();
        release(dir.path(), &cid, "old").await.unwrap();
        cleanup(dir.path()).await.unwrap();
        put(dir.path(), &cid, "new", 60, b"shared").await.unwrap();
        assert!(owner_expiry(dir.path(), &cid, "old").await.unwrap() <= now());
        assert!(owner_expiry(dir.path(), &cid, "new").await.unwrap() > now());
        assert!(put(dir.path(), &cid, "old", 60, b"shared").await.is_err());
        assert_eq!(get(dir.path(), &cid).await.unwrap(), b"shared");
    }
    #[tokio::test]
    async fn native_blob_reads_serve_only_unexpired_chunks() {
        let dir = tempfile::tempdir().unwrap();
        let store = BlobStore::new(dir.path()).await.unwrap();
        let cid = BlobStore::compute_cid(b"payload").to_string();
        put(dir.path(), &cid, "task", 60, b"payload").await.unwrap();
        assert_eq!(store.get_by_address(&cid).await.unwrap(), b"payload");
        let (_, meta) = paths(dir.path(), &cid).unwrap();
        fs::write(&meta, br#"{"owners":{"task":1}}"#).await.unwrap();
        assert!(!store.exists_by_address(&cid).await.unwrap());
        assert!(store.get_by_address(&cid).await.is_err());
        cleanup(dir.path()).await.unwrap();
        assert!(get(dir.path(), &cid).await.is_err());
        assert!(put(dir.path(), &cid, "task", 60, b"wrong").await.is_err());
    }
}
