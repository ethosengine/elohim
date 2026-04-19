//! Integration test: RS encode → simulate peer distribution → lose peers → reconstruct
//!
//! Proves Reed-Solomon pipeline works end-to-end without a running P2P network.
//! This test simulates:
//! 1. Encoding a blob into RS shards
//! 2. Distributing shards across 5 simulated peers
//! 3. Simulating peer failures (nodes going offline)
//! 4. Verifying that reconstruction fails with too few shards
//! 5. Verifying that reconstruction succeeds with sufficient shards
//! 6. Validating hash integrity after reconstruction

use elohim_storage::sharding::{ShardConfig, ShardEncoder};

#[test]
fn test_full_rs_pipeline_with_simulated_peer_failure() {
    // 1. Create test data above RS threshold
    let data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();

    let config = ShardConfig {
        shard_size: 100,
        rs_data_shards: 4,
        rs_parity_shards: 3,
        rs_threshold: 500,
        single_shard_max: 100,
    };
    let encoder = ShardEncoder::new(config);

    // 2. Encode — simulates blob ingest
    let manifest = encoder.create_manifest(&data, "application/octet-stream", "commons").unwrap();
    assert_eq!(manifest.encoding, "rs-4-7");
    assert_eq!(manifest.shard_hashes.len(), 7);

    let shards = encoder.create_shards(&data, &manifest.encoding).unwrap();
    assert_eq!(shards.len(), 7);

    // 3. Simulate distribution to 5 peers (round-robin)
    // Peer 0: shards [0, 5]
    // Peer 1: shards [1, 6]
    // Peer 2: shards [2]
    // Peer 3: shards [3]
    // Peer 4: shards [4]
    let peer_shards: Vec<Vec<usize>> = vec![vec![0, 5], vec![1, 6], vec![2], vec![3], vec![4]];

    // 4. Simulate 2 peers going offline (lose shards 0, 1, 5, 6)
    let offline_peers = [0usize, 1];
    let available_after_2_offline: Vec<usize> = peer_shards
        .iter()
        .enumerate()
        .filter(|(i, _)| !offline_peers.contains(i))
        .flat_map(|(_, indices)| indices.iter().copied())
        .collect();

    // Only shards [2, 3, 4] remain — 3 shards, need 4
    assert_eq!(available_after_2_offline.len(), 3);

    let mut shard_opts_fail: Vec<Option<Vec<u8>>> = vec![None; 7];
    for &i in &available_after_2_offline {
        shard_opts_fail[i] = Some(shards[i].clone());
    }
    let result = encoder.reconstruct(&manifest, &shard_opts_fail);
    assert!(
        result.is_err(),
        "Should fail with only 3 of 4 needed shards"
    );

    // 5. Simulate only 1 peer going offline (lose shards 0, 5)
    let offline_one = [0usize];
    let available_after_1_offline: Vec<usize> = peer_shards
        .iter()
        .enumerate()
        .filter(|(i, _)| !offline_one.contains(i))
        .flat_map(|(_, indices)| indices.iter().copied())
        .collect();

    // Shards [1, 2, 3, 4, 6] remain — 5 shards, need 4
    assert_eq!(available_after_1_offline.len(), 5);

    let mut shard_opts_ok: Vec<Option<Vec<u8>>> = vec![None; 7];
    for &i in &available_after_1_offline {
        shard_opts_ok[i] = Some(shards[i].clone());
    }
    let reconstructed = encoder.reconstruct(&manifest, &shard_opts_ok).unwrap();
    assert_eq!(
        reconstructed, data,
        "Must reconstruct perfectly from 5 of 7 shards"
    );

    // 6. Verify hash integrity
    use sha2::{Digest, Sha256};
    let original_hash = format!("sha256-{}", hex::encode(Sha256::digest(&data)));
    let reconstructed_hash = format!("sha256-{}", hex::encode(Sha256::digest(&reconstructed)));
    assert_eq!(
        original_hash, reconstructed_hash,
        "Hash must match after reconstruction"
    );
}

