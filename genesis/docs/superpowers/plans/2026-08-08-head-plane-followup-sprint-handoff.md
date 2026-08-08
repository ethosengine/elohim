---
id: head-plane-followup-sprint-handoff
title: Head-Plane Follow-Up Sprint — flips, measurement windows, and the four strategic legs
status: Draft
class: protocol-canonical
topic: [dataplane, head-plane, trust-gradient, rollout, quiesce, conductor-fork, upstream-pr]
domain: D5
sprint: head-plane-followup
cites:
  - head-plane-trust-gradient-program-plan | The executed program this sprint continues — its T5/T10 rollout legs and §6 measurement transfer here; decompose fires when F1/F2/F6 conclude it | sha256:aee96a34080d4efa | path: genesis/docs/superpowers/plans/2026-08-08-head-plane-trust-gradient-program-plan.md
  - conductor-call-deadline-capability-spike | Carries the upstream PR draft (F4) and the §6.4 fork-deploy decision that F2 queue_wait window feeds | sha256:6db22ee8fcdb985c | path: genesis/docs/superpowers/specs/2026-08-08-conductor-call-deadline-capability-spike.md
  - genesis/data/timeline/backlog/storage-client-pin-skew-fork-app-interface.md
  - genesis/data/timeline/backlog/content-store-integrity-link-validation-gap.md
---

# Head-Plane Follow-Up Sprint Handoff

**Context (evidence, 2026-08-08):** the head-plane trust-gradient program's
implementation waves landed on dev (f19aa74b0..ecc840ebd, 22 commits) and the
rollout shift closed DONE at stability 2/2: DNA pipeline #1394 SUCCESS, edge
#1326 SUCCESS (deploy + coordinator hot-swap). Functional verification on
alpha: 246 batch calls / 17,392 ids resolved through the new batch externs,
zero unknown-function fallbacks (hot-swap landed fleet-wide), AIMD converged
LAN pods to ceiling 128 and WAN-side pods to floor 8 (heterogeneity preserved),
fleet p90 queue_wait ≈ 683ms. Genesis/app pipelines UNSTABLE in the same class
as their pre-push baselines (floor held). Everything below is the remaining
program scope plus the four strategic legs deliberately deferred at the
push decision ("take risks, harvest cycle savings" — operator, 2026-08-08).

## Tasks

| ID | Task | Tier / owner | Gate |
|----|------|--------------|------|
| F1 | **T5 flip — requester digest leg.** Evidence-gather: responder deployed fleet-wide by edge #1326 (this wave); confirm per-peer responder behavior on live traffic (inventory replies carrying `in_sync` when probed), then flip the requester config flag (landed 934e41cf8, default off) and watch one sync cycle. | Orchestrator + operator flag gate | Responder confirm evidence BEFORE flip; flip is config-only, revertible |
| F2 | **Quiesce §6 measurement + fork-deploy decision.** On the NEXT edge deploy after F1, record the fleet-quiesce-gate window and compare against the program plan §6 prediction (~90 min cadence-floor vs ~150 min baseline); verify PTxnGuard rate stays FLAT while quiesce falls; pull `elohim_head_batch_queue_wait_ms` p50/p90 per node. Feed the numbers into the T13 spike memo §6.4 three-outcome decision (fork patch deploy: yes / defer / drop). | Orchestrator measures; **operator decides** | The decision memo's own probe; do not deploy the fork patch before this window |
| F3 | **Client-pin skew reconciliation.** Lift the `shift_objective` from `backlog/storage-client-pin-skew-fork-app-interface.md`. Blocks any fleet use of the fork's per-call deadline leg. | rust-architect shift (Sonnet legwork) | Full `cargo test` (dependency-bump rule), admin-seam call-site audit |
| F4 | **Upstream PR** (T13 instrument 2). Operator provides GitHub identity + fork; author sweettest scenarios against `upstream/develop`; open the PR from the draft in the spike spec (§ PR draft). Runs on its own clock. | **Operator-gated**; Opus assists scenario port | PR draft already contribution-grade; anchors verified byte-identical on develop |
| F5 | **Integrity link validation** (hash-moving lineage change). Lift the `shift_objective` from `backlog/content-store-integrity-link-validation-gap.md`; plan the genesis-pair migration sequencing explicitly (ALLOW_DNA_REINSTALL calculus). | rust-architect + operator gate | Poison-link sweettest scenarios refuse both consequences at validation time |
| F6 | **T10 Simulacra activation** (after F1). ManifestStakesResolver + `ELOHIM_NETWORK_STAKES` + per-fixture seeder reach opt-in. Trap from the program plan stands: per-fixture opt-in, AFTER the digest baseline is stable — a blanket seed flip silently resizes the corpus L1/L2 are measured against. | Opus + operator gate | `@requires:alpha-cluster-6peer` activation leg; floor property tests already pin Simulacra-never-cheapens |
| F7 | **T19 dependency note.** Memo reads (`ELOHIM_TRUST_MEMO_READS`, default off) stay off until the SDK-promise program's standing_projector T19 wires FeedbackSignal→`invalidate_verification_memo_for_subject` (seam landed, commit 51aff16ad/ecc840ebd). Cross-program dependency — track, don't build here. | — | Flag flips only with T19's invalidation live |

## Housekeeping (small, capture-complete)

- **notary-authority habit:** edge #1326 SUCCESS ran the Dataplane Validation tag set that hosts the `@concern:notary-authority` scenario — verify the scenario executed (not skipped) in that build and flip the habit red→green with build #1326 as evidence if so.
- **Agent-prompt hardening:** subagents parked on self-backgrounded cargo runs 4× this sprint despite foreground instructions — consider a standing line in dispatch templates (never background the gate run; wait synchronously).
- **MemPalace re-mine:** overdue (index behind front-link since 2026-08-02); this sprint added substantial canonical surface (trust seam, batch contract, registry rows). Librarian dispatch.
- **Program-plan decompose:** the trust-gradient program plan decomposes to zero residue (verify-gate via ci-investigator) when F1/F2/F6 conclude it — the BACK fire point belongs to THIS follow-up sprint's close, not the last one.

## First parallel wave suggestion

{F1} and {F3} and {F4-prep} are mutually disjoint. F2 rides F1's next deploy.
F5 and F6 are operator-sequenced. Same handoff contract as the program plan:
tiered legwork, commit-only workers, orchestrator reviews every diff, one push
per batch.
