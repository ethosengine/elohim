//! The advisory rules digest includes only the crossing kinds and steward resource names.

use eprfs_peer::Rules;
use serde_json::json;

#[test]
fn a_comment_only_edit_keeps_the_digest() {
    let first = json!({
        "berth": {
            "channel": {
                "crosses": {
                    "dev-berth-events@1": ["release", "claim"]
                },
                "confirm_wait_s": 10
            },
            "resources": {
                "dev-integration": {"capacity": 1, "scope": "steward"},
                "lane:": {"capacity": 1, "scope": "steward"},
                "cargo": 2
            }
        }
    });
    let second = json!({
        "unrelated": {"anything": true},
        "berth": {
            "_comment": "different prose",
            "resources": {
                "cargo": 99,
                "lane:": {"_comment": "prefix", "scope": "steward", "capacity": 7},
                "dev-integration": {"scope": "steward", "capacity": 4},
                "_future": {"scope": "steward", "capacity": 1}
            },
            "channel": {
                "max_skew_s": 300,
                "confirm_wait_s": 42,
                "crosses": {
                    "dev-berth-events@1": ["claim", "release"]
                }
            }
        }
    });

    assert_eq!(
        Rules::from_policy(&first).expect("first policy").digest(),
        Rules::from_policy(&second).expect("second policy").digest()
    );
}

#[test]
fn a_new_kind_or_steward_resource_moves_the_digest() {
    let base = json!({
        "berth": {
            "channel": {"crosses": {"dev-berth-events@1": ["claim"]}},
            "resources": {"dev-integration": {"capacity": 1, "scope": "steward"}}
        }
    });
    let new_kind = json!({
        "berth": {
            "channel": {"crosses": {"dev-berth-events@1": ["claim", "release"]}},
            "resources": {"dev-integration": {"capacity": 1, "scope": "steward"}}
        }
    });
    let new_resource = json!({
        "berth": {
            "channel": {"crosses": {"dev-berth-events@1": ["claim"]}},
            "resources": {
                "dev-integration": {"capacity": 1, "scope": "steward"},
                "lane:": {"capacity": 1, "scope": "steward"}
            }
        }
    });
    let digest = Rules::from_policy(&base).expect("base policy").digest();

    assert_ne!(
        digest,
        Rules::from_policy(&new_kind).expect("kind policy").digest()
    );
    assert_ne!(
        digest,
        Rules::from_policy(&new_resource)
            .expect("resource policy")
            .digest()
    );
}

#[test]
fn two_checkouts_on_different_bases_produce_one_digest() {
    let first = tempfile::NamedTempFile::new().expect("first checkout policy");
    let second = tempfile::NamedTempFile::new().expect("second checkout policy");
    std::fs::write(
        first.path(),
        r#"{"berth":{"channel":{"crosses":{"dev-berth-events@1":["claim"]}},"resources":{"dev-integration":{"capacity":1,"scope":"steward"}}},"branch":"old"}"#,
    )
    .expect("write first policy");
    std::fs::write(
        second.path(),
        r#"{"branch":"new","berth":{"resources":{"dev-integration":{"scope":"steward","capacity":9}},"channel":{"fence_margin_s":90,"crosses":{"dev-berth-events@1":["claim"]}}}}"#,
    )
    .expect("write second policy");

    assert_eq!(
        Rules::from_policy_file(first.path())
            .expect("first policy")
            .digest(),
        Rules::from_policy_file(second.path())
            .expect("second policy")
            .digest()
    );
}

#[test]
fn a_malformed_policy_is_refused() {
    let policy = tempfile::NamedTempFile::new().expect("policy");
    std::fs::write(policy.path(), "{not JSON").expect("write malformed policy");

    assert!(Rules::from_policy_file(policy.path()).is_err());
}

#[test]
fn serves_enters_the_rules_digest_and_relays_still_do_not() {
    let base = json!({
        "berth": {
            "channel": {"crosses": {"dev-berth-events@1": ["claim"]}},
            "resources": {"dev-integration": {"capacity": 1, "scope": "steward"}}
        }
    });
    let mut serving = base.clone();
    serving["berth"]["artifacts"] = json!({
        "_comment": "prose never moves the digest",
        "serves": {"mesh-build": ["steward"]}
    });
    let mut other_method = base.clone();
    other_method["berth"]["artifacts"] = json!({"serves": {"other-build": ["steward"]}});
    let mut serving_with_relays = serving.clone();
    serving_with_relays["berth"]["channel"]["relays"] = json!([
        "https://relay.alpha.elohim.host",
        "https://relay.elohim.host"
    ]);
    serving_with_relays["berth"]["channel"]["relay_mode"] = json!("single");
    serving_with_relays["berth"]["artifacts"]["_comment"] = json!("different prose");
    let mut serving_nothing = base.clone();
    serving_nothing["berth"]["artifacts"] = json!({"serves": {}});

    let digest = |policy: &serde_json::Value| Rules::from_policy(policy).expect("policy").digest();
    // A grant is enforced, so a sibling with another grant shows as skew.
    assert_ne!(digest(&base), digest(&serving));
    assert_ne!(digest(&serving), digest(&other_method));
    // Relays and prose stay outside the digest, grant or no grant.
    assert_eq!(digest(&serving), digest(&serving_with_relays));
    // A policy that serves nothing keeps the digest it had before the grant existed.
    assert_eq!(digest(&base), digest(&serving_nothing));
    // The serve grant is loaded with the rest.
    assert!(!Rules::from_policy(&serving)
        .expect("serving")
        .serves()
        .is_empty());
}
