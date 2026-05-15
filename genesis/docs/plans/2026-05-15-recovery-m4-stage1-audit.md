# Recovery M4 — Stage 1 Audit: Cross-DNA Gate Readers

**Sprint:** Recovery M4 Completion + Shamir Optional
**Stage:** 1 (audit)
**Task:** T1 — Audit imagodei zome for legacy entry-type readers
**Date:** 2026-05-15
**Files audited (read-only):**
- `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs`
- `elohim/holochain/dna/imagodei/zomes/imagodei/src/submit_specialist_revocation.rs`

## Purpose

Before Stage 2 can bridge the create-side of `RecoveryRequest`, `KeyRevocation`, and
`IdentityFreeze` onto the consolidated `Content` discriminator pattern
(`governance-action:recovery-request`, `governance-action:key-revocation`,
`governance-action:identity-freeze`), every gate-reader that currently decodes one of
those three entry types from the imagodei DHT via `to_app_option()` must be cataloged
and migrated to a cross-DNA `Content` decode. If the create-side moves before the
read-side migrates, every gate breaks silently because the underlying entry type at
the resolved `ActionHash` will be a `Content` envelope, not the bespoke legacy struct.

This audit is the catalog. Stages 3–5 use it as their work list.

## Method

1. Verbatim grep (Step 1):
   ```bash
   grep -n "to_app_option::<RecoveryRequest>|to_app_option::<KeyRevocation>|to_app_option::<IdentityFreeze>|::<RecoveryRequest>()|::<KeyRevocation>()|::<IdentityFreeze>()" \
     elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs \
     elohim/holochain/dna/imagodei/zomes/imagodei/src/submit_specialist_revocation.rs
   ```
   Found only a documentation comment at `lib.rs:2912`. All actual reader sites use
   bare `.to_app_option()` with type inference driven by the let-binding's type
   annotation (`let foo: T = …` or `let Some(foo): Option<T> = …`).

2. Broader sweep across the imagodei zome (operator-suggested):
   ```bash
   grep -rn "to_app_option" elohim/holochain/dna/imagodei/zomes/imagodei/src/ \
     | grep -iE "Recovery|Revocation|Freeze"
   ```
   Plus a targeted search for the typed let-bindings that drive type inference:
   ```bash
   grep -n "Option<IdentityFreeze>|Option<KeyRevocation>|Option<RecoveryRequest>|: KeyRevocation =|: IdentityFreeze =|: RecoveryRequest =" \
     elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs \
     elohim/holochain/dna/imagodei/zomes/imagodei/src/submit_specialist_revocation.rs
   ```

3. Read each site's surrounding ~20 lines to capture the gate's semantic role and
   determine which envelope fields the post-migration code must extract from the
   `Content.metadata` JSON.

`submit_specialist_revocation.rs:151` is a `Human` decode (not legacy recovery type),
out of scope.

## Reader site catalog

Five reader sites decode one of the three legacy types. All five live in
`elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs`. None live in
`submit_specialist_revocation.rs`.

