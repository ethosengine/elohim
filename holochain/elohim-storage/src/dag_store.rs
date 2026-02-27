//! IPLD DAG store — typed put/get/links over BlobStore.
//!
//! Wraps `BlobStore` with IPLD-aware operations for DAG-CBOR content:
//! - `put()` / `get()` — typed serialize/deserialize via DAG-CBOR
//! - `get_block()` — raw bytes for Bitswap
//! - `extract_links()` — parse IPLD and return linked CIDs
//! - `put_epr_head()` — typed EPR Head helper

use cid::Cid;
use ipld_core::ipld::Ipld;
use multihash_codetable::{Code, MultihashDigest};
use serde::{de::DeserializeOwned, Serialize};

use crate::blob_store::BlobStore;
use crate::epr_codec::{self, DAG_CBOR_CODEC};
use crate::error::StorageError;

/// IPLD-aware DAG store backed by BlobStore.
pub struct DagStore {
    blob_store: BlobStore,
}

impl DagStore {
    /// Create a new DagStore wrapping the given BlobStore.
    pub fn new(blob_store: BlobStore) -> Self {
        Self { blob_store }
    }

    /// Get a reference to the underlying BlobStore.
    pub fn blob_store(&self) -> &BlobStore {
        &self.blob_store
    }

    /// Encode a value as DAG-CBOR and store it, returning the CID.
    pub async fn put<T: Serialize>(&self, value: &T) -> Result<Cid, StorageError> {
        let bytes = serde_ipld_dagcbor::to_vec(value)
            .map_err(|e| StorageError::Codec(format!("DAG-CBOR encode: {e}")))?;
        let result = self.blob_store.store_dag_cbor(&bytes).await?;
        let cid = result
            .cid
            .parse::<Cid>()
            .map_err(|e| StorageError::Codec(format!("CID parse: {e}")))?;
        Ok(cid)
    }

    /// Retrieve and decode a DAG-CBOR value by CID.
    pub async fn get<T: DeserializeOwned>(&self, cid: &Cid) -> Result<T, StorageError> {
        let data = self.blob_store.get_by_address(&cid.to_string()).await?;
        let value = serde_ipld_dagcbor::from_slice(&data)
            .map_err(|e| StorageError::Codec(format!("DAG-CBOR decode: {e}")))?;
        Ok(value)
    }

