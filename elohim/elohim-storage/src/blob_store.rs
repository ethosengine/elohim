//! Content-addressed byte store, byte profile epoch 0.
//!
//! Every payload is named by its byte CID (`raw 0x55 + BLAKE3-256`, `bafkr4i…`,
//! [`ByteCid`]). There is no string address API: callers parse a wire spelling with
//! [`ByteCid::parse`] (full decode) at their own boundary and hand this store the typed
//! value, so a `sha256-…`, `blake3-…` or `bafkrei…` spelling cannot reach a lookup.
//!
//! ## Layout (under the store root)
//!
//! - `blobs/<cid[7..11]>/<cid>`: the payload itself when it cuts into at most one chunk
//!   under the profile's chunker (inline), otherwise a **marker** naming the payload's
//!   layout record (see [`MARKER_MAGIC`]).
//! - `chunks/<cid[7..11]>/<cid>`: one chunk, named by its own byte CID. Chunks are shared
//!   between payloads that contain them.
//! - `index/<cid>`: the payload's dag-cbor layout record `{chunks, profile, byte_len,
//!   artifact_cid}` ([`LayoutRecord`]). The marker carries the record's CID, so a
//!   substituted or edited index is refused before a byte is read.
//! - `records/<cid[7..11]>/<cid>`: dag-cbor records (`0x71 + sha2-256`), the record lane.
//! - `quarantine/<cid>-<micros>/`: whatever failed verification, moved aside with a
//!   `reason` file. Nothing that fails verification is deleted.
//! - `tmp/<uuid>`: in-flight writes; emptied when the store opens.
//!
//! ## Write protocol
//!
//! Every file is written to `tmp/<uuid>`, `sync_all`ed, renamed into place and its parent
//! directory fsynced. A chunked payload writes its chunks, then its layout record, then
//! the marker **last**: the marker's presence is the claim that everything it names is
//! durable, so a crash at any earlier step leaves the payload absent.
//!
//! ## Read verification
//!
//! [`BlobStore::get`] recomputes the whole payload's BLAKE3 before returning a byte; a
//! mismatch moves the payload (and every chunk that fails its own CID) to quarantine and
//! returns [`StorageError::HashMismatch`]. Range reads are a full verified read and a slice;
//! verified ranges wait for BAO (sprint 2). [`BlobStore::exists`] is structural: marker,
//! layout record bound to the marker, and every chunk file at its declared length — or, for
//! an inline payload (at most one maximum chunk), the full hash.

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use bytes::Bytes;
use cid::Cid;
use elohim_epr::address::{compute_bytes_cid, compute_record_cid, ByteCid, RECORD_CODEC, SHA2_256};
use elohim_epr::profile::{
    chunk_bytes, chunk_stream, ByteProfile, ValidatedProfile, BYTE_PROFILE_0,
};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncRead;
use tracing::{info, warn};

use crate::error::StorageError;

/// The first bytes of a marker file. A marker is `MARKER_MAGIC ‖ layout-record CID bytes`.
pub const MARKER_MAGIC: &[u8; 16] = b"elohim-layout-v1";

/// A dag-cbor + sha2-256 CIDv1 is 36 bytes (`01 71 12 20` + digest).
const RECORD_CID_BYTES: usize = 36;

/// Length of every marker file.
const MARKER_LEN: usize = MARKER_MAGIC.len() + RECORD_CID_BYTES;

/// How many locks serialise mutations of the same payload. A CID always maps to the
/// same stripe, so a store and a delete of one payload never interleave and the tally
/// moves exactly once per payload created or removed.
const MUTATION_STRIPES: usize = 256;

/// The profile this store writes under. Local and constant, so `chunk_bytes` cannot
/// panic on it; profiles read back from a layout record are validated first.
pub const STORE_PROFILE: ByteProfile = BYTE_PROFILE_0;

/// One chunk of a payload: its byte CID and length.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkRef {
    /// The chunk's own byte CID.
    #[serde(with = "byte_cid_serde")]
    pub cid: ByteCid,
    /// The chunk's length in bytes (a chunk is at most the chunker's max, 4 MiB at epoch 0).
    pub len: u32,
}

/// What [`BlobStore::store`] reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreResult {
    /// The payload's byte CID.
    pub cid: ByteCid,
    /// The payload's length in bytes.
    pub byte_len: u64,
    /// The CID of the payload's layout record (computed for an inline payload too, so
    /// every caller can declare it; written to disk only for a chunked payload).
    pub layout_cid: Cid,
    /// The payload's chunks in order (one chunk naming the payload itself when inline,
    /// none for the empty payload).
    pub chunks: Vec<ChunkRef>,
    /// Whether a verified copy was already held.
    pub already_existed: bool,
}

/// What [`BlobStore::exists`] found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exists {
    /// Held and hash-verified: every byte hashed to its CID (inline, or every chunk
    /// and the whole of a chunked payload).
    Verified,
    /// Not held (including a write that never reached its marker).
    Absent,
    /// Held at some point, failed verification and is in quarantine; no copy is served.
    Quarantined,
}

/// What the hashing presence check concluded about a held payload.
enum Verdict {
    /// Every byte hashed to its CID.
    Whole,
    /// A definitive defect (missing part, wrong length, hash mismatch): quarantine.
    Defect(String),
    /// A read failed for another reason: not verified, and not evidence of corruption.
    Unknown(String),
}

/// What [`BlobStore::store_record`] reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordStoreResult {
    /// The record's CID (`dag-cbor 0x71 + sha2-256`).
    pub cid: Cid,
    /// The record's encoded length.
    pub byte_len: u64,
    /// Whether the record was already held.
    pub already_existed: bool,
}

/// A payload's layout record: written to `index/<cid>`, its CID bound in the marker.
///
/// Field order is the dag-cbor canonical key order (length first, then bytes) so the
/// encoding is the canonical one; the record CID is computed over the stored bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutRecord {
    /// `(chunk CID, length)` in payload order.
    pub chunks: Vec<(Cid, u32)>,
    /// The profile the chunks were cut under.
    pub profile: ByteProfile,
    /// The payload's length.
    pub byte_len: u64,
    /// The payload's byte CID.
    pub artifact_cid: Cid,
}

impl LayoutRecord {
    fn new(cid: &ByteCid, byte_len: u64, chunks: &[ChunkRef]) -> Self {
        Self {
            chunks: chunks.iter().map(|c| (*c.cid.as_cid(), c.len)).collect(),
            profile: STORE_PROFILE,
            byte_len,
            artifact_cid: *cid.as_cid(),
        }
    }

    fn encode(&self) -> Result<Vec<u8>, StorageError> {
        serde_ipld_dagcbor::to_vec(self)
            .map_err(|error| StorageError::Internal(format!("layout record encode: {error}")))
    }

    /// Decode and check a stored record against the payload it is filed under.
    fn decode_for(bytes: &[u8], cid: &ByteCid) -> Result<(Self, Vec<ChunkRef>), String> {
        let record: Self = serde_ipld_dagcbor::from_slice(bytes)
            .map_err(|error| format!("layout record decode: {error}"))?;
        record
            .profile
            .validate()
            .map_err(|error| format!("layout record profile: {error}"))?;
        let artifact = ByteCid::try_from(record.artifact_cid)
            .map_err(|error| format!("layout artifact cid: {}", error.check()))?;
        if &artifact != cid {
            return Err(format!("layout record names {artifact}, filed under {cid}"));
        }
        let mut chunks = Vec::with_capacity(record.chunks.len());
        let mut total = 0u64;
        for (chunk_cid, len) in &record.chunks {
            let chunk = ByteCid::try_from(*chunk_cid)
                .map_err(|error| format!("layout chunk cid: {}", error.check()))?;
            total += u64::from(*len);
            chunks.push(ChunkRef {
                cid: chunk,
                len: *len,
            });
        }
        if total != record.byte_len {
            return Err(format!(
                "layout chunks sum to {total}, record declares {}",
                record.byte_len
            ));
        }
        if chunks.len() < 2 {
            return Err("a layout record names at least two chunks".to_string());
        }
        Ok((record, chunks))
    }
}

/// Byte-CID serde in string form, parsed on read with [`ByteCid::parse`] (full decode).
pub mod byte_cid_serde {
    use elohim_epr::address::ByteCid;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(cid: &ByteCid, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(cid)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<ByteCid, D::Error> {
        let text = String::deserialize(deserializer)?;
        ByteCid::parse(&text).map_err(|error| serde::de::Error::custom(error.check()))
    }

    /// The same for a `Vec<ByteCid>`.
    pub mod vec {
        use elohim_epr::address::ByteCid;
        use serde::ser::SerializeSeq;
        use serde::{Deserialize, Deserializer, Serializer};

        pub fn serialize<S: Serializer>(
            cids: &[ByteCid],
            serializer: S,
        ) -> Result<S::Ok, S::Error> {
            let mut seq = serializer.serialize_seq(Some(cids.len()))?;
            for cid in cids {
                seq.serialize_element(&cid.to_string())?;
            }
            seq.end()
        }

        pub fn deserialize<'de, D: Deserializer<'de>>(
            deserializer: D,
        ) -> Result<Vec<ByteCid>, D::Error> {
            Vec::<String>::deserialize(deserializer)?
                .iter()
                .map(|text| {
                    ByteCid::parse(text).map_err(|error| serde::de::Error::custom(error.check()))
                })
                .collect()
        }
    }
}

/// Storage statistics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StorageStats {
    pub total_blobs: u64,
    pub total_bytes: u64,
    pub chunked_blobs: u64,
}

/// Blob storage manager. Cheap to share: the state lives behind one `Arc`.
pub struct BlobStore {
    inner: Arc<Inner>,
}

struct Inner {
    root: PathBuf,
    /// What [`BlobStore::stats`] reports, moved by every store and delete.
    tally: BlobTally,
    /// Per-CID mutation locks; see [`MUTATION_STRIPES`].
    stripes: Box<[Mutex<()>]>,
    /// Stores hold it shared while their chunks are unreferenced by any marker; a delete
    /// holds it exclusively while it decides which chunks no layout record names.
    chunk_gc: RwLock<()>,
    #[cfg(test)]
    faults: Mutex<tests::Faults>,
}

impl BlobStore {
    /// Open (or create) a store at `root_dir`.
    ///
    /// Empties `tmp/` (no write is in flight before the store exists) and walks the
    /// payload tree once, on a blocking thread, to seed the running tally.
    pub async fn new<P: AsRef<Path>>(root_dir: P) -> Result<Self, StorageError> {
        let root = root_dir.as_ref().to_path_buf();
        let inner = tokio::task::spawn_blocking(move || Inner::open(root))
            .await
            .map_err(|error| StorageError::Internal(format!("blob store open: {error}")))??;
        let store = Self {
            inner: Arc::new(inner),
        };
        let stats = store.stats();
        info!(
            path = %store.inner.root.display(),
            blobs = stats.total_blobs,
            bytes = stats.total_bytes,
            "Initialized blob store"
        );

        if std::env::var("ELOHIM_COMPUTE_LOCAL_API").as_deref() == Ok("1") {
            let payload_root = store.inner.root.clone();
            tokio::spawn(async move {
                let mut tick = tokio::time::interval(std::time::Duration::from_secs(60));
                loop {
                    tick.tick().await;
                    if let Err(error) = crate::compute_payload_store::cleanup(&payload_root).await {
                        warn!(%error, "compute payload cleanup failed");
                    }
                }
            });
        }
        Ok(store)
    }

