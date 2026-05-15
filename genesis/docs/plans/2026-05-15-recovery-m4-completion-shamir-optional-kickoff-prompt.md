# Recovery M4 Completion — Stage G Follow-up + Shamir Optionality Finalization: Kickoff Prompt

**Date:** 2026-05-15
**Status:** Brainstorm complete; ready for plan-writing
**Precondition:** Attestation Consolidation Sprint A→F merged on `dev` at `34fcf1070`; CID-decode fix at `a01e274e3` (verified by orchestrator dev #950 / elohim-holochain #1231).
**Brainstorm:** `genesis/docs/plans/2026-05-15-recovery-m4-brainstorm.md` (D1–D4 resolved; cross-sprint binding to EPR D3 = duality)
**Companion sprint audit:** `genesis/docs/plans/2026-05-15-epr-wave0-audit-results.md`

---

## Framing

This sprint **finishes Recovery Protocol Phase 2 M4 (fast-path revocation) on top of the consolidated attestation substrate** and finalizes Shamir off-chain transport as a **fully optional** cryptographic proof layer. It also retires the stale `feature/recovery-m4-fast-path-revocation` branch (916 commits behind dev — fully lapped by consolidation) and re-applies the still-relevant M4 work directly on dev using the new `Content` discriminator pattern.

The architectural commitment from the attestation sprint: **Shamir is OPTIONAL cryptographic proof ON TOP of attestation-DHT-driven social-threshold process.** Recovery must succeed via social-threshold alone (intimate quorum → qahal → global witness). Shamir, when present, adds a cryptographic proof channel; when absent, the same recovery path still works. This sprint enforces that invariant in code.

---

## Context (self-contained)

### What landed in the consolidation (read these first)

- Sprint memory: `.claude/memory/project_attestation_consolidation_sprint_state.md`
- Merge commit: `34fcf1070 merge(attestation): Stage A→G — attestation consolidation sprint`
- 22 attestation subtypes + 7 governance-action kinds emitted from JSON Schema + pillar manifests via codegen
- 22+ legacy entry types removed (mishpat 15→8; infra -2; elohim DNA vestigial -2; imagodei safe-removals -5)
- Validator floors F1/F5/F7/F8 live (F3 deferred per HDI `no-get_links`)
- 22 legacy projection tables → 2 unified + 1 derived tally; AttestationProjector signal handler; tally_projector
- 25+ legacy HTTP routes → 8 unified routes; ts-rs export + schema contract tests pass
- Stage G partial: humanness bridge (`c01fe0334`) + Shamir off-chain scaffold (`7900ae6c8`)

### What's still on legacy entries (the TODO markers)

**Imagodei zome — `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs:2925+`** — `TODO(stage-G-followup)` block stating:

> `create_recovery_request` cannot be bridged yet because `submit_intimate_witness` Gate 1 and `commit_key_rotation` revocation-floor gate both deserialize `RecoveryRequest` / `KeyRevocation` entries from the imagodei DHT via `to_app_option()`. Bridging `create_recovery_request` to elohim would leave those entries on elohim's DHT in `Content` encoding, breaking all downstream gate readers. Requires a coordinated migration: all gate readers must switch to cross-DNA `get()` + `Content` deserialization before the `create_entry` calls can move. Tracked as Stage G follow-up (G.A.2 deferred functions).

Affected functions:
- `create_recovery_request`
- `create_self_revocation`
- `create_revocation_request`
- `submit_revocation_vote`
- `IdentityFreeze`

**Storage P2P — `elohim/elohim-storage/src/p2p/shamir_transport.rs:20`** — `TODO(G.1-swarm-wiring)`:

> `ShamirShareCodec` is ready for registration but adding a new field to `ElohimStorageBehaviour` requires touching the behaviour struct + `From` impls + the swarm event loop match arm — a multi-file change deferred so G.1 lands cleanly on its own commit boundary. Registration follows the exact same pattern as `trust_protocol` (see `behaviour.rs:88` and `mod.rs:2292`). Track under the share-custody epic.

### What's stale and should be retired

- `feature/recovery-m4-fast-path-revocation` — 916 commits behind dev, 0 ahead. Last commit `5fa8d621f`. The consolidation has fully lapped it. **Recommendation:** retire the branch and re-apply the still-relevant M4 work directly on dev using the consolidated `Content` discriminator pattern. Treat the branch as a reference, not a base.
- `feature/recovery-m5-auth-portal-and-revocation-ux` — held off (M5 is the UX-side follow-on; M4 must land first).
- Original M4 kickoff: `genesis/docs/plans/2026-04-24-recovery-m4-fast-path-revocation-kickoff-prompt.md` — content remains useful for spec/floor logic but its "Post-M3 state" section is stale.

### P2P Design Gate — entity classification (mandatory)

This sprint introduces **zero new DHT entry types**, **zero new HTTP routes**, **zero new SQLite tables**, and **zero new wire-protocol message kinds beyond what already exists** in the schemas. Every entity in scope is either (a) a `content_type` discriminator added to an existing schema file, or (b) a new manifest entry that maps to an existing primitive. The classifications below pin source-of-truth.

| Entity | Category | Source of truth | Notes |
|---|---|---|---|
| `RecoveryRequest` (post-migration) | **A** — notarized | Existing `Content` entry type in elohim DNA; discriminated by `content_type: "recovery-request:<kind>"`. Schema addition only: `elohim/sdk/schemas/v1/protocol-schema.json` (extend the `content_type` enum). No new DHT type. | Reuses content_store consolidation pattern. The data lives on the elohim DHT, projected to elohim-storage's existing unified `attestations` table or sibling `recovery_flows` projection (see D1). |
| `KeyRevocation` (post-migration) | **A** — notarized | Same as RecoveryRequest: `Content` + `content_type: "key-revocation:<kind>"`. Schema extension only. | Projection target: existing `key_revocations` table proposed by EPR W2D (cross-sprint coordination). |
| `IdentityFreeze` (post-migration) | **A** — notarized | Same pattern: `Content` + `content_type: "identity-freeze"`. Schema extension only. | Read by `commit_key_rotation` freeze-floor gate. |
| `HumanityWitness` (already in M3) | **A2** — derived via link | Existing pattern from M3: link from `RecoveryRequestToHumanityWitness`. No change this sprint. | Confirm during Stage 1 audit. |
| `RecoveryFlowProjector` ack state (if D1 chooses sibling projector) | **C** — operational | Either new rows on existing `projector_acks` table (introduced by EPR Phase 4 Wave-1) or extended `attestations` projection. **No new table without explicit D1 ratification + a source-of-truth declaration in the brainstorm output.** | Rebuildable from `Content` entries on the DHT. |
| Shamir share custody envelopes | **C** — operational, ephemeral | Encrypted blobs delivered over the existing `/elohim/shamir-share/1.0.0` request-response protocol; codec at `elohim/elohim-storage/src/p2p/shamir_transport.rs`. No persistent storage of share material. | Per consolidation memory: Shamir is OPTIONAL cryptographic layer; not DHT-notarized. |
| DNA signal stream contracts (`KeyRotation`, `KeyRevocation`, `AgentPeerBinding`, `RevocationAttestation`) | **wire-protocol** — already declared | `elohim/sdk/schemas/v1/dna-signal-stream.schema.json` + `dna-signals/*.schema.json` (all 4 present). This sprint emits against existing schemas; does not change them. | Cross-references EPR W2B (consumer). |

**DNA capacity impact:** zero. Lamad stays at ~73/~100, elohim DNA gets no new entry types (only discriminator additions to `Content`), imagodei loses entries as legacy types are removed in Stage 2.

**Net new tables/routes:** zero from this sprint. Any projection-table or HTTP-route proposal that surfaces during brainstorm MUST be re-classified through this gate before being added.

### Cross-references

- `project_epr2b_recovery_m4_convergence` memory — EPR Phase 2B Batch A (consumer) ↔ Recovery M4 (producer) share `dna-signal-stream.schema.json` + `dna-signals/{agent-peer-binding,key-revocation,key-rotation,revocation-attestation}.schema.json`. **All four sub-schemas exist on dev today.** The convergence surface is in place; this sprint produces the events that flow through it.
- `project_graduated_recovery_authority` memory — intimate circle → qahal → global witness; absolute lockout = failure.
- `project_recovery_grandma_standard` memory — user never sees seeds; UX bar = "log in with help from your people."
- `project_socially_derived_security` memory — Shamir-split seed; doorway blind proxy; biometrics/2FA pluggable.
- `project_elohim_as_counsel` + `project_elohim_subagent_specialists` memories — elohim-as-defender has first-class standing to represent a human under duress; recovery flows should make space for elohim agent involvement.

---

## Sprint scope

### Stage 1 — Cross-DNA gate-reader migration (unblocks bridging)

**Goal:** Every reader that currently does `to_app_option::<RecoveryRequest>()` / `to_app_option::<KeyRevocation>()` against an imagodei-local entry must switch to cross-DNA `get()` + `Content` deserialization. This is the precondition for bridging the create-side.

Affected readers (grep for `to_app_option` + the entry types):
- `submit_intimate_witness` Gate 1 (RecoveryRequest read)
- `commit_key_rotation` revocation-floor gate (KeyRevocation read)
- Any other call sites the audit surfaces

**Pattern:** Mirror the consolidated `decode_content_entry` helper in `attestation_validator.rs` (added by `a01e274e3`). The reader fetches the cross-DNA `Content` entry and discriminates on `content_type` ("recovery-request:..." / "key-revocation:..." etc.).

**Acceptance:** All gate readers compile against a hypothetical `Content`-encoded RecoveryRequest/KeyRevocation/IdentityFreeze. No reader assumes the entry lives on the imagodei DHT.

### Stage 2 — Bridge create-side to consolidated pattern

**Goal:** Move `create_recovery_request`, `create_self_revocation`, `create_revocation_request`, `submit_revocation_vote`, and `IdentityFreeze` creation onto the consolidated `Content` entry pattern (matching `issue_attestation` / `propose_governance_action`).

For each function:
- Add a `content_type` to the protocol schema (`elohim/sdk/schemas/v1/protocol-schema.json`) — e.g. `recovery-request:<kind>`, `key-revocation:<kind>`, `identity-freeze`.
- Add the discriminator entry to the appropriate pillar manifest (likely imagodei manifest under `recovery-flows.json` or similar).
- Replace the bespoke `create_entry::<RecoveryRequestEntryTypes>(...)` with the consolidated coordinator pattern.
- Update post-commit signal emission to route through the **sibling `RecoveryFlowProjector`** (per brainstorm D1) at `elohim/elohim-storage/src/services/recovery_flow_projector.rs` — does not yet exist; primary deliverable of Stage 2. The call-site signal dispatcher gains a prefix-routing step: `attestation:*` | `governance-action:*` (non-recovery) → `AttestationProjector`; `governance-action:recovery-request` | `governance-action:key-revocation` | `key-revocation:*` | `identity-freeze` → `RecoveryFlowProjector`. The `attestation:revocation-vote` and `attestation:recovery-approval` children continue routing to AttestationProjector (correct — they land in the `attestations` accumulator). EPR W2D's `key_revocations` table writer is **co-located** in `RecoveryFlowProjector`.

**Acceptance:** Zero direct uses of `RecoveryRequestEntryTypes`, `KeyRevocationEntryTypes`, `IdentityFreezeEntryTypes` in coordinator code. All recovery primitives flow through `Content` with `content_type` discrimination. Sweettest scenarios `@recovery-m4` pass.

### Stage 3 — Producer-side signal emission (M4 ↔ EPR W2B convergence)

**Goal:** The DNA emits `KeyRotation`, `KeyRevocation`, `AgentPeerBinding`, `RevocationAttestation` signals on recovery events, matching the schemas at `elohim/sdk/schemas/v1/dna-signals/`. The EPR Phase 4-onward sprint owns the consumer side (`IntegrityNotify` pipeline — see companion prompt `2026-05-15-epr-foundation-completion-post-attestation-kickoff-prompt.md` W2B).

For each signal (all 4 contracts are pre-existing — source-of-truth at `elohim/sdk/schemas/v1/dna-signal-stream.schema.json` + `elohim/sdk/schemas/v1/dna-signals/{key-rotation,key-revocation,agent-peer-binding,revocation-attestation}.schema.json`):
- Confirm the post-commit signal emission references the payload shape declared in those existing files (no new schema introduced).
- Confirm wire fields match `elohim/sdk/schemas/v1/dna-signals/*.schema.json` (re-run `pnpm run schema:codegen:rs` if drift suspected from those existing source files).
- For `RevocationAttestation` specifically: per brainstorm D2, **duality wins** — the existing `revocation-attestation.schema.json` contract stands. The DNA emits the slim operational payload (`actionHash`, `currentVotes`, `requiredVotes`, `thresholdReached`, `attestedAt`); the consumer (EPR W2B) reads from it directly. **No `contentEnvelope` field is added** to the signal. When the consumer needs Content-envelope fields (e.g., threshold configuration), it reads from the local `governance_actions` projection table (already written by the earlier `governance-action:key-revocation` signal). The consolidation envelope and the DNA signal coexist as distinct artifacts at distinct grain.

**Cross-sprint coordination note (audit finding):** EPR W2B's `KeyRotation` IntegrityNotify arm is **already wired** at `epr_atom_service.rs:340–384` with `p2p/recovery_rotation.rs` present. The producer side from this sprint feeds an existing consumer; the cross-stack integration test below validates the full loop.

**Acceptance:** `IntegrityNotify` handlers on the EPR side receive structurally-valid payloads for all 4 signal kinds. A cross-stack integration test (M4 producer + 2B-A consumer) round-trips a KeyRevocation signal through `dna-signal-stream`.

### Stage 4 — Shamir as a fully optional layer

**Goal:** The recovery path **must** succeed without Shamir. With Shamir, the same path **also** succeeds and produces an additional cryptographic proof artifact. The two outcomes are not architecturally distinguished at the recovery-success level — only at the proof-strength level.

Tasks:

**4a. Swarm wiring** — register `ShamirShareCodec` with `ElohimStorageBehaviour`. Follow `trust_protocol` exactly (`behaviour.rs:88`, `mod.rs:2292`). Add the `shamir_share_protocol: RequestResponse<ShamirShareCodec>` field, the corresponding `From` impl mapping `request_response::Event<ShamirShareRequest, ShamirShareResponse>` to a new `ShamirShareProtocol(...)` variant (mirroring `TrustProtocol` at lines 135–139), and the swarm event loop match arm in `p2p/mod.rs`. Per brainstorm D4 (manifest-declared discovery), the swarm dials custodians by PeerId derived from the manifest-declared custodian CID list — **no gossipsub capability scan**.

**4b. ShareAssembler primitive** — locate or implement (likely under `elohim-storage/src/crypto/shamir_combine.rs`). Verify it consumes shares delivered via the swarm and reconstructs the seed material. Reads custodian identity from the manifest (per D4) — `governance-action:recovery-request` metadata or a dedicated `governance-action:shamir-custody-setup` entry committed at onboarding; resolve which carries the custody manifest during the Stage 1 audit of the imagodei zome's recovery-request creator (open sub-question per brainstorm).

**4c. Optionality enforcement** — audit the recovery completion path to confirm:
- Path A (social-threshold-only): intimate quorum → qahal → key rotation commits with valid attestations; no Shamir invocation; recovery succeeds.
- Path B (social-threshold + Shamir): same path PLUS the share-recovery channel runs in parallel; if it completes, an additional "shamir-reconstructed" attestation is emitted; if it doesn't, the recovery is still successful (Path A's commit stands).
- **There must be no code path where Shamir failure aborts an otherwise-valid recovery.**

