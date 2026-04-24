I want to plan and implement Milestone M4 of the Recovery Protocol Phase 2 — **Fast-path revocation**.

## Context (self-contained)

The revised Phase 2 spec: `genesis/docs/superpowers/specs/2026-04-22-recovery-protocol-phase-2-revised-design.md`
(§2.1 lists fast-path revocation as in-scope for Phase 2.)

The M3 spec + plan just shipped:
- Spec: `genesis/docs/superpowers/specs/2026-04-24-recovery-protocol-phase-2-m3-coordinator-and-storage-design.md`
- Plan: `genesis/docs/superpowers/plans/2026-04-24-recovery-protocol-phase-2-m3-coordinator-and-storage.md`

Post-M3 state on `dev` (commit `e833ff65` or later):

**imagodei DNA has:**
- `KeyRotation` entry + validator (IntimateQuorum, CryptographicQuorum implemented; the other three layers stub-rejected — M4 does not widen this).
- `RecoveryRequest` entry with human_id + required_witness_count populated.
- `HumanityWitness` entry + `RecoveryRequestToHumanityWitness` link.
- `IdentityFreeze` entry + freeze-floor gate on `commit_key_rotation` (CryptographicQuorum exempt).
- Coordinator functions `create_recovery_request`, `commit_key_rotation`, `submit_intimate_witness`, plus the M3 helpers (`resolve_human_id_for_agent`, `count_active_emergency_contacts`, `compute_required_witness_count`, `collect_active_freezes_for_human`, `is_active_emergency_contact`, `has_existing_witness_for_request`).
- `RecoveryV2Signal` has three variants: `RecoveryRequestCreated`, `IntimateWitnessSubmitted`, `KeyRotationCommitted` — all rich payloads, serde tag `"type"`.

**Pre-existing primitives M4 should use (not reinvent):**
- `KeyRevocation` entry — already on the DHT per the Phase 2 primitive inventory (spec §4).
- `RevocationVote` entry — peer attestations supporting a revocation.
- Existing link types: `HumanToKeyRevocation`, `RevokedKeyToRevocation`, `PendingRevocations`, `EffectiveRevocations`, `RevocationToVote`, `StewardToRevocationVote`, `IdToRevocationVote`, `IdToKeyRevocation` — all present in integrity LinkTypes (imagodei_integrity/src/lib.rs). Check them first before adding new ones.

**elohim-storage has:**
- `recovery_requests`, `key_rotations`, `recovery_witnesses` projection tables + schema-contract-tested views.
- `recovery.invitation` gossipsub topic on the EPR-2C swarm; `RecoveryInvitation` MessagePack wire contract.
- `handle_recovery_v2_signal` projection dispatcher + `recovery_invitation_from_signal` publish-intent extractor.

**Not yet in place:**
- Revocation-side coordinator functions (self, emergency-contact, specialist-attested).
- Revocation signal variants + storage projection (`key_revocations` + optional `revocation_votes` table).
- `recovery.revocation` gossipsub topic (if needed) or reuse of `recovery.invitation`.
- Anti-rotation interaction: does a pending `KeyRevocation` for the current key block `commit_key_rotation`? Per intuition yes — resolve in brainstorm.
- a2o scenarios tagged `@recovery-m4`.

## M4 scope (per Phase 2 spec §2.1, as refined by brainstorm)

M4 is the "kill a compromised key quickly" milestone. Deliverables:

1. **Self-revocation coordinator** — the human with a still-valid agent key voluntarily revokes a different (compromised) key they control. Single-cell authority, no quorum, no witnesses required.

2. **Emergency-contact revocation coordinator (quorum path)** — when the human's key is captured and they cannot sign themselves, enough emergency contacts (thresholded like intimate-quorum) can commit a `KeyRevocation` + `RevocationVote` entries that, when threshold is reached, flip the revocation to `effective`. Use the M3 threshold formula `max(2, ceil(M/2)+1)` or a variant — brainstorm picks.