    /// A store in a fresh temporary directory (tests).
    pub fn new_memory() -> Self {
        let root = std::env::temp_dir().join(format!("elohim-blobs-{}", uuid::Uuid::new_v4()));
        // A fresh directory under the system temp dir; failure here is a broken host.
        let inner = Inner::open(root).unwrap_or_else(|error| panic!("temp blob store: {error}"));
        Self {
            inner: Arc::new(inner),
        }
    }

    /// The store's root directory (free-space probes measure its filesystem).
    pub fn root_dir(&self) -> &Path {
        &self.inner.root
    }

    /// Store a payload, returning its byte CID, layout and chunks.
    pub async fn store(&self, data: &[u8]) -> Result<StoreResult, StorageError> {
        let inner = self.inner.clone();
        let data = Bytes::copy_from_slice(data);
        blocking(move || inner.store(&data)).await
    }

    /// Store a payload read from `reader`, holding at most one maximum chunk (plus the
    /// first chunk, kept until a second arrives) in memory.
    pub async fn store_stream<R>(&self, reader: R) -> Result<StoreResult, StorageError>
    where
        R: AsyncRead + Send + Unpin + 'static,
    {
        let inner = self.inner.clone();
        let handle = tokio::runtime::Handle::current();
        blocking(move || inner.store_stream(BlockingReader { handle, reader })).await
    }

    /// Read a whole payload, recomputing its BLAKE3 before returning a byte.
    pub async fn get(&self, cid: &ByteCid) -> Result<Bytes, StorageError> {
        let inner = self.inner.clone();
        let cid = *cid;
        blocking(move || inner.get(&cid)).await
    }

    /// Read one chunk by its own byte CID (a chunk file, or an inline payload, which is
    /// its own single chunk), verified against that CID.
    pub async fn get_chunk(&self, cid: &ByteCid) -> Result<Bytes, StorageError> {
        let inner = self.inner.clone();
        let cid = *cid;
        blocking(move || inner.get_chunk(&cid)).await
    }

    /// `[start, end)` of a payload: a full verified read, then a slice. `end` is clamped
    /// to the payload's length; `start` past it is an empty range.
    pub async fn get_range(
        &self,
        cid: &ByteCid,
        start: u64,
        end: u64,
    ) -> Result<Bytes, StorageError> {
        let full = self.get(cid).await?;
        let len = full.len() as u64;
        let end = end.min(len);
        let start = start.min(end);
        // Both bounds are clamped to `len`, which came from a `usize`.
        Ok(full.slice(start as usize..end as usize))
    }

    /// A payload's length, from the inline file or its layout record, without reading it.
    pub async fn size(&self, cid: &ByteCid) -> Result<u64, StorageError> {
        let inner = self.inner.clone();
        let cid = *cid;
        blocking(move || inner.size(&cid)).await
    }

    /// Verified presence; see [`Exists`]. Hashes every byte of the payload (BLAKE3,
    /// one chunk in memory at a time). A held payload that fails is moved to
    /// quarantine and reported [`Exists::Quarantined`].
    pub async fn exists(&self, cid: &ByteCid) -> Exists {
        let inner = self.inner.clone();
        let cid = *cid;
        tokio::task::spawn_blocking(move || inner.exists(&cid))
            .await
            .unwrap_or(Exists::Absent)
    }

    /// Structural presence with no payload hashing and no side effects (see the inner
    /// `present`). For advertising probes only; `exists` is the verified answer.
    pub async fn present(&self, cid: &ByteCid) -> bool {
        let inner = self.inner.clone();
        let cid = *cid;
        tokio::task::spawn_blocking(move || inner.present(&cid))
            .await
            .unwrap_or(false)
    }

    /// Remove a payload: marker (or inline file) first, then its layout record, then the
    /// chunks no remaining layout record names. Removing an absent payload is not an error.
    pub async fn delete(&self, cid: &ByteCid) -> Result<(), StorageError> {
        let inner = self.inner.clone();
        let cid = *cid;
        blocking(move || inner.delete(&cid)).await
    }

    /// Move a payload (marker or inline file, and its layout record) to quarantine.
    pub async fn quarantine(&self, cid: &ByteCid, reason: &str) -> Result<(), StorageError> {
        let inner = self.inner.clone();
        let cid = *cid;
        let reason = reason.to_string();
        blocking(move || inner.quarantine_payload(&cid, &reason)).await
    }

    /// Every payload CID filed under `blobs/`. Names that do not parse as a byte CID are
    /// skipped, as are zero-length files other than the empty payload's own CID (a
    /// zero-length file under any other name cannot verify, so it is never advertised).
    /// Synchronous walk: call from a blocking context when latency matters.
    pub fn list_cids(&self) -> Result<Vec<ByteCid>, StorageError> {
        let empty = compute_bytes_cid(&[]);
        let mut cids = Vec::new();
        walk_fanout(&self.inner.root.join("blobs"), |_, name, len| {
            if let Ok(cid) = ByteCid::parse(name) {
                if len > 0 || cid == empty {
                    cids.push(cid);
                }
            }
        });
        Ok(cids)
    }

    /// Every custody unit this store holds AND can read back verified: the chunk files
    /// under `chunks/` and the inline payloads under `blobs/` (an inline payload is its
    /// own single chunk, and a Reed-Solomon shard pushed to this node is stored as one).
    /// A chunked payload's marker is a payload, not a unit, and is skipped. Where
    /// [`BlobStore::list_cids`] names what `blobs/` holds, this hashes each candidate
    /// against its own CID (`get_chunk`): a unit that fails is quarantined, never listed.
    /// Synchronous and proportional to the bytes held: call from a blocking context.
    pub fn list_units(&self) -> Result<Vec<ByteCid>, StorageError> {
        let mut verified = Vec::new();
        for cid in self.unit_names() {
            if self.inner.get_chunk(&cid).is_ok() {
                verified.push(cid);
            }
        }
        Ok(verified)
    }

    /// Whether this store holds `cid` AND can read it back verified: a payload whose
    /// every chunk and whole hash (`exists` == `Verified`) or a single unit (a chunk, an
    /// inline payload or a shard) whose bytes hash to its CID. Hashes the bytes; a
    /// failure is quarantined by the underlying read. Synchronous: call from a blocking
    /// context. This is the question custody acts ask instead of "is it named?".
    pub fn holds_verified(&self, cid: &ByteCid) -> bool {
        self.inner.exists(cid) == Exists::Verified || self.inner.get_chunk(cid).is_ok()
    }

    /// Every CID this store can serve verified: the verified units of
    /// [`BlobStore::list_units`] plus each chunked payload whose marker verifies
    /// (`exists` == `Verified`). What a peer reconciling against this node may be told it
    /// holds; a named-but-corrupt payload or chunk is left out (and quarantined).
    pub fn list_servable(&self) -> Result<Vec<ByteCid>, StorageError> {
        let mut servable: std::collections::HashSet<ByteCid> =
            self.list_units()?.into_iter().collect();
        for cid in self.list_cids()? {
            if !servable.contains(&cid) && self.inner.exists(&cid) == Exists::Verified {
                servable.insert(cid);
            }
        }
        Ok(servable.into_iter().collect())
    }

    /// The names [`BlobStore::list_units`] would verify, with no byte read: a filename set,
    /// which is a candidate list for the caller to verify (`get_chunk`) one unit at a time
    /// before it records anything. Never evidence that a unit is whole.
    pub fn unit_names(&self) -> Vec<ByteCid> {
        let empty = compute_bytes_cid(&[]);
        let mut seen = std::collections::HashSet::new();
        for area in ["chunks", "blobs"] {
            walk_fanout(&self.inner.root.join(area), |_, name, len| {
                if let Ok(cid) = ByteCid::parse(name) {
                    if len > 0 || cid == empty {
                        seen.insert(cid);
                    }
                }
            });
        }
        seen.into_iter().collect()
    }

    /// Store a dag-cbor record (`0x71 + sha2-256`) on the record lane.
    pub async fn store_record(&self, canonical: &[u8]) -> Result<RecordStoreResult, StorageError> {
        let inner = self.inner.clone();
        let bytes = Bytes::copy_from_slice(canonical);
        blocking(move || inner.store_record(&bytes)).await
    }

    /// Read a dag-cbor record, verified against its CID. Any CID that is not
    /// `dag-cbor + sha2-256` is refused.
    pub async fn get_record(&self, cid: &Cid) -> Result<Bytes, StorageError> {
        let inner = self.inner.clone();
        let cid = *cid;
        blocking(move || inner.get_record(&cid)).await
    }

    /// Constant-time statistics from the running tally (seeded once at open).
    pub fn stats(&self) -> StorageStats {
        self.inner.tally.snapshot()
    }
}

/// Run a store operation on a blocking thread.
async fn blocking<T, F>(op: F) -> Result<T, StorageError>
where
    F: FnOnce() -> Result<T, StorageError> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(op)
        .await
        .map_err(|error| StorageError::Internal(format!("blob store task: {error}")))?
}

/// A synchronous `Read` over an `AsyncRead`, for the chunker on a blocking thread.
struct BlockingReader<R> {
    handle: tokio::runtime::Handle,
    reader: R,
}

impl<R: AsyncRead + Unpin> Read for BlockingReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        use tokio::io::AsyncReadExt;
        self.handle.block_on(self.reader.read(buf))
    }
}

/// Which tree a file lives in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Area {
    Blob,
    Chunk,
    Index,
    Record,
}

/// What a `blobs/` file is.
enum Entry {
    Inline(Vec<u8>),
    Marker(Cid),
}

/// How many times a read re-reads a payload a writer changed under it.
const READ_RETRIES: usize = 4;

/// Why one read attempt did not return bytes.
enum ReadFailure {
    /// A structural miss (layout record or chunk missing or the wrong size): settle it.
    Structural(String),
    /// A verdict already (not found, hash mismatch, I/O).
    Final(StorageError),
}

impl From<StorageError> for ReadFailure {
    fn from(error: StorageError) -> Self {
        Self::Final(error)
    }
}

/// What a look under the stripe found after a structural miss.
enum Settled {
    /// The payload verifies: the miss was a race with a writer.
    Healthy,
    /// The read's final error.
    Refused(StorageError),
}

impl Inner {
    fn open(root: PathBuf) -> Result<Self, StorageError> {
        for dir in ["blobs", "chunks", "index", "records", "quarantine", "tmp"] {
            fs::create_dir_all(root.join(dir))?;
        }
        // Crash residue: a write that never reached its rename.
        if let Ok(entries) = fs::read_dir(root.join("tmp")) {
            for entry in entries.flatten() {
                let _ = fs::remove_file(entry.path());
            }
        }
        let seeded = scan_blob_tree(&root);
        Ok(Self {
            root,
            tally: BlobTally::seeded(seeded),
            stripes: (0..MUTATION_STRIPES).map(|_| Mutex::new(())).collect(),
            chunk_gc: RwLock::new(()),
            #[cfg(test)]
            faults: Mutex::new(tests::Faults::default()),
        })
    }

