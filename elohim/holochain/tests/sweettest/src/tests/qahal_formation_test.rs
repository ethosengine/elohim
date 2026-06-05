//! @dna-scope: imagodei
//! Household formation ceremony T0: issue_household_invite + affirm_membership.
//! Spec: genesis/docs/superpowers/specs/2026-06-04-household-formation-ceremony-design.md §4.1
//!
//! Recognition-of-the-given flow: a current Steward of a household Collective
//! issues a single-use, signed, TTL'd invite token (Category-C operational, NOT
//! on the DHT). The invited member affirms with the token from THEIR own agent —
//! their identity is theirs from day one; the token's sponsor chain carries the
//! issuer's side of the mutual witness. Replay is rejected via a consumed-nonce
//! anchor link on the Collective.
//!
//! ## Harness conventions (mirrors qahal_collab_t0_test.rs two-conductor section)
//! - `two_agent_conductors()` returns `[(SweetConductor, AgentPubKey); 2]`.
//! - ONE DNA loaded with agent 0 as progenitor/bootstrap-steward, installed on
//!   both conductors → same DNA hash → same DHT.
//! - `SweetConductor::exchange_peer_info([&c0, &c1])` in a timeout loop, then
//!   `await_consistency(60, [&cell0, &cell1])` before cross-conductor reads.
//! - Happy path: `conductor.call(...)` (panics on WasmError).
//! - Rejection paths: `conductor.call_fallible(...)` returns `Result<T, _>`.

use anyhow::Result;
use hdk::prelude::{ActionHash, AgentPubKey, Signature};
use serde::{Deserialize, Serialize};

use elohim_sweettest::common::{
    conductors::{load_dna, two_agent_conductors},
    fixtures::network_seed,
};
use holochain::sweettest::{await_consistency, SweetConductor};

const DNA: &str = "imagodei";
const ZOME: &str = "imagodei";

// Local I/O mirrors (field names must match coordinator structs for msgpack).
#[derive(Serialize, Debug, Clone)]
struct CreateCollectiveInput {
    charter: String,
    display_name: String,
    salt: String,
}

#[derive(Serialize, Debug, Clone)]
struct IssueHouseholdInviteInput {
    collective_cid: String,
    role: String,
    expires_at_micros: i64,
    nonce: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct HouseholdInviteToken {
    collective_cid: String,
    role: String,
    sponsor_cid: String,
    expires_at_micros: i64,
    nonce: String,
    issuer_pubkey: AgentPubKey,
    signature: Signature,
}

#[derive(Serialize, Debug, Clone)]
struct AffirmMembershipInput {
    token: HouseholdInviteToken,
}

fn far_future_micros() -> i64 {
    4_102_444_800_000_000
} // 2100-01-01

/// Settle the DHT across both conductors so cross-agent reads see committed
/// entries + links. Mirrors the qahal_collab_t0_test two-conductor pattern.
async fn settle(c0: &SweetConductor, c1: &SweetConductor, cells: [&holochain::sweettest::SweetCell; 2]) -> Result<()> {
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        while !SweetConductor::exchange_peer_info([c0, c1]).await {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!("Timeout waiting for peer info exchange"))?;
    await_consistency(60, cells)
        .await
        .map_err(|e| anyhow::anyhow!("DHT consistency timeout: {e}"))?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn affirm_membership_happy_path_then_replay_rejected() -> Result<()> {
    let [(mut c0, a0), (mut c1, a1)] = two_agent_conductors().await?;
    // ONE DNA, agent 0 as progenitor; installed on both conductors → same DHT.
    let dna = load_dna(DNA, &network_seed(DNA), Some(a0.clone())).await?;
    let app0 = c0
        .setup_app_for_agent("imagodei-app", a0.clone(), &[dna.clone()])
        .await?;
    let app1 = c1
        .setup_app_for_agent("imagodei-app", a1.clone(), &[dna])
        .await?;
    let cell0 = app0.cells().first().expect("cell0").clone();
    let cell1 = app1.cells().first().expect("cell1").clone();

    // Founder (agent 0) creates the household collective.
    let collective_hash: ActionHash = c0
        .call(
            &cell0.zome(ZOME),
            "create_collective",
            CreateCollectiveInput {
                charter: r#"{"kind":"household","rubric":"recognition-of-given","slugAlias":"family-test"}"#.into(),
                display_name: "Test Household".into(),
                salt: "0123456789abcdef0123456789abcdef".into(),
            },
        )
        .await;
    let collective_cid = format!("collective:{collective_hash}");

    // Founder issues an invite (Steward authority gate inside).
    let token: HouseholdInviteToken = c0
        .call(
            &cell0.zome(ZOME),
            "issue_household_invite",
            IssueHouseholdInviteInput {
                collective_cid: collective_cid.clone(),
                role: "contributor".into(),
                expires_at_micros: far_future_micros(),
                nonce: "fedcba9876543210fedcba9876543210".into(),
            },
        )
        .await;

    // Member (agent 1) affirms with the token — authored by THEIR agent.
    settle(&c0, &c1, [&cell0, &cell1]).await?;
    let _membership_hash: ActionHash = c1
        .call(
            &cell1.zome(ZOME),
            "affirm_membership",
            AffirmMembershipInput {
                token: token.clone(),
            },
        )
        .await;

    settle(&c0, &c1, [&cell0, &cell1]).await?;
    let memberships: Vec<holochain_types::prelude::Record> = c0
        .call(
            &cell0.zome(ZOME),
            "list_memberships_for_collective",
            collective_hash,
        )
        .await;
    assert_eq!(
        memberships.len(),
        2,
        "founder Steward + affirmed Contributor expected"
    );

    // Replay: same token a second time must be rejected (consumed-nonce link).
    let replay: Result<ActionHash, _> = c1
        .call_fallible(
            &cell1.zome(ZOME),
            "affirm_membership",
            AffirmMembershipInput { token },
        )
        .await;
    assert!(replay.is_err(), "replayed invite token must be rejected");
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn affirm_membership_rejects_expired_token() -> Result<()> {
    let [(mut c0, a0), (mut c1, a1)] = two_agent_conductors().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(a0.clone())).await?;
    let app0 = c0
        .setup_app_for_agent("imagodei-app", a0.clone(), &[dna.clone()])
        .await?;
    let app1 = c1
        .setup_app_for_agent("imagodei-app", a1.clone(), &[dna])
        .await?;
    let cell0 = app0.cells().first().expect("cell0").clone();
    let cell1 = app1.cells().first().expect("cell1").clone();

    let collective_hash: ActionHash = c0
        .call(
            &cell0.zome(ZOME),
            "create_collective",
            CreateCollectiveInput {
                charter: r#"{"kind":"household"}"#.into(),
                display_name: "Expiry Household".into(),
                salt: "00112233445566770011223344556677".into(),
            },
        )
        .await;

    let token: HouseholdInviteToken = c0
        .call(
            &cell0.zome(ZOME),
            "issue_household_invite",
            IssueHouseholdInviteInput {
                collective_cid: format!("collective:{collective_hash}"),
                role: "contributor".into(),
                expires_at_micros: 1, // 1970 — guaranteed expired
                nonce: "aaaabbbbccccddddaaaabbbbccccdddd".into(),
            },
        )
        .await;

    settle(&c0, &c1, [&cell0, &cell1]).await?;
    let expired: Result<ActionHash, _> = c1
        .call_fallible(
            &cell1.zome(ZOME),
            "affirm_membership",
            AffirmMembershipInput { token },
        )
        .await;
    assert!(expired.is_err(), "expired invite token must be rejected");
    Ok(())
}