#[test]
fn test_rs_pipeline_reconstruction_threshold() {
    // Test edge case: exactly at the threshold for reconstruction success/failure

    let data: Vec<u8> = (0..512).map(|i| (i % 256) as u8).collect();

    let config = ShardConfig {
        shard_size: 128,
        rs_data_shards: 4,
        rs_parity_shards: 3,
        rs_threshold: 500,
        single_shard_max: 50,
    };
    let encoder = ShardEncoder::new(config);

    let manifest = encoder.create_manifest(&data, "application/octet-stream", "commons").unwrap();
    assert_eq!(manifest.encoding, "rs-4-7");
    assert_eq!(manifest.data_shards, 4);
    assert_eq!(manifest.total_shards, 7);

    let shards = encoder.create_shards(&data, &manifest.encoding).unwrap();

    // Test 1: Exactly 4 shards (minimum needed) - should succeed
    let mut shard_opts: Vec<Option<Vec<u8>>> = vec![None; 7];
    for i in 0..4 {
        shard_opts[i] = Some(shards[i].clone());
    }
    let result = encoder.reconstruct(&manifest, &shard_opts);
    assert!(
        result.is_ok(),
        "Should succeed with exactly data_shards (4) present"
    );

    // Test 2: Only 3 shards (one below minimum) - should fail
    let mut shard_opts: Vec<Option<Vec<u8>>> = vec![None; 7];
    for i in 0..3 {
        shard_opts[i] = Some(shards[i].clone());
    }
    let result = encoder.reconstruct(&manifest, &shard_opts);
    assert!(result.is_err(), "Should fail with only 3 shards");
}

#[test]
fn test_rs_pipeline_with_mixed_shard_losses() {
    // Test that losing any combination of shards totaling less than parity_shards still works

    let data: Vec<u8> = (0..2000).map(|i| (i % 256) as u8).collect();

    let config = ShardConfig {
        shard_size: 512,
        rs_data_shards: 4,
        rs_parity_shards: 3,
        rs_threshold: 500,
        single_shard_max: 100,
    };
    let encoder = ShardEncoder::new(config);

    let manifest = encoder.create_manifest(&data, "application/octet-stream", "commons").unwrap();
    assert_eq!(manifest.encoding, "rs-4-7");

    let shards = encoder.create_shards(&data, &manifest.encoding).unwrap();

    // Test scenario: lose data shards [0, 1] and parity shard [4]
    // Remaining: [2, 3] (data) + [5, 6] (parity) = 4 total
    let mut shard_opts: Vec<Option<Vec<u8>>> = vec![None; 7];
    shard_opts[2] = Some(shards[2].clone()); // data shard 2
    shard_opts[3] = Some(shards[3].clone()); // data shard 3
    shard_opts[5] = Some(shards[5].clone()); // parity shard 1
    shard_opts[6] = Some(shards[6].clone()); // parity shard 2

    let reconstructed = encoder.reconstruct(&manifest, &shard_opts).unwrap();
    assert_eq!(
        reconstructed, data,
        "Must reconstruct from 4 remaining shards (4 data, 3 parity, lost 3)"
    );

    // Verify hash integrity
    use sha2::{Digest, Sha256};
    let original_hash = format!("sha256-{}", hex::encode(Sha256::digest(&data)));
    let reconstructed_hash = format!("sha256-{}", hex::encode(Sha256::digest(&reconstructed)));
    assert_eq!(original_hash, reconstructed_hash);
}

#[test]
fn test_rs_pipeline_all_parity_shards_lost() {
    // Test that losing all parity shards but keeping all data shards works

    let data: Vec<u8> = (0..1500).map(|i| (i % 256) as u8).collect();

    let config = ShardConfig {
        shard_size: 400,
        rs_data_shards: 4,
        rs_parity_shards: 3,
        rs_threshold: 500,
        single_shard_max: 100,
    };
    let encoder = ShardEncoder::new(config);

    let manifest = encoder.create_manifest(&data, "application/octet-stream", "commons").unwrap();
    assert_eq!(manifest.encoding, "rs-4-7");

    let shards = encoder.create_shards(&data, &manifest.encoding).unwrap();

    // Keep only data shards [0, 1, 2, 3], lose all parity shards [4, 5, 6]
    let mut shard_opts: Vec<Option<Vec<u8>>> = vec![None; 7];
    for i in 0..4 {
        shard_opts[i] = Some(shards[i].clone());
    }

    let reconstructed = encoder.reconstruct(&manifest, &shard_opts).unwrap();
    assert_eq!(
        reconstructed, data,
        "Must reconstruct from data shards alone (no parity needed)"
    );

    // Verify hash integrity
    use sha2::{Digest, Sha256};
    let original_hash = format!("sha256-{}", hex::encode(Sha256::digest(&data)));
    let reconstructed_hash = format!("sha256-{}", hex::encode(Sha256::digest(&reconstructed)));
    assert_eq!(original_hash, reconstructed_hash);
}
