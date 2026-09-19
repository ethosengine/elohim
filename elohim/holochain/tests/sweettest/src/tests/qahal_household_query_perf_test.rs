//! @dna-scope: imagodei
//! Regression/parity test for `get_my_household_collective_cids`'s two-phase
//! chain read (perf fix — entry_type-filtered headers + `get_details`
//! materialisation, replacing an unfiltered `query(include_entries(true))`
//! that batch-loaded every entry on the caller's chain every 300s per node
//! via elohim-storage's `identity_fill`) AND for the latest-state-wins fix to
//! its withdrawal handling (see the coordinator's doc comment).
//!
//! Exercises the household discriminator across membership shapes on ONE
//! agent's own source chain — all reads are local (no cross-conductor DHT
//! consistency wait needed, unlike qahal_formation_test.rs):
//!
//!   (a) a household Membership                 -> included
//!   (b) a non-household Membership              -> excluded (charter isn't household)
//!   (c) a withdrawn household Membership        -> excluded
//!   (d) a DELETED Membership record              -> NOT covered here (see note below)
//!   (e) a withdrawn Steward's own re-invite      -> REFUSED (authority-in-integrity fix)
//!
//! ## (c) — latest-state-wins over withdrawal
//!
//! `withdraw_membership_clean` uses `update_entry`, which appends a NEW
//! Update action to the chain rather than rewriting the ORIGINAL Create
//! action's entry data — both actions carry `entry_type == Membership`.
//! `get_my_household_collective_cids` walks records in ascending chain order
//! and treats the LAST record touching a given collective_cid as authoritative:
//! a non-withdrawn record inserts the cid, a withdrawn record removes it. So
//! (c) a withdrawn household is excluded.
//!
//! ## (e) — a withdrawn Steward can no longer act, at all
//!
//! This scenario used to prove the OPPOSITE: it had the withdrawn founder
//! self-issue a `steward` invite for the collective they had just withdrawn
//! from, and affirm it, re-joining as Steward — which only "worked" because
//! `require_caller_is_steward_of` read `withdrawn_at_block_height` off the
//! record fetched by the Membership's ORIGINAL Create ActionHash, which
//! never reflects `withdraw_membership_clean`'s `update_entry`. A withdrawn
//! Steward kept full standing forever. That gate now resolves currency via
//! `qahal_coordinator::membership_is_current` (Create not withdrawn AND no
//! same-author Update withdrawn), so this scenario instead asserts the
//! authority gate holds: the same (former) founder's `issue_household_invite`
//! for that collective is REFUSED post-withdrawal.
//!
//! A genuine re-join — a SECOND, still-current Steward inviting the withdrawn
//! member back — is not added here as (f): it would require converting this
//! suite's single-agent, local-only-reads design (explicitly no
//! cross-conductor DHT consistency wait, per the module doc above) into a
//! two-conductor test with peer-info exchange and `await_consistency_s`, the
//! `qahal_formation_test.rs` pattern. That is not a cheap addition to a test
//! whose stated purpose is the two-phase LOCAL chain read, and the
//! invite/affirm flow across two conductors is already covered there — a
//! withdrawn-then-genuinely-rejoined variant belongs alongside it, not here.
//!
//! ## (d) — deleted Membership record: not constructible via public externs
//!
//! No coordinator extern in this DNA deletes a Membership entry (only
//! `update_entry` is used, by `withdraw_membership_clean`). Constructing a
//! genuinely tombstoned Membership record would require a test-only delete
//! extern on the zome, which is explicitly out of scope for this change.
//! The get_details-vs-get distinction this scenario would exercise (get()
//! silently drops a deleted/tombstoned record; get_details() still returns
//! it, matching the raw-chain-scan semantics of the `query()` this function
//! replaces) is instead documented at the call site in
//! `qahal_coordinator::get_my_household_collective_cids`.

use anyhow::Result;
use elohim_sweettest::common::{
    conductors::{load_dna, single_agent_conductor},
    fixtures::network_seed,
};
use hdk::prelude::{AgentPubKey, Signature};
use holo_hash::ActionHash;

const DNA: &str = "imagodei";
const ZOME: &str = "imagodei";

