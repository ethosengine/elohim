//! Phase 4 T6 — tests for connectivity helper.

use std::collections::HashSet;

use elohim_storage::services::connectivity::{any_online_in, is_online};

#[test]
fn is_online_true_when_peer_in_snapshot() {
    let snapshot: HashSet<String> = ["12D3KooWa".into(), "12D3KooWb".into()].into();
    assert!(is_online("12D3KooWa", &snapshot));
}

#[test]
fn is_online_false_when_peer_not_in_snapshot() {
    let snapshot: HashSet<String> = ["12D3KooWa".into()].into();
    assert!(!is_online("12D3KooWb", &snapshot));
}

#[test]
fn any_online_in_returns_true_if_any_peer_present() {
    let snapshot: HashSet<String> = ["12D3KooWb".into()].into();
    assert!(any_online_in(
        &["12D3KooWa", "12D3KooWb", "12D3KooWc"],
        &snapshot
    ));
}

#[test]
fn any_online_in_returns_false_when_none_match() {
    let snapshot: HashSet<String> = ["12D3KooWx".into()].into();
    assert!(!any_online_in(&["12D3KooWa", "12D3KooWb"], &snapshot));
}
