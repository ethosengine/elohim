//! Shard encoding and manifest generation
//!
//! Implements the unified shard model where every blob has a manifest,
//! even single-shard blobs. This enables consistent handling across:
//! - Single blob on single host (encoding="none", 1 shard)
//! - Chunked large files (encoding="chunked", N sequential shards)
//! - Reed-Solomon distributed (encoding="rs-4-7", N+M erasure-coded shards)
//!
//! The manifest is designed to be stored in Holochain DHT while actual
//! shard bytes are stored locally and served via HTTP.

use crate::blob_store::BlobStore;
use reed_solomon_erasure::galois_8::ReedSolomon;
use serde::{Deserialize, Serialize};
use std::io;
use tracing::{debug, info};

/// Default shard size (1MB)
pub const DEFAULT_SHARD_SIZE: usize = 1024 * 1024;

/// Threshold for Reed-Solomon encoding (10MB)
pub const RS_THRESHOLD: usize = 10 * 1024 * 1024;

/// Maximum blob size for single shard (16MB - Holochain limit)
pub const SINGLE_SHARD_MAX: usize = 16 * 1024 * 1024;

/// Shard manifest - matches DNA ShardManifest structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardManifest {
    /// CID (Content Identifier) of the blob - IPFS-compatible
    pub blob_cid: String,

    /// Original blob hash (sha256-xxx) - for backward compatibility
    pub blob_hash: String,

    /// Total size of the original blob in bytes
    pub total_size: u64,

    /// MIME type (video/mp4, audio/mpeg, etc.)
    pub mime_type: String,

    /// Encoding type (none, chunked, rs-4-7, rs-8-12)
    pub encoding: String,

    /// Number of data shards (for RS encoding, minimum needed to reconstruct)
    pub data_shards: u8,

    /// Total number of shards (data + parity)
    pub total_shards: u8,

    /// Size of each shard in bytes (last data shard may be smaller)
    pub shard_size: u64,

    /// Ordered list of shard hashes
    pub shard_hashes: Vec<String>,

    /// Visibility level
    pub reach: String,

    /// Author agent ID (optional)
    pub author_id: Option<String>,

    /// When manifest was created (ISO8601)
    pub created_at: String,

    /// When manifest was last verified
    pub verified_at: Option<String>,
}

/// Configuration for shard encoding
#[derive(Debug, Clone)]
pub struct ShardConfig {
    /// Size of each shard in bytes
    pub shard_size: usize,

    /// Number of data shards for Reed-Solomon
    pub rs_data_shards: u8,

    /// Number of parity shards for Reed-Solomon
    pub rs_parity_shards: u8,

    /// Threshold for using Reed-Solomon (bytes)
    pub rs_threshold: usize,

    /// Maximum size for single-shard blobs (bytes)
    /// Blobs larger than this use chunked or RS encoding
    pub single_shard_max: usize,
}

impl Default for ShardConfig {
    fn default() -> Self {
        Self {
            shard_size: DEFAULT_SHARD_SIZE,
            rs_data_shards: 4,
            rs_parity_shards: 3,
            rs_threshold: RS_THRESHOLD,
            single_shard_max: SINGLE_SHARD_MAX,
        }
    }
}

/// Shard encoder for creating manifests and encoding/decoding shards
pub struct ShardEncoder {
    config: ShardConfig,
}

impl ShardEncoder {
    /// Create a new shard encoder with given config
    pub fn new(config: ShardConfig) -> Self {
        Self { config }
    }