    fn stripe(&self, cid: &ByteCid) -> std::sync::MutexGuard<'_, ()> {
        let digest = cid.digest();
        let index = (usize::from(digest[0]) << 8) | usize::from(digest[1]);
        lock(&self.stripes[index % self.stripes.len()])
    }

    fn blob_path(&self, cid: &ByteCid) -> PathBuf {
        fanout_path(&self.root.join("blobs"), &cid.to_string())
    }

    fn chunk_path(&self, cid: &ByteCid) -> PathBuf {
        fanout_path(&self.root.join("chunks"), &cid.to_string())
    }

    fn index_path(&self, cid: &ByteCid) -> PathBuf {
        self.root.join("index").join(cid.to_string())
    }

    fn record_path(&self, cid: &Cid) -> PathBuf {
        fanout_path(&self.root.join("records"), &cid.to_string())
    }

    /// Write `bytes` at `dest`: `tmp/<uuid>` → `sync_all` → rename → fsync the parent.
    /// A failure before the rename removes the temporary file.
    fn write_atomic(&self, area: Area, dest: &Path, bytes: &[u8]) -> io::Result<()> {
        let tmp = self.root.join("tmp").join(uuid::Uuid::new_v4().to_string());
        #[cfg(test)]
        let fault = self.next_fault();
        let written = (|| {
            let mut file = File::create(&tmp)?;
            #[cfg(test)]
            if let Some(tests::Fault::Enospc) = fault {
                file.write_all(&bytes[..bytes.len() / 2])?;
                return Err(io::Error::from_raw_os_error(libc::ENOSPC));
            }
            file.write_all(bytes)?;
            file.sync_all()
        })();
        if let Err(error) = written {
            let _ = fs::remove_file(&tmp);
            return Err(error);
        }
        #[cfg(test)]
        if let Some(tests::Fault::Crash) = fault {
            // A process that dies here leaves the synced temporary file and nothing else.
            return Err(io::Error::other("injected crash before rename"));
        }
        let parent = dest
            .parent()
            .ok_or_else(|| io::Error::other("destination has no parent"))?;
        self.ensure_bucket(parent)?;
        #[cfg(test)]
        self.event(tests::Event::Rename(dest.to_path_buf()));
        if let Err(error) = fs::rename(&tmp, dest) {
            let _ = fs::remove_file(&tmp);
            return Err(error);
        }
        File::open(parent)?.sync_all()?;
        #[cfg(test)]
        self.journal(area);
        #[cfg(test)]
        if let Some(tests::Fault::CrashAfter) = fault {
            // A process that dies here has made the write durable and told no one.
            return Err(io::Error::other("injected crash after the final fsync"));
        }
        let _ = area;
        Ok(())
    }

    /// Make sure the fan-out bucket `parent` exists and that its own directory entry is
    /// durable: a freshly created bucket is a new entry in `blobs/` (or `chunks/`), and
    /// an fsync of the bucket after the rename persists the file's entry, never the
    /// bucket's. Without syncing the bucket's parent a power loss can drop the bucket
    /// and with it every file renamed into it.
    fn ensure_bucket(&self, parent: &Path) -> io::Result<()> {
        if parent.is_dir() {
            return Ok(());
        }
        fs::create_dir_all(parent)?;
        if let Some(grandparent) = parent.parent() {
            File::open(grandparent)?.sync_all()?;
            #[cfg(test)]
            self.event(tests::Event::BucketSynced(grandparent.to_path_buf()));
        }
        Ok(())
    }

    fn store(&self, data: &[u8]) -> Result<StoreResult, StorageError> {
        let cid = compute_bytes_cid(data);
        let byte_len = data.len() as u64;
        let cuts = chunk_bytes(&ValidatedProfile::EPOCH_0, data);
        let chunks: Vec<ChunkRef> = cuts
            .iter()
            .map(|&(offset, len)| ChunkRef {
                cid: compute_bytes_cid(&data[offset..offset + len]),
                // A chunk is at most the chunker's max (4 MiB), well inside u32.
                len: len as u32,
            })
            .collect();
        let layout = LayoutRecord::new(&cid, byte_len, &chunks);
        let layout_bytes = layout.encode()?;
        let layout_cid = compute_record_cid(&layout_bytes);
        let chunk_refs = if chunks.len() <= 1 {
            // The inline payload is its own single chunk.
            chunks
                .first()
                .map(|_| {
                    vec![ChunkRef {
                        cid,
                        len: byte_len as u32,
                    }]
                })
                .unwrap_or_default()
        } else {
            chunks.clone()
        };
        let result = |already_existed| StoreResult {
            cid,
            byte_len,
            layout_cid,
            chunks: chunk_refs.clone(),
            already_existed,
        };

        let _gc = read_lock(&self.chunk_gc);
        let _stripe = self.stripe(&cid);
        // "Already exists" means exists AND verifies: `exists_locked` hashes every byte
        // it vouches for. Anything less (absent, quarantined just now) is rewritten
        // below; `put_chunk` replaces a held chunk that fails its CID.
        if self.exists_locked(&cid) == Exists::Verified {
            return Ok(result(true));
        }

        if chunks.len() <= 1 {
            self.write_atomic(Area::Blob, &self.blob_path(&cid), data)?;
            self.tally.add(Footprint::inline(byte_len));
        } else {
            for (chunk, &(offset, len)) in chunks.iter().zip(&cuts) {
                self.put_chunk(chunk, &data[offset..offset + len])?;
            }
            self.write_layout(&cid, byte_len, &layout_bytes, &layout_cid)?;
        }
        info!(cid = %cid, size = byte_len, chunks = chunk_refs.len(), "Stored blob");
        Ok(result(false))
    }

    /// Write one chunk unless a file already holds exactly its bytes. A held file that
    /// fails the chunk's CID is quarantined and rewritten, so a store heals it.
    fn put_chunk(&self, chunk: &ChunkRef, bytes: &[u8]) -> Result<(), StorageError> {
        let path = self.chunk_path(&chunk.cid);
        match fs::read(&path) {
            Ok(held) if compute_bytes_cid(&held) == chunk.cid => return Ok(()),
            Ok(_) => {
                self.quarantine_files(
                    &chunk.cid,
                    std::slice::from_ref(&path),
                    "held chunk fails its cid",
                )?;
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        self.write_atomic(Area::Chunk, &path, bytes)?;
        Ok(())
    }

    /// Write the layout record, then the marker, and count the payload.
    fn write_layout(
        &self,
        cid: &ByteCid,
        byte_len: u64,
        layout_bytes: &[u8],
        layout_cid: &Cid,
    ) -> Result<(), StorageError> {
        self.write_atomic(Area::Index, &self.index_path(cid), layout_bytes)?;
        let mut marker = Vec::with_capacity(MARKER_LEN);
        marker.extend_from_slice(MARKER_MAGIC);
        marker.extend_from_slice(&layout_cid.to_bytes());
        self.write_atomic(Area::Blob, &self.blob_path(cid), &marker)?;
        self.tally.add(Footprint::chunked(byte_len));
        Ok(())
    }

    fn store_stream<R: Read>(&self, reader: R) -> Result<StoreResult, StorageError> {
        let stream = chunk_stream(&STORE_PROFILE, reader)
            .map_err(|error| StorageError::Internal(format!("chunker: {error}")))?;
        let _gc = read_lock(&self.chunk_gc);
        let mut whole = blake3::Hasher::new();
        let mut chunks: Vec<ChunkRef> = Vec::new();
        let mut first: Option<Vec<u8>> = None;
        let mut byte_len = 0u64;
        for chunk in stream {
            let chunk = chunk?;
            whole.update(&chunk.data);
            byte_len += chunk.data.len() as u64;
            let chunk_ref = ChunkRef {
                cid: compute_bytes_cid(&chunk.data),
                len: chunk.data.len() as u32,
            };
            match chunks.len() {
                0 => first = Some(chunk.data),
                1 => {
                    // A second chunk: the payload is chunked, so the first is a chunk file.
                    if let Some(held) = first.take() {
                        self.put_chunk(&chunks[0], &held)?;
                    }
                    self.put_chunk(&chunk_ref, &chunk.data)?;
                }
                _ => self.put_chunk(&chunk_ref, &chunk.data)?,
            }
            chunks.push(chunk_ref);
        }
        let cid = ByteCid::from_digest(*whole.finalize().as_bytes());
        let layout = LayoutRecord::new(&cid, byte_len, &chunks);
        let layout_bytes = layout.encode()?;
        let layout_cid = compute_record_cid(&layout_bytes);

        let _stripe = self.stripe(&cid);
        let already_existed = self.exists_locked(&cid) == Exists::Verified;
        if chunks.len() <= 1 {
            let data = first.unwrap_or_default();
            if !already_existed {
                self.write_atomic(Area::Blob, &self.blob_path(&cid), &data)?;
                self.tally.add(Footprint::inline(byte_len));
            }
            let chunks = if data.is_empty() && chunks.is_empty() {
                Vec::new()
            } else {
                vec![ChunkRef {
                    cid,
                    len: byte_len as u32,
                }]
            };
            return Ok(StoreResult {
                cid,
                byte_len,
                layout_cid,
                chunks,
                already_existed,
            });
        }
        if !already_existed {
            self.write_layout(&cid, byte_len, &layout_bytes, &layout_cid)?;
        }
        Ok(StoreResult {
            cid,
            byte_len,
            layout_cid,
            chunks,
            already_existed,
        })
    }

    /// Read and classify `blobs/<cid>`. `Ok(None)` when absent.
    fn read_entry(&self, cid: &ByteCid) -> Result<Option<Entry>, StorageError> {
        let bytes = match fs::read(self.blob_path(cid)) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            // A failed read is not evidence of absence.
            Err(error) => return Err(error.into()),
        };
        Ok(Some(classify(cid, bytes)))
    }

    /// Load the layout record the marker names, refusing a substituted one.
    fn read_layout(&self, cid: &ByteCid, bound: &Cid) -> Result<Vec<ChunkRef>, String> {
        let bytes =
            fs::read(self.index_path(cid)).map_err(|error| format!("layout record: {error}"))?;
        let actual = compute_record_cid(&bytes);
        if &actual != bound {
            return Err(format!(
                "layout record {actual} is not the {bound} the marker binds"
            ));
        }
        LayoutRecord::decode_for(&bytes, cid).map(|(_, chunks)| chunks)
    }

    /// A whole verified read. A read holds no lock, so a racing delete or store can change
    /// the payload between its marker, layout record and chunks; such a structural miss is
    /// settled under the payload's stripe (see [`Inner::settle`]) and retried when the
    /// payload turns out to be whole, so a read returns the whole payload or a typed
    /// error, never a partial one.
    fn get(&self, cid: &ByteCid) -> Result<Bytes, StorageError> {
        for _ in 0..READ_RETRIES {
            match self.get_once(cid) {
                Ok(bytes) => return Ok(bytes),
                Err(ReadFailure::Final(error)) => return Err(error),
                Err(ReadFailure::Structural(reason)) => match self.settle(cid, &reason) {
                    Settled::Healthy => continue,
                    Settled::Refused(error) => return Err(error),
                },
            }
        }
        Err(StorageError::Internal(format!(
            "payload {cid} kept changing under the read"
        )))
    }

    fn get_once(&self, cid: &ByteCid) -> Result<Bytes, ReadFailure> {
        let Some(entry) = self.read_entry(cid)? else {
            return Err(StorageError::BlobNotFound(cid.to_string()).into());
        };
        let data = match entry {
            Entry::Inline(data) => data,
            Entry::Marker(bound) => {
                let chunks = match self.read_layout(cid, &bound) {
                    Ok(chunks) => chunks,
                    Err(reason) => return Err(ReadFailure::Structural(reason)),
                };
                let total: u64 = chunks.iter().map(|c| u64::from(c.len)).sum();
                let mut data = Vec::with_capacity(usize::try_from(total).unwrap_or(0));
                for chunk in &chunks {
                    match fs::read(self.chunk_path(&chunk.cid)) {
                        Ok(bytes) if bytes.len() == chunk.len as usize => {
                            data.extend_from_slice(&bytes)
                        }
                        Ok(bytes) => {
                            let reason = format!(
                                "chunk {} is {} bytes, not {}",
                                chunk.cid,
                                bytes.len(),
                                chunk.len
                            );
                            return Err(ReadFailure::Structural(reason));
                        }
                        Err(error) => {
                            let reason = format!("chunk {}: {error}", chunk.cid);
                            return Err(ReadFailure::Structural(reason));
                        }
                    }
                }
                data
            }
        };
        let actual = compute_bytes_cid(&data);
        if &actual != cid {
            self.quarantine_bad_chunks(cid);
            let reason = format!("whole payload hashes to {actual}");
            let _ = self.quarantine_payload(cid, &reason);
            return Err(StorageError::HashMismatch {
                expected: cid.to_string(),
                actual: actual.to_string(),
            }
            .into());
        }
        Ok(Bytes::from(data))
    }

    /// A read found `cid` structurally broken. Under the payload's stripe (a store or
    /// delete cannot interleave), look again: gone is a typed absence; whole is a read
    /// that raced a writer and should be retried; still broken is quarantined.
    fn settle(&self, cid: &ByteCid, reason: &str) -> Settled {
        let _stripe = self.stripe(cid);
        match self.read_entry(cid) {
            Ok(None) => return Settled::Refused(StorageError::BlobNotFound(cid.to_string())),
            Err(error) => return Settled::Refused(error),
            Ok(Some(_)) => {}
        }
        match self.exists_locked(cid) {
            Exists::Verified => Settled::Healthy,
            Exists::Absent => Settled::Refused(StorageError::BlobNotFound(cid.to_string())),
            Exists::Quarantined => Settled::Refused(StorageError::HashMismatch {
                expected: cid.to_string(),
                actual: format!("quarantined: {reason}"),
            }),
        }
    }

    /// Quarantine a payload that failed a structural check and build the read's error
    /// (a payload that is no longer there is a typed absence).
    fn refuse(&self, cid: &ByteCid, reason: &str) -> StorageError {
        match self.settle(cid, reason) {
            Settled::Refused(error) => error,
            Settled::Healthy => {
                StorageError::Internal(format!("payload {cid} changed during the read"))
            }
        }
    }

    /// After a whole-payload mismatch, find and quarantine every chunk that fails its own
    /// CID, so no later read assembles it again.
    fn quarantine_bad_chunks(&self, cid: &ByteCid) {
        let Ok(Some(Entry::Marker(bound))) = self.read_entry(cid) else {
            return;
        };
        let Ok(chunks) = self.read_layout(cid, &bound) else {
            return;
        };
        for chunk in chunks {
            let path = self.chunk_path(&chunk.cid);
            if let Ok(bytes) = fs::read(&path) {
                if compute_bytes_cid(&bytes) != chunk.cid {
                    let _ = self.quarantine_files(&chunk.cid, &[path], "chunk fails its own cid");
                }
            }
        }
    }

    fn get_chunk(&self, cid: &ByteCid) -> Result<Bytes, StorageError> {
        let path = self.chunk_path(cid);
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                // An inline payload is its own single chunk.
                return match self.read_entry(cid)? {
                    Some(Entry::Inline(data)) => self.verified(cid, data, Area::Blob),
                    _ => Err(StorageError::BlobNotFound(cid.to_string())),
                };
            }
            Err(error) => return Err(error.into()),
        };
        self.verified(cid, bytes, Area::Chunk)
    }

    /// Return `bytes` if they hash to `cid`, else quarantine their file and refuse.
    fn verified(&self, cid: &ByteCid, bytes: Vec<u8>, area: Area) -> Result<Bytes, StorageError> {
        let actual = compute_bytes_cid(&bytes);
        if &actual == cid {
            return Ok(Bytes::from(bytes));
        }
        let reason = format!("bytes hash to {actual}");
        match area {
            Area::Chunk => {
                let _ = self.quarantine_files(cid, &[self.chunk_path(cid)], &reason);
            }
            _ => {
                let _ = self.quarantine_payload(cid, &reason);
            }
        }
        Err(StorageError::HashMismatch {
            expected: cid.to_string(),
            actual: actual.to_string(),
        })
    }

    fn size(&self, cid: &ByteCid) -> Result<u64, StorageError> {
        match self.read_entry(cid)? {
            None => Err(StorageError::BlobNotFound(cid.to_string())),
            Some(Entry::Inline(data)) => Ok(data.len() as u64),
            Some(Entry::Marker(bound)) => self
                .read_layout(cid, &bound)
                .map(|chunks| chunks.iter().map(|c| u64::from(c.len)).sum())
                .map_err(|reason| self.refuse(cid, &reason)),
        }
    }

    fn exists(&self, cid: &ByteCid) -> Exists {
        let _stripe = self.stripe(cid);
        self.exists_locked(cid)
    }

    /// [`Self::exists`] with the CID's stripe held by the caller.
    ///
    /// `Verified` means the bytes were hashed: an inline payload against its CID, a
    /// chunked one chunk by chunk against each chunk's CID and as a whole against the
    /// payload CID (BLAKE3 streamed over the chunks, one chunk in memory at a time). A
    /// length check alone never answers `Verified`: a same-length corrupt chunk passes
    /// it. A chunk that fails its own CID is quarantined with the payload, so the next
    /// `store` rewrites it.
    fn exists_locked(&self, cid: &ByteCid) -> Exists {
        let entry = match self.read_entry(cid) {
            Ok(Some(entry)) => entry,
            Ok(None) | Err(_) => return self.absent_or_quarantined(cid),
        };
        let verdict = match entry {
            Entry::Inline(data) => {
                let actual = compute_bytes_cid(&data);
                if &actual == cid {
                    Verdict::Whole
                } else {
                    Verdict::Defect(format!("inline bytes hash to {actual}"))
                }
            }
            Entry::Marker(bound) => match fs::read(self.index_path(cid)) {
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    Verdict::Defect("layout record: missing".to_string())
                }
                // A failed read is not evidence of corruption (as in `read_entry`).
                Err(error) => Verdict::Unknown(format!("layout record: {error}")),
                Ok(_) => match self.read_layout(cid, &bound) {
                    Err(reason) => Verdict::Defect(reason),
                    Ok(chunks) => self.chunked_verdict(cid, &chunks),
                },
            },
        };
        match verdict {
            Verdict::Whole => Exists::Verified,
            Verdict::Defect(reason) => {
                warn!(cid = %cid, %reason, "held blob failed verification — quarantined");
                let _ = self.quarantine_payload(cid, &reason);
                Exists::Quarantined
            }
            Verdict::Unknown(reason) => {
                warn!(cid = %cid, %reason, "held blob could not be read — not verified, not quarantined");
                self.absent_or_quarantined(cid)
            }
        }
    }

    /// Hash every chunk of a chunked payload against its own CID and the whole against
    /// `cid`. Only a definitive defect — a missing chunk, a wrong length, a hash
    /// mismatch — condemns the payload; a chunk that fails its own CID is moved to
    /// quarantine before returning, so no later read or `store` reuses it. Any other
    /// read error is [`Verdict::Unknown`]: nothing is quarantined on a transient fault.
    fn chunked_verdict(&self, cid: &ByteCid, chunks: &[ChunkRef]) -> Verdict {
        let mut whole = blake3::Hasher::new();
        for chunk in chunks {
            let path = self.chunk_path(&chunk.cid);
            let bytes = match fs::read(&path) {
                Ok(bytes) => bytes,
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    return Verdict::Defect(format!("chunk {}: missing", chunk.cid))
                }
                Err(error) => return Verdict::Unknown(format!("chunk {}: {error}", chunk.cid)),
            };
            if bytes.len() as u64 != u64::from(chunk.len) {
                return Verdict::Defect(format!(
                    "chunk {} is {} bytes, not {}",
                    chunk.cid,
                    bytes.len(),
                    chunk.len
                ));
            }
            let actual = compute_bytes_cid(&bytes);
            if actual != chunk.cid {
                let _ = self.quarantine_files(
                    &chunk.cid,
                    std::slice::from_ref(&path),
                    "chunk fails its own cid",
                );
                return Verdict::Defect(format!("chunk {} hashes to {actual}", chunk.cid));
            }
            whole.update(&bytes);
        }
        let actual = ByteCid::from_digest(*whole.finalize().as_bytes());
        if &actual == cid {
            Verdict::Whole
        } else {
            Verdict::Defect(format!("whole payload hashes to {actual}"))
        }
    }

    /// Structural presence without hashing a payload byte and without side effects:
    /// the inline file, or the marker, the layout record it binds and every chunk file
    /// at its declared length. For probes that only advertise (`ShardRequest::Have`,
    /// `HEAD /shard`); a reader verifies on read, and `store`'s idempotence path uses the
    /// hashing [`Self::exists_locked`].
    fn present(&self, cid: &ByteCid) -> bool {
        let _stripe = self.stripe(cid);
        match self.read_entry(cid) {
            Ok(Some(Entry::Inline(_))) => true,
            Ok(Some(Entry::Marker(bound))) => self.read_layout(cid, &bound).is_ok_and(|chunks| {
                chunks.iter().all(|chunk| {
                    fs::metadata(self.chunk_path(&chunk.cid))
                        .is_ok_and(|meta| meta.len() == u64::from(chunk.len))
                })
            }),
            Ok(None) | Err(_) => false,
        }
    }

    fn absent_or_quarantined(&self, cid: &ByteCid) -> Exists {
        let prefix = format!("{cid}-");
        let quarantined = fs::read_dir(self.root.join("quarantine")).is_ok_and(|entries| {
            entries
                .flatten()
                .any(|entry| entry.file_name().to_string_lossy().starts_with(&prefix))
        });
        if quarantined {
            Exists::Quarantined
        } else {
            Exists::Absent
        }
    }

    /// Move a payload's `blobs/` entry and layout record to quarantine and uncount it.
    fn quarantine_payload(&self, cid: &ByteCid, reason: &str) -> Result<(), StorageError> {
        let footprint = self.footprint(cid);
        let moved =
            self.quarantine_files(cid, &[self.blob_path(cid), self.index_path(cid)], reason)?;
        if moved.first().copied().unwrap_or(false) {
            if let Some(footprint) = footprint {
                self.tally.remove(footprint);
            }
        }
        Ok(())
    }

    /// Move each existing file in `paths` into `quarantine/<cid>-<micros>/` beside a
    /// `reason` file. Returns, per path, whether it was moved.
    fn quarantine_files(
        &self,
        cid: &ByteCid,
        paths: &[PathBuf],
        reason: &str,
    ) -> Result<Vec<bool>, StorageError> {
        let micros = chrono::Utc::now().timestamp_micros();
        let dir = self.root.join("quarantine").join(format!("{cid}-{micros}"));
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("reason"), reason)?;
        let mut moved = Vec::with_capacity(paths.len());
        for (i, path) in paths.iter().enumerate() {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let area = path
                .parent()
                .and_then(|p| p.parent())
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let target = dir.join(format!("{i}-{area}-{name}"));
            moved.push(fs::rename(path, &target).is_ok());
        }
        if let Some(parent) = dir.parent() {
            let _ = File::open(parent).and_then(|d| d.sync_all());
        }
        warn!(cid = %cid, %reason, "quarantined");
        Ok(moved)
    }

    /// What a held payload contributes to the tally, read from disk.
    fn footprint(&self, cid: &ByteCid) -> Option<Footprint> {
        match self.read_entry(cid).ok()?? {
            Entry::Inline(data) => Some(Footprint::inline(data.len() as u64)),
            Entry::Marker(bound) => Some(Footprint::chunked(
                self.read_layout(cid, &bound)
                    .map(|chunks| chunks.iter().map(|c| u64::from(c.len)).sum())
                    .unwrap_or(0),
            )),
        }
    }

    fn delete(&self, cid: &ByteCid) -> Result<(), StorageError> {
        let _gc = write_lock(&self.chunk_gc);
        let _stripe = self.stripe(cid);
        let footprint = self.footprint(cid);
        let chunks = match self.read_entry(cid)? {
            Some(Entry::Marker(bound)) => self.read_layout(cid, &bound).unwrap_or_default(),
            _ => Vec::new(),
        };
        let blob = self.blob_path(cid);
        match fs::remove_file(&blob) {
            Ok(()) => {
                if let Some(parent) = blob.parent() {
                    File::open(parent)?.sync_all()?;
                }
                if let Some(footprint) = footprint {
                    self.tally.remove(footprint);
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        match fs::remove_file(self.index_path(cid)) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        if !chunks.is_empty() {
            let referenced = self.referenced_chunks();
            for chunk in chunks {
                if !referenced.contains(&chunk.cid) {
                    let _ = fs::remove_file(self.chunk_path(&chunk.cid));
                }
            }
        }
        info!(cid = %cid, "Deleted blob");
        Ok(())
    }

    /// Every chunk CID a remaining layout record names.
    fn referenced_chunks(&self) -> std::collections::HashSet<ByteCid> {
        let mut referenced = std::collections::HashSet::new();
        let Ok(entries) = fs::read_dir(self.root.join("index")) else {
            return referenced;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Ok(cid) = ByteCid::parse(&name.to_string_lossy()) else {
                continue;
            };
            if let Ok(bytes) = fs::read(entry.path()) {
                if let Ok((_, chunks)) = LayoutRecord::decode_for(&bytes, &cid) {
                    referenced.extend(chunks.into_iter().map(|c| c.cid));
                }
            }
        }
        referenced
    }

    fn store_record(&self, bytes: &[u8]) -> Result<RecordStoreResult, StorageError> {
        let cid = compute_record_cid(bytes);
        let path = self.record_path(&cid);
        let byte_len = bytes.len() as u64;
        if fs::read(&path).is_ok_and(|held| compute_record_cid(&held) == cid) {
            return Ok(RecordStoreResult {
                cid,
                byte_len,
                already_existed: true,
            });
        }
        self.write_atomic(Area::Record, &path, bytes)?;
        Ok(RecordStoreResult {
            cid,
            byte_len,
            already_existed: false,
        })
    }

    fn get_record(&self, cid: &Cid) -> Result<Bytes, StorageError> {
        if cid.codec() != RECORD_CODEC || cid.hash().code() != SHA2_256 {
            return Err(StorageError::InvalidContentAddress(format!(
                "{cid} is not a dag-cbor sha2-256 record cid"
            )));
        }
        let path = self.record_path(cid);
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err(StorageError::BlobNotFound(cid.to_string()))
            }
            Err(error) => return Err(error.into()),
        };
        let actual = compute_record_cid(&bytes);
        if &actual != cid {
            return Err(StorageError::HashMismatch {
                expected: cid.to_string(),
                actual: actual.to_string(),
            });
        }
        Ok(Bytes::from(bytes))
    }
}