| # | Function | File:line | Legacy entry type | Gate semantic role | Envelope fields needed after migration |
|---|---|---|---|---|---|
| 1 | `submit_revocation_vote` | `lib.rs:2469` | `KeyRevocation` | Load the pending revocation under vote — Gate A (existence: anchor → record), Gate B (`trigger_type == "steward_vote"`), Gate C (`!threshold_reached`), Gate D (steward is active emergency contact for `human_id`). Subsequent block also reads `required_votes`, `current_votes`, `revoked_key`, `id`, `human_id` for vote accounting + the post-threshold `KeyRevocationEffective` signal payload. | `human_id`, `revoked_key`, `id`, `trigger_type`, `required_votes`, `current_votes`, `threshold_reached`, `effective_at`, `created_at`, `updated_at`. All currently in the bespoke `KeyRevocation` struct; all must be readable from `Content.metadata` JSON for `governance-action:key-revocation` (Stage 2 schema). |
| 2 | `submit_revocation_vote` (pending-link cleanup loop) | `lib.rs:2614` | `KeyRevocation` | Walks `PendingRevocations` global anchor links to find the link whose target entry's `id` matches `input.revocation_id`, then `delete_link` to migrate it out of the pending set after the threshold-meeting vote. Identifier-only read: `rev.id`. | `id` only. (Cheaper post-migration: the link tag can carry `revocation_id` bytes the same way Stage G.A.2 made `RecoveryRequestToHumanityWitness` tag-based — flagged as a Stage 5 simplification candidate, but in-scope work is just the `Content.metadata` decode.) |
| 3 | `collect_active_freezes_for_human` (called from `commit_key_rotation` M3 freeze-floor gate at lib.rs:2722–2733) | `lib.rs:2696` | `IdentityFreeze` | Traverses `HumanToFreeze` anchor links for the human, decodes each `IdentityFreeze`, keeps `is_active == true`. Result feeds `check_freeze_floor_rules(authority, human_id, &freeze_refs)` which inspects freeze fields to decide whether the key rotation is allowed under the claimed `RecoveryAuthority`. | At minimum: `is_active`, plus whatever fields `check_freeze_floor_rules` reads (likely `reason`, `frozen_at`, `frozen_until`, `frozen_by`, `human_id`, scope/level). The full `IdentityFreeze` struct is currently returned and shared by reference — post-migration the helper should return a thin `FreezeView` struct (or `Content.metadata`-parsed projection) so call sites are not coupled to wire shape. |
| 4 | `commit_key_rotation` revocation-floor gate | `lib.rs:2769` | `KeyRevocation` | Walks `PendingRevocations` + `EffectiveRevocations` global anchors. For each linked revocation, decodes the entry and rejects the rotation if `rev.revoked_key == rotating_from_str`. Error message also includes `rev.id` and the pending/effective `status` for operator-readable diagnostics. | `revoked_key`, `id`. Two-field read; cleanly maps to `Content.metadata` JSON. |
| 5 | `submit_intimate_witness` Gate 1 | `lib.rs:2942` | `RecoveryRequest` | Gate 1 (existence: `get(input.recovery_request_hash)` must resolve to a `RecoveryRequest`), Gate 1b (must have a populated `human_id`). The decoded `human_id` is then used by Gates 2 (`is_active_emergency_contact(&human_id, &authorizer_human_id)`) and 3 (tag-based dedupe is keyed on the authorizer — but `human_id` flows into the signal payload + the synthesised `HumanityWitness` for elohim DNA). | `human_id` is the load-bearing field; the post-Gate-1 flow also reads other envelope context implicitly through the bridge call. Stage 3 must decode `Content` at `recovery_request_hash` and extract `metadata.human_id`. |

**Count: 5 reader sites.**

## Out-of-scope sites found and rejected

| Site | Type decoded | Why excluded |
|---|---|---|
| `submit_specialist_revocation.rs:151` | `Human` | Not one of the three legacy recovery types. Resolves the target `Human.id` from `input.human_action_hash`. Stays as-is. |
| `lib.rs:1099–1105` | `AgentProgress` | Out-of-scope domain (learning progress). |
| `lib.rs:1206–1212`, `1281`, `1329` | `ContentMastery` | Out-of-scope domain. |
| `lib.rs:1557`, `1730`, `1761` | `ContributorPresence` | Out-of-scope domain. |
| `lib.rs:1952–1958`, `233`, `369`, `403` | `Human` | Identity primitive; not in migration scope. |
| `lib.rs:240`, `991`, `3094`, `3180`, `3443`, `3552`, `3598`, `agent_peer_binding.rs:*`, `sign_for_agent.rs:143` | `Agent`, `AgentPeerBinding`, `RecoveryHint`, `AgentRetirement`, `RelationshipRenewal`, etc. | Other identity-domain primitives; not part of the recovery-primitive migration. |
| `lib.rs:249`, `563`, `2857–2864`, `1981–1988` | `HumanRelationship` | Relationship primitive used by recovery gates but not itself a recovery primitive being migrated. |
| `lib.rs:192`, `220`, `portal_host.rs:78,171` | `PortalHost` | Operational entry, out of scope. |
| `lib.rs:2519–2525` | `RevocationVote` | Vote entries are a separate primitive; their migration is not in this sprint (see Plan §non-goals: "Revocation vote primitives… stay on imagodei DNA"). |
| `stewardship.rs:*` | `StewardshipGrant`, `DevicePolicy`, `StewardshipAppeal`, `ActivityLog` | Stewardship domain; out of scope. |

