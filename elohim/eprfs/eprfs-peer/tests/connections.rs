//! Refused connections are counted per node id only for node ids the roster names; every other
//! node id (free to mint) folds into one strangers total, so a stranger cannot grow this device's
//! memory or its head table (T7 review I2).

use eprfs_peer::ConnectionBook;
use iroh::{NodeId, SecretKey};

fn node(seed: u32) -> NodeId {
    let mut bytes = [7u8; 32];
    bytes[..4].copy_from_slice(&seed.to_be_bytes());
    SecretKey::from_bytes(&bytes).public()
}

fn hex(node: NodeId) -> String {
    node.as_bytes().iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn ten_thousand_strangers_leave_the_map_bounded_by_the_roster() {
    let book = ConnectionBook::default();
    let (device_b, device_c) = (node(1), node(2));
    book.name([hex(device_b), hex(device_c)]);
    book.refused(device_b);
    book.refused(device_b);
    for seed in 100..10_100 {
        book.refused(node(seed));
    }
    let counts = book.refused_counts();
    assert!(
        counts.len() <= 2,
        "only roster-named node ids are kept: {}",
        counts.len()
    );
    assert_eq!(counts.get(&hex(device_b)), Some(&2));
    assert_eq!(book.refused_strangers(), 10_000);

    // A node id the roster stops naming counts as a stranger from then on.
    book.name([hex(device_c)]);
    book.refused(device_b);
    assert_eq!(book.refused_counts().get(&hex(device_b)), Some(&2));
    assert_eq!(book.refused_strangers(), 10_001);
}
