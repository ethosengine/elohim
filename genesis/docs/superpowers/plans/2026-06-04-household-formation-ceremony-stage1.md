---
title: "Household Formation Ceremony — Stage 1 implementation plan"
id: household-formation-ceremony-stage1
status: Draft
class: protocol-canonical
domain: D7
topic: [qahal, household, formation, affirm-membership, seeder, custody-blob, quiltPolicy, a2o, plan]
cites:
  - household-formation-ceremony-design | the spec this plan implements — Stage 1 of its §12 build order (ceremony floor: coordinators, projection, soft action gate, seeder driver, scenario spine, quiltPolicy); stages 2/3 get their own plans | sha256:c4c55b654b2cb763
derived_from: genesis/docs/superpowers/specs/2026-06-04-household-formation-ceremony-design.md
---

# Household Formation Ceremony (Stage 1) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A family (matthew/jessica/james) forms a household through real multi-agent zome calls — collective + affirmed memberships + anchored custody reciprocity — driven by the seeder as each persona's own conductor agent, visible in storage projections, validated by a2o scenarios on `household-nodes`.

**Architecture:** Zero new DHT entry types. Two new imagodei coordinator functions (`issue_household_invite`, `affirm_membership`) ride the existing post_commit→`ImagodeiSignal`→`DnaSignal`→reconcile-controller projection pipeline (already landed, Wave 2 T5) — we extend the translation to carry `charter` so household-ness reaches `governance_layer`. Custody-blob commitments climb from diesel-direct to conductor-anchored (soft action-gate extension) and the new `seed-household-formation.ts` drives the whole ceremony per-conductor (rung 3). Scenario spine + retags land the story on `household-nodes`.

**Tech Stack:** Holochain HDK (imagodei DNA, plain cargo), Rust elohim-storage (diesel + `RUSTFLAGS='--cfg getrandom_backend="custom"'` + cargo-pool slot), TypeScript seeder (tsx, @holochain/client 0.20.x, vitest), cucumber-js a2o, JSON manifests.

**P2P design gate:** passed at brainstorm time (spec §2) — ZERO new DHT entry types, ZERO new tables, ZERO new HTTP routes in this plan. Every write targets existing tables (`collectives`, `collective_participations`, `rea_commitments` — migrations carry `-- Source of truth: DHT`); every code snippet below quoting SQL/diesel is test or projector code against those existing schemas. Any in-plan audit hits on quoted snippets are misfires, not gaps.

**Worktree:** execute on a feature branch via `superpowers:using-git-worktrees`. DNA + sweettest + storage builds have DIFFERENT cargo conventions — each task states its exact build env. NEVER set CARGO_TARGET_DIR for DNA workspaces; ALWAYS set it (pool slot via `genesis/agentic/bin/cargo-pool key`) for elohim-storage.

**Cross-task constants** (referenced by multiple tasks):
- Canonical triad humanIds: `human-matthew-manager`, `human-jessica-spouse`, `human-james-student`.
- Household charter JSON (single line, ≤16KiB validated): `{"kind":"household","rubric":"recognition-of-given","slugAlias":"family-dowell"}`
- Provenance markers: fixture rows `{"fixture":"formation-output","retireAt":"ceremony-landing"}`; ceremony rows `{"seedGeneration":"ceremony"}`.

---

### Task 1: Interim fixture — triad custody pairs with loud provenance (rung 1, retired in Task 10)

The settled fork: emergent + explicit interim fixtures. Views light up NOW; Task 10 retires this.

**Files:**
- Modify: `genesis/seeder/src/seed-commitments.ts` (defaultM1Pairs ~line 168; CustodyPair/body builder)
- Test: `genesis/seeder/src/__tests__/seed-commitments.spec.ts`

- [ ] **Step 1: Write the failing test** — append to `seed-commitments.spec.ts`:

```ts
describe('defaultM1Pairs triad fixture', () => {
  it('includes the james fixture pairs with formation-output provenance', () => {
    process.env.M1_BLOB_HASH = 'sha256-cafebabe';
    process.env.M1_BLOB_SIZE_BYTES = '64';
    const pairs = defaultM1Pairs();
    // 2 M1 pairs (matthew<->jessica) + 4 fixture pairs (james with each parent, both directions)
    expect(pairs).toHaveLength(6);
    const jamesPairs = pairs.filter(
      p => p.providerHumanId === 'human-james-student' || p.receiverHumanId === 'human-james-student'
    );
    expect(jamesPairs).toHaveLength(4);
    for (const p of jamesPairs) expect(p.fixture).toBe('formation-output');
  });

  it('stamps fixture provenance into the commitment body metadata', () => {
    const body = buildCustodyCommitmentBody({
      providerHumanId: 'human-jessica-spouse', providerArchetype: 'desktop',
      receiverHumanId: 'human-james-student', receiverArchetype: 'mobile',
      blobHash: 'sha256-deadbeef', blobSizeBytes: 1, fixture: 'formation-output',
    });
    expect(body.metadata.fixture).toBe('formation-output');
    expect(body.metadata.retireAt).toBe('ceremony-landing');
  });
});
```

(`defaultM1Pairs` is currently not exported — the test import will fail; that is part of the red.)

- [ ] **Step 2: Run to verify it fails**

Run: `cd genesis/seeder && pnpm exec vitest run seed-commitments`
Expected: FAIL — `defaultM1Pairs` not exported / `fixture` not a property.

- [ ] **Step 3: Implement** — in `seed-commitments.ts`: (a) add `fixture?: string` to the `CustodyPair` interface; (b) `export` `defaultM1Pairs` and extend it:

```ts
export function defaultM1Pairs(): CustodyPair[] {
  if (!M1_DEFAULT_BLOB_HASH || M1_DEFAULT_BLOB_SIZE <= 0) {
    console.error('ERROR: M1_BLOB_HASH and M1_BLOB_SIZE_BYTES must be set (or pass CUSTODY_PAIRS_JSON).');
    process.exit(1);
  }
  const m1 = { blobHash: M1_DEFAULT_BLOB_HASH, blobSizeBytes: M1_DEFAULT_BLOB_SIZE };
  const fixture = { ...m1, fixture: 'formation-output' as const };
  return [
    // M1 named-pair flag (anti-drift, stays after Task 10)
    { providerHumanId: 'human-matthew-manager', providerArchetype: 'desktop',
      receiverHumanId: 'human-jessica-spouse', receiverArchetype: 'desktop', ...m1 },
    { providerHumanId: 'human-jessica-spouse', providerArchetype: 'desktop',
      receiverHumanId: 'human-matthew-manager', receiverArchetype: 'desktop', ...m1 },
    // INTERIM FIXTURES (2026-06-04 fork: emergent + marked fixtures) — retired at
    // ceremony landing (Task 10 of the stage-1 plan). Loud provenance below.
    { providerHumanId: 'human-matthew-manager', providerArchetype: 'desktop',
      receiverHumanId: 'human-james-student', receiverArchetype: 'mobile', ...fixture },
    { providerHumanId: 'human-james-student', providerArchetype: 'mobile',
      receiverHumanId: 'human-matthew-manager', receiverArchetype: 'desktop', ...fixture },
    { providerHumanId: 'human-jessica-spouse', providerArchetype: 'desktop',
      receiverHumanId: 'human-james-student', receiverArchetype: 'mobile', ...fixture },
    { providerHumanId: 'human-james-student', providerArchetype: 'mobile',
      receiverHumanId: 'human-jessica-spouse', receiverArchetype: 'desktop', ...fixture },
  ];
}
```

(c) in `buildCustodyCommitmentBody`, extend the metadata object:

```ts
metadata: {
  seedGeneration: 'genesis',
  blobHash: pair.blobHash,
  providerHumanId: pair.providerHumanId,
  receiverHumanId: pair.receiverHumanId,
  ...(pair.fixture ? { fixture: pair.fixture, retireAt: 'ceremony-landing' } : {}),
},
```

- [ ] **Step 4: Run tests + typecheck**

Run: `cd genesis/seeder && pnpm exec vitest run seed-commitments && pnpm typecheck`
Expected: PASS / clean.

- [ ] **Step 5: Commit**

```bash
git add genesis/seeder/src/seed-commitments.ts genesis/seeder/src/__tests__/seed-commitments.spec.ts
git commit -m "feat(seeder): triad custody fixture pairs with formation-output provenance markers"
```

---

### Task 2: `issue_household_invite` + `affirm_membership` coordinators (imagodei DNA)

The net-new substrate. No new entry types; no post_commit change (the existing `to_app_option::<Membership>()` arm auto-emits `MembershipCommitted`). Replay guard built from scratch (verified: no existing token pattern).