/// Classify a `blobs/` file. A marker is exactly [`MARKER_LEN`] bytes, starts with
/// [`MARKER_MAGIC`] and does not itself hash to the CID it is filed under (an inline
/// payload whose bytes happen to look like a marker does).
fn classify(cid: &ByteCid, bytes: Vec<u8>) -> Entry {
    if bytes.len() == MARKER_LEN && bytes.starts_with(MARKER_MAGIC) {
        if let Ok(layout) = Cid::try_from(&bytes[MARKER_MAGIC.len()..]) {
            if &compute_bytes_cid(&bytes) != cid {
                return Entry::Marker(layout);
            }
        }
    }
    Entry::Inline(bytes)
}

/// `<root>/<name[7..11]>/<name>`: four base32 characters after the profile prefix.
fn fanout_path(root: &Path, name: &str) -> PathBuf {
    let bucket = name.get(7..11).unwrap_or("____");
    root.join(bucket).join(name)
}

/// Visit every file two levels under `root` with its name and length.
fn walk_fanout(root: &Path, mut visit: impl FnMut(&Path, &str, u64)) {
    let Ok(buckets) = fs::read_dir(root) else {
        return;
    };
    for bucket in buckets.flatten() {
        let Ok(entries) = fs::read_dir(bucket.path()) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            if meta.is_file() {
                visit(&path, &entry.file_name().to_string_lossy(), meta.len());
            }
        }
    }
}

