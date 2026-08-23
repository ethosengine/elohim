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

/// Default shard size (1MB)
pub const DEFAULT_SHARD_SIZE: usize = 1024 * 1024;

/// Maximum blob size for single shard (16MB - Holochain limit).
/// Blobs ≤ this stay as a single shard with no encoding overhead.
pub const SINGLE_SHARD_MAX: usize = 16 * 1024 * 1024;

/// Threshold for switching from chunked to Reed-Solomon (64MB).
/// Must be strictly greater than `SINGLE_SHARD_MAX` so the `"chunked"`
/// band is reachable: `(SINGLE_SHARD_MAX, RS_THRESHOLD]` → chunked,
/// `> RS_THRESHOLD` → RS-coded with parity.
pub const RS_THRESHOLD: usize = 64 * 1024 * 1024;

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

/// Role derived from a manifest's Reed-Solomon boundary. This deliberately
/// does not ride `ShardAssignment`: the role is immutable manifest math, not
/// a second registry/DNA fact that can drift from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShardRole {
    Data,
    Parity,
}

/// Derive the role of one shard. Non-RS encodings have no optional parity, so
/// every shard is data/required.
pub fn shard_role(manifest: &ShardManifest, index: usize) -> ShardRole {
    if manifest.encoding.starts_with("rs-") && index >= manifest.data_shards as usize {
        ShardRole::Parity
    } else {
        ShardRole::Data
    }
}

/// One deterministic round-robin placement slot over an already diversity-
/// ranked holder list. Manifest order is preserved, which means RS data shards
/// consume the strongest household-diverse prefix before parity does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShardPlacementSlot {
    pub shard_index: usize,
    pub holder_index: usize,
    pub role: ShardRole,
}

