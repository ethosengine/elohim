//! The device's iroh peer key: generated once, private from creation, reused across restarts and
//! never shown.

use std::fs;
use std::os::unix::fs::PermissionsExt;

use eprfs_peer::{PeerError, PeerKey};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn key_generated_once_with_mode_0600() {
    let dir = tempfile::tempdir().expect("tempdir");
    let peer_dir = dir.path().join("epr").join("peer");

    let key = PeerKey::load_or_generate(&peer_dir).expect("first start generates the key");

    let path = peer_dir.join(PeerKey::FILE_NAME);
    let meta = fs::metadata(&path).expect("the key file exists");
    assert_eq!(
        meta.permissions().mode() & 0o777,
        0o600,
        "private from creation"
    );
    assert_eq!(meta.len(), 32, "the 32-byte ed25519 secret, nothing else");
    let dir_mode = fs::metadata(&peer_dir).expect("dir").permissions().mode() & 0o777;
    assert_eq!(
        dir_mode & 0o077,
        0,
        "a directory this call creates is private too"
    );

    let before = fs::read(&path).expect("read");
    let again = PeerKey::load_or_generate(&peer_dir).expect("second start loads");
    assert_eq!(fs::read(&path).expect("read"), before, "never regenerated");
    assert_eq!(again.node_id(), key.node_id());
    let leftovers: Vec<_> = fs::read_dir(&peer_dir)
        .expect("ls")
        .map(|e| e.expect("entry").file_name())
        .collect();
    assert_eq!(
        leftovers,
        vec![PeerKey::FILE_NAME],
        "no temp file is left behind"
    );
}

#[test]
fn key_reused_across_restarts() {
    let dir = tempfile::tempdir().expect("tempdir");
    let first = PeerKey::load_or_generate(dir.path()).expect("generate");
    let second = PeerKey::load_or_generate(dir.path()).expect("load");
    assert_eq!(first.node_id(), second.node_id(), "one device, one node id");

    let other = tempfile::tempdir().expect("tempdir");
    let stranger = PeerKey::load_or_generate(other.path()).expect("generate");
    assert_ne!(
        first.node_id(),
        stranger.node_id(),
        "each device mints its own key"
    );
}

#[test]
fn key_never_in_debug_output() {
    let dir = tempfile::tempdir().expect("tempdir");
    let key = PeerKey::load_or_generate(dir.path()).expect("generate");
    let secret = fs::read(dir.path().join(PeerKey::FILE_NAME)).expect("read");

    let shown = format!("{key:?} {key:#?}");
    assert!(
        !shown.contains(&hex(&secret)),
        "the secret's hex is never shown: {shown}"
    );
    assert!(
        !shown.contains(&format!("{:?}", secret.as_slice())),
        "nor its bytes: {shown}"
    );
    assert!(
        shown.contains(&key.node_id().to_string()),
        "the public node id is what Debug shows: {shown}"
    );
}

#[test]
fn a_key_file_readable_by_group_or_others_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    PeerKey::load_or_generate(dir.path()).expect("generate");
    let path = dir.path().join(PeerKey::FILE_NAME);
    for mode in [0o640, 0o604, 0o644] {
        fs::set_permissions(&path, fs::Permissions::from_mode(mode)).expect("chmod");
        match PeerKey::load_or_generate(dir.path()) {
            Err(PeerError::KeyFileExposed { mode: seen, .. }) => assert_eq!(seen, mode),
            other => panic!("mode {mode:o} must be refused, got {other:?}"),
        }
        assert_eq!(
            fs::metadata(&path).expect("meta").permissions().mode() & 0o777,
            mode,
            "a refused key file is left as found, never silently repaired or replaced"
        );
    }
}

#[test]
fn a_key_file_of_the_wrong_length_is_refused_not_regenerated() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join(PeerKey::FILE_NAME);
    for bytes in [&b"short"[..], &[7u8; 33][..]] {
        fs::write(&path, bytes).expect("write");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).expect("chmod");
        assert!(
            matches!(
                PeerKey::load_or_generate(dir.path()),
                Err(PeerError::KeyFileCorrupt { .. })
            ),
            "{} bytes is not a key",
            bytes.len()
        );
        assert_eq!(fs::read(&path).expect("read"), bytes, "left as found");
    }
}