**Files:**
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei/src/qahal_coordinator.rs`
- Test: `elohim/holochain/tests/sweettest/src/tests/qahal_formation_test.rs` (new)
- Modify: `elohim/holochain/tests/sweettest/src/tests/mod.rs` (register module — match how `qahal_collab_t0_test` is declared)

- [ ] **Step 1: Write the failing sweettest** — create `qahal_formation_test.rs`:

```rust
//! @dna-scope: imagodei
//! Household formation ceremony T0: issue_household_invite + affirm_membership.
//! Spec: genesis/docs/superpowers/specs/2026-06-04-household-formation-ceremony-design.md §4.1

use anyhow::Result;
use hdk::prelude::{ActionHash, AgentPubKey, Signature};
use serde::{Deserialize, Serialize};

use elohim_sweettest::common::{
    conductors::{load_dna, two_agent_conductors},
    fixtures::network_seed,
};

const DNA: &str = "imagodei";
const ZOME: &str = "imagodei";

// Local I/O mirrors (field names must match coordinator structs for msgpack).
#[derive(Serialize, Debug, Clone)]
struct CreateCollectiveInput { charter: String, display_name: String, salt: String }

#[derive(Serialize, Debug, Clone)]
struct IssueHouseholdInviteInput { collective_cid: String, role: String, expires_at_micros: i64, nonce: String }

#[derive(Serialize, Deserialize, Debug, Clone)]
struct HouseholdInviteToken {
    collective_cid: String, role: String, sponsor_cid: String,
    expires_at_micros: i64, nonce: String,
    issuer_pubkey: AgentPubKey, signature: Signature,
}

#[derive(Serialize, Debug, Clone)]
struct AffirmMembershipInput { token: HouseholdInviteToken }

fn far_future_micros() -> i64 { 4_102_444_800_000_000 } // 2100-01-01

#[tokio::test(flavor = "multi_thread")]
async fn affirm_membership_happy_path_then_replay_rejected() -> Result<()> {
    let (mut conductors, agents) = two_agent_conductors().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(agents[0].clone())).await?;
    let app0 = conductors[0].setup_app_for_agent("imagodei-app", agents[0].clone(), &[dna.clone()]).await?;
    let app1 = conductors[1].setup_app_for_agent("imagodei-app", agents[1].clone(), &[dna]).await?;
    let cell0 = app0.cells().first().expect("cell0").clone();
    let cell1 = app1.cells().first().expect("cell1").clone();

    // Founder (agent 0) creates the household collective.
    let collective_hash: ActionHash = conductors[0].call(&cell0.zome(ZOME), "create_collective",
        CreateCollectiveInput {
            charter: r#"{"kind":"household","rubric":"recognition-of-given","slugAlias":"family-test"}"#.into(),
            display_name: "Test Household".into(),
            salt: "0123456789abcdef0123456789abcdef".into(),
        }).await;
    let collective_cid = format!("collective:{collective_hash}");

    // Founder issues an invite (Steward authority gate inside).
    let token: HouseholdInviteToken = conductors[0].call(&cell0.zome(ZOME), "issue_household_invite",
        IssueHouseholdInviteInput {
            collective_cid: collective_cid.clone(), role: "contributor".into(),
            expires_at_micros: far_future_micros(),
            nonce: "fedcba9876543210fedcba9876543210".into(),
        }).await;

    // Member (agent 1) affirms with the token — authored by THEIR agent.
    elohim_sweettest::common::mirrors::settle_dht(&mut conductors).await;
    let _membership_hash: ActionHash = conductors[1].call(&cell1.zome(ZOME), "affirm_membership",
        AffirmMembershipInput { token: token.clone() }).await;

    elohim_sweettest::common::mirrors::settle_dht(&mut conductors).await;
    let memberships: Vec<holochain_types::prelude::Record> =
        conductors[0].call(&cell0.zome(ZOME), "list_memberships_for_collective", collective_hash).await;
    assert_eq!(memberships.len(), 2, "founder Steward + affirmed Contributor expected");

    // Replay: same token a second time must be rejected (consumed-nonce link).
    let replay: Result<ActionHash, _> = conductors[1]
        .call_fallible(&cell1.zome(ZOME), "affirm_membership", AffirmMembershipInput { token }).await;
    assert!(replay.is_err(), "replayed invite token must be rejected");
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn affirm_membership_rejects_expired_token() -> Result<()> {
    let (mut conductors, agents) = two_agent_conductors().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(agents[0].clone())).await?;
    let app0 = conductors[0].setup_app_for_agent("imagodei-app", agents[0].clone(), &[dna.clone()]).await?;
    let app1 = conductors[1].setup_app_for_agent("imagodei-app", agents[1].clone(), &[dna]).await?;
    let cell0 = app0.cells().first().expect("cell0").clone();
    let cell1 = app1.cells().first().expect("cell1").clone();

    let collective_hash: ActionHash = conductors[0].call(&cell0.zome(ZOME), "create_collective",
        CreateCollectiveInput {
            charter: r#"{"kind":"household"}"#.into(),
            display_name: "Expiry Household".into(),
            salt: "00112233445566770011223344556677".into(),
        }).await;

    let token: HouseholdInviteToken = conductors[0].call(&cell0.zome(ZOME), "issue_household_invite",
        IssueHouseholdInviteInput {
            collective_cid: format!("collective:{collective_hash}"), role: "contributor".into(),
            expires_at_micros: 1, // 1970 — guaranteed expired
            nonce: "aaaabbbbccccddddaaaabbbbccccdddd".into(),
        }).await;

    elohim_sweettest::common::mirrors::settle_dht(&mut conductors).await;
    let expired: Result<ActionHash, _> = conductors[1]
        .call_fallible(&cell1.zome(ZOME), "affirm_membership", AffirmMembershipInput { token }).await;
    assert!(expired.is_err(), "expired invite token must be rejected");
    Ok(())
}
```

NOTE for executor: if `two_agent_conductors()` or `call_fallible` differ in name, check `elohim/holochain/tests/sweettest/src/common/conductors.rs` for the exact two-agent helper and the fallible-call method on the sweettest conductor handle (`SweetConductor::call_fallible` is the upstream holochain name); adjust ONLY those call sites — the test logic stands.

- [ ] **Step 2: Pack the DNA and run the test to verify it fails**

```bash
cd elohim/holochain/dna/imagodei && just build
cd ../../tests/sweettest
CARGO_TARGET_DIR=target/native-tests cargo test -p elohim_sweettest --release qahal_formation -- --include-ignored
```
Expected: FAIL — zome fn `issue_household_invite` not found.

- [ ] **Step 3: Implement the coordinators** — append to `qahal_coordinator.rs` (after `withdraw_membership_clean`):

```rust
// =============================================================================
// Household formation — recognition-of-the-given membership flow
// Spec: genesis/docs/superpowers/specs/2026-06-04-household-formation-ceremony-design.md §4.1
// The graduated flow (request/attest) is the sibling path; this is affirmation:
// the relationship pre-exists, the substrate witnesses, it does not gate.
// =============================================================================

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IssueHouseholdInviteInput {
    pub collective_cid: String,
    /// "steward" | "contributor" | "observer"
    pub role: String,
    pub expires_at_micros: i64,
    /// Caller-supplied randomness (32 hex chars by convention, like salt).
    /// The replay guard is keyed on this nonce.
    pub nonce: String,
}

