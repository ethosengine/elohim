//! Sled-backed Kademlia record store.
//!
//! Persists Kademlia DHT routing records across restarts so that desktop
//! stewards whose laptops sleep/wake frequently don't lose their routing table.
//! Uses the existing `sync.sled` database (shared with CRDT doc store).

use libp2p::kad::store::{Error as StoreError, RecordStore, Result as StoreResult};
use libp2p::kad::{ProviderRecord, Record, RecordKey};
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashSet;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

// libp2p's Record and ProviderRecord don't implement serde, so we use
// intermediate types for persistence.

/// Records persisted before expiries were stored carry none. They are given
/// this long from the first sweep that sees them, the libp2p-kad default
/// `record_ttl`, after which they leave the store like any other record.
pub const LEGACY_RECORD_TTL: Duration = Duration::from_secs(48 * 60 * 60);

/// Wall-clock milliseconds since the Unix epoch.
fn wall_ms(at: SystemTime) -> u64 {
    at.duration_since(UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

/// One instant on both clocks, so a monotonic expiry can be stored as wall
/// time and read back as monotonic time after a restart.
#[derive(Debug, Clone, Copy)]
struct Clocks {
    instant: Instant,
    wall_ms: u64,
}

impl Clocks {
    fn now() -> Self {
        Self {
            instant: Instant::now(),
            wall_ms: wall_ms(SystemTime::now()),
        }
    }

    fn to_wall_ms(self, expires: Instant) -> u64 {
        match expires.checked_duration_since(self.instant) {
            Some(ahead) => self
                .wall_ms
                .saturating_add(u64::try_from(ahead.as_millis()).unwrap_or(u64::MAX)),
            None => {
                let behind = self.instant.duration_since(expires);
                self.wall_ms
                    .saturating_sub(u64::try_from(behind.as_millis()).unwrap_or(u64::MAX))
            }
        }
    }

    /// An expiry already past reads as an instant at or before
    /// `self.instant`, which `Record::is_expired` treats as expired.
    fn to_instant(self, expires_at_ms: u64) -> Instant {
        if expires_at_ms >= self.wall_ms {
            let ahead = Duration::from_millis(expires_at_ms - self.wall_ms);
            self.instant.checked_add(ahead).unwrap_or(self.instant)
        } else {
            let behind = Duration::from_millis(self.wall_ms - expires_at_ms);
            self.instant.checked_sub(behind).unwrap_or(self.instant)
        }
    }
}

#[derive(Serialize, Deserialize)]
struct StoredRecord {
    key: Vec<u8>,
    value: Vec<u8>,
    publisher: Option<Vec<u8>>,
    /// When the record expires, as wall-clock Unix milliseconds (`Instant` is
    /// not meaningful across a restart). `None` for a record that never
    /// expires, and for every record written before this field existed: it is
    /// the trailing field so those rows still decode.
    #[serde(default)]
    expires_at_ms: Option<u64>,
}

impl StoredRecord {
    fn from_record(r: &Record, clocks: Clocks) -> Self {
        Self {
            key: r.key.as_ref().to_vec(),
            value: r.value.clone(),
            publisher: r.publisher.map(|p| p.to_bytes()),
            expires_at_ms: r.expires.map(|t| clocks.to_wall_ms(t)),
        }
    }

    fn into_record(self, clocks: Clocks) -> Record {
        Record {
            key: RecordKey::new(&self.key),
            value: self.value,
            publisher: self.publisher.and_then(|b| PeerId::from_bytes(&b).ok()),
            expires: self.expires_at_ms.map(|ms| clocks.to_instant(ms)),
        }
    }
}

/// What one [`SledRecordStore::prune_expired_records`] sweep did.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RecordPrune {
    /// Records whose expiry had passed, removed.
    pub removed: usize,
    /// Records with no stored expiry, given [`LEGACY_RECORD_TTL`].
    pub stamped: usize,
    /// Records left in the store.
    pub kept: usize,
}

#[derive(Serialize, Deserialize)]
struct StoredProvider {
    key: Vec<u8>,
    provider: Vec<u8>,
    // addresses not stored — re-discovered via identify/Kademlia after restart
}

impl From<&ProviderRecord> for StoredProvider {
    fn from(p: &ProviderRecord) -> Self {
        Self {
            key: p.key.as_ref().to_vec(),
            provider: p.provider.to_bytes(),
        }
    }
}

impl StoredProvider {
    fn into_provider_record(self) -> Option<ProviderRecord> {
        let provider = PeerId::from_bytes(&self.provider).ok()?;
        Some(ProviderRecord {
            key: RecordKey::new(&self.key),
            provider,
            expires: None,
            addresses: vec![],
        })
    }
}

/// Sled-backed record store for Kademlia DHT persistence.
///
/// Stores records in two sled trees:
/// - `kademlia_records`: Key -> serialized Record
/// - `kademlia_providers`: Key -> serialized set of ProviderRecords
pub struct SledRecordStore {
    db: sled::Db,
    records_tree: sled::Tree,
    providers_tree: sled::Tree,
    /// Local providers we've announced
    local_providers: HashSet<RecordKey>,
    /// Maximum number of records
    max_records: usize,
    /// Maximum record size in bytes
    max_record_size: usize,
    /// Maximum providers per key
    max_providers_per_key: usize,
}

impl SledRecordStore {
    /// Create a sled-backed Kademlia store from an existing sled::Db handle.
    ///
    /// Shares the database with DocStore to avoid flock() conflicts.
    pub fn from_db(db: sled::Db) -> Result<Self, String> {
        let records_tree = db
            .open_tree("kademlia_records")
            .map_err(|e| format!("Failed to open kademlia_records tree: {}", e))?;
        let providers_tree = db
            .open_tree("kademlia_providers")
            .map_err(|e| format!("Failed to open kademlia_providers tree: {}", e))?;

        debug!(
            records = records_tree.len(),
            providers = providers_tree.len(),
            "Opened sled Kademlia store"
        );

        let store = Self {
            db,
            records_tree,
            providers_tree,
            local_providers: HashSet::new(),
            max_records: 65536,
            max_record_size: 65536,
            max_providers_per_key: 32,
        };

        // Records persisted before their expiry was stored would otherwise
        // live forever. One sweep at open bounds them; after that the
        // Kademlia replication job removes expired records as it iterates.
        let prune = store.prune_expired_records();
        if prune.removed > 0 || prune.stamped > 0 {
            info!(
                removed = prune.removed,
                stamped = prune.stamped,
                kept = prune.kept,
                "Swept expired Kademlia records"
            );
        }

        Ok(store)
    }

    fn serialize_record(record: &Record, clocks: Clocks) -> Result<Vec<u8>, String> {
        let stored = StoredRecord::from_record(record, clocks);
        rmp_serde::to_vec(&stored).map_err(|e| format!("Failed to serialize record: {}", e))
    }

    fn deserialize_record(data: &[u8], clocks: Clocks) -> Result<Record, String> {
        let stored: StoredRecord = rmp_serde::from_slice(data)
            .map_err(|e| format!("Failed to deserialize record: {}", e))?;
        Ok(stored.into_record(clocks))
    }

    /// Remove every record whose stored expiry has passed, and give each
    /// record that has no stored expiry [`LEGACY_RECORD_TTL`] from now.
    ///
    /// A record that fails to decode is left alone, as `get` and `records`
    /// already skip it. Removal goes through `compare_and_swap`, so a record
    /// re-put while the sweep runs is never deleted on the strength of the
    /// copy the sweep read.
    pub fn prune_expired_records(&self) -> RecordPrune {
        self.prune_expired_records_at(Clocks::now())
    }

    fn prune_expired_records_at(&self, clocks: Clocks) -> RecordPrune {
        let mut out = RecordPrune::default();
        for item in self.records_tree.iter() {
            let Ok((key, data)) = item else {
                break;
            };
            let Ok(mut stored) = rmp_serde::from_slice::<StoredRecord>(&data) else {
                out.kept += 1;
                continue;
            };
            let replacement = match stored.expires_at_ms {
                Some(ms) if ms <= clocks.wall_ms => None,
                Some(_) => {
                    out.kept += 1;
                    continue;
                }
                None => {
                    stored.expires_at_ms = Some(clocks.wall_ms.saturating_add(
                        u64::try_from(LEGACY_RECORD_TTL.as_millis()).unwrap_or(u64::MAX),
                    ));
                    match rmp_serde::to_vec(&stored) {
                        Ok(bytes) => Some(bytes),
                        Err(_) => {
                            out.kept += 1;
                            continue;
                        }
                    }
                }
            };
            let stamping = replacement.is_some();
            match self
                .records_tree
                .compare_and_swap(&key, Some(&data), replacement)
            {
                Ok(Ok(())) if stamping => {
                    out.stamped += 1;
                    out.kept += 1;
                }
                Ok(Ok(())) => out.removed += 1,
                // Re-put (or removed) while the sweep ran: the newer copy stands.
                Ok(Err(_)) => out.kept += 1,
                Err(e) => {
                    warn!("Failed to sweep Kademlia record: {}", e);
                    out.kept += 1;
                }
            }
        }
        out
    }

    fn serialize_providers(providers: &[ProviderRecord]) -> Result<Vec<u8>, String> {
        let stored: Vec<StoredProvider> = providers.iter().map(StoredProvider::from).collect();
        rmp_serde::to_vec(&stored).map_err(|e| format!("Failed to serialize providers: {}", e))
    }

    fn deserialize_providers(data: &[u8]) -> Result<Vec<ProviderRecord>, String> {
        let stored: Vec<StoredProvider> = rmp_serde::from_slice(data)
            .map_err(|e| format!("Failed to deserialize providers: {}", e))?;
        Ok(stored
            .into_iter()
            .filter_map(|s| s.into_provider_record())
            .collect())
    }

    /// Flush all pending writes to disk
    pub fn flush(&self) -> Result<(), String> {
        self.db
            .flush()
            .map_err(|e| format!("Failed to flush sled DB: {}", e))?;
        Ok(())
    }
}

impl RecordStore for SledRecordStore {
    type RecordsIter<'a> = SledRecordIter<'a>;
    type ProvidedIter<'a> = SledProvidedIter<'a>;

    fn get(&self, key: &RecordKey) -> Option<Cow<'_, Record>> {
        match self.records_tree.get(key.as_ref()) {
            Ok(Some(data)) => match Self::deserialize_record(&data, Clocks::now()) {
                Ok(record) => Some(Cow::Owned(record)),
                Err(e) => {
                    warn!("Corrupted Kademlia record: {}", e);
                    None
                }
            },
            Ok(None) => None,
            Err(e) => {
                warn!("Sled read error for Kademlia record: {}", e);
                None
            }
        }
    }

    fn put(&mut self, record: Record) -> StoreResult<()> {
        if record.value.len() > self.max_record_size {
            return Err(StoreError::ValueTooLarge);
        }
        // `Tree::len` is a full scan in sled 0.34; an overwrite of a key the
        // store already holds never needs it, and is never refused at the cap.
        let is_new = !self
            .records_tree
            .contains_key(record.key.as_ref())
            .unwrap_or(false);
        if is_new && self.records_tree.len() >= self.max_records {
            return Err(StoreError::MaxRecords);
        }

        let data = Self::serialize_record(&record, Clocks::now())
            .map_err(|_| StoreError::ValueTooLarge)?;
        self.records_tree
            .insert(record.key.as_ref(), data)
            .map_err(|_| StoreError::ValueTooLarge)?;
        Ok(())
    }

    fn remove(&mut self, key: &RecordKey) {
        if let Err(e) = self.records_tree.remove(key.as_ref()) {
            warn!("Failed to remove Kademlia record: {}", e);
        }
    }

    fn records(&self) -> Self::RecordsIter<'_> {
        SledRecordIter {
            inner: self.records_tree.iter(),
            clocks: Clocks::now(),
            _lifetime: std::marker::PhantomData,
        }
    }

    fn add_provider(&mut self, record: ProviderRecord) -> StoreResult<()> {
        let key_bytes = record.key.as_ref().to_vec();

        // Load existing providers for this key
        let mut providers: Vec<ProviderRecord> = match self.providers_tree.get(&key_bytes) {
            Ok(Some(data)) => Self::deserialize_providers(&data).unwrap_or_default(),
            _ => Vec::new(),
        };

        // Check if this provider already exists (update it)
        if let Some(existing) = providers.iter_mut().find(|p| p.provider == record.provider) {
            *existing = record;
        } else {
            if providers.len() >= self.max_providers_per_key {
                return Err(StoreError::MaxProvidedKeys);
            }
            providers.push(record);
        }

        let data = Self::serialize_providers(&providers).map_err(|_| StoreError::ValueTooLarge)?;
        self.providers_tree
            .insert(key_bytes, data)
            .map_err(|_| StoreError::ValueTooLarge)?;
        Ok(())
    }

    fn providers(&self, key: &RecordKey) -> Vec<ProviderRecord> {
        match self.providers_tree.get(key.as_ref()) {
            Ok(Some(data)) => Self::deserialize_providers(&data).unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    fn provided(&self) -> Self::ProvidedIter<'_> {
        SledProvidedIter {
            inner: self.local_providers.iter(),
        }
    }

    fn remove_provider(&mut self, key: &RecordKey, provider: &PeerId) {
        let key_bytes = key.as_ref();

        if let Ok(Some(data)) = self.providers_tree.get(key_bytes) {
            if let Ok(mut providers) = Self::deserialize_providers(&data) {
                providers.retain(|p| &p.provider != provider);
                if providers.is_empty() {
                    let _ = self.providers_tree.remove(key_bytes);
                } else if let Ok(data) = Self::serialize_providers(&providers) {
                    let _ = self.providers_tree.insert(key_bytes, data);
                }
            }
        }
    }
}