**4d. UX bar** — confirm the recovery UX surface (Angular `recovery.service.ts` + recovery flow components) never expose seed material to the user, never require the user to choose between Path A and Path B, and gracefully degrade when share custodians are offline.

**Acceptance:** A `@recovery-shamir-optional` a2o scenario passes both with and without simulated share custodians online. The verbal contract: *"recovery works with help from your people; the cryptographic proof is icing, not foundation."*

### Stage 5 — a2o scenario @wip lift

Lift `@wip` markers on `@recovery-m4` scenarios. Likely targets in:
- `genesis/a2o/features/auth/recovery-*.feature`
- `genesis/a2o/features/recovery/revocation-emergency-quorum.feature`

Verify scenarios cover both Path A (social-only) and Path B (social + Shamir) outcomes.

---

## Decisions Resolved (2026-05-15 brainstorm)

Full rationale + alternatives in `genesis/docs/plans/2026-05-15-recovery-m4-brainstorm.md`. Summary:

1. **D1 — Sibling `RecoveryFlowProjector`.** New file at `elohim/elohim-storage/src/services/recovery_flow_projector.rs` (does not yet exist; primary deliverable of Stage 2). The projector is a state-machine controller (Open → Quorum → Effective), not an accumulator like `AttestationProjector`. Call-site dispatcher gains a prefix-routing step (see Stage 2 above). EPR W2D's `key_revocations` table writer is co-located in this module.