/// Walk `blobs/` once (at open) for the tally: every parseable payload counts; a marker
/// counts its layout record's length.
fn scan_blob_tree(root: &Path) -> StorageStats {
    let mut stats = StorageStats::default();
    let index = root.join("index");
    walk_fanout(&root.join("blobs"), |path, name, len| {
        let Ok(cid) = ByteCid::parse(name) else {
            return;
        };
        let footprint = if len == MARKER_LEN as u64 {
            match fs::read(path).map(|bytes| classify(&cid, bytes)) {
                Ok(Entry::Marker(_)) => Footprint::chunked(
                    fs::read(index.join(name))
                        .ok()
                        .and_then(|bytes| LayoutRecord::decode_for(&bytes, &cid).ok())
                        .map_or(0, |(record, _)| record.byte_len),
                ),
                _ => Footprint::inline(len),
            }
        } else {
            Footprint::inline(len)
        };
        stats.total_blobs += 1;
        stats.total_bytes += footprint.bytes;
        if footprint.chunked {
            stats.chunked_blobs += 1;
        }
    });
    stats
}

fn lock(mutex: &Mutex<()>) -> std::sync::MutexGuard<'_, ()> {
    // The guarded value is `()`: a poisoned lock protects no state worth refusing over.
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn read_lock(lock: &RwLock<()>) -> std::sync::RwLockReadGuard<'_, ()> {
    lock.read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn write_lock(lock: &RwLock<()>) -> std::sync::RwLockWriteGuard<'_, ()> {
    lock.write()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// What one payload contributes to [`StorageStats`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Footprint {
    bytes: u64,
    chunked: bool,
}

impl Footprint {
    fn inline(bytes: u64) -> Self {
        Self {
            bytes,
            chunked: false,
        }
    }

    fn chunked(bytes: u64) -> Self {
        Self {
            bytes,
            chunked: true,
        }
    }
}

/// The running [`StorageStats`] of one store.
#[derive(Debug, Default)]
struct BlobTally {
    blobs: AtomicU64,
    bytes: AtomicU64,
    chunked: AtomicU64,
}

impl BlobTally {
    fn seeded(stats: StorageStats) -> Self {
        Self {
            blobs: AtomicU64::new(stats.total_blobs),
            bytes: AtomicU64::new(stats.total_bytes),
            chunked: AtomicU64::new(stats.chunked_blobs),
        }
    }

    fn add(&self, footprint: Footprint) {
        self.blobs.fetch_add(1, Ordering::Relaxed);
        self.bytes.fetch_add(footprint.bytes, Ordering::Relaxed);
        if footprint.chunked {
            self.chunked.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Subtract a payload, saturating at zero (a payload placed by another process after
    /// the seed walk was never counted).
    fn remove(&self, footprint: Footprint) {
        let saturating_sub = |counter: &AtomicU64, amount: u64| {
            // The closure always returns `Some`, so the update cannot fail.
            let _ = counter.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                Some(current.saturating_sub(amount))
            });
        };
        saturating_sub(&self.blobs, 1);
        saturating_sub(&self.bytes, footprint.bytes);
        if footprint.chunked {
            saturating_sub(&self.chunked, 1);
        }
    }

    fn snapshot(&self) -> StorageStats {
        StorageStats {
            total_blobs: self.blobs.load(Ordering::Relaxed),
            total_bytes: self.bytes.load(Ordering::Relaxed),
            chunked_blobs: self.chunked.load(Ordering::Relaxed),
        }
    }
}

#[cfg(test)]
impl Inner {
    fn next_fault(&self) -> Option<tests::Fault> {
        let mut faults = self
            .faults
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        faults.writes += 1;
        let n = faults.writes;
        if faults.enospc_at == Some(n) {
            return Some(tests::Fault::Enospc);
        }
        if faults.crash_at == Some(n) {
            return Some(tests::Fault::Crash);
        }
        if faults.crash_after == Some(n) {
            return Some(tests::Fault::CrashAfter);
        }
        None
    }

    fn event(&self, event: tests::Event) {
        self.faults
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .events
            .push(event);
    }

    fn journal(&self, area: Area) {
        self.faults
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .journal
            .push(area);
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use tempfile::TempDir;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(super) enum Fault {
        Enospc,
        Crash,
        /// The write completes and is durable; the process then dies before returning.
        CrashAfter,
    }

    /// What `write_atomic` did to the directory tree, in order.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(super) enum Event {
        /// A new fan-out bucket's parent was fsynced.
        BucketSynced(PathBuf),
        /// A file is about to be renamed to this destination.
        Rename(PathBuf),
    }

    #[derive(Debug, Default)]
    pub(super) struct Faults {
        pub writes: usize,
        pub enospc_at: Option<usize>,
        pub crash_at: Option<usize>,
        pub crash_after: Option<usize>,
        pub journal: Vec<Area>,
        pub events: Vec<Event>,
    }

    fn faults(store: &BlobStore) -> std::sync::MutexGuard<'_, Faults> {
        store.inner.faults.lock().unwrap()
    }

    /// The kernel's xorshift64* stream (seed `0x454c4f48494d`); every xorshift fixture in
    /// `tree-vectors.json` is a prefix of it.
    pub(crate) fn xorshift_bytes(len: usize) -> Vec<u8> {
        let mut state: u64 = 0x454c_4f48_494d;
        let mut out = Vec::with_capacity(len + 8);
        while out.len() < len {
            state ^= state >> 12;
            state ^= state << 25;
            state ^= state >> 27;
            out.extend_from_slice(&state.wrapping_mul(0x2545_F491_4F6C_DD1D).to_le_bytes());
        }
        out.truncate(len);
        out
    }

    /// A payload that cuts into several chunks.
    fn chunked_payload() -> Vec<u8> {
        xorshift_bytes(6 * 1024 * 1024)
    }

    async fn open() -> (TempDir, BlobStore) {
        let dir = TempDir::new().unwrap();
        let store = BlobStore::new(dir.path()).await.unwrap();
        (dir, store)
    }

    fn tmp_is_empty(dir: &TempDir) -> bool {
        fs::read_dir(dir.path().join("tmp"))
            .unwrap()
            .next()
            .is_none()
    }