## Cross-cutting observations for Stages 2–5

1. **All five sites use bare `.to_app_option()` with let-binding type inference.** None
   use the turbofish form `::<T>()`. Stage 2's cross-DNA `Content` decoder helper
   should expose an equally lightweight signature so the migration is mechanical
   (e.g., `decode_content_metadata::<T: DeserializeOwned>(record: &Record) -> ExternResult<T>`),
   matching the existing call-site shape.

2. **The TODO at `lib.rs:2925–2932` is the canonical statement of why this audit exists.**
   The Stage-G follow-up note explicitly says: "create_recovery_request cannot be
   bridged yet because submit_intimate_witness Gate 1 and commit_key_rotation
   revocation-floor gate both deserialize RecoveryRequest / KeyRevocation entries from
   the imagodei DHT via `to_app_option()`. Bridging create_recovery_request to elohim
   would leave those entries on elohim's DHT in Content encoding, breaking all
   downstream gate readers." That comment must be removed in Task 15 after this
   migration lands.

3. **One site (#2, the pending-link cleanup loop) reads only `id`.** It is a candidate
   for a Stage G.A.2–style tag-based simplification (move `revocation_id` into the
   `PendingRevocations` link tag, eliminate the entry decode entirely). Flagged here
   but **not required** by Task 5 — Task 5's mandate is the `Content.metadata` decode
   path; the tag-based simplification is a Stage-G-followup follow-up.

4. **Site #3 (`collect_active_freezes_for_human`) returns `Vec<IdentityFreeze>`** which
   then feeds `check_freeze_floor_rules` taking `&[&IdentityFreeze]`. Migrating this
   site cleanly will require either (a) keeping the `IdentityFreeze` Rust struct as a
   pure-Rust view type (decoupled from `EntryTypes`) for in-zome data flow, or (b)
   refactoring `check_freeze_floor_rules` to take a thinner projection. Option (a) is
   smaller-blast-radius and matches the audit's read-only scope; recommended for Task 4.

5. **All five sites read fields that are already present in the existing
   `RecoveryRequest`, `KeyRevocation`, `IdentityFreeze` structs.** No new fields need to
   be added to the `Content.metadata` JSON beyond a faithful camelCase serialization
   of the legacy struct. Stage 2's schemas (`governance-action:recovery-request`,
   `governance-action:key-revocation`, `governance-action:identity-freeze`) should
   mirror the existing struct shape one-for-one. This keeps Stage 3–5's migration
   purely mechanical: replace `to_app_option()` with the cross-DNA decoder helper,
   keep the gate logic verbatim.

## D4.1 sub-question — Shamir custody manifest discriminator

**Decision:** The Shamir custodian manifest is carried by a **dedicated
`governance-action:shamir-custody-setup`** entry, committed at onboarding/setup time as
a ceremony distinct from the recovery-request opener (`governance-action:recovery-request`).

**Reasoning:**

1. **Slim recovery-request bodies.** The recovery-request opener fires every time a
   recovery flow begins; it is hot-path. Embedding the full custodian manifest
   (custodian CIDs, share thresholds, custody-attestation hashes) inflates every
   recovery-request entry whether or not Shamir is in play. Keeping the custody
   manifest in a separate entry lets the recovery-request body stay focused on
   in-flight flow state (initiator, authority claim, opened-at, expected-witnesses).