2. **D2 — Duality (existing `revocation-attestation.schema.json` contract stands).** No `contentEnvelope` inlined; signal carries slim operational payload (`actionHash`, `currentVotes`, `requiredVotes`, `thresholdReached`, `attestedAt`). Consumer reads slim payload + projects via `governance_actions` table when envelope fields needed. **Binds EPR D3 = duality.** No schema changes for either sprint.

3. **D3 — Retire `feature/recovery-m4-fast-path-revocation` outright.** `git log feature/recovery-m4-fast-path-revocation ^dev` produces zero output — the branch tip is the merge-base with dev; dev is a strict superset of every commit on the branch. Zero unique commits to salvage. Deletion command in "How to start" below.

4. **D4 — Manifest-declared custodian discovery.** Custodian CIDs committed to DHT at recovery-setup time (`governance-action:recovery-request` metadata or a dedicated `governance-action:shamir-custody-setup` entry — sub-question resolved during Stage 1 audit of imagodei zome's recovery-request creator). At recovery time, substrate reads DHT manifest, resolves CIDs to PeerIds via `peer_identity_bindings`, dials via `ShamirShareCodec`. **No gossipsub capability scan** (Stage 4a wiring assumption). `ShamirShareRequest::custodian_cid` at `shamir_transport.rs:75` is replay-prevention/identity-confirmation, not discovery.

### Open sub-questions (resolved during execution, not blockers)

- **D4.1** — Which `governance-action:*` kind carries the custody manifest (`recovery-request` metadata vs. dedicated `shamir-custody-setup`)? Resolve during Stage 1 imagodei-zome audit; Stage 4b's `ShareAssembler` consumes whichever is chosen.
- **D1.1** — Exact file:line of the central signal dispatcher gaining the prefix-routing step. Resolve as a Stage 2 pre-condition (likely in the HTTP/WebSocket signal handler or service orchestrator).

---

## Out of scope (handed off)

- Full social-reach nervous system (provenance back-prop, quarantine, restitution) — graph-native sprint scope per `project_social_reach_nervous_system` memory.
- M5 auth-portal convergence and revocation UX polish — owned by `feature/recovery-m5-auth-portal-and-revocation-ux`; lands after M4.
- Browser-side step defs for `@recovery-shamir-optional` if they require new Angular components — surface as a follow-up; this sprint writes the cucumber + Rust harness only.

---

## Acceptance for the sprint as a whole

1. orchestrator dev returns SUCCESS or UNSTABLE-not-regressed for 2 consecutive runs (one fresh trigger from this sprint's push).
2. elohim-holochain dev passes `attestation_coordinator` + new `recovery_flows` sweettest suites.
3. All `TODO(stage-G-followup)` markers are resolved (or moved to an explicit follow-up issue link with rationale).
4. `TODO(G.1-swarm-wiring)` is resolved.
5. `@recovery-shamir-optional` a2o scenario passes in both modes.
6. `feature/recovery-m4-fast-path-revocation` branch is either retired (recommended) or merged into a tracked archive.

---

## How to start

Brainstorm complete (`genesis/docs/plans/2026-05-15-recovery-m4-brainstorm.md`); D1–D4 resolved. Next step: dispatch via `superpowers:writing-plans` to convert Stages 1–5 into the master-plan sub-plan format (checkbox-per-task), then `superpowers:executing-plans` (or `superpowers:subagent-driven-development` for parallel Stage 1 readers + Stage 4a swarm wiring).

### Branch retirement (D3) — execute when operator approves

```bash
# Confirm zero unique commits before deletion (defensive re-check)
git log feature/recovery-m4-fast-path-revocation ^dev | head

# Delete local branch (safe — `-d` requires fully-merged ancestry)
git branch -d feature/recovery-m4-fast-path-revocation

# Delete remote branch
git push origin --delete feature/recovery-m4-fast-path-revocation
```

If `git branch -d` refuses (which would indicate the brainstorm's analysis was wrong), STOP and investigate before falling back to `-D`.
