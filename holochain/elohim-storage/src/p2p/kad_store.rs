//! Sled-backed Kademlia record store.
//!
//! Persists Kademlia routing records across restarts so that desktop stewards
//! whose laptops sleep/wake frequently don't lose their routing table.
//! Uses the existing `sync.sled` database (shared with CRDT doc store).

use libp2p::kad::store::{Error as StoreError, RecordStore, Result as StoreResult};
use libp2p::kad::{ProviderRecord, Record, RecordKey};
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashSet;
use std::path::Path;
use tracing::{debug, warn};

// libp2p's Record and ProviderRecord don't implement serde, so we use
// intermediate types for persistence.

#[derive(Serialize, Deserialize)]
struct StoredRecord {
    key: Vec<u8>,
    value: Vec<u8>,
    publisher: Option<Vec<u8>>,
    // expires is Instant-based and not serializable — we drop it on persist.
    // Records will be re-validated via Kademlia protocol after restart.
}

impl From<&Record> for StoredRecord {
    fn from(r: &Record) -> Self {
        Self {
            key: r.key.as_ref().to_vec(),
            value: r.value.clone(),
            publisher: r.publisher.map(|p| p.to_bytes()),
            // expires dropped intentionally
        }
    }
}

impl StoredRecord {
    fn into_record(self) -> Record {
        Record {
            key: RecordKey::new(&self.key),
            value: self.value,
            publisher: self.publisher.and_then(|b| PeerId::from_bytes(&b).ok()),
            expires: None,
        }
    }
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
    #[allow(dead_code)]
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
    /// Open or create a sled-backed Kademlia store.
    ///
    /// Uses the given sled database path (typically the same `sync.sled` DB).
    pub fn new(db_path: &Path) -> Result<Self, String> {
        let db = sled::open(db_path)
            .map_err(|e| format!("Failed to open sled DB at {}: {}", db_path.display(), e))?;
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

        Ok(Self {
            db,
            records_tree,
            providers_tree,
            local_providers: HashSet::new(),
            max_records: 65536,
            max_record_size: 65536,
            max_providers_per_key: 32,
        })
    }

    fn serialize_record(record: &Record) -> Result<Vec<u8>, String> {
        let stored = StoredRecord::from(record);
        rmp_serde::to_vec(&stored).map_err(|e| format!("Failed to serialize record: {}", e))
    }

    fn deserialize_record(data: &[u8]) -> Result<Record, String> {
        let stored: StoredRecord =
            rmp_serde::from_slice(data).map_err(|e| format!("Failed to deserialize record: {}", e))?;
        Ok(stored.into_record())
    }

    fn serialize_providers(providers: &[ProviderRecord]) -> Result<Vec<u8>, String> {
        let stored: Vec<StoredProvider> = providers.iter().map(StoredProvider::from).collect();
        rmp_serde::to_vec(&stored).map_err(|e| format!("Failed to serialize providers: {}", e))
    }

    fn deserialize_providers(data: &[u8]) -> Result<Vec<ProviderRecord>, String> {
        let stored: Vec<StoredProvider> =
            rmp_serde::from_slice(data).map_err(|e| format!("Failed to deserialize providers: {}", e))?;
        Ok(stored.into_iter().filter_map(|s| s.into_provider_record()).collect())
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
    type RecordsIter<'a> = SledRecordIter;
    type ProvidedIter<'a> = SledProvidedIter<'a>;

    fn get(&self, key: &RecordKey) -> Option<Cow<'_, Record>> {
        match self.records_tree.get(key.as_ref()) {
            Ok(Some(data)) => match Self::deserialize_record(&data) {
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
        if self.records_tree.len() >= self.max_records {
            return Err(StoreError::MaxRecords);
        }

        let data = Self::serialize_record(&record).map_err(|_| StoreError::ValueTooLarge)?;
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
pub struct SledRecordIter {
    inner: sled::Iter,
}

impl Iterator for SledRecordIter {
    type Item = Cow<'static, Record>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.inner.next()? {
                Ok((_key, data)) => match SledRecordStore::deserialize_record(&data) {
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
        let mut store = SledRecordStore::new(dir.path()).unwrap();

        let key = RecordKey::new(&b"test-key"[..]);
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

        let key = RecordKey::new(&b"persist-key"[..]);
        let record = Record {
            key: key.clone(),
            value: b"persist-value".to_vec(),
            publisher: None,
            expires: None,
        };

        // Write
        {
            let mut store = SledRecordStore::new(dir.path()).unwrap();
            store.put(record).unwrap();
            store.flush().unwrap();
        }

        // Reopen and verify
        {
            let store = SledRecordStore::new(dir.path()).unwrap();
            let retrieved = store.get(&key).unwrap();
            assert_eq!(retrieved.value, b"persist-value");
        }
    }
}
