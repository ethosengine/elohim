---
name: Attestation Consolidation Sprint — CLOSED (merged to local dev 2026-05-11)
description: Sprint result; A→G partial merged at 34fcf1070; Stage G follow-up scoped with TODO markers in code
type: project
originSessionId: 3dce0458-182f-4537-ad78-3c66bb35701b
---
Sprint merged to local dev at `34fcf1070`. Worktree removed. Stage G partially landed (humanness bridge + Shamir scaffold); remaining recovery migration deferred to follow-up sprint with `TODO(stage-G-followup)` markers in `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs:2925+` and `elohim/elohim-storage/src/p2p/shamir_transport.rs` (swarm wiring).

**Local dev tip:** `34fcf1070` (86 commits ahead of origin/dev — NOT pushed per husky guards held off)
**Worktree:** removed (`.claude/worktrees/attestation-consolidation` gone)
**Branch `worktree-attestation-consolidation`:** deleted

**Sprint result (46 commits A→G):**
- A — 22 attestation subtypes + 7 governance-action kinds (JSON Schema + pillar manifests + codegen)
- B — consolidated `content_store::issue_attestation` / `revoke_attestation` / `propose_governance_action` / `vote_on_governance_action` / queries + bridges in 3 DNAs
- C — 22+ legacy entry types removed (mishpat 15→8; infra -2; elohim DNA vestigial -2; imagodei safe-removals -5); validator floors F1/F5/F7/F8 live (F3 deferred per HDI no-get_links)
- D — 22 legacy projection tables → 2 unified + 1 derived tally; AttestationProjector signal handler; tally_projector; 1317/1317 tests pass
- E — 25+ legacy HTTP routes → 8 unified routes; ts-rs export + schema contract tests pass; storage-client-ts regenerated
- F — Angular AttestationApiService + GovernanceActionApiService; F.3-F.7 audit no-ops (app was already pillar-agnostic); 7393 tests pass
- G partial — humanness bridge (commit c01fe0334) + Shamir off-chain scaffold (commit 7900ae6c8)

**Merge conflicts resolved (additive merges on observation sprint overlap):**
- `elohim/elohim-storage/src/db/models.rs` — observation Rows + attestation/governance-action Rows coexist
- `elohim/elohim-storage/src/views.rs` — ObservationView + AttestationView/GovernanceActionView/GovernanceActionTallyView coexist

**Architectural outcome:**
- 0 new DHT entry types (Content reused with content_type discriminator)
- DNA capacity reclaimed: lamad freed ~5 slots, mishpat 15→8
- Shamir positioned as OPTIONAL cryptographic proof layer ON TOP of attestation-DHT-driven social-threshold process
- Social-threshold layer: humanness migrated; other recovery types still on legacy entries pending follow-up

**Stage G follow-up sprint scope (TODO markers in code):**
- `imagodei/zomes/imagodei/src/lib.rs:2925+` — TODO(stage-G-followup) for create_recovery_request, create_self_revocation, create_revocation_request, submit_revocation_vote, IdentityFreeze. Read-back graph coupling means each gate reader needs cross-DNA Content deserialization migration BEFORE the create-side can be bridged.
- `elohim/elohim-storage/src/p2p/shamir_transport.rs` — TODO(G.1-swarm-wiring) — registration with ElohimStorageBehaviour deferred (mirrors trust_protocol pattern at behaviour.rs:88 + mod.rs:2292)
- ShareAssembler primitive lookup (likely shamir_combine in elohim-storage/src/crypto/)
- Angular recovery.service.ts updates to use tally-projection status
- E2E integration test full mocking

**Pushable status:**
- Local dev: ready
- Origin push: held off per user instruction ("we'll shake all that out" re husky guards)
- Final build verification (cargo check on merged dev) not run in this session — recommended before push

**Disk:** 64% / 44G free