2. **Custody is set-up-once, used-many.** A human establishes Shamir custodians once
   at onboarding (or rarely, when rotating custodians); recovery flows may open many
   times over a human's lifetime against the same custody manifest. The lifecycle
   mismatch maps cleanly onto separate entries: one durable, slowly-changing custody
   manifest; many transient recovery-request entries that reference it by hash.

3. **Independent revision.** The custody manifest is the kind of artifact a human
   revises out-of-band from a recovery event — adding a new custodian, retiring an
   old one, rotating a custodian's CID after their own key rotation. Doing those
   revisions through `update_entry` on the `governance-action:shamir-custody-setup`
   entry is the natural pattern. Embedding the manifest inside a recovery-request
   would force custody revisions to either reopen recovery flows (semantically wrong)
   or duplicate the manifest across every flow (storage waste + divergence risk).

4. **Optionality.** Per the sprint plan §non-goals, recovery must succeed without
   Shamir. If the custody manifest were embedded in `recovery-request`, every
   recovery-request schema would carry an optional Shamir block — encouraging client
   code to inspect it conditionally on every flow. A separate
   `governance-action:shamir-custody-setup` entry makes the absence of Shamir
   *literally absent from the DHT* for that human, which is the cleanest possible
   expression of "this human has not opted into Shamir." Stage 4b's `ShareAssembler`
   simply queries for the custody-setup entry; if none exists, Shamir is not
   attempted; if one exists, the manifest drives the dial list.

5. **Matches D4's manifest-declared discovery decision.** D4 (brainstorm §D4) settled
   on manifest-declared custodian discovery: custodians are committed to the DHT at
   setup time, dialed via `ShamirShareCodec` at recovery time. The setup-time commit
   is exactly a `governance-action:shamir-custody-setup` entry; the recovery-time
   dial reads it (potentially through `peer_identity_bindings` to resolve CID → PeerId)
   and uses the `custodian_cid` field of `ShamirShareRequest` for replay-prevention.

6. **Stage 4a swarm-wiring simplicity.** Stage 4a needs a deterministic list of
   custodian PeerIds to dial. A standalone custody-setup entry under a stable
   `HumanToShamirCustody` (or equivalent) link lets the recovery agent fetch the
   manifest by `human_id` alone — no need to first locate a specific recovery-request
   entry, decode it, and extract embedded custody fields.

**Therefore the D4.1 decision sentence is:** *The Shamir custodian manifest is carried by
a dedicated `governance-action:shamir-custody-setup` entry committed at onboarding
time, separate from `governance-action:recovery-request` — keeping recovery-request
bodies slim, allowing the custody manifest to be revised independently, and making
Shamir absence/presence a literal DHT-level fact rather than a per-flow flag.*

This unblocks Task 22 (custody manifest discriminator + `create_shamir_custody_setup`
extern) and Stage 4b's `ShareAssembler` (Task 21) which can rely on a stable
discriminator for custody-manifest lookup.

**References:**
- Brainstorm D4 — `genesis/docs/plans/2026-05-15-recovery-m4-brainstorm.md` §136–170
- Brainstorm D4 sub-question (recommended resolution moment was Stage 1 gate-reader
  audit) — `genesis/docs/plans/2026-05-15-recovery-m4-brainstorm.md` §187
- Sprint plan Tasks 21, 22 — `genesis/docs/plans/2026-05-15-recovery-m4-completion-shamir-optional-plan.md`

## Sign-off

This audit is read-only and complete. Stage 2 (cross-DNA `Content` decoder helper)
proceeds against the five reader sites enumerated above. Stages 3, 4, 5 migrate them
in the order:
- Stage 3 / Task 3 → site #5 (`submit_intimate_witness` Gate 1)
- Stage 4 / Task 4 → sites #3 + #4 (`commit_key_rotation` freeze-floor and
  revocation-floor gates)
- Stage 5 / Task 5 → sites #1 + #2 (`submit_revocation_vote`)
