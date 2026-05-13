---
name: EPR delivery sprint — 2026-05-11 overnight result
description: Wave 0 audit + Phase 4 substrate + Wave 2A/B + Wave 3A narrative landed. Substrate is essentially complete; W2D + W3B + W4 deferred. Convergent gaps from audit closed at expected sites. Non-obvious discovery — the substrate was 98.3% landed, not 0%; the plans were tracking debt, not implementation debt.
type: project
originSessionId: 60007cbf-4a59-4bce-9be7-6e57d1568cf6
---
The EPR delivery master sprint (kicked off 2026-05-11, overnight execution under auto mode) closed all named substrate gaps. The headline non-obvious finding inverted the sprint scope: the four phase plans (P2B / P3 / P3.5 / Light Up the Graph) showed 0/479 unchecked boxes, but the implementations were substantively landed — the checkboxes were never converted as the work shipped. Wave 0 audit walked each plan task-by-task, grep-confirmed code presence, and ticked 471/479 boxes. The actual remaining backlog was 2 concrete gaps + a topology-substrate piece, not a fresh-from-zero substrate sprint.

**Why this matters going forward:** when scoping any "I should pick up the EPR work" sprint, audit FIRST. The plan files lie about progress. The aunt-and-rage-bait integration test (the canonical Phase 3.5 closure scenario) passes in 12s on dev today — the substrate is real.

**What landed overnight (commits on dev, not yet pushed):**

| Commit | Wave | What |
|---|---|---|
| 4ea4e1558 | W0 | LUG audit — 116/124 checkboxes ticked; T18 record_predecessor identified as the single gap |
| e122ec072 | W0 | P3.5 audit — 22/22 tasks (117/117 boxes); aunt-and-rage-bait passes; only T22 remained |
| 33438cdd8 | W0 | P3 audit — 16/16 tasks (83/83 boxes); cold_fetch_resolves_manifest_from_peer #[ignore] lifted; clean |
| 0f3ffa20d | W0 | P2B audit — 150/155 boxes; 5 sweettests blocked on Stage 2 derive_compromise_at (W2D scope) |
| 8e37033a3 | docs | Master plan revised with audit-confirmed status |
| d3e749d05 | docs | Phase 4 sub-plan (Opus-authored) |
| 15ceba5d3..131ba689a | W1 | Phase 4 T1–T13: 11 commits + 1 cleanup; all 7 TODO(Phase 4) sites resolved across 3 view services + main.rs:1129 manifest layer-1 |
| 61b900017 | W3A | Wave 3 narrative — 6 new EPR scenarios with `# Why` annotations; opening blurbs grounded in manifesto |
| 7aa6db4ea | docs | W2A sub-plan |
| 379da16e2 | docs | W2B sub-plan |
| 2aaa12e4e | W2A | record_predecessor wired on libp2p Announce — closes T18 + T22 (convergent gap) |
| 97a36cc8c | W2B | IntegrityNotify KeyRotation handler — mirrors KeyRevocation pattern |

**Topology last-mile bridge — DELIVERED:** Phase 4 was the EPR-substrate piece blocking the parallel topology delivery. Every `TODO(Phase 4 follow-up)` site is resolved. `services/{distribution,reciprocity,peer_topology}_view.rs` now return real data. Helper API (`imagodei_lookup`, `connectivity`, `device_capacity`, `peer_diversity`) absorbed the glue per master-plan D2 — topology UI sprint can now render against substrate with no glue debt.

**Convergent gap closure:** T18 (LUG) and T22 (P3.5) were the same item — `record_predecessor` wiring on libp2p Announce. Both audits independently surfaced it. W2A landed it at the correct site (`p2p/mod.rs::handle_epr_atom_request`, where the sender PeerId is available — NOT `api/epr.rs` HTTP path where there is no remote sender). Required adding `sealing_keys: Option<Arc<SealingKeyPair>>` to P2PNode + builder method — small struct extension, not scope creep.

**Discoveries worth pinning:**
1. `key_revocations` table + writer + `derive_compromise_at` function ALREADY exist (Recovery M4 landed them) — but `derive_compromise_at` is Stage 1 fallback (returns `effective_at`); Stage 2 needs a `compromise_at` column added to `key_revocations` AND to the DNA-side `KeyRevocation` integrity entry AND through the `KeyRevocationRequested` signal payload. That's DNA + storage cross-boundary work — too risky overnight alongside the parallel tiered-storage sprint. Deferred (W2D, see task #14).
2. `key_rotations` table + `KeyRotationCommitted` signal handler also already exist. W2B's wire format uses canonical `KeyRotationPayload` field names (`human_agent_pubkey`, `new_agent_pubkey`, `superseded_agent_pubkey`, `recovery_request_hash`, `rotated_at`) — Recovery M4 producer side MUST match these names when publishing IntegrityNotify direct-notify.
3. Direct-notify is delivery-optimistic, NOT the canonical write path. Both KeyRevocation and KeyRotation IntegrityNotify handlers decode + dedup + log + return `IntegrityAck { received: true }` — the canonical write happens via the local conductor's `RecoveryV2Signal` stream (signals.rs). Two paths to the same projection; signal stream wins on consistency.

**What's still pending:**
- **W2D** — Stage 2 `derive_compromise_at` (needs DNA + storage coordination; not safely overnight scope). 5 P2B sweettests + the storage-layer rotation→revocation→sweep loop assertion are blocked on this. Task #14 has full scope.
- **W3B** — step-def glue for the 6 new EPR scenarios authored in Wave 3A. Mechanical Sonnet/Haiku work; was deferred because step defs without implementation behind them just keep scenarios `@wip` — no urgency. Task #17.
- **W4** — soak + cross-stack validation (aunt-and-rage-bait on libp2p AND iroh; Phase 2B↔Recovery M4 signal-stream coordination soak). Needs CI / Jenkins, not local execution. Task #16.

**Disk lessons:** PVC went from 88% → 58% via `cargo-pool legacy-targets --clean --yes` (recovered 35.6G of duplicate target/ dirs outside the shared pool). Threshold: act at 85%+ used. Memory pin `feedback_pvc_threshold_and_recovery` documents the procedure.

**Coordination with parallel tiered-storage sprint:** Both sprints landed commits on dev concurrently. Tiered-storage shipped their Wave 0 plan + spec + sweettest fix; EPR shipped substrate + W2A/W2B + Phase 4. No file conflicts because scopes were disjoint (tiered-storage = eae/qahal; EPR = elohim-storage views + p2p). Cargo formatting: W2A's commit included `cargo fmt --check` cleanup of 14 files left unformatted by the parallel sprint — courtesy not scope creep.

**Closing condition status (per master plan):**
- ✅ Audit complete (471/479 boxes converted)
- ✅ Phase 4 landed (all 7 TODO sites resolved; topology surfaces render real data)
- ✅ Runtime gaps closed for in-scope items (record_predecessor; KeyRotation handler)
- ⚠ A2o coverage lifted partially (narrative authored; step-def glue deferred)
- ⏸ Pre-push hook validation — not yet run (would block on uncommitted parallel-sprint work potentially; defer to operator)
- ⏸ Push lands on origin/dev — explicitly deferred per operator request (multi-sprint coordination decision)
- ⏸ Cross-stack integration — needs iroh master + Jenkins
- ✅ Sprint-result memory entry — this file