    #[tokio::test]
    async fn store_is_idempotent_and_verified() {
        let (_dir, store) = open().await;
        for data in [b"Hello, Elohim!".to_vec(), chunked_payload(), Vec::new()] {
            let first = store.store(&data).await.unwrap();
            assert_eq!(first.cid, compute_bytes_cid(&data));
            assert!(first.cid.to_string().starts_with("bafkr4i"));
            assert_eq!(first.byte_len, data.len() as u64);
            assert!(!first.already_existed);
            let second = store.store(&data).await.unwrap();
            assert!(second.already_existed);
            assert_eq!(first.cid, second.cid);
            assert_eq!(first.layout_cid, second.layout_cid);
            assert_eq!(first.chunks, second.chunks);
            assert_eq!(store.exists(&first.cid).await, Exists::Verified);
            assert_eq!(&store.get(&first.cid).await.unwrap()[..], &data[..]);
            assert_eq!(store.size(&first.cid).await.unwrap(), data.len() as u64);
        }
        let stats = store.stats();
        assert_eq!(stats.total_blobs, 3);
        assert_eq!(stats.chunked_blobs, 1);
    }

    #[tokio::test]
    async fn inline_iff_at_most_one_chunk() {
        let (dir, store) = open().await;
        let small = store.store(b"one chunk").await.unwrap();
        assert_eq!(small.chunks.len(), 1);
        assert_eq!(
            small.chunks[0].cid, small.cid,
            "an inline payload is its own chunk"
        );
        let inline = fs::read(fanout_path(
            &dir.path().join("blobs"),
            &small.cid.to_string(),
        ));
        assert_eq!(inline.unwrap(), b"one chunk");
        assert!(!dir
            .path()
            .join("index")
            .join(small.cid.to_string())
            .exists());

        let big = store.store(&chunked_payload()).await.unwrap();
        assert!(big.chunks.len() >= 2);
        let marker = fs::read(fanout_path(&dir.path().join("blobs"), &big.cid.to_string()));
        assert_eq!(marker.unwrap().len(), MARKER_LEN);
        assert_eq!(
            store.get_chunk(&small.cid).await.unwrap(),
            &b"one chunk"[..]
        );
    }

    #[tokio::test]
    async fn marker_absent_until_all_chunks_fsynced() {
        let (_dir, store) = open().await;
        let result = store.store(&chunked_payload()).await.unwrap();
        let journal = faults(&store).journal.clone();
        let marker_at = journal.iter().rposition(|a| *a == Area::Blob).unwrap();
        let index_at = journal.iter().position(|a| *a == Area::Index).unwrap();
        let chunk_writes: Vec<usize> = journal
            .iter()
            .enumerate()
            .filter(|(_, a)| **a == Area::Chunk)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(chunk_writes.len(), result.chunks.len());
        assert_eq!(
            marker_at,
            journal.len() - 1,
            "the marker is the last durable write"
        );
        assert!(chunk_writes
            .iter()
            .all(|i| *i < index_at && index_at < marker_at));
    }

    #[tokio::test]
    async fn layout_record_cid_is_bound_in_marker() {
        let (dir, store) = open().await;
        let result = store.store(&chunked_payload()).await.unwrap();
        let marker = fs::read(fanout_path(
            &dir.path().join("blobs"),
            &result.cid.to_string(),
        ))
        .unwrap();
        assert_eq!(&marker[..MARKER_MAGIC.len()], MARKER_MAGIC);
        let bound = Cid::try_from(&marker[MARKER_MAGIC.len()..]).unwrap();
        assert_eq!(bound, result.layout_cid);
        let index = fs::read(dir.path().join("index").join(result.cid.to_string())).unwrap();
        assert_eq!(compute_record_cid(&index), bound);
        let (record, chunks) = LayoutRecord::decode_for(&index, &result.cid).unwrap();
        assert_eq!(record.profile, BYTE_PROFILE_0);
        assert_eq!(chunks, result.chunks);
    }

    /// Rewrite a chunked payload's layout record and re-bind the marker to it, as a writer
    /// with access to the store's disk could.
    fn rebind(dir: &TempDir, cid: &ByteCid, record: &LayoutRecord) {
        let bytes = record.encode().unwrap();
        fs::write(dir.path().join("index").join(cid.to_string()), &bytes).unwrap();
        let mut marker = MARKER_MAGIC.to_vec();
        marker.extend_from_slice(&compute_record_cid(&bytes).to_bytes());
        fs::write(
            fanout_path(&dir.path().join("blobs"), &cid.to_string()),
            marker,
        )
        .unwrap();
    }

    #[tokio::test]
    async fn reordered_valid_chunks_are_refused_by_whole_blob_verify() {
        let (dir, store) = open().await;
        let data = chunked_payload();
        let result = store.store(&data).await.unwrap();
        let mut chunks = result.chunks.clone();
        chunks.swap(0, 1);
        rebind(
            &dir,
            &result.cid,
            &LayoutRecord::new(&result.cid, result.byte_len, &chunks),
        );
        // Every chunk is valid and present, so the structure passes...
        let err = store.get(&result.cid).await.unwrap_err();
        // ...and only the whole-payload BLAKE3 refuses the reordering.
        assert!(matches!(err, StorageError::HashMismatch { .. }), "{err:?}");
        assert_eq!(store.exists(&result.cid).await, Exists::Quarantined);
    }

    #[tokio::test]
    async fn substituted_index_is_refused() {
        let (dir, store) = open().await;
        let result = store.store(&chunked_payload()).await.unwrap();
        let mut chunks = result.chunks.clone();
        chunks.swap(0, 1);
        let substitute = LayoutRecord::new(&result.cid, result.byte_len, &chunks)
            .encode()
            .unwrap();
        // Replace the index only: the marker still binds the original record.
        fs::write(
            dir.path().join("index").join(result.cid.to_string()),
            substitute,
        )
        .unwrap();
        assert!(matches!(
            store.get(&result.cid).await,
            Err(StorageError::HashMismatch { .. })
        ));
        assert_eq!(store.exists(&result.cid).await, Exists::Quarantined);
    }

    #[tokio::test]
    async fn corrupt_chunk_is_quarantined_not_served() {
        let (dir, store) = open().await;
        let data = chunked_payload();
        let result = store.store(&data).await.unwrap();
        let victim = result.chunks[1];
        let path = fanout_path(&dir.path().join("chunks"), &victim.cid.to_string());
        let mut bytes = fs::read(&path).unwrap();
        bytes[7] ^= 0xff; // same length: the structural check cannot see it
        fs::write(&path, &bytes).unwrap();

        // `exists` hashes every chunk (R3): a same-length corruption is never Verified.
        assert_eq!(store.exists(&result.cid).await, Exists::Quarantined);
        assert!(store.get(&result.cid).await.is_err());
        assert!(!path.exists(), "the corrupt chunk left the serving tree");
        assert!(store.get_chunk(&victim.cid).await.is_err());
        assert_eq!(store.exists(&result.cid).await, Exists::Quarantined);
        let quarantined = fs::read_dir(dir.path().join("quarantine")).unwrap().count();
        assert!(
            quarantined >= 2,
            "payload and chunk both kept in quarantine"
        );

        // Fresh bytes heal it.
        let healed = store.store(&data).await.unwrap();
        assert!(!healed.already_existed);
        assert_eq!(&store.get(&result.cid).await.unwrap()[..], &data[..]);
    }

    #[tokio::test]
    async fn corrupt_inline_payload_is_quarantined_not_served() {
        let (dir, store) = open().await;
        let result = store.store(b"inline payload").await.unwrap();
        let path = fanout_path(&dir.path().join("blobs"), &result.cid.to_string());
        fs::write(&path, b"inline paYload").unwrap();
        assert!(store.get(&result.cid).await.is_err());
        assert!(!path.exists());
        assert_eq!(store.exists(&result.cid).await, Exists::Quarantined);
        assert_eq!(store.stats().total_blobs, 0);
    }

    #[test]
    fn sha256_spelling_is_refused_at_type_boundary() {
        let sha = format!("sha256-{}", "a".repeat(64));
        assert!(ByteCid::parse(&sha).is_err());
        assert!(ByteCid::parse(&"a".repeat(64)).is_err());
        assert!(ByteCid::parse(&format!("blake3-{}", "a".repeat(64))).is_err());
        // A raw sha2-256 CID (the citation profile) is not a byte CID either.
        let cite = elohim_epr::address::compute_raw_sha2_cid(b"hello world").to_string();
        assert!(cite.starts_with("bafkrei"));
        assert!(ByteCid::parse(&cite).is_err());
        // The one admitted spelling.
        let byte = compute_bytes_cid(b"hello world");
        assert_eq!(ByteCid::parse(&byte.to_string()).unwrap(), byte);
    }

    #[tokio::test]
    async fn chunk_boundaries_match_kernel_vector() {
        // gate-reads: elohim/eprfs/fixtures/tree-vectors.json
        let vectors: serde_json::Value =
            serde_json::from_str(include_str!("../../eprfs/fixtures/tree-vectors.json")).unwrap();
        let main = vectors["chunker"]["vectors"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["name"] == "main-10MiB")
            .unwrap();
        let data = xorshift_bytes(main["length"].as_u64().unwrap() as usize);
        let (_dir, store) = open().await;
        let result = store.store(&data).await.unwrap();
        assert_eq!(result.cid.to_string(), main["wholeCid"].as_str().unwrap());
        let expected: Vec<(u64, u64, String)> = main["chunks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| {
                (
                    c[0].as_u64().unwrap(),
                    c[1].as_u64().unwrap(),
                    c[2].as_str().unwrap().to_string(),
                )
            })
            .collect();
        let mut offset = 0u64;
        let actual: Vec<(u64, u64, String)> = result
            .chunks
            .iter()
            .map(|c| {
                let row = (offset, u64::from(c.len), c.cid.to_string());
                offset += u64::from(c.len);
                row
            })
            .collect();
        assert_eq!(actual, expected);