/// Iterator over sled-stored Kademlia records
pub struct SledRecordIter<'a> {
    inner: sled::Iter,
    clocks: Clocks,
    _lifetime: std::marker::PhantomData<&'a ()>,
}

impl<'a> Iterator for SledRecordIter<'a> {
    type Item = Cow<'a, Record>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.inner.next()? {
                Ok((_key, data)) => match SledRecordStore::deserialize_record(&data, self.clocks) {
                    Ok(record) => return Some(Cow::Owned(record)),
                    Err(e) => {
                        warn!("Skipping corrupted Kademlia record: {}", e);
                        continue;
                    }
                },
                Err(e) => {
                    warn!("Sled iterator error: {}", e);
                    return None;
                }
            }
        }
    }
}

/// Iterator over locally provided keys, yielding ProviderRecords
pub struct SledProvidedIter<'a> {
    inner: std::collections::hash_set::Iter<'a, RecordKey>,
}

impl<'a> Iterator for SledProvidedIter<'a> {
    type Item = Cow<'a, ProviderRecord>;

    fn next(&mut self) -> Option<Self::Item> {
        // The provided() iterator yields the keys we've announced as provider for.
        // We construct a minimal ProviderRecord from each key.
        // The full provider info (addresses, etc.) lives in the providers_tree.
        self.inner.next().map(|key| {
            Cow::Owned(ProviderRecord {
                key: key.clone(),
                provider: PeerId::random(),
                expires: None,
                addresses: vec![],
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let db = sled::open(dir.path()).unwrap();
        let mut store = SledRecordStore::from_db(db).unwrap();

        let key = RecordKey::new(b"test-key");
        let record = Record {
            key: key.clone(),
            value: b"test-value".to_vec(),
            publisher: None,
            expires: None,
        };

        store.put(record.clone()).unwrap();

        let retrieved = store.get(&key).unwrap();
        assert_eq!(retrieved.value, b"test-value");

        store.remove(&key);
        assert!(store.get(&key).is_none());
    }

    #[test]
    fn test_records_persist_across_reopen() {
        let dir = tempfile::tempdir().unwrap();

        let key = RecordKey::new(b"persist-key");
        let record = Record {
            key: key.clone(),
            value: b"persist-value".to_vec(),
            publisher: None,
            expires: None,
        };

        // Write
        {
            let db = sled::open(dir.path()).unwrap();
            let mut store = SledRecordStore::from_db(db).unwrap();
            store.put(record).unwrap();
            store.flush().unwrap();
        }

        // Reopen and verify
        {
            let db = sled::open(dir.path()).unwrap();
            let store = SledRecordStore::from_db(db).unwrap();
            let retrieved = store.get(&key).unwrap();
            assert_eq!(retrieved.value, b"persist-value");
        }
    }

    fn record(key: &[u8], expires: Option<Instant>) -> Record {
        Record {
            key: RecordKey::new(&key),
            value: b"value".to_vec(),
            publisher: None,
            expires,
        }
    }

    /// The record shape written before expiries were persisted.
    #[derive(Serialize)]
    struct LegacyStoredRecord {
        key: Vec<u8>,
        value: Vec<u8>,
        publisher: Option<Vec<u8>>,
    }

    /// A record's expiry survives being stored and read back, so the
    /// Kademlia replication job (which removes a record once
    /// `is_expired`) can ever remove it. Before, every persisted record
    /// read back with `expires: None` and lived forever.
    #[test]
    fn a_stored_record_keeps_its_expiry() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = SledRecordStore::from_db(sled::open(dir.path()).unwrap()).unwrap();
        let key = RecordKey::new(b"expiring");
        let expires = Instant::now() + Duration::from_secs(3600);
        store.put(record(b"expiring", Some(expires))).unwrap();

        let back = store.get(&key).unwrap().expires.expect("expiry kept");
        let drift = if back > expires {
            back - expires
        } else {
            expires - back
        };
        assert!(
            drift < Duration::from_secs(2),
            "expiry drifted by {drift:?}"
        );
        assert!(!store.get(&key).unwrap().is_expired(Instant::now()));
    }

    /// The expiry is wall-clock, so it survives a restart too.
    #[test]
    fn a_record_expiry_survives_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let key = RecordKey::new(b"reopen");
        {
            let mut store = SledRecordStore::from_db(sled::open(dir.path()).unwrap()).unwrap();
            store
                .put(record(
                    b"reopen",
                    Some(Instant::now() + Duration::from_secs(600)),
                ))
                .unwrap();
            store.flush().unwrap();
        }
        let store = SledRecordStore::from_db(sled::open(dir.path()).unwrap()).unwrap();
        let back = store.get(&key).unwrap();
        let left = back
            .expires
            .expect("expiry kept across reopen")
            .saturating_duration_since(Instant::now());
        assert!(
            left > Duration::from_secs(590) && left <= Duration::from_secs(600),
            "{left:?}"
        );
    }

    /// An expired record leaves on the next sweep; a live one and a
    /// never-expiring one stay.
    #[test]
    fn the_sweep_removes_expired_records_only() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = SledRecordStore::from_db(sled::open(dir.path()).unwrap()).unwrap();
        let clocks = Clocks::now();
        store
            .put(record(
                b"live",
                Some(clocks.instant + Duration::from_secs(60)),
            ))
            .unwrap();
        store
            .put(record(
                b"expired",
                Some(clocks.instant + Duration::from_millis(1)),
            ))
            .unwrap();

        let later = Clocks {
            instant: clocks.instant + Duration::from_secs(1),
            wall_ms: clocks.wall_ms + 1_000,
        };
        let prune = store.prune_expired_records_at(later);
        assert_eq!(prune.removed, 1, "{prune:?}");
        assert!(store.get(&RecordKey::new(b"expired")).is_none());
        assert!(store.get(&RecordKey::new(b"live")).is_some());
        assert_eq!(store.records().count(), 1);
    }

    /// Records written before this change decode, and the sweep gives each
    /// the default TTL instead of letting it live forever.
    #[test]
    fn a_legacy_record_without_expiry_is_stamped_then_expires() {
        let dir = tempfile::tempdir().unwrap();
        let db = sled::open(dir.path()).unwrap();
        let legacy = rmp_serde::to_vec(&LegacyStoredRecord {
            key: b"legacy".to_vec(),
            value: b"old".to_vec(),
            publisher: None,
        })
        .unwrap();
        db.open_tree("kademlia_records")
            .unwrap()
            .insert(b"legacy", legacy)
            .unwrap();

        // Opening sweeps once: the legacy row decodes and is stamped.
        let store = SledRecordStore::from_db(db).unwrap();
        let back = store.get(&RecordKey::new(b"legacy")).unwrap();
        assert_eq!(back.value, b"old");
        let left = back
            .expires
            .expect("stamped with the legacy TTL")
            .saturating_duration_since(Instant::now());
        assert!(
            left > LEGACY_RECORD_TTL - Duration::from_secs(60),
            "{left:?}"
        );

        let clocks = Clocks::now();
        let past_ttl = Clocks {
            instant: clocks.instant + LEGACY_RECORD_TTL,
            wall_ms: clocks.wall_ms + u64::try_from(LEGACY_RECORD_TTL.as_millis()).unwrap(),
        };
        assert_eq!(store.prune_expired_records_at(past_ttl).removed, 1);
        assert!(store.get(&RecordKey::new(b"legacy")).is_none());
    }

    /// At the record cap a new key is refused, but a key already held can
    /// still be overwritten (republish/replication of what we hold).
    #[test]
    fn the_record_cap_refuses_new_keys_but_not_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = SledRecordStore::from_db(sled::open(dir.path()).unwrap()).unwrap();
        store.max_records = 1;
        store.put(record(b"held", None)).unwrap();
        assert!(matches!(
            store.put(record(b"new", None)),
            Err(StoreError::MaxRecords)
        ));
        store
            .put(record(
                b"held",
                Some(Instant::now() + Duration::from_secs(5)),
            ))
            .unwrap();
        assert!(store
            .get(&RecordKey::new(b"held"))
            .unwrap()
            .expires
            .is_some());
    }
}
