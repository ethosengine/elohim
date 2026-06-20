# Deliverable handoff — Phase-2a attestation cleanup arc (2026-06-20)

**Branch:** `feat/frontend-eyes-sprint` (SHARED — a concurrent doorway/EAE session commits here too).
**Scope:** complete the Phase-2a attestation consolidation's loose ends — the dropped-table-still-consumed
bugs, the codegen gap, the planning-doc drift, and the frontend read surface. Commit-only; integrator owns
the dev merge.

## What landed (13 commits, `ff4bbca46`..`e9a920b5d`) — verified

### Storage (the real runtime bugs) — fixed + tested
- `e413523ff` — **rebuilt the prerequisite access gate** on `PREREQUISITE` graph edges + `content_mastery`,
  plumbed a GraphEngine into all 3 transports (HTTP/iroh/libp2p), migrated off the dropped
  `content_attestations`. `/code-review`-verified (deny+allow access-control test). The old gate was dead
  (zero writers); this is the first functional prereq enforcement.
- `ce3ede44b` — **`gate_decision_attestations`** → unified `attestation:gate-decision` (writer + readers
  repointed; round-trip test, 14 fields preserved; 39 tests pass; grep→0).
- `b281e0900` — **`statement_votes`** → unified `attestation:statement-vote` (latest-wins preserved; recount
  side-effect kept; 5 round-trip tests + **full `--lib` 1695 pass**; grep→0).
- `b78908924` — **codegen `$ref` fix**: lamad's 4 `attestation:*` subtypes now register/mint (grep-proven).

### Frontend — consolidation read surface completed
- `e9a920b5d` — **repointed the trust-badge read** onto the unified surface (`AttestationApiService.listBySubject`,
  `attestation:content-quality`), retired the legacy `ContentAttestationApiService`/`CONTENT_ATTESTATION`
  token/`IContentAttestation`, the elohim-app twin, and the `ContentAttestationView`/`CreateAttestationInputView`
  ts-rs types (+ regen). **+164/−1336.** Angular `tsc --noEmit` GREEN (lamad + elohim-app + elohim-service lib);
  ts-rs regen 373 pass; 0 live refs to retired symbols.

### Coherence + strategy
- `9f84c0003`, `a92efcd9a` — corrected the stale planning docs (consolidation design → Implemented; wave-0
  Stage-A LANDED banner; residual-tails Task-2 superseded; gate-challenge GateDecisionAttestation banner).
- Backlog: `b95aa3c7f` (content_attestations bug, now fixed), `241fbf1fe` (gate fail-open hardening),
  `548c6b0bd`/`9e00b2587` (the cleanup-surface catalog + results), `b0306a157` (the **mastery-credential epic
  seed** + current-state map).

## Verification summary
Storage `--lib` 1695 pass · per-migration round-trip tests · gate deny+allow access-control test · Angular
`tsc` GREEN · all `grep <dropped-table/retired-symbol>` → 0 live refs · `/code-review` on the gate rebuild.

## ⚠ Branch-level merge readiness (the one thing gating a clean dev push)
The SHARED branch does NOT cleanly compile `--tests` (the workspace test build) due to **concurrent-session
in-flight work, NOT this arc**:
- `tests/provenance_gate_integration.rs` — `CreateContentInput.dht_anchor_hash` mismatch (their provenance-gate
  commits `e2098657e`/`0c7bafca2`/`417707000`, mid-development).
- `gate_client`/EAE crate breakage (their crate, `E0433/E0425` in the working tree at session start).
These block only the full `--tests` workspace compile — `--lib`, per-binary tests, and the Angular build (this
arc's verification) all pass. **Before a clean `dev` merge, the concurrent session's work must settle** (or the
merge waits for their green). This is a coordination point for the integrator, not a defect in this arc.

## Loose ends (tracked, operator/owner-gated — none block the storage/frontend correctness)
- **DNA reinstall** (the codegen `$ref` moved the integrity-zome hash) — `ALLOW_DNA_REINSTALL` on adam+matthew
  together; + the deferred mint sweettest in CI.
- **Gate fail-mode policy** (`241fbf1fe`) — the prereq gate fails-open if the graph subsystem fails to init;
  operator decides open-and-loud / fail-closed / refuse-startup.
- **Orphaned `create-attestation-input.schema.json`** (protocol-truth layer, no Rust struct now) — flagged in
  `lamad.rs` for rust-architect; not deleted.
- **Mastery-credential epic** (`genesis/data/timeline/backlog/mastery-attestation-credential-epic.md`) — the
  operator's vision (quiz→`attestation:mastery`→role gating) is mostly unwired; for a `/brainstorm →
  p2p-design-gate → /plan`. The trust-badge is plumbed-but-empty until a content-quality minter exists.