    /// Determine encoding type based on blob size and config
    pub fn determine_encoding(&self, size: usize) -> &'static str {
        if size <= self.config.single_shard_max {
            "none"
        } else if size < self.config.rs_threshold {
            "chunked"
        } else {
            "rs-4-7"
        }
    }

    /// Create a manifest for a blob
    pub fn create_manifest(&self, data: &[u8], mime_type: &str, reach: &str) -> ShardManifest {
        let (blob_cid, blob_hash) = BlobStore::compute_addresses(data);
        let blob_cid_str = blob_cid.to_string();
        let total_size = data.len() as u64;
        let encoding = self.determine_encoding(data.len());

        let (data_shards, total_shards, shard_size, shard_hashes) = match encoding {
            "none" => {
                // Single shard is the entire blob
                (1, 1, total_size, vec![blob_hash.clone()])
            }
            "chunked" => {
                // Split into sequential chunks
                let shard_size = self.config.shard_size as u64;
                let num_shards = (data.len() + self.config.shard_size - 1) / self.config.shard_size;
                let mut hashes = Vec::with_capacity(num_shards);

                for i in 0..num_shards {
                    let start = i * self.config.shard_size;
                    let end = ((i + 1) * self.config.shard_size).min(data.len());
                    let chunk = &data[start..end];
                    hashes.push(BlobStore::compute_hash(chunk));
                }

                (num_shards as u8, num_shards as u8, shard_size, hashes)
            }
            _ => {
                // Reed-Solomon encoding
                let rs = ReedSolomon::new(
                    self.config.rs_data_shards as usize,
                    self.config.rs_parity_shards as usize,
                )
                .unwrap();

                let shard_size = (data.len() + self.config.rs_data_shards as usize - 1)
                    / self.config.rs_data_shards as usize;

                // Pad data to align with shard size
                let mut padded_data = data.to_vec();
                padded_data.resize(shard_size * self.config.rs_data_shards as usize, 0);

                // Split into data shards
                let mut shards: Vec<Vec<u8>> = (0..self.config.rs_data_shards as usize)
                    .map(|i| padded_data[i * shard_size..(i + 1) * shard_size].to_vec())
                    .collect();

                // Add parity shards
                for _ in 0..self.config.rs_parity_shards {
                    shards.push(vec![0u8; shard_size]);
                }

                // Encode parity
                let mut shard_refs: Vec<&mut [u8]> =
                    shards.iter_mut().map(|s| s.as_mut_slice()).collect();
                rs.encode(&mut shard_refs).unwrap();

                // Compute hashes
                let hashes: Vec<String> = shards.iter().map(|s| BlobStore::compute_hash(s)).collect();

                (
                    self.config.rs_data_shards,
                    self.config.rs_data_shards + self.config.rs_parity_shards,
                    shard_size as u64,
                    hashes,
                )
            }
        };

        let now = chrono::Utc::now().to_rfc3339();

        ShardManifest {
            blob_cid: blob_cid_str,
            blob_hash,
            total_size,
            mime_type: mime_type.to_string(),
            encoding: encoding.to_string(),
            data_shards,
            total_shards,
            shard_size,
            shard_hashes,
            reach: reach.to_string(),
            author_id: None,
            created_at: now.clone(),
            verified_at: Some(now),
        }
    }

    /// Create shards from data based on encoding type
    pub fn create_shards(&self, data: &[u8], encoding: &str) -> Vec<Vec<u8>> {
        match encoding {
            "none" => vec![data.to_vec()],
            "chunked" => {
                data.chunks(self.config.shard_size)
                    .map(|chunk| chunk.to_vec())
                    .collect()
            }
            _ => {
                // Reed-Solomon encoding
                let rs = ReedSolomon::new(
                    self.config.rs_data_shards as usize,
                    self.config.rs_parity_shards as usize,
                )
                .unwrap();

                let shard_size = (data.len() + self.config.rs_data_shards as usize - 1)
                    / self.config.rs_data_shards as usize;

                // Pad data
                let mut padded_data = data.to_vec();
                padded_data.resize(shard_size * self.config.rs_data_shards as usize, 0);

                // Split into shards
                let mut shards: Vec<Vec<u8>> = (0..self.config.rs_data_shards as usize)
                    .map(|i| padded_data[i * shard_size..(i + 1) * shard_size].to_vec())
                    .collect();

                // Add parity shards
                for _ in 0..self.config.rs_parity_shards {
                    shards.push(vec![0u8; shard_size]);
                }

                // Encode
                let mut shard_refs: Vec<&mut [u8]> =
                    shards.iter_mut().map(|s| s.as_mut_slice()).collect();
                rs.encode(&mut shard_refs).unwrap();

                shards
            }
        }
    }

    /// Reconstruct data from shards (with possible missing shards for RS encoding)
    pub fn reconstruct(
        &self,
        manifest: &ShardManifest,
        shards: &[Option<Vec<u8>>],
    ) -> Result<Vec<u8>, io::Error> {
        match manifest.encoding.as_str() {
            "none" => {
                shards[0]
                    .clone()
                    .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Missing shard"))
            }
            "chunked" => {
                // All shards must be present for chunked encoding
                let mut data = Vec::with_capacity(manifest.total_size as usize);
                for (i, shard) in shards.iter().enumerate() {
                    match shard {
                        Some(s) => data.extend_from_slice(s),
                        None => {
                            return Err(io::Error::new(
                                io::ErrorKind::NotFound,
                                format!("Missing shard {}", i),
                            ))
                        }
                    }
                }
                data.truncate(manifest.total_size as usize);
                Ok(data)
            }
            _ => {
                // Reed-Solomon reconstruction
                let rs = ReedSolomon::new(
                    manifest.data_shards as usize,
                    (manifest.total_shards - manifest.data_shards) as usize,
                )
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

                let shard_size = manifest.shard_size as usize;
                let total = manifest.total_shards as usize;

                // Count present shards
                let present_count = shards.iter().filter(|s| s.is_some()).count();
                if present_count < manifest.data_shards as usize {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "Need at least {} shards, only have {}",
                            manifest.data_shards, present_count
                        ),
                    ));
                }

                // Build shards array for reed-solomon (fill missing with zeros)
                let mut shard_vecs: Vec<Vec<u8>> = Vec::with_capacity(total);
                let mut present_flags: Vec<bool> = Vec::with_capacity(total);

                for shard_opt in shards.iter() {
                    if let Some(data) = shard_opt {
                        shard_vecs.push(data.clone());
                        present_flags.push(true);
                    } else {
                        shard_vecs.push(vec![0u8; shard_size]);
                        present_flags.push(false);
                    }
                }

                // Create mutable slice refs for reconstruction
                let mut shard_refs: Vec<(&mut [u8], bool)> = shard_vecs
                    .iter_mut()
                    .zip(present_flags.iter())
                    .map(|(s, &p)| (s.as_mut_slice(), p))
                    .collect();

                rs.reconstruct_data(&mut shard_refs)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

                // Concatenate data shards (shard_vecs updated in place by reconstruct)
                let mut data = Vec::with_capacity(manifest.total_size as usize);
                for i in 0..manifest.data_shards as usize {
                    data.extend_from_slice(&shard_vecs[i]);
                }
                data.truncate(manifest.total_size as usize);

                Ok(data)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_shard_manifest() {
        let encoder = ShardEncoder::new(ShardConfig::default());
        let data = b"Hello, Elohim!";
        let manifest = encoder.create_manifest(data, "text/plain", "commons");

        assert_eq!(manifest.encoding, "none");
        assert_eq!(manifest.data_shards, 1);
        assert_eq!(manifest.total_shards, 1);
        assert_eq!(manifest.shard_hashes.len(), 1);
        assert_eq!(manifest.blob_hash, manifest.shard_hashes[0]);
        // Verify CID is present and valid
        assert!(manifest.blob_cid.starts_with("bafkrei")); // CIDv1 with raw codec
    }

    #[test]
    fn test_chunked_manifest() {
        let encoder = ShardEncoder::new(ShardConfig {
            shard_size: 10,
            single_shard_max: 50, // Force chunking for data > 50 bytes
            rs_threshold: 500,    // RS only for data > 500 bytes
            ..Default::default()
        });

        // Data larger than single shard max but smaller than RS threshold
        let mut data = vec![0u8; 100];
        for (i, byte) in data.iter_mut().enumerate() {
            *byte = (i % 256) as u8;
        }

        let manifest = encoder.create_manifest(&data, "application/octet-stream", "family");

        // With shard_size=10 and 100 bytes, we get 10 chunks
        assert_eq!(manifest.encoding, "chunked");
        assert_eq!(manifest.shard_hashes.len(), 10);
    }

    #[test]
    fn test_create_and_reconstruct_shards() {
        let encoder = ShardEncoder::new(ShardConfig {
            shard_size: 10,
            rs_data_shards: 4,
            rs_parity_shards: 3,
            rs_threshold: 50,      // RS for data > 50 bytes
            single_shard_max: 10,  // Force RS encoding for test data
        });

        let data: Vec<u8> = (0..100).map(|i| (i % 256) as u8).collect();
        let manifest = encoder.create_manifest(&data, "application/octet-stream", "commons");

        // Verify RS encoding was used
        assert_eq!(manifest.encoding, "rs-4-7");

        // Create shards
        let shards = encoder.create_shards(&data, &manifest.encoding);
        assert_eq!(shards.len(), manifest.total_shards as usize);

        // Reconstruct with all shards present
        let shard_opts: Vec<Option<Vec<u8>>> = shards.iter().map(|s| Some(s.clone())).collect();
        let reconstructed = encoder.reconstruct(&manifest, &shard_opts).unwrap();
        assert_eq!(reconstructed, data);
    }

    #[test]
    fn test_reconstruct_with_missing_shards() {
        let encoder = ShardEncoder::new(ShardConfig {
            shard_size: 25,
            rs_data_shards: 4,
            rs_parity_shards: 3,
            rs_threshold: 50,      // RS for data > 50 bytes
            single_shard_max: 10,  // Force RS encoding for test data
        });

        let data: Vec<u8> = (0..100).map(|i| (i % 256) as u8).collect();
        let manifest = encoder.create_manifest(&data, "application/octet-stream", "commons");

        // Verify RS encoding was used
        assert_eq!(manifest.encoding, "rs-4-7");
        assert!(manifest.total_shards >= 7, "Expected at least 7 shards for rs-4-7");

        let shards = encoder.create_shards(&data, &manifest.encoding);

        // Remove 2 shards (we can lose up to 3 with rs-4-7)
        let mut shard_opts: Vec<Option<Vec<u8>>> = shards.iter().map(|s| Some(s.clone())).collect();
        shard_opts[0] = None; // Remove first data shard
        shard_opts[5] = None; // Remove a parity shard

        let reconstructed = encoder.reconstruct(&manifest, &shard_opts).unwrap();
        assert_eq!(reconstructed, data);
    }
}