// Local I/O mirrors (field names must match the coordinator structs for
// msgpack round-tripping) — pattern mirrors qahal_collab_t0_test.rs /
// qahal_formation_test.rs.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct CreateCollectiveInput {
    charter: String,
    display_name: String,
    salt: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct WithdrawMembershipInput {
    membership_action_hash: ActionHash,
    collab_qahal_cid: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct IssueHouseholdInviteInput {
    collective_cid: String,
    role: String,
    expires_at_micros: i64,
    nonce: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct HouseholdInviteToken {
    collective_cid: String,
    role: String,
    sponsor_cid: String,
    expires_at_micros: i64,
    nonce: String,
    issuer_pubkey: AgentPubKey,
    signature: Signature,
}

fn far_future_micros() -> i64 {
    4_102_444_800_000_000
} // 2100-01-01

#[tokio::test(flavor = "multi_thread")]
async fn get_my_household_collective_cids_two_phase_read_parity() -> Result<()> {
    let (mut conductor, agent) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(agent.clone())).await?;
    let app = conductor
        .setup_app_for_agent("imagodei-household-query-perf", agent.clone(), &[dna])
        .await?;
    let cell = app.cells().first().expect("cell installed").clone();

    // (a) A household Collective — founder Membership must be included.
    let household_hash: ActionHash = conductor
        .call(
            &cell.zome(ZOME),
            "create_collective",
            CreateCollectiveInput {
                charter: r#"{"kind":"household"}"#.into(),
                display_name: "Household A".into(),
                salt: "aaaa1111aaaa1111aaaa1111aaaa1111".into(),
            },
        )
        .await;
    let household_cid = format!("collective:{household_hash}");

    // (b) A non-household Collective — same member_kind (Person Steward) as
    //     (a), but the charter does not declare "kind":"household", so the
    //     discriminator must exclude it.
    let community_hash: ActionHash = conductor
        .call(
            &cell.zome(ZOME),
            "create_collective",
            CreateCollectiveInput {
                charter: "We steward the community garden together.".into(),
                display_name: "Community Garden".into(),
                salt: "bbbb2222bbbb2222bbbb2222bbbb2222".into(),
            },
        )
        .await;
    let community_cid = format!("collective:{community_hash}");

    // (c)/(e) A third household Collective, whose founder Membership is then
    //     cleanly withdrawn (spec §6.4 clean-exit path).
    let withdrawn_household_hash: ActionHash = conductor
        .call(
            &cell.zome(ZOME),
            "create_collective",
            CreateCollectiveInput {
                charter: r#"{"kind":"household"}"#.into(),
                display_name: "Household C (withdrawn, then re-joined)".into(),
                salt: "cccc3333cccc3333cccc3333cccc3333".into(),
            },
        )
        .await;
    let withdrawn_household_cid = format!("collective:{withdrawn_household_hash}");

    let memberships: Vec<holochain_types::prelude::Record> = conductor
        .call(
            &cell.zome(ZOME),
            "list_memberships_for_collective",
            withdrawn_household_hash.clone(),
        )
        .await;
    assert_eq!(
        memberships.len(),
        1,
        "create_collective must atomically create exactly one founder Steward Membership"
    );
    let founder_membership_hash = memberships[0].action_hashed().hash.clone();

    let (): () = conductor
        .call(
            &cell.zome(ZOME),
            "withdraw_membership_clean",
            WithdrawMembershipInput {
                membership_action_hash: founder_membership_hash,
                collab_qahal_cid: withdrawn_household_cid.clone(),
            },
        )
        .await;

    // ---- Exercise the two-phase, latest-state-wins read under test ----
    let cids: Vec<String> = conductor
        .call(&cell.zome(ZOME), "get_my_household_collective_cids", ())
        .await;

    assert!(
        cids.contains(&household_cid),
        "(a) household Membership must be included; got {cids:?}"
    );
    assert!(
        !cids.contains(&community_cid),
        "(b) non-household Membership must be excluded; got {cids:?}"
    );
    // (c) — latest-state-wins: the withdrawal Update is the last record
    // touching this collective_cid, so it must be excluded.
    assert!(
        !cids.contains(&withdrawn_household_cid),
        "(c) latest-state-wins: a withdrawn household must be excluded; got {cids:?}"
    );

    // (e) The withdrawn founder is no longer a current Steward of that
    // collective: `issue_household_invite` (which gates on
    // `require_caller_is_steward_of` -> `membership_is_current`) must refuse
    // them, even though the caller is the ONLY agent who was ever a Steward
    // of this collective. Prior to the authority-in-integrity fix, this same
    // call succeeded (the withdrawn founder could self-issue a fresh
    // `steward` invite and re-join) because the gate read the stale,
    // never-updated Create record.
    let invite_after_withdrawal: Result<HouseholdInviteToken, _> = conductor
        .call_fallible(
            &cell.zome(ZOME),
            "issue_household_invite",
            IssueHouseholdInviteInput {
                collective_cid: withdrawn_household_cid.clone(),
                role: "steward".into(),
                expires_at_micros: far_future_micros(),
                nonce: "dddd4444dddd4444dddd4444dddd4444".into(),
            },
        )
        .await;
    assert!(
        invite_after_withdrawal.is_err(),
        "(e) a withdrawn Steward's own issue_household_invite must be refused, not succeed"
    );
    let err_str = format!("{:?}", invite_after_withdrawal.unwrap_err());
    assert!(
        err_str.to_lowercase().contains("steward"),
        "(e) refusal should name the Steward-authority gate; got: {err_str}"
    );

    Ok(())
}
