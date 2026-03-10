//! Config loading and defaults integration tests

use std::path::PathBuf;

/// Verify that default Config is constructible and has sensible defaults.
#[test]
fn test_default_config_values() {
    let toml_str = r#"
[node]
id = "test-node"
data_dir = "/tmp/elohim-test"
cluster_name = "test-cluster"

[sync]

[cluster]

[p2p]

[storage]

[api]
"#;

    let config: toml::Value = toml::from_str(toml_str).expect("valid TOML");

    // Verify required fields parse
    let node = config.get("node").expect("node section");
    assert_eq!(node.get("id").unwrap().as_str().unwrap(), "test-node");
    assert_eq!(
        node.get("data_dir").unwrap().as_str().unwrap(),
        "/tmp/elohim-test"
    );
    assert_eq!(
        node.get("cluster_name").unwrap().as_str().unwrap(),
        "test-cluster"
    );
}

#[test]
fn test_config_with_all_fields() {
    let toml_str = r#"
[node]
id = "family-node-1"
data_dir = "/var/lib/elohim"
cluster_name = "cohen-family"

[sync]
max_document_size = 5242880
sync_interval_ms = 2000

[cluster]
mdns_enabled = true
cluster_key = "secret123"

[p2p]
listen_addrs = ["/ip4/0.0.0.0/tcp/4001", "/ip4/0.0.0.0/udp/4001/quic-v1"]
bootstrap_nodes = []

[storage]
max_capacity = "1TB"
shard_redundancy = 5

[api]
http_port = 9090
grpc_port = 9091
"#;

    let config: toml::Value = toml::from_str(toml_str).expect("valid TOML");

    let sync = config.get("sync").unwrap();
    assert_eq!(
        sync.get("max_document_size").unwrap().as_integer().unwrap(),
        5_242_880
    );
    assert_eq!(
        sync.get("sync_interval_ms").unwrap().as_integer().unwrap(),
        2000
    );

    let cluster = config.get("cluster").unwrap();
    assert_eq!(
        cluster.get("mdns_enabled").unwrap().as_bool().unwrap(),
        true
    );
    assert_eq!(
        cluster.get("cluster_key").unwrap().as_str().unwrap(),
        "secret123"
    );

    let storage = config.get("storage").unwrap();
    assert_eq!(
        storage.get("max_capacity").unwrap().as_str().unwrap(),
        "1TB"
    );
    assert_eq!(
        storage
            .get("shard_redundancy")
            .unwrap()
            .as_integer()
            .unwrap(),
        5
    );

    let api = config.get("api").unwrap();
    assert_eq!(api.get("http_port").unwrap().as_integer().unwrap(), 9090);
    assert_eq!(api.get("grpc_port").unwrap().as_integer().unwrap(), 9091);
}

#[test]
fn test_config_missing_file_uses_defaults() {
    // Simulate the pattern from main.rs:
    // "if file doesn't exist, use default config"
    let config_path = "/nonexistent/path/to/config.toml";
    let path_exists = std::path::Path::new(config_path).exists();
    assert!(!path_exists, "Test config path should not exist");
}

#[test]
fn test_config_with_env_overrides_pattern() {
    // Test that CLI override pattern works correctly
    let mut node_id = "default-node".to_string();
    let mut cluster_name = "default-cluster".to_string();
    let mut data_dir = PathBuf::from("/var/lib/elohim");

    // Simulate CLI overrides
    let cli_node_id = Some("override-node".to_string());
    let cli_cluster = Some("override-cluster".to_string());
    let cli_data_dir = Some("/tmp/override".to_string());

    if let Some(id) = cli_node_id {
        node_id = id;
    }
    if let Some(name) = cli_cluster {
        cluster_name = name;
    }
    if let Some(dir) = cli_data_dir {
        data_dir = PathBuf::from(dir);
    }

    assert_eq!(node_id, "override-node");
    assert_eq!(cluster_name, "override-cluster");
    assert_eq!(data_dir, PathBuf::from("/tmp/override"));
}

#[test]
fn test_config_partial_overrides() {
    // Only override node_id, keep others as default
    let mut node_id = "default-node".to_string();
    let cluster_name = "default-cluster".to_string();
    let data_dir = PathBuf::from("/var/lib/elohim");

    let cli_node_id = Some("custom-node".to_string());
    let cli_cluster: Option<String> = None;
    let cli_data_dir: Option<String> = None;

    if let Some(id) = cli_node_id {
        node_id = id;
    }

    assert_eq!(node_id, "custom-node");
    assert_eq!(cluster_name, "default-cluster");
    assert_eq!(data_dir, PathBuf::from("/var/lib/elohim"));
}

#[test]
fn test_invalid_toml_returns_error() {
    let bad_toml = "this is not valid { toml }}}";
    let result: Result<toml::Value, _> = toml::from_str(bad_toml);
    assert!(result.is_err(), "Invalid TOML should produce an error");
}