    /// Retrieve raw block bytes by CID.
    pub async fn get_block(&self, cid: &Cid) -> Result<Option<Vec<u8>>, StorageError> {
        match self.blob_store.get_by_address(&cid.to_string()).await {
            Ok(data) => Ok(Some(data)),
            Err(StorageError::NotFound(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Check if a block exists by CID.
    pub async fn has_block(&self, cid: &Cid) -> Result<bool, StorageError> {
        self.blob_store.exists_by_address(&cid.to_string()).await
    }

    /// List CIDs linked from a DAG-CBOR block.
    pub async fn links(&self, cid: &Cid) -> Result<Vec<Cid>, StorageError> {
        let data = self.blob_store.get_by_address(&cid.to_string()).await?;
        Ok(extract_links(&data, cid.codec()))
    }

    /// Store an EPR Head and return its CID plus any linked CIDs.
    pub async fn put_epr_head(
        &self,
        head: &epr_codec::EprHead,
    ) -> Result<(Cid, Vec<Cid>), StorageError> {
        let (bytes, cid) = epr_codec::encode_epr_head(head)
            .map_err(|e| StorageError::Codec(format!("EPR encode: {e}")))?;
        self.blob_store.store_dag_cbor(&bytes).await?;
        let links = extract_links(&bytes, DAG_CBOR_CODEC);
        Ok((cid, links))
    }

    /// Retrieve and decode an EPR Head by CID.
    pub async fn get_epr_head(&self, cid: &Cid) -> Result<epr_codec::EprHead, StorageError> {
        let data = self.blob_store.get_by_address(&cid.to_string()).await?;
        epr_codec::decode_epr_head(&data)
            .map_err(|e| StorageError::Codec(format!("EPR decode: {e}")))
    }
}

/// Extract CID links from IPLD-encoded data.
///
/// For DAG-CBOR (codec 0x71), parses the CBOR structure and collects all
/// CBOR tag-42 CID links. For raw (0x55) and other codecs, returns empty.
///
/// This function is public so `elohim-node`'s BlobManager can call it
/// for Bitswap's `missing_blocks()` queries.
pub fn extract_links(data: &[u8], codec: u64) -> Vec<Cid> {
    match codec {
        0x71 => {
            // DAG-CBOR: deserialize to generic IPLD and collect Link nodes
            match serde_ipld_dagcbor::from_slice::<Ipld>(data) {
                Ok(ipld) => {
                    let mut links = Vec::new();
                    collect_links(&ipld, &mut links);
                    links
                }
                Err(_) => vec![],
            }
        }
        _ => vec![], // Raw and other codecs have no embedded links
    }
}

/// Compute a DAG-CBOR CID from raw bytes (without storing).
pub fn compute_dag_cbor_cid(data: &[u8]) -> Cid {
    let hash = Code::Sha2_256.digest(data);
    Cid::new_v1(DAG_CBOR_CODEC, hash)
}

/// Recursively collect CID links from an IPLD value.
fn collect_links(ipld: &Ipld, links: &mut Vec<Cid>) {
    match ipld {
        Ipld::Link(cid) => links.push(*cid),
        Ipld::List(list) => {
            for item in list {
                collect_links(item, links);
            }
        }
        Ipld::Map(map) => {
            for value in map.values() {
                collect_links(value, links);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::epr_codec::{EprHead, EprLamadContext, EprQahalContext, EprShefaContext};

    #[test]
    fn test_extract_links_raw() {
        // Raw codec (0x55) should return no links
        assert_eq!(extract_links(b"hello world", 0x55), Vec::<Cid>::new());
    }

    #[test]
    fn test_extract_links_dag_cbor_no_links() {
        // DAG-CBOR map with no CID links
        let val = serde_json::json!({"key": "value", "num": 42});
        let bytes = serde_ipld_dagcbor::to_vec(&val).unwrap();
        assert_eq!(extract_links(&bytes, 0x71), Vec::<Cid>::new());
    }

    #[test]
    fn test_extract_links_invalid_cbor() {
        // Invalid CBOR should return empty, not panic
        assert_eq!(
            extract_links(&[0xFF, 0xFE, 0xFD], 0x71),
            Vec::<Cid>::new()
        );
    }

    #[test]
    fn test_compute_dag_cbor_cid() {
        let data = b"test data";
        let cid = compute_dag_cbor_cid(data);
        assert_eq!(cid.codec(), DAG_CBOR_CODEC);
        assert!(cid.to_string().starts_with("bafyr"));
    }

    #[tokio::test]
    async fn test_dag_store_put_get() {
        let blob_store = BlobStore::new_memory();
        let dag_store = DagStore::new(blob_store);

        let value = serde_json::json!({"hello": "world", "count": 42});
        let cid = dag_store.put(&value).await.unwrap();

        assert_eq!(cid.codec(), DAG_CBOR_CODEC);

        let retrieved: serde_json::Value = dag_store.get(&cid).await.unwrap();
        assert_eq!(retrieved, value);
    }

    #[tokio::test]
    async fn test_dag_store_epr_head() {
        let blob_store = BlobStore::new_memory();
        let dag_store = DagStore::new(blob_store);

        let head = EprHead {
            version: 1,
            id: "test-concept".to_string(),
            content: "bafkreihdwdcefgh4dqkjv67uzcmw7ojee6xedzdetojuzjevtenxquvyku".to_string(),
            lamad: EprLamadContext {
                title: "Test".to_string(),
                content_type: "concept".to_string(),
                description: None,
                content_format: None,
                tags: vec![],
            },
            shefa: EprShefaContext {
                stewards: vec![],
                allocations: vec![],
            },
            qahal: EprQahalContext {
                reach: None,
                layer: None,
            },
            relationships: vec![],
            author: None,
            updated: None,
        };

        let (cid, _links) = dag_store.put_epr_head(&head).await.unwrap();
        assert_eq!(cid.codec(), DAG_CBOR_CODEC);

        let retrieved = dag_store.get_epr_head(&cid).await.unwrap();
        assert_eq!(retrieved, head);
    }

    #[tokio::test]
    async fn test_dag_store_get_block() {
        let blob_store = BlobStore::new_memory();
        let dag_store = DagStore::new(blob_store);

        let value = serde_json::json!({"data": true});
        let cid = dag_store.put(&value).await.unwrap();

        let block = dag_store.get_block(&cid).await.unwrap();
        assert!(block.is_some());

        // Non-existent CID should return None
        let fake_cid = compute_dag_cbor_cid(b"nonexistent");
        let missing = dag_store.get_block(&fake_cid).await.unwrap();
        assert!(missing.is_none());
    }
}
