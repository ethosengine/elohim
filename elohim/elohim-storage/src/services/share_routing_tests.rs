// Unit tests for share-routing function (Form A: Declared).
// Per spec §6.1.

use crate::services::share_routing::*;
use elohim_views::{ShareAllocation, ShareAllocationForm, DeclaredShare};

#[test]
fn declared_shares_distribute_proportionally() {
    let allocation = ShareAllocation {
        form: ShareAllocationForm::Declared,
        shares: Some(vec![
            DeclaredShare { collective_cid: "collective:a".into(), share: 0.4 },
            DeclaredShare { collective_cid: "collective:b".into(), share: 0.4 },
            DeclaredShare { collective_cid: "collective:c".into(), share: 0.15 },
        ]),
        affinity_window_blocks: None,
        rebalance_cadence_blocks: None,
        commons_pool_tribute: 0.05,
    };
    let event_value = 1000.0;
    let routed = evaluate_share_routing(&allocation, event_value, 0).unwrap();
    let lookup: std::collections::HashMap<_, _> =
        routed.iter().map(|r| (r.collective_cid.clone(), r.amount)).collect();
    assert!((lookup["collective:a"] - 400.0).abs() < 0.01);
    assert!((lookup["collective:b"] - 400.0).abs() < 0.01);
    assert!((lookup["collective:c"] - 150.0).abs() < 0.01);
    assert!((lookup["commons-pool"] - 50.0).abs() < 0.01);
    let total: f64 = routed.iter().map(|r| r.amount).sum();
    assert!((total - 1000.0).abs() < 0.01);
}

#[test]
fn form_b_not_yet_supported() {
    let allocation = ShareAllocation {
        form: ShareAllocationForm::AffinityDerived,
        shares: None,
        affinity_window_blocks: Some(1000),
        rebalance_cadence_blocks: Some(100),
        commons_pool_tribute: 0.05,
    };
    let result = evaluate_share_routing(&allocation, 1000.0, 0);
    assert!(result.is_err(), "M1 only supports Declared");
}

#[test]
fn zero_tribute_refused() {
    let allocation = ShareAllocation {
        form: ShareAllocationForm::Declared,
        shares: Some(vec![
            DeclaredShare { collective_cid: "collective:a".into(), share: 0.5 },
            DeclaredShare { collective_cid: "collective:b".into(), share: 0.5 },
        ]),
        affinity_window_blocks: None,
        rebalance_cadence_blocks: None,
        commons_pool_tribute: 0.0,
    };
    let result = evaluate_share_routing(&allocation, 1000.0, 0);
    assert!(result.is_err(), "zero tribute refused");
}

#[test]
fn shares_must_sum_to_one_with_tribute() {
    let allocation = ShareAllocation {
        form: ShareAllocationForm::Declared,
        shares: Some(vec![
            DeclaredShare { collective_cid: "collective:a".into(), share: 0.3 },
            DeclaredShare { collective_cid: "collective:b".into(), share: 0.3 },
        ]),
        affinity_window_blocks: None,
        rebalance_cadence_blocks: None,
        commons_pool_tribute: 0.05, // sum = 0.65, not 1.0
    };
    let result = evaluate_share_routing(&allocation, 1000.0, 0);
    assert!(result.is_err(), "shares + tribute must sum to 1.0");
}

#[test]
fn withdrawn_member_does_not_accrue() {
    let allocation = ShareAllocation {
        form: ShareAllocationForm::Declared,
        shares: Some(vec![
            DeclaredShare { collective_cid: "collective:a".into(), share: 0.5 },
            DeclaredShare { collective_cid: "collective:b".into(), share: 0.45 },
        ]),
        affinity_window_blocks: None,
        rebalance_cadence_blocks: None,
        commons_pool_tribute: 0.05,
    };
    let active_set: std::collections::HashSet<String> = vec!["collective:b".into()].into_iter().collect();
    let routed = evaluate_share_routing_active_only(&allocation, 1000.0, 0, &active_set).unwrap();
    let lookup: std::collections::HashMap<_, _> =
        routed.iter().map(|r| (r.collective_cid.clone(), r.amount)).collect();
    assert!(lookup.get("collective:a").is_none(), "A withdrew; no accrual");
    // B's relative share + commons-pool re-normalized over the remaining 0.45 + 0.05 fraction.
    // For M1 we DO NOT re-normalize after withdrawal — the unspent share flows entirely
    // to the commons pool of the Collab. This makes the substrate behavior predictable
    // and prevents oscillation around withdrawal events.
    let expected_b = 1000.0 * 0.45;
    let expected_commons = 1000.0 * (0.50 + 0.05); // A's 0.50 + base tribute 0.05
    assert!((lookup["collective:b"] - expected_b).abs() < 0.01);
    assert!((lookup["commons-pool"] - expected_commons).abs() < 0.01);
}