pub fn plan_shard_placement_slots(
    manifest: &ShardManifest,
    holder_count: usize,
) -> Vec<ShardPlacementSlot> {
    if holder_count == 0 {
        return Vec::new();
    }
    manifest
        .shard_hashes
        .iter()
        .enumerate()
        .map(|(shard_index, _)| ShardPlacementSlot {
            shard_index,
            holder_index: shard_index % holder_count,
            role: shard_role(manifest, shard_index),
        })
        .collect()
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

    /// Determine encoding type based on blob size and config.
    ///
    /// Three bands:
    /// - `[0, single_shard_max]` → "none" (single shard, no overhead)
    /// - `(single_shard_max, rs_threshold]` → "chunked" (sequential, no parity)
    /// - `(rs_threshold, ∞)` → "rs-4-7" (Reed-Solomon with parity)
    ///
    /// Requires `single_shard_max < rs_threshold` in the config; the
    /// default constants enforce this invariant.
    pub fn determine_encoding(&self, size: usize) -> &'static str {
        if size <= self.config.single_shard_max {
            "none"
        } else if size <= self.config.rs_threshold {
            "chunked"
        } else {
            "rs-4-7"
        }
    }

    /// Create a manifest for a blob.
    ///
    /// Returns `io::Error` if Reed-Solomon setup or encoding fails for
    /// the "rs-4-7" path. `"none"` and `"chunked"` paths never fail
    /// (they do no math); they still ride on the Result signature for
    /// caller uniformity.
    pub fn create_manifest(
        &self,
        data: &[u8],
        mime_type: &str,
        reach: &str,
    ) -> io::Result<ShardManifest> {
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
                let num_shards = data.len().div_ceil(self.config.shard_size);
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
                .map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!(
                            "ReedSolomon::new(data={}, parity={}): {}",
                            self.config.rs_data_shards, self.config.rs_parity_shards, e
                        ),
                    )
                })?;

                let shard_size = data.len().div_ceil(self.config.rs_data_shards as usize);

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
                rs.encode(&mut shard_refs).map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("reed-solomon encode: {}", e),
                    )
                })?;

                // Compute hashes
                let hashes: Vec<String> =
                    shards.iter().map(|s| BlobStore::compute_hash(s)).collect();

                (
                    self.config.rs_data_shards,
                    self.config.rs_data_shards + self.config.rs_parity_shards,
                    shard_size as u64,
                    hashes,
                )
            }
        };

        let now = chrono::Utc::now().to_rfc3339();

        Ok(ShardManifest {
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
        })
    }

    /// Create shards from data based on encoding type.
    ///
    /// Returns `io::Error` if Reed-Solomon setup or encoding fails.
    pub fn create_shards(&self, data: &[u8], encoding: &str) -> io::Result<Vec<Vec<u8>>> {
        match encoding {
            "none" => Ok(vec![data.to_vec()]),
            "chunked" => Ok(data
                .chunks(self.config.shard_size)
                .map(|chunk| chunk.to_vec())
                .collect()),
            _ => {
                // Reed-Solomon encoding
                let rs = ReedSolomon::new(
                    self.config.rs_data_shards as usize,
                    self.config.rs_parity_shards as usize,
                )
                .map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("ReedSolomon::new: {}", e),
                    )
                })?;

                let shard_size = data.len().div_ceil(self.config.rs_data_shards as usize);

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
                rs.encode(&mut shard_refs).map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("reed-solomon encode: {}", e),
                    )
                })?;

                Ok(shards)
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
            "none" => shards[0]
                .clone()
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Missing shard")),
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
                for shard in shard_vecs.iter().take(manifest.data_shards as usize) {
                    data.extend_from_slice(shard);
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

    /// N1: the default config used to make "chunked" unreachable because
    /// SINGLE_SHARD_MAX (16MB) > RS_THRESHOLD (10MB, old). Prove the band
    /// is reachable now and the three boundaries behave correctly.
    #[test]
    fn determine_encoding_mid_band_is_reachable_with_default_config() {
        let enc = ShardEncoder::new(ShardConfig::default());
        assert_eq!(enc.determine_encoding(SINGLE_SHARD_MAX), "none");
        assert_eq!(enc.determine_encoding(SINGLE_SHARD_MAX + 1), "chunked");
        assert_eq!(enc.determine_encoding(32 * 1024 * 1024), "chunked");
        assert_eq!(enc.determine_encoding(RS_THRESHOLD), "chunked");
        assert_eq!(enc.determine_encoding(RS_THRESHOLD + 1), "rs-4-7");
    }

    /// N2: impossible RS config now propagates as Err instead of panicking.
    /// reed-solomon-erasure galois_8 supports at most 256 total shards.
    #[test]
    fn create_manifest_returns_err_on_impossible_rs_config() {
        let bad = ShardConfig {
            rs_data_shards: 200,
            rs_parity_shards: 200,
            rs_threshold: 8,
            single_shard_max: 1,
            ..ShardConfig::default()
        };
        let enc = ShardEncoder::new(bad);
        let data = vec![0u8; 128];
        let result = enc.create_manifest(&data, "application/octet-stream", "commons");
        assert!(result.is_err(), "expected Err from 200+200 RS config");
    }

    #[test]
    fn test_single_shard_manifest() {
        let encoder = ShardEncoder::new(ShardConfig::default());
        let data = b"Hello, Elohim!";
        let manifest = encoder
            .create_manifest(data, "text/plain", "commons")
            .unwrap();

        assert_eq!(manifest.encoding, "none");
        assert_eq!(manifest.data_shards, 1);
        assert_eq!(manifest.total_shards, 1);
        assert_eq!(manifest.shard_hashes.len(), 1);
        assert_eq!(manifest.blob_hash, manifest.shard_hashes[0]);
        // Verify CID is present and valid
        assert!(manifest.blob_cid.starts_with("bafkrei")); // CIDv1 with raw codec
    }

    #[test]
    fn rs_roles_are_derived_from_the_data_shard_boundary() {
        let encoder = ShardEncoder::new(ShardConfig {
            rs_threshold: 50,
            single_shard_max: 10,
            ..Default::default()
        });
        let manifest = encoder
            .create_manifest(&[7; 100], "application/octet-stream", "commons")
            .unwrap();

        assert_eq!(manifest.encoding, "rs-4-7");
        assert!(
            (0..4).all(|index| shard_role(&manifest, index) == ShardRole::Data),
            "the data prefix is derived from data_shards"
        );
        assert!(
            (4..7).all(|index| shard_role(&manifest, index) == ShardRole::Parity),
            "the remaining RS shards are parity"
        );
    }

    #[test]
    fn rs_data_prefix_consumes_the_household_diverse_holder_prefix_first() {
        let encoder = ShardEncoder::new(ShardConfig {
            rs_threshold: 50,
            single_shard_max: 10,
            ..Default::default()
        });
        let manifest = encoder
            .create_manifest(&[7; 100], "application/octet-stream", "commons")
            .unwrap();
        // The live peer selector orders one holder per household first. Three
        // households with two peers each therefore arrive as A,B,C,A,B,C.
        let households = ["A", "B", "C", "A", "B", "C"];
        let plan = plan_shard_placement_slots(&manifest, households.len());
        let data_households: std::collections::HashSet<_> = plan
            .iter()
            .filter(|slot| slot.role == ShardRole::Data)
            .map(|slot| households[slot.holder_index])
            .collect();

        assert_eq!(data_households.len(), 3);
        assert_eq!(
            plan.iter()
                .take(4)
                .filter(|s| s.role == ShardRole::Data)
                .count(),
            4
        );
        assert_eq!(
            plan.iter()
                .skip(4)
                .filter(|s| s.role == ShardRole::Parity)
                .count(),
            3
        );
    }

    #[test]
    fn chunked_placement_keeps_the_existing_round_robin_order() {
        let encoder = ShardEncoder::new(ShardConfig {
            shard_size: 10,
            single_shard_max: 5,
            rs_threshold: 500,
            ..Default::default()
        });
        let manifest = encoder
            .create_manifest(&[7; 73], "application/octet-stream", "commons")
            .unwrap();
        let plan = plan_shard_placement_slots(&manifest, 3);

        assert_eq!(manifest.encoding, "chunked");
        assert_eq!(
            plan.iter()
                .map(|slot| slot.holder_index)
                .collect::<Vec<_>>(),
            vec![0, 1, 2, 0, 1, 2, 0, 1]
        );
        assert!(plan.iter().all(|slot| slot.role == ShardRole::Data));
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

        let manifest = encoder
            .create_manifest(&data, "application/octet-stream", "family")
            .unwrap();

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
            rs_threshold: 50,     // RS for data > 50 bytes
            single_shard_max: 10, // Force RS encoding for test data
        });

        let data: Vec<u8> = (0..100).map(|i| (i % 256) as u8).collect();
        let manifest = encoder
            .create_manifest(&data, "application/octet-stream", "commons")
            .unwrap();

        // Verify RS encoding was used
        assert_eq!(manifest.encoding, "rs-4-7");

        // Create shards
        let shards = encoder.create_shards(&data, &manifest.encoding).unwrap();
        assert_eq!(shards.len(), manifest.total_shards as usize);

        // Reconstruct with all shards present
        let shard_opts: Vec<Option<Vec<u8>>> = shards.iter().map(|s| Some(s.clone())).collect();
        let reconstructed = encoder.reconstruct(&manifest, &shard_opts).unwrap();
        assert_eq!(reconstructed, data);
    }

    /// The invariant the ingest path now leans on: for an erasure-coded blob,
    /// `create_shards` produces exactly the bytes `create_manifest` hashed —
    /// including when the payload length is NOT a multiple of `data_shards`,
    /// which is where the padded tail lives.
    ///
    /// The ingest path used to hand-slice the raw body as sequential chunks
    /// instead, which for `rs-4-7` mis-hashes the unpadded tail data shard and
    /// then indexes past the end of the body at the first parity shard.
    #[test]
    fn rs_create_shards_match_the_hashes_create_manifest_minted() {
        let encoder = ShardEncoder::new(ShardConfig {
            shard_size: 10,
            rs_data_shards: 4,
            rs_parity_shards: 3,
            rs_threshold: 50,
            single_shard_max: 10,
        });

        // 102 % 4 == 2 — the same non-multiple shape as the live 71,763,974-byte
        // bundle that panicked the HTTP task.
        let data: Vec<u8> = (0..102u32)
            .map(|i| (i.wrapping_mul(37) % 251) as u8)
            .collect();
        let manifest = encoder
            .create_manifest(&data, "application/octet-stream", "commons")
            .unwrap();
        assert_eq!(manifest.encoding, "rs-4-7");

        let shards = encoder.create_shards(&data, &manifest.encoding).unwrap();
        assert_eq!(shards.len(), manifest.shard_hashes.len());

        for (i, (shard, named)) in shards.iter().zip(manifest.shard_hashes.iter()).enumerate() {
            assert_eq!(
                BlobStore::compute_hash(shard),
                *named,
                "shard {i} produced by create_shards must hash to the manifest's name for it"
            );
            assert_eq!(
                shard.len(),
                manifest.shard_size as usize,
                "every RS shard, including the padded tail, is shard_size long"
            );
        }
    }

    #[test]
    fn test_reconstruct_with_missing_shards() {
        let encoder = ShardEncoder::new(ShardConfig {
            shard_size: 25,
            rs_data_shards: 4,
            rs_parity_shards: 3,
            rs_threshold: 50,     // RS for data > 50 bytes
            single_shard_max: 10, // Force RS encoding for test data
        });

        let data: Vec<u8> = (0..100).map(|i| (i % 256) as u8).collect();
        let manifest = encoder
            .create_manifest(&data, "application/octet-stream", "commons")
            .unwrap();

        // Verify RS encoding was used
        assert_eq!(manifest.encoding, "rs-4-7");
        assert!(
            manifest.total_shards >= 7,
            "Expected at least 7 shards for rs-4-7"
        );

        let shards = encoder.create_shards(&data, &manifest.encoding).unwrap();

        // Remove 2 shards (we can lose up to 3 with rs-4-7)
        let mut shard_opts: Vec<Option<Vec<u8>>> = shards.iter().map(|s| Some(s.clone())).collect();
        shard_opts[0] = None; // Remove first data shard
        shard_opts[5] = None; // Remove a parity shard

        let reconstructed = encoder.reconstruct(&manifest, &shard_opts).unwrap();
        assert_eq!(reconstructed, data);
    }

    #[test]
    fn test_rs_reconstruct_after_dropping_max_parity() {
        let encoder = ShardEncoder::new(ShardConfig {
            shard_size: 25,
            rs_data_shards: 4,
            rs_parity_shards: 3,
            rs_threshold: 50,
            single_shard_max: 10,
        });

        let data: Vec<u8> = (0..200).map(|i| (i % 256) as u8).collect();
        let manifest = encoder
            .create_manifest(&data, "application/octet-stream", "commons")
            .unwrap();
        let shards = encoder.create_shards(&data, &manifest.encoding).unwrap();

        assert_eq!(manifest.encoding, "rs-4-7");
        assert_eq!(shards.len(), 7);

        // Drop all 3 parity shards (indices 4, 5, 6)
        let mut shard_opts: Vec<Option<Vec<u8>>> = shards.iter().map(|s| Some(s.clone())).collect();
        shard_opts[4] = None;
        shard_opts[5] = None;
        shard_opts[6] = None;

        let reconstructed = encoder.reconstruct(&manifest, &shard_opts).unwrap();
        assert_eq!(
            reconstructed, data,
            "Must reconstruct from data shards alone"
        );
    }

    #[test]
    fn test_rs_reconstruct_after_dropping_3_mixed_shards() {
        let encoder = ShardEncoder::new(ShardConfig {
            shard_size: 25,
            rs_data_shards: 4,
            rs_parity_shards: 3,
            rs_threshold: 50,
            single_shard_max: 10,
        });

        let data: Vec<u8> = (0..200).map(|i| (i % 256) as u8).collect();
        let manifest = encoder
            .create_manifest(&data, "application/octet-stream", "commons")
            .unwrap();
        let shards = encoder.create_shards(&data, &manifest.encoding).unwrap();

        // Drop 2 data shards and 1 parity shard
        let mut shard_opts: Vec<Option<Vec<u8>>> = shards.iter().map(|s| Some(s.clone())).collect();
        shard_opts[0] = None; // data shard 0
        shard_opts[2] = None; // data shard 2
        shard_opts[5] = None; // parity shard 1

        let reconstructed = encoder.reconstruct(&manifest, &shard_opts).unwrap();
        assert_eq!(
            reconstructed, data,
            "Must reconstruct from 4 remaining shards"
        );
    }

    #[test]
    fn test_rs_fails_when_too_many_shards_lost() {
        let encoder = ShardEncoder::new(ShardConfig {
            shard_size: 25,
            rs_data_shards: 4,
            rs_parity_shards: 3,
            rs_threshold: 50,
            single_shard_max: 10,
        });

        let data: Vec<u8> = (0..200).map(|i| (i % 256) as u8).collect();
        let manifest = encoder
            .create_manifest(&data, "application/octet-stream", "commons")
            .unwrap();
        let shards = encoder.create_shards(&data, &manifest.encoding).unwrap();

        // Drop 4 shards — only 3 remain, but we need 4 data shards minimum
        let mut shard_opts: Vec<Option<Vec<u8>>> = shards.iter().map(|s| Some(s.clone())).collect();
        shard_opts[0] = None;
        shard_opts[1] = None;
        shard_opts[2] = None;
        shard_opts[4] = None;

        let result = encoder.reconstruct(&manifest, &shard_opts);
        assert!(
            result.is_err(),
            "Must fail when fewer than data_shards remain"
        );
    }

    #[test]
    fn test_chunked_roundtrip() {
        let encoder = ShardEncoder::new(ShardConfig {
            shard_size: 10,
            single_shard_max: 5,
            rs_threshold: 500,
            ..Default::default()
        });

        let data: Vec<u8> = (0..73).map(|i| (i % 256) as u8).collect();
        let manifest = encoder
            .create_manifest(&data, "text/plain", "commons")
            .unwrap();
        let shards = encoder.create_shards(&data, &manifest.encoding).unwrap();

        assert_eq!(manifest.encoding, "chunked");
        assert_eq!(shards.len(), 8); // 73 bytes / 10 = 8 chunks

        let shard_opts: Vec<Option<Vec<u8>>> = shards.iter().map(|s| Some(s.clone())).collect();
        let reconstructed = encoder.reconstruct(&manifest, &shard_opts).unwrap();
        assert_eq!(reconstructed, data);
    }

    #[test]
    fn test_chunked_fails_on_missing_shard() {
        let encoder = ShardEncoder::new(ShardConfig {
            shard_size: 10,
            single_shard_max: 5,
            rs_threshold: 500,
            ..Default::default()
        });

        let data: Vec<u8> = (0..73).map(|i| (i % 256) as u8).collect();
        let manifest = encoder
            .create_manifest(&data, "text/plain", "commons")
            .unwrap();
        let shards = encoder.create_shards(&data, &manifest.encoding).unwrap();

        let mut shard_opts: Vec<Option<Vec<u8>>> = shards.iter().map(|s| Some(s.clone())).collect();
        shard_opts[3] = None;

        let result = encoder.reconstruct(&manifest, &shard_opts);
        assert!(result.is_err(), "Chunked encoding requires all shards");
    }

    #[test]
    fn test_single_shard_roundtrip() {
        let encoder = ShardEncoder::new(ShardConfig::default());

        let data = b"The fruit back on the tree.";
        let manifest = encoder
            .create_manifest(data, "text/plain", "commons")
            .unwrap();
        let shards = encoder.create_shards(data, &manifest.encoding).unwrap();

        assert_eq!(manifest.encoding, "none");
        assert_eq!(shards.len(), 1);

        let shard_opts: Vec<Option<Vec<u8>>> = shards.iter().map(|s| Some(s.clone())).collect();
        let reconstructed = encoder.reconstruct(&manifest, &shard_opts).unwrap();
        assert_eq!(reconstructed, data);
    }
}