/// The unsigned portion — exactly what the issuer signs and the affirmer verifies.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HouseholdInvitePayload {
    pub collective_cid: String,
    pub role: String,
    pub sponsor_cid: String,
    pub expires_at_micros: i64,
    pub nonce: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HouseholdInviteToken {
    pub collective_cid: String,
    pub role: String,
    pub sponsor_cid: String,
    pub expires_at_micros: i64,
    pub nonce: String,
    pub issuer_pubkey: AgentPubKey,
    pub signature: Signature,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AffirmMembershipInput {
    pub token: HouseholdInviteToken,
}

fn parse_role(role: &str) -> ExternResult<MembershipRole> {
    match role {
        "steward" => Ok(MembershipRole::Steward),
        "contributor" => Ok(MembershipRole::Contributor),
        "observer" => Ok(MembershipRole::Observer),
        other => Err(wasm_error!("unknown membership role: {}", other)),
    }
}

fn invite_payload_of(token: &HouseholdInviteToken) -> HouseholdInvitePayload {
    HouseholdInvitePayload {
        collective_cid: token.collective_cid.clone(),
        role: token.role.clone(),
        sponsor_cid: token.sponsor_cid.clone(),
        expires_at_micros: token.expires_at_micros,
        nonce: token.nonce.clone(),
    }
}

/// Issue a single-use, signed, TTL'd household invite. Caller must be a current
/// Steward of the collective. The token travels OUT-OF-BAND (QR / LAN / deep
/// link / seeder memory) — it is deliberately NOT a DHT entity (entity model:
/// Category C; the durable proof of invitation is the resulting Membership's
/// sponsor chain).
#[hdk_extern]
pub fn issue_household_invite(input: IssueHouseholdInviteInput) -> ExternResult<HouseholdInviteToken> {
    if input.nonce.len() < 16 {
        return Err(wasm_error!("invite nonce must be at least 16 chars"));
    }
    let issuer_pubkey = agent_info()?.agent_initial_pubkey;
    let issuer_cid = encode_agent_cid(&issuer_pubkey);
    require_caller_is_steward_of(&issuer_cid, &input.collective_cid)?;

    let payload = HouseholdInvitePayload {
        collective_cid: input.collective_cid,
        role: input.role,
        sponsor_cid: issuer_cid,
        expires_at_micros: input.expires_at_micros,
        nonce: input.nonce,
    };
    let signature = sign(issuer_pubkey.clone(), &payload)?;
    Ok(HouseholdInviteToken {
        collective_cid: payload.collective_cid,
        role: payload.role,
        sponsor_cid: payload.sponsor_cid,
        expires_at_micros: payload.expires_at_micros,
        nonce: payload.nonce,
        issuer_pubkey,
        signature,
    })
}

/// Affirm membership in a household collective — the recognition-of-the-given
/// flow. The CALLER's own agent authors the Membership (their identity is
/// theirs from day one); the token's sponsor chain carries the issuer's side
/// of the mutual witness. Replay-guarded via a consumed-nonce anchor link.
#[hdk_extern]
pub fn affirm_membership(input: AffirmMembershipInput) -> ExternResult<ActionHash> {
    let token = input.token;

    // 1. Expiry.
    let now_micros = sys_time()?.as_micros();
    if now_micros > token.expires_at_micros {
        return Err(wasm_error!("invite token expired"));
    }

    // 2. Issuer authority: the signer must be a CURRENT Steward of the collective.
    let issuer_cid = encode_agent_cid(&token.issuer_pubkey);
    if issuer_cid != token.sponsor_cid {
        return Err(wasm_error!("token sponsor_cid does not match issuer key"));
    }
    require_caller_is_steward_of(&issuer_cid, &token.collective_cid)?;

    // 3. Signature over the canonical payload.
    let payload = invite_payload_of(&token);
    let valid = verify_signature(token.issuer_pubkey.clone(), token.signature.clone(), &payload)?;
    if !valid {
        return Err(wasm_error!("invite token signature invalid"));
    }

    // 4. Replay guard: consumed-nonce anchor. Coordinator-side by design —
    //    integrity validators are pure-data here (no link traversal allowed).
    let consumed_anchor = StringAnchor::new("invite-consumed", &token.nonce);
    let consumed_anchor_hash = hash_entry(&EntryTypes::StringAnchor(consumed_anchor.clone()))?;
    let collective_hash = decode_collective_cid_to_action(&token.collective_cid)?;
    let existing = get_links(
        GetLinksInputBuilder::try_new(consumed_anchor_hash.clone(), LinkTypes::CharterAnchor)?.build(),
    )?;
    if !existing.is_empty() {
        return Err(wasm_error!("invite token already consumed"));
    }
    create_entry(&EntryTypes::StringAnchor(consumed_anchor))?;
    create_link(consumed_anchor_hash, collective_hash.clone(), LinkTypes::CharterAnchor, ())?;

    // 5. The Membership — authored by the AFFIRMER's agent.
    let block_height = current_block_height()?;
    let member_cid = encode_agent_cid(&agent_info()?.agent_initial_pubkey);
    let membership = Membership {
        member_cid,
        member_kind: MemberKind::Person,
        collective_cid: token.collective_cid,
        role: parse_role(&token.role)?,
        sponsor_cid: Some(token.sponsor_cid),
        joined_at_block_height: block_height,
        withdrawn_at_block_height: None,
    };
    let membership_hash = create_entry(&EntryTypes::Membership(membership))?;
    create_link(collective_hash, membership_hash.clone(), LinkTypes::HasMembership, ())?;
    // post_commit's existing to_app_option::<Membership>() arm emits
    // MembershipCommitted — no signal code needed here.
    Ok(membership_hash)
}
```

HDK notes for the executor: `sign(key, data)` / `verify_signature(key, sig, data)` take any `Serialize` data; `Signature` and `AgentPubKey` are in `hdk::prelude` (already glob-imported). If `GetLinksInputBuilder` is not in this HDK version's prelude, use the same `get_links(...)` call shape found elsewhere in this file/crate (grep `get_links` in `qahal_coordinator.rs:128` `list_memberships_for_collective` and mirror it exactly).

- [ ] **Step 4: Build the DNA, re-pack, run sweettests**

```bash
cd elohim/holochain/dna/imagodei && just build
cd ../../tests/sweettest
CARGO_TARGET_DIR=target/native-tests cargo test -p elohim_sweettest --release qahal_formation -- --include-ignored
```
Expected: both tests PASS. (Do NOT mark them `#[ignore]` — CI runs `--run-ignored all` so ignore is a no-op anyway, and these must be CI-green.)

- [ ] **Step 5: Run the existing qahal sweettest to verify no regression**

```bash
CARGO_TARGET_DIR=target/native-tests cargo test -p elohim_sweettest --release qahal_collab -- --include-ignored
```
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add elohim/holochain/dna/imagodei/zomes/imagodei/src/qahal_coordinator.rs \
        elohim/holochain/tests/sweettest/src/tests/qahal_formation_test.rs \
        elohim/holochain/tests/sweettest/src/tests/mod.rs
git commit -m "feat(imagodei): issue_household_invite + affirm_membership — recognition-of-the-given flow with replay-guarded single-use tokens"
```

---

### Task 3: Charter reaches the projection — `governance_layer='family'` + slug-alias merge

The translation currently DROPS charter, so the projector hardcodes `governance_layer: COMMUNITY`. Carry charter through; parse household kind; merge onto a pre-coherence row by slugAlias.

**Files:**
- Modify: `elohim/elohim-storage/src/reconcile/signal_stream.rs:328` (CollectiveProjectedSignal — add field)
- Modify: `elohim/elohim-storage/src/reconcile/holochain_app_signal.rs:281` (translation) + its tests (~1081)
- Modify: `elohim/elohim-storage/src/reconcile/controller.rs` (`on_collective_projected`) + its tests (~1711)
- Build env (every storage step): `export RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=$(genesis/agentic/bin/cargo-pool key)`

- [ ] **Step 1: Write the failing translation test** — in `holochain_app_signal.rs` tests, find the existing `CollectiveProjected` translation test (~line 1075-1095) and add a sibling:

```rust
#[test]
fn collective_committed_carries_charter_through_translation() {
    let sig = imagodei_signal_collective_committed_with_charter(
        r#"{"kind":"household","rubric":"recognition-of-given","slugAlias":"family-dowell"}"#,
    );
    match translate_imagodei(sig).expect("translates") {
        DnaSignal::CollectiveProjected(c) => {
            assert_eq!(
                c.charter.as_deref(),
                Some(r#"{"kind":"household","rubric":"recognition-of-given","slugAlias":"family-dowell"}"#)
            );
        }
        other => panic!("expected CollectiveProjected, got {other:?}"),
    }
}
```

(Adapt the fixture-constructor name to whatever helper the existing test at ~1081 uses to build the incoming `ImagodeiSignal::CollectiveCommitted` — extend that helper with a charter parameter rather than duplicating it.)

- [ ] **Step 2: Run to verify it fails**

```bash
cd elohim/elohim-storage && cargo nextest run --lib reconcile::holochain_app_signal
```
Expected: FAIL — no field `charter` on `CollectiveProjectedSignal`.

- [ ] **Step 3: Implement the carry** —
(a) `signal_stream.rs` `CollectiveProjectedSignal`: add

```rust
    /// Full charter string from the Collective entry. Carried so the projector
    /// can derive household-ness (governance_layer='family') and the slug
    /// alias; `None` for legacy senders.
    #[serde(default)]
    pub charter: Option<String>,
```

(b) `holochain_app_signal.rs` translation arm (~281): add `charter: Some(collective.charter)` to the constructed `CollectiveProjectedSignal` (the `Collective` struct carries charter; it is currently dropped). Fix any other construction sites the compiler flags (tests) with `charter: None`.

- [ ] **Step 4: Write the failing projector test** — in `controller.rs` tests, copy the existing `CollectiveProjected` controller test pattern (~1711, `InMemoryDnaSignalStream::with_signals`) into a new test:

```rust
#[tokio::test]
async fn household_charter_sets_family_governance_and_merges_slug_alias() {
    // Arrange: a pre-coherence seed row 'family-dowell' exists (community layer, no CID).
    let (pool, ctx) = test_pool_with_ctx(); // mirror the existing test's pool helper
    {
        use crate::db::collectives::{create_collective, CreateCollectiveInput};
        let mut conn = pool.get().unwrap();
        create_collective(&mut conn, &ctx, &CreateCollectiveInput {
            id: "family-dowell".into(), name: "Dowell Family".into(), description: None,
            governance_layer: "family".into(), constitutional_parent_id: None,
            reach: "trusted".into(), metadata_json: None, created_by: None,
        }).unwrap();
    }
    let signal = CollectiveProjectedSignal {
        action_hash: "uhCkkFAKE".into(),
        collective_cid: "collective:uhCkkFAKE".into(),
        display_name: "Dowell Family".into(),
        founder_agent_cid: "agent:uhCAkFOUNDER".into(),
        anchor_agreement_cid: None,
        charter: Some(r#"{"kind":"household","slugAlias":"family-dowell"}"#.into()),
    };
    // Act: drive the controller over the signal (mirror existing test plumbing).
    run_controller_over(vec![DnaSignal::CollectiveProjected(signal)], &pool).await;
    // Assert: NO new row; the family-dowell row got the CID stamp + slug.
    let mut conn = pool.get().unwrap();
    use crate::db::diesel_schema::collectives::dsl::*;
    use diesel::prelude::*;
    let rows: Vec<(String, Option<String>, String, Option<String>)> = collectives
        .select((id, collective_cid, governance_layer, slug))
        .load(&mut conn).unwrap();
    assert_eq!(rows.len(), 1, "merge, not duplicate");
    let row = &rows[0];
    assert_eq!(row.0, "family-dowell");
    assert_eq!(row.1.as_deref(), Some("collective:uhCkkFAKE"));
    assert_eq!(row.2, "family");
    assert_eq!(row.3.as_deref(), Some("family-dowell"));
}
```

(Adapt `test_pool_with_ctx`/`run_controller_over` to the exact helpers the ~1711 test uses — same plumbing, new assertions.)

- [ ] **Step 5: Run to verify it fails**

```bash
cargo nextest run --lib reconcile::controller
```
Expected: FAIL — two rows (duplicate created) and/or governance_layer "community".

- [ ] **Step 6: Implement the projector household branch** — in `on_collective_projected`, before building `CreateCollectiveInput`, parse the charter:

```rust
        // Household coherence (2026-06-04 formation spec §5.4): a charter that
        // declares {"kind":"household"} projects as governance_layer='family',
        // and a declared slugAlias merges onto the pre-coherence seed row
        // (one household, one row — slug is the display alias, CID canonical).
        #[derive(serde::Deserialize, Default)]
        struct CharterHints {
            #[serde(default)] kind: Option<String>,
            #[serde(default, rename = "slugAlias")] slug_alias: Option<String>,
        }
        let hints: CharterHints = signal
            .charter
            .as_deref()
            .and_then(|c| serde_json::from_str(c).ok())
            .unwrap_or_default();
        let is_household = hints.kind.as_deref() == Some("household");
        let layer = if is_household {
            crate::db::models::governance_layers::FAMILY.to_string()
        } else {
            crate::db::models::governance_layers::COMMUNITY.to_string()
        };
```

(If `governance_layers::FAMILY` does not exist as a const, grep `mod governance_layers` in `db/models.rs` — add `pub const FAMILY: &str = "family";` beside `COMMUNITY` in the same style.)

Then add the merge branch BEFORE the create path:

```rust
        // Slug-alias merge: if a row already exists under the alias id, stamp
        // it instead of creating a duplicate.
        if let Some(alias) = hints.slug_alias.as_deref() {
            use crate::db::diesel_schema::collectives;
            use diesel::prelude::*;
            let stamped = diesel::update(
                collectives::table
                    .filter(collectives::h_app_id.eq(&ctx.h_app_id))
                    .filter(collectives::id.eq(alias)),
            )
            .set((
                collectives::collective_cid.eq(Some(signal.collective_cid.as_str())),
                collectives::slug.eq(Some(alias)),
                collectives::governance_layer.eq(&layer),
            ))
            .execute(&mut conn)
            .unwrap_or(0);
            if stamped > 0 {
                debug!(collective_cid = %signal.collective_cid, alias = %alias,
                       "household collective merged onto pre-coherence row");
                return Ok(());
            }
        }
```

and use `governance_layer: layer` (instead of the hardcoded COMMUNITY) in the existing `CreateCollectiveInput` construction, plus `reach: if is_household { "trusted".to_string() } else { "community".to_string() }`.

- [ ] **Step 7: Run the full storage lib gate**

```bash
cargo nextest run --lib && cargo clippy -- -D warnings && cargo fmt --check
```
Expected: all PASS (translation + controller + everything else green).

- [ ] **Step 8: Commit**

```bash
git add elohim/elohim-storage/src/reconcile/signal_stream.rs \
        elohim/elohim-storage/src/reconcile/holochain_app_signal.rs \
        elohim/elohim-storage/src/reconcile/controller.rs \
        elohim/elohim-storage/src/db/models.rs
git commit -m "feat(storage): charter rides CollectiveProjected — household charters project governance_layer=family and merge onto slug-alias rows"
```

---### Task 4: Soft action-gate extension — custody-blob anchors via conductor when available

**Files:**
- Modify: `elohim/elohim-storage/src/db/rea_commitments.rs` (add const beside `PROJECT_EPR_ACTION` at :408)
- Modify: `elohim/elohim-storage/src/services/rea_commitment_service.rs:38-51` (the gate)

- [ ] **Step 1: Write the failing test** — in `rea_commitment_service.rs`'s test module (or create one mirroring the file's existing test conventions; if the service has no test module, put the test in `db/rea_commitments.rs`'s test module where an in-memory pool already exists):

```rust
#[test]
fn custody_blob_routes_diesel_when_no_conductor() {
    // Soft gate: custody-blob WITHOUT hc_lamad must still succeed diesel-direct
    // (local dev / degraded), preserving today's behavior.
    let (mut conn, ctx) = test_conn_and_ctx(); // mirror existing db test helper
    let input = CreateReaCommitmentInput {
        id: Some("custody-blob-test01".into()),
        action: "custody-blob".into(),
        provider: "12D3KooWAAAA".into(),
        receiver: "12D3KooWBBBB".into(),
        ..Default::default()
    };
    let view = futures::executor::block_on(ReaCommitmentService::create(
        &mut conn, &ctx, input, None, None,
    )).expect("diesel fallback must succeed");
    assert_eq!(view.action, "custody-blob");
}
```

(If `CreateReaCommitmentInput` lacks `Default`, construct all 18 fields explicitly — copy the construction from an existing test in `db/rea_commitments.rs` and change action/id.)

- [ ] **Step 2: Run to verify current state** — `cargo nextest run --lib rea_commitment` — this test PASSES today (custody-blob is diesel-direct). It is the regression guard for the gate change; commit it red-green as written.

- [ ] **Step 3: Implement the gate** — in `db/rea_commitments.rs` beside `PROJECT_EPR_ACTION` (:408):

```rust
/// Actions that round-trip the conductor when one is connected (anchored,
/// gossiped) but degrade gracefully to diesel-direct when not. custody-blob
/// joined 2026-06-04 (household formation spec §5.3) — it is in the elohim
/// DNA's REA_ACTIONS vocabulary, so content_store::create_rea_commitment
/// accepts it. Mishpat-discriminated actions (delegates-compute,
/// replicates-dwelling) are NOT here: they need a mishpat-role client
/// (stage-2 work).
pub const CONDUCTOR_SOFT_ACTIONS: [&str; 1] = ["custody-blob"];
```

and in the service gate:

```rust
        if input.action == PROJECT_EPR_ACTION {
            return Self::create_via_conductor(conn, ctx, input, events, hc_lamad).await;
        }
        if rea_commitments::CONDUCTOR_SOFT_ACTIONS.contains(&input.action.as_str())
            && hc_lamad.is_some()
        {
            return Self::create_via_conductor(conn, ctx, input, events, hc_lamad).await;
        }
        // Legacy diesel-direct path — preserved for non-project-epr actions
        // pending follow-up migration. See module docs.
        Self::create_via_diesel(conn, ctx, input, events)
```

- [ ] **Step 4: Full storage gate**

```bash
cargo nextest run --lib && cargo clippy -- -D warnings && cargo fmt --check
```
Expected: PASS (incl. the Step-1 guard test).

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/db/rea_commitments.rs elohim/elohim-storage/src/services/rea_commitment_service.rs
git commit -m "feat(storage): custody-blob joins the conductor round-trip when a conductor is connected — soft gate, diesel fallback preserved"
```

---

### Task 5: Apply the DeliveryPeer household-enrichment patch

The prepared, verified-clean patch (DeliveryPeer gains `household_id` + `commitments`; `active_provide_reaches` + tests in rea_commitments.rs; soft-fail enrichment in `handle_delivery_peers`).

**Files:**
- Apply: `genesis/data/timeline/backlog/patches/deliverypeer-household-enrichment.patch`

- [ ] **Step 1: Verify it still applies** (concurrent sessions touch these files)

```bash
git apply --check genesis/data/timeline/backlog/patches/deliverypeer-household-enrichment.patch
```
Expected: exit 0, silent. If it conflicts: re-anchor the hunks by hand against the current `p2p/mod.rs:302` (struct), `http.rs:2362` (handler), `db/rea_commitments.rs` (append fns + test module) — the patch content is the specification.

- [ ] **Step 2: Apply + run its tests**

```bash
git apply genesis/data/timeline/backlog/patches/deliverypeer-household-enrichment.patch
cd elohim/elohim-storage && cargo nextest run --lib provide_reach
```
Expected: the patch's `provide_reach_tests` PASS. (No TS regen needed — DeliveryPeer is serde-only, no `#[derive(TS)]`; verified.)

- [ ] **Step 3: Full storage gate, then commit + remove the patch file (it has landed)**

```bash
cargo nextest run --lib && cargo clippy -- -D warnings && cargo fmt --check
git add elohim/elohim-storage/src/db/rea_commitments.rs elohim/elohim-storage/src/http.rs elohim/elohim-storage/src/p2p/mod.rs
git rm genesis/data/timeline/backlog/patches/deliverypeer-household-enrichment.patch
git commit -m "feat(storage): DeliveryPeer carries household_id + active provide reaches — delivery surface sees households (applies prepared patch)"
```

---

### Task 6: `seed-household-formation.ts` — the ceremony driver (rung 3)

Drives the real choreography per-conductor: matthew creates + invites; jessica/james affirm on THEIR conductors; each agent authors its custody commitments via `content_store.create_rea_commitment` (lamad cell); matthew grants stewardship over james.

**Files:**
- Create: `genesis/seeder/src/seed-household-formation.ts`
- Test: `genesis/seeder/src/__tests__/seed-household-formation.spec.ts`
- Modify: `genesis/seeder/package.json` (scripts)

- [ ] **Step 1: Write the failing builder tests**

```ts
import { describe, it, expect } from 'vitest';
import {
  buildHouseholdCharter, buildCeremonyCustodyInput, HOUSEHOLD_MEMBERS,
} from '../seed-household-formation.js';

describe('buildHouseholdCharter', () => {
  it('declares household kind, rubric, and the family-dowell slug alias', () => {
    const charter = JSON.parse(buildHouseholdCharter());
    expect(charter.kind).toBe('household');
    expect(charter.rubric).toBe('recognition-of-given');
    expect(charter.slugAlias).toBe('family-dowell');
  });
});

describe('buildCeremonyCustodyInput', () => {
  it('builds a snake_case zome input with ceremony provenance and collective scope', () => {
    const input = buildCeremonyCustodyInput({
      providerHumanId: 'human-jessica-spouse', providerArchetype: 'desktop',
      receiverHumanId: 'human-matthew-manager', receiverArchetype: 'desktop',
      blobHash: 'sha256-deadbeef', blobSizeBytes: 64,
      collectiveCid: 'collective:uhCkkFAKE',
    });
    expect(input.action).toBe('custody-blob');
    expect(input.provider.startsWith('12D3KooW')).toBe(true);
    expect(input.in_scope_of).toEqual(['collective:uhCkkFAKE']);
    expect(JSON.parse(input.metadata_json!).seedGeneration).toBe('ceremony');
    expect(JSON.parse(input.metadata_json!).providerHumanId).toBe('human-jessica-spouse');
  });
});

describe('HOUSEHOLD_MEMBERS', () => {
  it('is the canonical triad with the founder first', () => {
    expect(HOUSEHOLD_MEMBERS.map(m => m.humanId)).toEqual([
      'human-matthew-manager', 'human-jessica-spouse', 'human-james-student',
    ]);
    expect(HOUSEHOLD_MEMBERS[2].minor).toBe(true);
  });
});
```

- [ ] **Step 2: Run to verify it fails** — `cd genesis/seeder && pnpm exec vitest run seed-household-formation` → FAIL (module missing).

- [ ] **Step 3: Implement the seeder.** Create `seed-household-formation.ts`. Structure (complete; the connect helper is copied VERBATIM from `seed-conductor-identities.ts:126-238` — `toAdminUrl`, `withTimeout`, `connectToConductor`, both `cell_info` shapes — do not re-derive it):

```ts
#!/usr/bin/env npx tsx
/**
 * Seed: household formation ceremony (Stage 1, rung-3 realism).
 * Spec: genesis/docs/superpowers/specs/2026-06-04-household-formation-ceremony-design.md §3, §6.
 *
 * REALISM RUNG: 3 (conductor-zome-as-agent). Every act below is authored by the
 * persona's OWN conductor agent: matthew creates the collective + issues
 * invites; jessica and james affirm on THEIR conductors; custody-blob
 * commitments are authored pairwise by each provider's agent; matthew grants
 * stewardship over james. Genesis data = "this family ran the ceremony."
 *
 * Ordering: AFTER seed-conductor-identities (Human profiles must exist for
 * create_stewardship_grant) and seed-agent-bindings.
 *
 * Env: CONDUCTOR_URLS (comma-separated app WS urls), INSTALLED_APP_ID prefix
 * (default 'elohim'), M1_BLOB_HASH + M1_BLOB_SIZE_BYTES (custody payload),
 * HOUSEHOLD_SALT (32 hex; deterministic default), HOUSEHOLD_NONCE_PREFIX.
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import { AdminWebsocket, AppWebsocket } from '@holochain/client';
import { deterministicPeerId, type Archetype } from './peer-id.js';

// ---------- canonical triad ----------
export interface HouseholdMember {
  humanId: string;
  archetype: Archetype;
  role: 'steward' | 'contributor';
  minor: boolean;
}
export const HOUSEHOLD_MEMBERS: HouseholdMember[] = [
  { humanId: 'human-matthew-manager', archetype: 'desktop', role: 'steward', minor: false },
  { humanId: 'human-jessica-spouse', archetype: 'desktop', role: 'steward', minor: false },
  { humanId: 'human-james-student',  archetype: 'mobile',  role: 'contributor', minor: true },
];

export function buildHouseholdCharter(): string {
  return JSON.stringify({
    kind: 'household',
    rubric: 'recognition-of-given',
    slugAlias: 'family-dowell',
  });
}

// ---------- custody zome input (shefa_types::CreateReaCommitmentInput, snake_case) ----------
export interface CeremonyCustodyParams {
  providerHumanId: string; providerArchetype: Archetype;
  receiverHumanId: string; receiverArchetype: Archetype;
  blobHash: string; blobSizeBytes: number; collectiveCid: string;
}
export function buildCeremonyCustodyInput(p: CeremonyCustodyParams) {
  const provider = deterministicPeerId(p.providerHumanId, p.providerArchetype);
  const receiver = deterministicPeerId(p.receiverHumanId, p.receiverArchetype);
  const idDigest = createHash('sha256')
    .update(`${provider}|${receiver}|${p.blobHash}`).digest('hex').slice(0, 16);
  return {
    id: `custody-blob-${idDigest}`,
    action: 'custody-blob',
    provider,
    receiver,
    resource_classified_as: [p.blobHash],
    resource_quantity_value: p.blobSizeBytes,
    resource_quantity_unit: 'B',
    in_scope_of: [p.collectiveCid],
    note: `household custody: ${p.providerHumanId} -> ${p.receiverHumanId}`,
    metadata_json: JSON.stringify({
      seedGeneration: 'ceremony',
      blobHash: p.blobHash,
      providerHumanId: p.providerHumanId,
      receiverHumanId: p.receiverHumanId,
    }),
  };
}

// ---------- conductor session plumbing: copy VERBATIM from seed-conductor-identities.ts ----------
// (toAdminUrl, withTimeout, connectToConductor — including both cell_info
// shapes and the 300s single-use token. ALSO copy its cell-role resolution but
// parameterize the role name: this script needs BOTH the 'imagodei' cell (formation)
// and the 'lamad' cell (custody commitments) from the same matchingApp.)
// ... [paste here unchanged, with `cellForRole(matchingApp, role)` extracted] ...

interface MemberSession { member: HouseholdMember; appWs: AppWebsocket; imagodeiCell: CellId; lamadCell: CellId | null; url: string; }

async function findMemberSessions(urls: string[], appIdPrefix: string): Promise<Map<string, MemberSession>> {
  // For each conductor URL, connect and identify WHICH member it hosts by
  // calling imagodei.get_my_human and matching human.id against the triad.
  // (Identity seeding has already run; every member conductor has a Human.)
  const sessions = new Map<string, MemberSession>();
  for (const url of urls) {
    const conn = await connectToConductor(url, appIdPrefix).catch(() => null);
    if (!conn) continue;
    const me: { human?: { id?: string } } | null = await conn.appWs.callZome({
      cell_id: conn.imagodeiCell, zome_name: 'imagodei', fn_name: 'get_my_human', payload: null,
    }).catch(() => null);
    const member = HOUSEHOLD_MEMBERS.find(m => m.humanId === me?.human?.id);
    if (member && !sessions.has(member.humanId)) {
      sessions.set(member.humanId, { member, ...conn, url });
    } else {
      await conn.appWs.client.close();
    }
  }
  return sessions;
}

async function main(): Promise<void> {
  const urls = (process.env.CONDUCTOR_URLS ?? '').split(',').map(u => u.trim()).filter(Boolean);
  const appIdPrefix = process.env.INSTALLED_APP_ID ?? 'elohim';
  const blobHash = process.env.M1_BLOB_HASH ?? '';
  const blobSize = parseInt(process.env.M1_BLOB_SIZE_BYTES ?? '0', 10);
  if (urls.length === 0) { console.error('ERROR: CONDUCTOR_URLS required'); process.exit(1); }

  const sessions = await findMemberSessions(urls, appIdPrefix);
  const founder = sessions.get('human-matthew-manager');
  if (!founder) { console.error('ERROR: founder conductor (matthew) not reachable — cannot run ceremony'); process.exit(1); }

  // 1. FOUNDER: create the household collective (idempotent: probe first).
  //    Probe: list collectives for an existing family-dowell-affiliated one is
  //    zome-side absent; use deterministic salt so re-create yields the SAME
  //    entry hash (Holochain entry identity is content-derived) — re-running is
  //    a no-op at the DHT level.
  const salt = process.env.HOUSEHOLD_SALT ?? 'f00df00df00df00df00df00df00df00d';
  const collectiveHash: unknown = await founder.appWs.callZome({
    cell_id: founder.imagodeiCell, zome_name: 'imagodei', fn_name: 'create_collective',
    payload: { charter: buildHouseholdCharter(), display_name: 'Dowell Family', salt },
  });
  const collectiveCid = `collective:${collectiveHash}`;
  console.log(`[+] household collective: ${collectiveCid}`);

  // 2. AFFIRMATIONS: founder issues an invite per member; member affirms on THEIR conductor.
  const farFuture = (Date.now() + 24 * 3600 * 1000) * 1000; // 24h, micros
  const affirmed: string[] = ['human-matthew-manager'];
  for (const member of HOUSEHOLD_MEMBERS.slice(1)) {
    const session = sessions.get(member.humanId);
    if (!session) { console.warn(`[~] ${member.humanId}: conductor unreachable — skipping affirm`); continue; }
    const nonce = createHash('sha256').update(`${process.env.HOUSEHOLD_NONCE_PREFIX ?? 'genesis'}:${member.humanId}`).digest('hex').slice(0, 32);
    const token = await founder.appWs.callZome({
      cell_id: founder.imagodeiCell, zome_name: 'imagodei', fn_name: 'issue_household_invite',
      payload: { collective_cid: collectiveCid, role: member.role, expires_at_micros: farFuture, nonce },
    });
    try {
      await session.appWs.callZome({
        cell_id: session.imagodeiCell, zome_name: 'imagodei', fn_name: 'affirm_membership',
        payload: { token },
      });
      affirmed.push(member.humanId);
      console.log(`[+] ${member.humanId} affirmed membership`);
    } catch (err) {
      const msg = String(err);
      if (msg.includes('already consumed')) { affirmed.push(member.humanId); console.log(`[=] ${member.humanId} already affirmed (idempotent)`); }
      else { console.error(`[!] ${member.humanId} affirm failed: ${msg}`); }
    }
  }

  // 3. KID STEWARDSHIP: founder grants over james (requires founder Human profile).
  if (affirmed.includes('human-james-student')) {
    try {
      await founder.appWs.callZome({
        cell_id: founder.imagodeiCell, zome_name: 'imagodei', fn_name: 'create_stewardship_grant',
        payload: {
          subject_id: 'human-james-student',
          authority_basis: 'parental',
          evidence_hash: null,
          verified_by: 'household-formation-ceremony',
          content_filtering: true, time_limits: true, feature_restrictions: true,
          activity_monitoring: true, policy_delegation: false,
          delegatable: false, expires_in_days: 365, review_in_days: 90,
        },
      });
      console.log('[+] stewardship grant: matthew -> james');
    } catch (err) {
      console.warn(`[~] stewardship grant failed (non-fatal for ceremony): ${String(err)}`);
    }
  }

  // 4. AMBIENT CUSTODY: each affirmed pair, both directions, authored BY THE PROVIDER's agent.
  let custodyOk = 0, custodyFail = 0;
  if (blobHash && blobSize > 0) {
    for (const provider of HOUSEHOLD_MEMBERS) {
      if (!affirmed.includes(provider.humanId)) continue;
      const ps = sessions.get(provider.humanId);
      if (!ps?.lamadCell) { console.warn(`[~] ${provider.humanId}: no lamad cell — custody skipped`); continue; }
      for (const receiver of HOUSEHOLD_MEMBERS) {
        if (receiver.humanId === provider.humanId || !affirmed.includes(receiver.humanId)) continue;
        const input = buildCeremonyCustodyInput({
          providerHumanId: provider.humanId, providerArchetype: provider.archetype,
          receiverHumanId: receiver.humanId, receiverArchetype: receiver.archetype,
          blobHash, blobSizeBytes: blobSize, collectiveCid,
        });
        try {
          await ps.appWs.callZome({
            cell_id: ps.lamadCell, zome_name: 'content_store', fn_name: 'create_rea_commitment', payload: input,
          });
          custodyOk++;
        } catch (err) {
          const msg = String(err);
          if (msg.includes('already') || msg.includes('duplicate')) custodyOk++;
          else { custodyFail++; console.error(`[!] custody ${provider.humanId}->${receiver.humanId}: ${msg}`); }
        }
      }
    }
  } else {
    console.warn('[~] M1_BLOB_HASH/M1_BLOB_SIZE_BYTES unset — custody layer skipped');
  }

  // 5. Result artifact + exit code (identities-script convention).
  const partial = affirmed.length < HOUSEHOLD_MEMBERS.length || custodyFail > 0;
  const out = {
    schemaVersion: 1, seededAt: new Date().toISOString(), script: 'seed-household-formation.ts',
    collectiveCid, affirmed, custodyOk, custodyFail, partial,
  };
  writeFileSync(resolve(dirname(fileURLToPath(import.meta.url)), '../seed-results-household-formation.json'),
    JSON.stringify(out, null, 2));
  for (const s of sessions.values()) await s.appWs.client.close();
  console.log(JSON.stringify(out));
  process.exit(partial ? 2 : 0);
}

const isMain = import.meta.url === `file://${process.argv[1]}`;
if (isMain) main().catch(err => { console.error('FATAL:', err); process.exit(1); });
```

Executor notes: (a) the elided `connectToConductor` block must also resolve the `lamad` cell (`matchingApp.cell_info['lamad']`, same dual-shape iteration; set `lamadCell: null` when the role is absent so custody soft-skips); (b) zome payloads are snake_case throughout — this is the conductor boundary, not the doorway REST boundary; (c) `create_collective` is idempotent at the DHT level via the deterministic salt (same content → same entry hash), and the affirm path is replay-guarded zome-side — `[=]` paths make re-runs clean.

- [ ] **Step 4: Tests + typecheck**

```bash
cd genesis/seeder && pnpm exec vitest run seed-household-formation && pnpm typecheck
```
Expected: PASS.

- [ ] **Step 5: Register the scripts** — `genesis/seeder/package.json`, beside `seed:commitments`:

```json
"seed:household": "npx tsx src/seed-household-formation.ts",
"seed:household:dev": "CONDUCTOR_URLS='ws://localhost:4445' npx tsx src/seed-household-formation.ts",
```

- [ ] **Step 6: Commit**

```bash
git add genesis/seeder/src/seed-household-formation.ts genesis/seeder/src/__tests__/seed-household-formation.spec.ts genesis/seeder/package.json
git commit -m "feat(seeder): seed-household-formation — the ceremony driven per-conductor as each persona's real agent (rung 3)"
```

---

### Task 7: Jenkins stage — Seed Household Formation

**Files:**
- Modify: `genesis/Jenkinsfile` (new stage after 'Seed Agent Peer Bindings' ~line 1721, BEFORE 'Upload M1 Blob-Backed Content'; plus a custody-env note)

- [ ] **Step 1: Add the stage** — mirror the Seed Conductor Identities stage shape (~1694-1716) exactly:

```groovy
        stage('Seed Household Formation') {
            when { allOf { expression { env.PIPELINE_SKIPPED != 'true' }; expression { params.SEED_DATA } } }
            steps {
                script {
                    // Ceremony driver (rung 3): collective + affirmations + stewardship
                    // grant run BEFORE the blob upload; the custody layer inside the
                    // script self-skips when M1_BLOB_HASH is absent and the
                    // Seed Custody Commitments stage remains the M1 anti-drift flag.
                    def res = runProbedSeeder('Seed Household Formation', 'seed-household-formation.ts',
                        'seed-results-household-formation.json')
                    if (res.skipped) return
                    if (res.exitCode == 2) { unstable("Household formation partial: ${res.seedReport?.affirmed} affirmed") }
                    else if (res.exitCode != 0) { unstable("Household formation exited ${res.exitCode}") }
                }
            }
        }
```

- [ ] **Step 2: Lint check** — Jenkinsfile is groovy; no local runner. Verify by eye: stage sits between the two existing seed stages, uses only existing helpers (`runProbedSeeder`), no inline logic beyond the calls (64KB CPS limit discipline).

- [ ] **Step 3: Commit**

```bash
git add genesis/Jenkinsfile
git commit -m "ci(genesis): Seed Household Formation stage — ceremony runs after bindings, custody layer self-skips pre-blob"
```

---

### Task 8: `qahal/household` quiltPolicy class + codegen gate

**Files:**
- Modify: `elohim/sdk/domains/qahal/manifest.json` (vocabulary block)
- Modify: `elohim/sdk/domains/qahal/scripts/codegen.mjs` (add the ref-integrity gate)
- Modify: `package.json` (root — extend `manifest:codegen:verify`)

- [ ] **Step 1: Declare the class** — in `qahal/manifest.json` `vocabulary`, after `signals` and before `observations`, insert:

```json
"quiltPolicies": {
  "household": {
    "defaultTierFloor": "stocked",
    "shelveAfter": "90d",
    "holdWarmMin": "7d",
    "preferDestinations": [
      "peer-cellar://household/{any}",
      "federated-dwelling://family/{family-id}"
    ]
  }
},
"quiltPolicyDefault": "household",
```

and on the `collective` contentType declaration add:

```json
"quiltPolicy": "household",
```

(Pledge-clamp note for reviewers: the declared `stocked` floor is backed at CommitmentFactory negotiation by the ceremony's custody-blob commitments — spec §8; floors are matched, never silently degraded.)

- [ ] **Step 2: Wire the gate into qahal codegen** — in `qahal/scripts/codegen.mjs`, mirror lamad's pattern (lamad `codegen.mjs:22` + `:444-449`):

```js
import { validateQuiltPolicyRefs } from '../../../schemas/scripts/lib/manifest-quilt-refs.mjs';
// ... immediately after the manifest is loaded/parsed:
const quiltRefErrors = validateQuiltPolicyRefs(manifest);
if (quiltRefErrors.length > 0) {
  console.error('Manifest quilt-policy referential-integrity errors:');
  for (const e of quiltRefErrors) console.error(`  - ${e}`);
  process.exit(1);
}
```

- [ ] **Step 3: Extend the root gate** — root `package.json` line ~60:

```json
"manifest:codegen:verify": "node elohim/sdk/schemas/scripts/codegen-manifest.mjs --gate elohim/sdk/domains/lamad/manifest.json --gate elohim/sdk/domains/qahal/manifest.json",
```

(If `codegen-manifest.mjs` accepts only one `--gate`, check its arg parsing; if single-valued, chain instead: `"... --gate .../lamad/manifest.json && node .../codegen-manifest.mjs --gate .../qahal/manifest.json"`.)

- [ ] **Step 4: Validate + regen + run the schema tests**

```bash
pnpm run qahal:codegen && pnpm run manifest:test && pnpm run schema:test
```
Expected: codegen exits 0 (refs valid), both test scripts PASS (the quilt-policy test validates shape: kebab-case name, duration literals, no unknown fields). Stage any regenerated files under `app/elohim-app/src/app/qahal/generated/`.

- [ ] **Step 5: Commit**

```bash
git add elohim/sdk/domains/qahal/manifest.json elohim/sdk/domains/qahal/scripts/codegen.mjs package.json app/elohim-app/src/app/qahal/generated/
git commit -m "feat(qahal): household quiltPolicy class — stocked floor, household/family destinations; qahal codegen gains the quilt-refs gate"
```

---

### Task 9: Scenario spine — household-formation.feature + reciprocity extension + steps + retags

**Files:**
- Create: `genesis/a2o/features/qahal/household-formation.feature`
- Create: `genesis/a2o/steps/qahal-formation.steps.ts`
- Modify: `genesis/a2o/features/resilience/household-reciprocity.feature`
- Modify: `genesis/a2o/features/shefa/human-resilience.feature` (retags at L27, L82, L94, L105; persona swap Susan→Jessica in scenarios 2/7/8)
- Modify: `genesis/a2o/held/features/lamad/love-map-negotiation.feature:1` (feature tag)
- Modify: `genesis/data/collectives/collectives.json` (family-dowell "Terrance"→"James")

- [ ] **Step 1: Write the formation feature**

```gherkin
@e2e @qahal @household-formation @requires:household-nodes
Feature: Household formation — recognition of the given
  A family — each member with a device, hub or not — forms a household and
  immediately sees the protocol working among themselves. The ceremony is the
  ONLY canonical mint of the reciprocity bundle (spec §1: emergent + marked
  interim fixtures). These structural scenarios assert the ceremony's OUTPUT
  as projected to elohim-storage; the seeder (seed-household-formation.ts)
  drives the real per-conductor choreography in CI.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And elohim-storage is reachable at "E2E_STORAGE_URL"

  Scenario: The household collective is coherent — family-layer, CID-stamped
    When I fetch the collective "family-dowell"
    Then the collective has governance layer "family"
    And the collective is anchored with a canonical collective CID

  Scenario: All three members are affirmed participants
    When I list participants of collective "family-dowell"
    Then the participant set includes the canonical household triad

  @wip
  Scenario: James's membership is sponsored, not self-granted
    When I list participants of collective "family-dowell"
    Then the participation of "human-james-student" carries a sponsor

  Scenario: The ambient custody mesh emerged from the ceremony
    When I list active "custody-blob" commitments
    Then an active "custody-blob" commitment exists from "human-matthew-manager" to "human-james-student"
    And an active "custody-blob" commitment exists from "human-james-student" to "human-matthew-manager"
    And an active "custody-blob" commitment exists from "human-jessica-spouse" to "human-james-student"
    And an active "custody-blob" commitment exists from "human-james-student" to "human-jessica-spouse"

  @wip
  Scenario: Ceremony custody is anchored, fixture custody is marked
    When I list active "custody-blob" commitments
    Then every "custody-blob" commitment with ceremony provenance is DHT-anchored
    And every "custody-blob" commitment with fixture provenance declares its retirement
```

(Scenario 3 and 5 carry `@wip`: sponsor surfacing on the participants view and the anchored/fixture provenance assertions depend on Task 3+5 landing in the deployed environment before they can run; the others run as soon as a seeded environment exists. The triad custody scenario is satisfiable by the Task-1 FIXTURES until the ceremony lands — that is the deliberate interim posture; Task 10 flips its provenance.)

- [ ] **Step 2: Write the step definitions** — `steps/qahal-formation.steps.ts` (auto-loaded by the `steps/**/*.ts` glob; copy the `storageGet` import/idiom from `steps/resilience.steps.ts`):

```ts
import { Given, When, Then } from '@cucumber/cucumber';
import assert from 'node:assert';
import type { E2EWorld } from '../src/framework/world.js'; // match the import used in resilience.steps.ts
// Reuse the storageGet helper convention from resilience.steps.ts — if it is
// file-local there, lift it into a shared module or duplicate the 6-line
// undici request wrapper here exactly as written there.

const collectiveKey = Symbol('qahal:collective');
const participantsKey = Symbol('qahal:participants');

When('I fetch the collective {string}', async function (this: E2EWorld, id: string) {
  const data = await storageGet(`/db/collectives/${encodeURIComponent(id)}`);
  (this as unknown as Record<symbol, unknown>)[collectiveKey] = data;
});

Then('the collective has governance layer {string}', function (this: E2EWorld, layer: string) {
  const c = (this as unknown as Record<symbol, Record<string, unknown>>)[collectiveKey];
  assert.ok(c, 'no collective fetched');
  assert.strictEqual(c['governanceLayer'] ?? c['governance_layer'], layer);
});

Then('the collective is anchored with a canonical collective CID', function (this: E2EWorld) {
  const c = (this as unknown as Record<symbol, Record<string, unknown>>)[collectiveKey];
  const cid = (c['collectiveCid'] ?? c['collective_cid']) as string | null;
  assert.ok(cid && cid.startsWith('collective:'),
    `collective_cid not stamped — formation projection has not run (got: ${cid})`);
});

When('I list participants of collective {string}', async function (this: E2EWorld, id: string) {
  const data = await storageGet(`/db/collectives/${encodeURIComponent(id)}/participants`);
  const rows = Array.isArray(data) ? data : ((data['items'] ?? data['participants'] ?? []) as unknown[]);
  (this as unknown as Record<symbol, unknown>)[participantsKey] = rows;
});

Then('the participant set includes the canonical household triad', function (this: E2EWorld) {
  const rows = ((this as unknown as Record<symbol, unknown>)[participantsKey] ?? []) as Array<Record<string, unknown>>;
  for (const member of ['human-matthew-manager', 'human-jessica-spouse', 'human-james-student']) {
    assert.ok(rows.some(r => r['humanId'] === member || r['human_id'] === member || r['memberCid'] === member),
      `triad member missing from participants: ${member}`);
  }
});

Then('the participation of {string} carries a sponsor', function (this: E2EWorld, humanId: string) {
  const rows = ((this as unknown as Record<symbol, unknown>)[participantsKey] ?? []) as Array<Record<string, unknown>>;
  const row = rows.find(r => r['humanId'] === humanId || r['human_id'] === humanId);
  assert.ok(row, `no participation row for ${humanId}`);
  assert.ok(row['sponsorCid'] ?? row['sponsor_cid'], `participation of ${humanId} has no sponsor`);
});

Then('every {string} commitment with ceremony provenance is DHT-anchored', function (this: E2EWorld, action: string) {
  const rows = readStashedCommitments(this); // same Symbol stash the resilience steps fill — import/reuse it
  const ceremony = rows.filter(r => r.action === action && r.metadata?.seedGeneration === 'ceremony');
  assert.ok(ceremony.length > 0, 'no ceremony-provenance commitments found');
  for (const r of ceremony) assert.ok(r.dhtAnchorHash ?? r.dht_anchor_hash, `unanchored ceremony commitment: ${r.id}`);
});

Then('every {string} commitment with fixture provenance declares its retirement', function (this: E2EWorld, action: string) {
  const rows = readStashedCommitments(this);
  for (const r of rows.filter(x => x.action === action && x.metadata?.fixture === 'formation-output')) {
    assert.strictEqual(r.metadata?.retireAt, 'ceremony-landing', `fixture row missing retireAt: ${r.id}`);
  }
});
```

Executor note: `readStashedCommitments` must read the same `Symbol('resilience:commitmentList')` stash the existing `When('I list active {string} commitments')` step writes — export that Symbol from `resilience.steps.ts` (one-line `export`) rather than re-fetching.

- [ ] **Step 3: Dry-run binding check**

```bash
cd genesis/a2o && npx cucumber-js features/qahal/household-formation.feature --dry-run
```
Expected: all steps BIND (no undefined); scenarios listed.

- [ ] **Step 4: Extend household-reciprocity.feature** — append after the existing scenario:

```gherkin
  Scenario: The triad mesh — James is in the household's custody, both ways
    When I list active "custody-blob" commitments
    Then an active "custody-blob" commitment exists from "human-matthew-manager" to "human-james-student"
    And an active "custody-blob" commitment exists from "human-james-student" to "human-matthew-manager"
    And an active "custody-blob" commitment exists from "human-jessica-spouse" to "human-james-student"
    And an active "custody-blob" commitment exists from "human-james-student" to "human-jessica-spouse"
```

- [ ] **Step 5: The retags + persona reconciliation** — in `features/shefa/human-resilience.feature`:
  - L27 `  @wip @requires:shem` → `  @wip @requires:household-nodes` and in scenario 2 (L28+) replace every `Susan` → `Jessica` (title + steps).
  - L82 `  @wip @requires:shem` → `  @wip @requires:household-nodes` (scenario 6; Maria/Susan personas left as-is — 2-node mechanics, household-class).
  - L94 `  @wip @requires:shem` → `  @wip @requires:household-nodes` + `Susan` → `Jessica` (scenario 7).
  - L105 `  @wip @requires:shem` → `  @wip @requires:household-nodes` + `Susan` → `Jessica` (scenario 8).
  - L39 (scenario 3) and L55 (scenario 4): KEEP `@requires:shem`; add above L39: `  # household arm is household-testable; Pete's congregation reach is the shem dependency — split when shem returns`.
  - `held/features/lamad/love-map-negotiation.feature` L1: `@e2e @lamad @love-map @requires:shem` → `@e2e @lamad @love-map @requires:household-nodes` and add comment line 2: `# Adam/Eve dyad on a single doorway — household-class compute; Adam persona is shem-deployed today, so scenarios stay @wip until a household-deployable dyad or persona swap (see formation spec §9).`
  - `genesis/data/collectives/collectives.json`: in the `family-dowell` entry, replace `"Matthew, Jessica, Terrance"` description with `"Matthew, Jessica, James"`.

- [ ] **Step 6: Run scope-reconcile to return love-map to live**

```bash
python3 .claude/scripts/memory-kit/scope-reconcile.py --apply
```
Expected output includes: `git mv genesis/a2o/held/features/lamad/love-map-negotiation.feature → genesis/a2o/features/lamad/love-map-negotiation.feature` (held→live escape; household-nodes is available). Verify: `test -f genesis/a2o/features/lamad/love-map-negotiation.feature && echo MOVED`.

- [ ] **Step 7: Full a2o dry-run + commit**

```bash
cd genesis/a2o && npx cucumber-js --dry-run --tags '@e2e'
git add genesis/a2o/features/qahal/household-formation.feature genesis/a2o/steps/qahal-formation.steps.ts \
        genesis/a2o/steps/resilience.steps.ts genesis/a2o/features/resilience/household-reciprocity.feature \
        genesis/a2o/features/shefa/human-resilience.feature genesis/a2o/features/lamad/love-map-negotiation.feature \
        genesis/data/collectives/collectives.json
git commit -m "feat(a2o): household-formation spine + triad reciprocity scenarios; retag household scenarios off @requires:shem (shem≠multi-node); love-map returns to live"
```

---

### Task 10: Fixture retirement gate (LAST — requires Tasks 2-7 deployed and formation green in CI)

**Precondition (verify, do not assume):** a CI run shows `seed-results-household-formation.json` with `partial: false`, and `GET /api/v1/commitments?action=custody-blob&state=active` returns triad rows with `metadata.seedGeneration == "ceremony"`.

**Files:**
- Modify: `genesis/seeder/src/seed-commitments.ts` (remove the 4 fixture pairs; keep the M1 named pair)
- Modify: `genesis/seeder/src/__tests__/seed-commitments.spec.ts`
- Modify: `genesis/a2o/features/qahal/household-formation.feature` (drop `@wip` from the provenance scenario)

- [ ] **Step 1: Flip the test first** — change the triad-fixture test to assert `pairs` has length **2** and NO james pairs; keep the provenance-builder test (the builder stays for any future explicit fixture use).
- [ ] **Step 2: Run to verify it fails** — `pnpm exec vitest run seed-commitments` → FAIL (6 pairs).
- [ ] **Step 3: Remove the 4 fixture pairs** from `defaultM1Pairs` (delete the fixture block + the `fixture` const; keep the M1 pair and the `fixture?: string` capability).
- [ ] **Step 4: Tests pass** — `pnpm exec vitest run seed-commitments && pnpm typecheck` → PASS.
- [ ] **Step 5: Un-`@wip` the provenance scenario** in household-formation.feature (ceremony rows now exist to assert against).
- [ ] **Step 6: Commit**

```bash
git add genesis/seeder/src/seed-commitments.ts genesis/seeder/src/__tests__/seed-commitments.spec.ts genesis/a2o/features/qahal/household-formation.feature
git commit -m "feat(seeder): retire formation-output fixtures — household reciprocity now ceremony-minted only (retirement gate, spec §7)"
```

---

## Out of scope (Stage 2/3 — separate plans)

- Doorway headless persona auth (`/auth/service-login`) + doorway thin ceremony proxy + `conductor_writes` collective wrappers (spec §6 stage 2).
- Seeder service-agent `delegates-compute` standing / X-API-Key displacement (spec §6 stage 3).
- `delegates-compute` / `replicates-dwelling` HTTP-path anchoring (needs a mishpat-role HcClient; the per-conductor mishpat zome call for james-contributes-compute rides with stage 2).
- Elohim-facilitated ceremony ceiling; browser onboarding UX (`@browser-only` scenarios).
- The five validation intents beyond the reciprocity-view slice (steady-state mesh, member-offline continuity, grandma-standard recovery) — they need step vocabulary that depends on deployed formation output; planned with stage 2.

## Verification ledger (spec §11 flags → resolved)

- **V1** charter capacity: RESOLVED — free String ≤16KiB, no signature change (Task 2 uses it; Task 3 carries it).
- **V2** StewardshipGrant writability: RESOLVED — `create_stewardship_grant` is a live coordinator (Task 6 calls it); requires the founder's Human profile (ordering: after seed-conductor-identities).
- **V3** qahal manifest: RESOLVED — exists (single-file); Task 8 adds the block + the missing codegen gate.
