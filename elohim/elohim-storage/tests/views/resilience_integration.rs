//! Integration test: RS encode → simulate peer distribution → lose peers → reconstruct
//!
//! Proves the Reed-Solomon pipeline works end-to-end without a running P2P network.
//! This test simulates:
//! 1. Encoding a blob into one RS stripe (4 data + 3 parity shards)
//! 2. Distributing shards across 5 simulated peers
//! 3. Simulating peer failures (nodes going offline)
//! 4. Verifying that reconstruction fails with too few shards
//! 5. Verifying that reconstruction succeeds with sufficient shards
//! 6. Validating byte-CID integrity after reconstruction

use elohim_epr::address::compute_bytes_cid;
use elohim_storage::sharding::{ShardConfig, ShardEncoder, ShardManifest};

/// Erasure forced on (threshold below the payload) with the RS(4,3) code.
fn rs_4_3() -> ShardEncoder {
    ShardEncoder::new(ShardConfig {
        rs_threshold: 500,
        rs_data_shards: 4,
        rs_parity_shards: 3,
    })
}

/// Encode `data` as one stripe and return the manifest with that stripe's shards.
fn encode_one_stripe(data: &[u8]) -> (ShardManifest, Vec<Vec<u8>>) {
    let (manifest, mut stripes) = rs_4_3()
        .encode(data, "application/octet-stream", "commons")
        .unwrap();
    assert_eq!(manifest.stripes.len(), 1, "one stripe covers the payload");
    assert_eq!(manifest.data_shards(), 4);
    assert_eq!(manifest.stripes[0].shards.len(), 7);
    assert_eq!(manifest.blob_cid, compute_bytes_cid(data));
    (manifest, stripes.remove(0))
}

fn reconstruct(manifest: &ShardManifest, shards: Vec<Option<Vec<u8>>>) -> Option<Vec<u8>> {
    ShardEncoder::reconstruct(manifest, &[shards]).ok()
}

#[test]
fn test_full_rs_pipeline_with_simulated_peer_failure() {
    let data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
    let (manifest, shards) = encode_one_stripe(&data);

    // Distribution to 5 peers (round-robin):
    // Peer 0: [0, 5]  Peer 1: [1, 6]  Peer 2: [2]  Peer 3: [3]  Peer 4: [4]
    let peer_shards: Vec<Vec<usize>> = vec![vec![0, 5], vec![1, 6], vec![2], vec![3], vec![4]];
    let survivors = |offline: &[usize]| -> Vec<usize> {
        peer_shards
            .iter()
            .enumerate()
            .filter(|(i, _)| !offline.contains(i))
            .flat_map(|(_, indices)| indices.iter().copied())
            .collect()
    };
    let present = |indices: &[usize]| -> Vec<Option<Vec<u8>>> {
        let mut opts = vec![None; 7];
        for &i in indices {
            opts[i] = Some(shards[i].clone());
        }
        opts
    };

    // Two peers offline: only shards [2, 3, 4] remain — 3 of the 4 needed.
    let after_two = survivors(&[0, 1]);
    assert_eq!(after_two.len(), 3);
    assert!(
        reconstruct(&manifest, present(&after_two)).is_none(),
        "Should fail with only 3 of 4 needed shards"
    );

    // One peer offline: shards [1, 2, 3, 4, 6] remain — 5, need 4.
    let after_one = survivors(&[0]);
    assert_eq!(after_one.len(), 5);
    let reconstructed = reconstruct(&manifest, present(&after_one)).unwrap();
    assert_eq!(
        reconstructed, data,
        "Must reconstruct perfectly from 5 of 7 shards"
    );
    assert_eq!(
        compute_bytes_cid(&reconstructed),
        manifest.blob_cid,
        "the reconstruction hashes to the manifest's byte CID"
    );
}

#[test]
fn test_rs_pipeline_reconstruction_threshold() {
    let data: Vec<u8> = (0..512).map(|i| (i % 256) as u8).collect();
    let (manifest, shards) = encode_one_stripe(&data);

    // Exactly 4 shards (minimum needed): succeeds.
    let mut opts: Vec<Option<Vec<u8>>> = vec![None; 7];
    for (slot, shard) in opts.iter_mut().zip(&shards).take(4) {
        *slot = Some(shard.clone());
    }
    assert!(
        reconstruct(&manifest, opts).is_some(),
        "Should succeed with exactly data_shards (4) present"
    );

    // Only 3 shards: fails.
    let mut opts: Vec<Option<Vec<u8>>> = vec![None; 7];
    for (slot, shard) in opts.iter_mut().zip(&shards).take(3) {
        *slot = Some(shard.clone());
    }
    assert!(
        reconstruct(&manifest, opts).is_none(),
        "Should fail with only 3 shards"
    );
}

#[test]
fn test_rs_pipeline_with_mixed_shard_losses() {
    let data: Vec<u8> = (0..2000).map(|i| (i % 256) as u8).collect();
    let (manifest, shards) = encode_one_stripe(&data);

    // Lose data shards [0, 1] and parity shard [4]; keep [2, 3] + [5, 6].
    let mut opts: Vec<Option<Vec<u8>>> = vec![None; 7];
    for i in [2, 3, 5, 6] {
        opts[i] = Some(shards[i].clone());
    }
    let reconstructed = reconstruct(&manifest, opts).unwrap();
    assert_eq!(
        reconstructed, data,
        "Must reconstruct from 4 remaining shards"
    );
    assert_eq!(compute_bytes_cid(&reconstructed), manifest.blob_cid);
}

#[test]
fn test_rs_pipeline_all_parity_shards_lost() {
    let data: Vec<u8> = (0..1500).map(|i| (i % 256) as u8).collect();
    let (manifest, shards) = encode_one_stripe(&data);

    // Keep only data shards [0..4], lose all parity shards.
    let mut opts: Vec<Option<Vec<u8>>> = vec![None; 7];
    for (slot, shard) in opts.iter_mut().zip(&shards).take(4) {
        *slot = Some(shard.clone());
    }
    let reconstructed = reconstruct(&manifest, opts).unwrap();
    assert_eq!(
        reconstructed, data,
        "Must reconstruct from data shards alone"
    );
    assert_eq!(compute_bytes_cid(&reconstructed), manifest.blob_cid);
}

#[test]
fn test_rs_pipeline_refuses_an_altered_shard() {
    let data: Vec<u8> = (0..1200).map(|i| (i % 251) as u8).collect();
    let (manifest, shards) = encode_one_stripe(&data);

    // Exactly the 4 data shards, one of them altered: its CID no longer matches,
    // so it is not used and the stripe falls below the threshold.
    let mut opts: Vec<Option<Vec<u8>>> = vec![None; 7];
    for (slot, shard) in opts.iter_mut().zip(&shards).take(4) {
        *slot = Some(shard.clone());
    }
    opts[1].as_mut().unwrap()[0] ^= 0xff;
    assert!(
        reconstruct(&manifest, opts).is_none(),
        "an altered shard must never contribute to a reconstruction"
    );
}