3. **Attestation-stub for specialist revocation (M5 seam)** — the `KeyRevocation` variant that carries an `anomaly_attestation` discriminator lands in integrity as an accepted shape, with coordinator-level rejection of "specialist" authority until M5 wires the elohim defender path. Consistent with how M3 left `CommunityConsensus`/`GovernanceAct`/`NetworkWitness` as validator-accepted / coordinator-stub.

4. **Rotation-vs-revocation interaction** — if `PendingRevocations` or `EffectiveRevocations` anchor points to a revocation for the rotating agent's current key, `commit_key_rotation` blocks with a descriptive error. Mirror the M3 freeze-floor gate pattern (pre-commit, in the coordinator, using must_get_entry — never `get_links` in a validator).

5. **Storage projection** — `key_revocations` table (and optionally `revocation_votes` if the three-paths design splits them). **Source of truth: DHT** (imagodei `KeyRevocation` + `RevocationVote` entries). The table is a read-optimized projection rebuildable via signal replay, not a canonical record. Schema-first: JSON schema → hand-written Rust view → schema-contract test → ts-rs codegen → distribution to all three generated-ts locations.

6. **Signal variants** — extend `RecoveryV2Signal` with `KeyRevocationRequested`, `RevocationVoteSubmitted`, `KeyRevocationEffective` (or similar — brainstorm names). Rich payloads. serde tag stays `"type"` across DNA and storage mirror.

7. **Mesh substrate decision** — reuse `recovery.invitation` topic (with payload discriminator) OR open a second topic `recovery.revocation`. Brainstorm picks. Whatever ships, carry a MessagePack wire contract consistent with EPR-2C convention (NOT CBOR — that was a design drift in M3; see memory `feedback_swarm_composition_fresh_tree_build`).

8. **Tests** — `elohim_sweettest` crate: scenarios for self-revocation happy path, quorum-revocation threshold met, quorum-revocation threshold not met, rotation-blocked-by-pending-revocation. Plus 1-2 a2o features tagged `@recovery-m4`.

**Out of scope for M4 (deferred):**
- Specialist elohim attack detection (M5 — the elohim defender pattern).
- Account/login layer graduation (the OAuth-pattern supersession — memory `project_peer_native_account_canonical_surface`; M5 and its own brainstorm).
- Hosted-cell bootstrap, browser session handoff (M5).
- Hashcash / rate limiting (M5).
- CommunityConsensus, GovernanceAct, NetworkWitness variant impls (Phase 2b+).
- Fast-path revocation UX in elohim-app (M5).
- Anti-lockout audit suite (M6).

## Gaps worth resolving during brainstorm

Run **`p2p-design-gate`** early — this is a new set of DHT-touching entities (KeyRevocation, RevocationVote, new link traversals). Classification per-entity before any approach is proposed.

Then resolve:

1. **Threshold for emergency-contact revocation** — same as M3 (max(2, ceil(M/2)+1))? Or lower (revocation is less destructive than rotation — it only removes authority)? Or higher (revocation is destructive to the human's continued access)? Make a defensible decision.

2. **What makes it "fast" path?** Compared to full graduated recovery, fast-path skips KeyStewardship provisioning. Nothing else. Document this precisely — otherwise reviewers will ask.

3. **Revocation vs rotation ordering** — can a human invoke `commit_key_rotation` while a `KeyRevocation` is pending on their current key? Intuition: no. But the blocking gate needs to be precise about which keys are affected (the key being rotated *from*, not *to*).

4. **Anchor convention** — key off `human_id` (consistent with M3) or off the revoked pubkey (since the revocation's primary subject is a specific key)? Both? Decision log like M3 §12.

5. **Signal shape** — one variant per event (Requested / Voted / Effective) or a single rich variant that carries state? M3 used per-event variants; consider parity.

6. **Topic sharding** — reuse `recovery.invitation` with a typed payload union, or dedicated `recovery.revocation`? Brainstorm: is the subscriber set the same (elohim specialists watching for their represented humans)?

7. **Subagent split** — DNA + Storage + Tests (same as M3) or different? Given revocation is smaller in scope than M3 recovery rotation, consider a single DNA+Storage dispatch with a follow-up Tests dispatch.

## How to run this session

1. Start with `/superpowers:brainstorming` on M4 using the scope above. **Invoke `p2p-design-gate` early** per the mandatory CLAUDE.md rule.

2. Once the approach is locked, invoke `/superpowers:writing-plans` to produce:
   - Design spec: `genesis/docs/superpowers/specs/2026-04-<DD>-recovery-protocol-phase-2-m4-fast-path-revocation-design.md`
   - Plan: `genesis/docs/superpowers/plans/2026-04-<DD>-recovery-protocol-phase-2-m4-fast-path-revocation.md`

3. Execute via `/superpowers:subagent-driven-development`.

4. Push with husky when clean. Do NOT bypass with `HUSKY=0`.

## Constraints & conventions

- Working branch: confirm at session start. Cut `feature/recovery-m4-fast-path-revocation` from current `dev` HEAD. Do NOT continue on any stale feature branch.
- Build commands (unchanged from M3):
  - imagodei DNA: `cd /projects/elohim/elohim/holochain/dna/imagodei && just check && just pack`.
  - Storage: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release`.
  - Sweettest: `cd /projects/elohim/elohim/holochain/tests/sweettest && CARGO_TARGET_DIR=target/native-tests cargo test` (requires nix shell with datachannel C deps — local Eclipse Che can't run it; CI does).
- **Fresh-tree cargo build on elohim-storage is MANDATORY before committing any swarm/behaviour edit** — per memory `feedback_swarm_composition_fresh_tree_build`. `just check` on a DNA worktree does NOT verify elohim-storage crate-level references resolve.
- Schema-first IoC: JSON schemas FIRST for any new storage columns or signal payloads. Rust and TS comply.
- HDI validators cannot use `get_links`; coordinator pre-commit gates are authoritative for cross-entity checks (see `project_hdi_no_get_links_in_validators`).
- Subagent dispatches MUST include explicit scope guardrails per `feedback_subagent_scope_guardrails`: forbid `git revert`/`reset` on pre-existing commits, forbid out-of-scope file modifications, require BLOCKED report on scope conflicts. Orchestrator scans SHA range post-dispatch.
- Pre-push hook runs fmt + clippy + tests. Run locally BEFORE `git push` — failing on the remote after the fact creates cleanup-debt commits.
- If another agent is concurrently editing the same file, coordinate through the file-sync-reminder hooks (treat as high-coordination surface).

## Memories worth checking on start

- `project_graduated_recovery_authority` — the five-layer authority stack.
- `project_elohim_as_counsel` — informs the specialist-revocation M5 seam.
- `project_socially_derived_security` — why the emergency-contact path exists.
- `project_bootstrap_to_elohim_security_gradient` — Stage 1 (M4 structural) vs Stage 2 (M5 elohim) — M4 ships Stage 1.
- `project_hdi_no_get_links_in_validators` — freeze-floor/revocation-floor gates live in the coordinator.
- `feedback_schema_first_ioc` — schemas before code.
- `feedback_subagent_scope_guardrails` — explicit forbidding language in dispatch prompts.
- `feedback_swarm_composition_fresh_tree_build` — fresh cargo build on the main storage crate before commit if touching swarm composition.
- `feedback_session_orchestrate_vs_implement` — if resuming a previous session, classify the thread mode first.
- `feedback_shift_measure_jenkins` — sweettest doesn't run locally in Che; CI is the measure.
- `project_peer_native_account_canonical_surface` — the graduation frame. M4 should stay initiator-agnostic (don't couple revocation paths to either hosted-doorway login OR peer-native login). The account-layer sprint comes AFTER M4.

Go.