        // The streaming path cuts the same boundaries.
        let (_dir2, streamed) = open().await;
        let reader = std::io::Cursor::new(data);
        let from_stream = streamed.store_stream(reader).await.unwrap();
        assert_eq!(from_stream.chunks, result.chunks);
        assert_eq!(from_stream.layout_cid, result.layout_cid);
    }

    #[tokio::test]
    async fn crash_mid_write_leaves_absent() {
        let dir = TempDir::new().unwrap();
        let data = chunked_payload();
        let cid = compute_bytes_cid(&data);
        {
            let store = BlobStore::new(dir.path()).await.unwrap();
            faults(&store).crash_at = Some(2);
            assert!(store.store(&data).await.is_err());
            assert_eq!(store.exists(&cid).await, Exists::Absent);
            assert!(
                !tmp_is_empty(&dir),
                "the dead write's temporary file is still there"
            );
        }
        // Restart: the residue is swept, the payload is still absent, a retry lands it.
        let store = BlobStore::new(dir.path()).await.unwrap();
        assert!(tmp_is_empty(&dir));
        assert_eq!(store.exists(&cid).await, Exists::Absent);
        assert_eq!(store.stats(), StorageStats::default());
        store.store(&data).await.unwrap();
        assert_eq!(store.exists(&cid).await, Exists::Verified);
    }

    #[tokio::test]
    async fn crash_before_marker_leaves_absent() {
        let (_dir, store) = open().await;
        let data = chunked_payload();
        let chunk_count = chunk_bytes(&ValidatedProfile::EPOCH_0, &data).len();
        // Every chunk and the index land; the marker write dies.
        faults(&store).crash_at = Some(chunk_count + 2);
        assert!(store.store(&data).await.is_err());
        assert_eq!(
            store.exists(&compute_bytes_cid(&data)).await,
            Exists::Absent
        );
    }

    #[tokio::test]
    async fn enospc_returns_error_and_no_file() {
        let (dir, store) = open().await;
        faults(&store).enospc_at = Some(1);
        let err = store.store(b"no room").await.unwrap_err();
        assert!(
            matches!(&err, StorageError::Io(e) if e.raw_os_error() == Some(libc::ENOSPC)),
            "{err:?}"
        );
        let cid = compute_bytes_cid(b"no room");
        assert!(!fanout_path(&dir.path().join("blobs"), &cid.to_string()).exists());
        assert!(tmp_is_empty(&dir), "the partial temporary file was removed");
        assert_eq!(store.exists(&cid).await, Exists::Absent);
        assert_eq!(store.stats(), StorageStats::default());
    }

    #[tokio::test]
    async fn cleanup_failure_is_caught_by_verify_on_exists() {
        let (dir, store) = open().await;
        // A chunked payload whose delete died after removing a chunk but before the
        // marker (the order an older cleanup used): the marker still claims it.
        let chunked = store.store(&chunked_payload()).await.unwrap();
        fs::remove_file(fanout_path(
            &dir.path().join("chunks"),
            &chunked.chunks[0].cid.to_string(),
        ))
        .unwrap();
        assert_eq!(store.exists(&chunked.cid).await, Exists::Quarantined);
        assert!(store.get(&chunked.cid).await.is_err());

        // A truncated file left at an inline payload's final path by a failed cleanup.
        let inline = compute_bytes_cid(b"truncated by a failed cleanup");
        let path = fanout_path(&dir.path().join("blobs"), &inline.to_string());
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"truncated by").unwrap();
        assert_eq!(store.exists(&inline).await, Exists::Quarantined);
        assert!(!store.list_cids().unwrap().contains(&inline));
    }

    #[tokio::test]
    async fn delete_keeps_chunks_another_payload_names() {
        let (_dir, store) = open().await;
        let base = chunked_payload();
        let mut extended = base.clone();
        extended.extend_from_slice(&xorshift_bytes(9 * 1024 * 1024)[6 * 1024 * 1024..]);
        let a = store.store(&base).await.unwrap();
        let b = store.store(&extended).await.unwrap();
        let shared = a.chunks.iter().filter(|c| b.chunks.contains(c)).count();
        assert!(shared > 0, "a common prefix shares chunks under CDC");
        store.delete(&a.cid).await.unwrap();
        assert_eq!(store.exists(&a.cid).await, Exists::Absent);
        assert_eq!(&store.get(&b.cid).await.unwrap()[..], &extended[..]);
        store.delete(&b.cid).await.unwrap();
        store.delete(&b.cid).await.unwrap();
        assert_eq!(store.stats(), StorageStats::default());
    }

    #[tokio::test]
    async fn list_cids_and_stats_follow_the_tree() {
        let dir = TempDir::new().unwrap();
        let store = BlobStore::new(dir.path()).await.unwrap();
        let a = store.store(b"listed").await.unwrap();
        let b = store.store(&chunked_payload()).await.unwrap();
        let empty = store.store(b"").await.unwrap();
        let mut listed = store.list_cids().unwrap();
        listed.sort();
        let mut expected = vec![a.cid, b.cid, empty.cid];
        expected.sort();
        assert_eq!(listed, expected);
        let reopened = BlobStore::new(dir.path()).await.unwrap();
        assert_eq!(reopened.stats(), store.stats());
        assert_eq!(
            store.stats().total_bytes,
            6 + chunked_payload().len() as u64
        );
    }

    #[tokio::test]
    async fn range_reads_are_full_verified_reads() {
        let (_dir, store) = open().await;
        let data = chunked_payload();
        let r = store.store(&data).await.unwrap();
        let range = store.get_range(&r.cid, 100, 5_000_000).await.unwrap();
        assert_eq!(&range[..], &data[100..5_000_000]);
        assert!(store.get_range(&r.cid, 10, 5).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn record_lane_verifies_records_and_refuses_byte_cids() {
        let (_dir, store) = open().await;
        let canonical = serde_ipld_dagcbor::to_vec(&vec![1u8, 2, 3]).unwrap();
        let stored = store.store_record(&canonical).await.unwrap();
        assert_eq!(stored.cid.codec(), RECORD_CODEC);
        assert_eq!(
            &store.get_record(&stored.cid).await.unwrap()[..],
            &canonical[..]
        );
        assert!(
            store
                .store_record(&canonical)
                .await
                .unwrap()
                .already_existed
        );
        let byte = *compute_bytes_cid(&canonical).as_cid();
        assert!(matches!(
            store.get_record(&byte).await,
            Err(StorageError::InvalidContentAddress(_))
        ));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_stores_of_one_blob_count_it_once() {
        let (_dir, store) = open().await;
        let store = Arc::new(store);
        let writers: Vec<_> = (0..16)
            .map(|_| {
                let store = store.clone();
                tokio::spawn(async move { store.store(b"raced blob").await.unwrap() })
            })
            .collect();
        let mut created = 0;
        for writer in writers {
            if !writer.await.unwrap().already_existed {
                created += 1;
            }
        }
        assert_eq!(created, 1, "exactly one writer creates the blob");
        assert_eq!(store.stats().total_blobs, 1);
    }

    // ── R3: "exists" means exists and verifies ──────────────────────────────

    /// Flip one byte of a held chunk in place, keeping its length.
    fn corrupt_chunk_in_place(dir: &TempDir, chunk: &ByteCid, at: usize) -> PathBuf {
        let path = fanout_path(&dir.path().join("chunks"), &chunk.to_string());
        let mut bytes = fs::read(&path).unwrap();
        let at = at.min(bytes.len() - 1);
        bytes[at] ^= 0x01;
        fs::write(&path, &bytes).unwrap();
        path
    }

    #[tokio::test]
    async fn same_length_corrupt_chunk_is_never_reported_verified() {
        let data = chunked_payload();
        for (victim, at) in [(0usize, 0usize), (1, 7), (usize::MAX, usize::MAX)] {
            let (dir, store) = open().await;
            let result = store.store(&data).await.unwrap();
            assert!(result.chunks.len() > 2, "fixture must be chunked");
            let victim = result.chunks[victim.min(result.chunks.len() - 1)].cid;
            let path = corrupt_chunk_in_place(&dir, &victim, at);
            assert_eq!(
                fs::metadata(&path).unwrap().len(),
                result
                    .chunks
                    .iter()
                    .find(|c| c.cid == victim)
                    .map(|c| u64::from(c.len))
                    .unwrap(),
                "the corruption keeps the structural length"
            );
            let verdict = store.exists(&result.cid).await;
            assert_ne!(verdict, Exists::Verified, "chunk {victim} at {at}");
            assert_eq!(verdict, Exists::Quarantined);
            assert!(
                !path.exists(),
                "the failing chunk is out of the serving tree"
            );
            // Asked again, it stays not-Verified (the payload is in quarantine).
            assert_ne!(store.exists(&result.cid).await, Exists::Verified);
        }
    }

    #[tokio::test]
    async fn store_over_a_structurally_present_corrupt_object_repairs_it() {
        // Chunked: the marker, the layout record and every chunk file are present at
        // their declared lengths, but one chunk's bytes are wrong.
        let (dir, store) = open().await;
        let data = chunked_payload();
        let first = store.store(&data).await.unwrap();
        let victim = first.chunks[1].cid;
        corrupt_chunk_in_place(&dir, &victim, 3);

        let again = store.store(&data).await.unwrap();
        assert!(
            !again.already_existed,
            "a corrupt object is not 'already stored': it is rewritten"
        );
        assert_eq!(store.exists(&first.cid).await, Exists::Verified);
        assert_eq!(&store.get(&first.cid).await.unwrap()[..], &data[..]);
        let healed =
            fs::read(fanout_path(&dir.path().join("chunks"), &victim.to_string())).unwrap();
        assert_eq!(
            compute_bytes_cid(&healed),
            victim,
            "the chunk file is whole again"
        );
        assert_eq!(store.stats().total_blobs, 1, "counted once");

        // Inline: same length, wrong bytes.
        let (dir, store) = open().await;
        let small = b"an inline payload".to_vec();
        let held = store.store(&small).await.unwrap();
        let path = fanout_path(&dir.path().join("blobs"), &held.cid.to_string());
        let mut bytes = fs::read(&path).unwrap();
        bytes[0] ^= 0x01;
        fs::write(&path, &bytes).unwrap();
        let again = store.store(&small).await.unwrap();
        assert!(!again.already_existed);
        assert_eq!(&store.get(&held.cid).await.unwrap()[..], &small[..]);
        assert_eq!(store.exists(&held.cid).await, Exists::Verified);
    }

    /// Count `Event::BucketSynced` entries whose directory is `dir`.
    fn bucket_syncs(store: &BlobStore, dir: &Path) -> Vec<usize> {
        faults(store)
            .events
            .iter()
            .enumerate()
            .filter(|(_, e)| matches!(e, Event::BucketSynced(p) if p == dir))
            .map(|(i, _)| i)
            .collect()
    }

    /// The index of the first rename into a bucket under `area`.
    fn first_rename_under(store: &BlobStore, area: &Path) -> usize {
        faults(store)
            .events
            .iter()
            .position(|e| matches!(e, Event::Rename(p) if p.starts_with(area)))
            .expect("a rename under the area")
    }

    #[tokio::test]
    async fn new_bucket_parent_is_synced_before_rename() {
        let (dir, store) = open().await;
        let blobs = dir.path().join("blobs");
        let chunks = dir.path().join("chunks");

        // A chunked payload creates buckets in both `chunks/` and `blobs/`.
        let result = store.store(&chunked_payload()).await.unwrap();
        for area in [&blobs, &chunks] {
            let synced = bucket_syncs(&store, area);
            assert!(
                !synced.is_empty(),
                "{area:?} was fsynced after a new bucket"
            );
            assert!(
                synced[0] < first_rename_under(&store, area),
                "{area:?} is synced before the first file is renamed into its bucket"
            );
        }
        // Every bucket a rename landed in had been created and synced first.
        let distinct_buckets: std::collections::HashSet<PathBuf> = faults(&store)
            .events
            .iter()
            .filter_map(|e| match e {
                Event::Rename(p) => p.parent().map(Path::to_path_buf),
                _ => None,
            })
            .filter(|b| b.starts_with(&blobs) || b.starts_with(&chunks))
            .collect();
        let syncs = faults(&store)
            .events
            .iter()
            .filter(|e| matches!(e, Event::BucketSynced(_)))
            .count();
        assert_eq!(
            syncs,
            distinct_buckets.len(),
            "one parent sync per newly created bucket"
        );

        // A second write into an existing bucket syncs nothing more.
        let before = syncs;
        store.store(&chunked_payload()).await.unwrap();
        let after = faults(&store)
            .events
            .iter()
            .filter(|e| matches!(e, Event::BucketSynced(_)))
            .count();
        assert_eq!(before, after, "an existing bucket is not re-synced");
        assert_eq!(store.exists(&result.cid).await, Exists::Verified);
    }

    #[tokio::test]
    async fn power_loss_after_marker_reopens_verified() {
        let (dir, store) = open().await;
        let data = chunked_payload();
        let chunk_count = chunk_bytes(&ValidatedProfile::EPOCH_0, &data).len();
        // Every chunk, the index and the marker are durable; the process dies before
        // `store` returns, so no caller ever learns the payload landed.
        faults(&store).crash_after = Some(chunk_count + 2);
        assert!(store.store(&data).await.is_err());
        drop(store);

        let reopened = BlobStore::new(dir.path()).await.unwrap();
        let cid = compute_bytes_cid(&data);
        assert_eq!(reopened.exists(&cid).await, Exists::Verified);
        assert_eq!(&reopened.get(&cid).await.unwrap()[..], &data[..]);
        assert_eq!(reopened.stats().total_blobs, 1, "the open scan counts it");
        assert!(tmp_is_empty(&dir));
        // Storing it again is the idempotent path, not a rewrite.
        assert!(reopened.store(&data).await.unwrap().already_existed);
    }

    /// Read until told to stop; every read is the whole payload or a typed absence.
    async fn read_whole_or_absent(
        store: std::sync::Arc<BlobStore>,
        cid: ByteCid,
        data: std::sync::Arc<Vec<u8>>,
        stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> usize {
        use std::sync::atomic::Ordering;
        let mut whole = 0;
        while !stop.load(Ordering::Relaxed) {
            match store.get(&cid).await {
                Ok(bytes) => {
                    assert_eq!(&bytes[..], &data[..], "a read is never a partial payload");
                    whole += 1;
                }
                Err(StorageError::BlobNotFound(_)) => {}
                Err(other) => panic!("a racing read returned {other:?}, not whole-or-absent"),
            }
            tokio::task::yield_now().await;
        }
        whole
    }

    #[tokio::test]
    async fn delete_racing_store_never_serves_partial() {
        use std::sync::{atomic::AtomicBool, Arc};
        let (_dir, store) = open().await;
        let store = Arc::new(store);
        let data = Arc::new(chunked_payload());
        let cid = compute_bytes_cid(&data);
        for _ in 0..6 {
            let stop = Arc::new(AtomicBool::new(false));
            let readers: Vec<_> = (0..2)
                .map(|_| {
                    tokio::spawn(read_whole_or_absent(
                        store.clone(),
                        cid,
                        data.clone(),
                        stop.clone(),
                    ))
                })
                .collect();
            let writer = {
                let (store, data) = (store.clone(), data.clone());
                tokio::spawn(async move { store.store(&data).await.map(|_| ()) })
            };
            let deleter = {
                let store = store.clone();
                tokio::spawn(async move { store.delete(&cid).await })
            };
            writer.await.unwrap().unwrap();
            deleter.await.unwrap().unwrap();
            stop.store(true, std::sync::atomic::Ordering::Relaxed);
            for reader in readers {
                reader.await.unwrap();
            }
            // Whichever won, what remains is whole or absent.
            match store.exists(&cid).await {
                Exists::Verified => {
                    assert_eq!(&store.get(&cid).await.unwrap()[..], &data[..]);
                }
                Exists::Absent => {}
                Exists::Quarantined => panic!("a store/delete race must not quarantine"),
            }
        }
        // And the store still takes the payload afterwards.
        store.store(&data).await.unwrap();
        assert_eq!(store.exists(&cid).await, Exists::Verified);
    }

    #[tokio::test]
    async fn delete_racing_read_returns_whole_or_typed_absent() {
        use std::sync::{atomic::AtomicBool, Arc};
        let (dir, store) = open().await;
        let store = Arc::new(store);
        let data = Arc::new(chunked_payload());
        let cid = compute_bytes_cid(&data);
        for _ in 0..6 {
            store.store(&data).await.unwrap();
            let stop = Arc::new(AtomicBool::new(false));
            let readers: Vec<_> = (0..3)
                .map(|_| {
                    tokio::spawn(read_whole_or_absent(
                        store.clone(),
                        cid,
                        data.clone(),
                        stop.clone(),
                    ))
                })
                .collect();
            tokio::task::yield_now().await;
            store.delete(&cid).await.unwrap();
            stop.store(true, std::sync::atomic::Ordering::Relaxed);
            for reader in readers {
                reader.await.unwrap();
            }
            assert!(matches!(
                store.get(&cid).await,
                Err(StorageError::BlobNotFound(_))
            ));
            assert_eq!(store.exists(&cid).await, Exists::Absent);
            assert_eq!(
                fs::read_dir(dir.path().join("quarantine")).unwrap().count(),
                0,
                "a delete is not a defect: nothing was quarantined"
            );
        }
    }

    #[tokio::test]
    async fn quarantine_leaves_unrelated_objects_readable() {
        let (dir, store) = open().await;
        let victim_data = chunked_payload();
        // A different stream, so no chunk is shared with the victim.
        let other_data: Vec<u8> = chunked_payload().iter().map(|b| b ^ 0xa5).collect();
        let small = b"an unrelated inline payload".to_vec();
        let victim = store.store(&victim_data).await.unwrap();
        let other = store.store(&other_data).await.unwrap();
        let inline = store.store(&small).await.unwrap();
        assert!(
            other.chunks.iter().all(|c| !victim.chunks.contains(c)),
            "fixture shares no chunk"
        );

        corrupt_chunk_in_place(&dir, &victim.chunks[1].cid, 5);
        assert!(store.get(&victim.cid).await.is_err());
        assert_eq!(store.exists(&victim.cid).await, Exists::Quarantined);

        assert_eq!(&store.get(&other.cid).await.unwrap()[..], &other_data[..]);
        assert_eq!(store.exists(&other.cid).await, Exists::Verified);
        assert_eq!(&store.get(&inline.cid).await.unwrap()[..], &small[..]);
        assert_eq!(store.exists(&inline.cid).await, Exists::Verified);
        let listed = store.list_cids().unwrap();
        assert!(listed.contains(&other.cid) && listed.contains(&inline.cid));
        assert!(!listed.contains(&victim.cid));
        assert_eq!(store.stats().total_blobs, 2);
    }

    #[tokio::test]
    async fn list_units_names_only_verified_chunks_and_inline_payloads() {
        let (dir, store) = open().await;
        let chunked = store.store(&chunked_payload()).await.unwrap();
        let inline = store.store(b"one inline unit").await.unwrap();

        let units = store.list_units().unwrap();
        for chunk in &chunked.chunks {
            assert!(units.contains(&chunk.cid), "chunk {} is a unit", chunk.cid);
        }
        assert!(units.contains(&inline.cid), "an inline payload is a unit");
        assert!(
            !units.contains(&chunked.cid),
            "a marker is a payload, not a unit"
        );
        assert!(store.list_cids().unwrap().contains(&chunked.cid));

        // A chunk whose bytes are wrong is left out and quarantined, not listed.
        let victim = chunked.chunks[2].cid;
        corrupt_chunk_in_place(&dir, &victim, 9);
        assert!(
            store.unit_names().contains(&victim),
            "its name is still there"
        );
        let units = store.list_units().unwrap();
        assert!(!units.contains(&victim));
        assert!(units.contains(&chunked.chunks[0].cid));
    }

    #[tokio::test]
    async fn recon_set_excludes_unverified_units() {
        let (dir, store) = open().await;
        let good = store.store(&chunked_payload()).await.unwrap();
        let other: Vec<u8> = chunked_payload().iter().map(|b| b ^ 0x5a).collect();
        let broken = store.store(&other).await.unwrap();
        let inline = store.store(b"a whole inline payload").await.unwrap();
        let bad_inline = store.store(b"an inline payload to spoil").await.unwrap();
        let bad_chunk = broken.chunks[1].cid;
        corrupt_chunk_in_place(&dir, &bad_chunk, 4);
        let path = fanout_path(&dir.path().join("blobs"), &bad_inline.cid.to_string());
        fs::write(&path, b"an inline payload to spoiL").unwrap();

        assert!(store.list_cids().unwrap().contains(&broken.cid), "named");
        let servable = store.list_servable().unwrap();
        assert!(servable.contains(&good.cid) && servable.contains(&inline.cid));
        for chunk in &good.chunks {
            assert!(servable.contains(&chunk.cid));
        }
        assert!(
            !servable.contains(&broken.cid),
            "a payload with a corrupt chunk"
        );
        assert!(!servable.contains(&bad_chunk), "the corrupt chunk");
        assert!(
            !servable.contains(&bad_inline.cid),
            "the corrupt inline payload"
        );
        assert!(store.holds_verified(&good.cid) && store.holds_verified(&good.chunks[0].cid));
        assert!(!store.holds_verified(&broken.cid));
    }

    // ── R3 follow-up: only a definitive defect quarantines; probes do not hash ──

    #[tokio::test]
    async fn transient_read_error_never_quarantines() {
        let (dir, store) = open().await;
        let data = chunked_payload();
        let result = store.store(&data).await.unwrap();
        // A chunk path that cannot be read for a reason other than absence (here a
        // directory where the file should be): not verified, and nothing moves.
        let victim = result.chunks[1].cid;
        let path = fanout_path(&dir.path().join("chunks"), &victim.to_string());
        let held = fs::read(&path).unwrap();
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();
        assert_ne!(store.exists(&result.cid).await, Exists::Verified);
        assert!(
            fanout_path(&dir.path().join("blobs"), &result.cid.to_string()).exists(),
            "the marker stays in the serving tree"
        );
        assert!(
            fs::read_dir(dir.path().join("quarantine"))
                .map(|mut d| d.next().is_none())
                .unwrap_or(true),
            "nothing quarantined on a non-definitive read error"
        );
        // The fault clears: the payload verifies again, untouched.
        fs::remove_dir(&path).unwrap();
        fs::write(&path, &held).unwrap();
        assert_eq!(store.exists(&result.cid).await, Exists::Verified);

        // The same for the layout record.
        let index = dir.path().join("index").join(result.cid.to_string());
        let record = fs::read(&index).unwrap();
        fs::remove_file(&index).unwrap();
        fs::create_dir(&index).unwrap();
        assert_ne!(store.exists(&result.cid).await, Exists::Verified);
        fs::remove_dir(&index).unwrap();
        fs::write(&index, &record).unwrap();
        assert_eq!(store.exists(&result.cid).await, Exists::Verified);
    }

    #[tokio::test]
    async fn present_is_structural_and_moves_nothing() {
        let (dir, store) = open().await;
        let data = chunked_payload();
        let result = store.store(&data).await.unwrap();
        let victim = result.chunks[1].cid;
        let path = fanout_path(&dir.path().join("chunks"), &victim.to_string());
        let mut bytes = fs::read(&path).unwrap();
        bytes[5] ^= 0x01;
        fs::write(&path, &bytes).unwrap();
        // The structural probe answers from names and lengths: it cannot see a
        // same-length corruption, and it moves nothing.
        assert!(store.present(&result.cid).await);
        assert_eq!(fs::read(&path).unwrap(), bytes, "the probe touched no byte");
        assert!(path.exists());
        // The hashing check still refuses it.
        assert_eq!(store.exists(&result.cid).await, Exists::Quarantined);
        assert!(!store.present(&result.cid).await);
        let absent = compute_bytes_cid(b"never stored");
        assert!(!store.present(&absent).await);
    }
}
