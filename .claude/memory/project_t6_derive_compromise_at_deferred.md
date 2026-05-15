---
name: project-t6-derive-compromise-at-deferred
description: EPR Foundation Completion T6 (derive_compromise_at Stage-2 upgrade) deferred to legacy-path-removal task; substrate misalignment + low back-compat-window semantic gain.
metadata:
  type: project
---

# T6 derive_compromise_at upgrade — deferred to back-compat-window close

**Date:** 2026-05-15
**Sprint:** `2026-05-15-epr-foundation-completion.md` Task 6
**Verdict:** DEFER

## Gate state at decision time

- T6 was originally scoped as: replace the Stage-1 stub at `holochain_app_signal.rs:289–296` with a projection-lookup over `revocation_votes` returning the earliest approving-vote timestamp.
- Substrate misalignment: `revocation_votes` migration was NOT landed by M4. M4 landed `recovery_flows` (migration `2026-05-15-000000_recovery_flows`) + reused the existing `key_revocations` table (migration `2026-04-24-010000_key_revocations`, 13 cols, NOT the 15-col version with `derived_compromise_at` that the original plan envisioned).
- TODO(A.12) comment in the live code already shifted the target: `holochain_app_signal.rs:282–288` says the function "should query that projection to use the community's first declared compromise time" via `key_revocations.created_at` — that column exists and is populated.
- Sprint acceptance is met without T6: load-bearing work landed via T5 (A.8 sweep, commit `0104e5e48`) + T7 (RevocationAttestation IntegrityNotify arm, commit `68df467cd`) + a2o @wip lift (commits `1d3b34909` + `86d430833`).

## Why T6 is not load-bearing during the back-compat window

The A.8 sweep at `signals.rs:1220` (legacy `RecoveryV2Signal::KeyRevocationEffective` path) currently passes `effective_at` — NOT `derive_compromise_at`'s output — to the sweep function. T6 alone would only improve the `DnaSignal::KeyRevocation` envelope's `compromise_at` field for legacy-emitted signals; it would NOT tighten the sweep window.

Fully tightening the legacy sweep window (capturing the `[created_at, effective_at)` interval where attestations were issued by an already-compromised key) requires BOTH:
1. T6's projection lookup (~30 min), AND
2. Plumbing the derived `compromise_at` into the legacy sweep callsite at `signals.rs:1220` (non-trivial refactor of the legacy signal handler path).

Step 2 is the load-bearing piece, and step 2 alone is not justified for a path that is `#[deprecated]` and removed in one release cycle.

Meanwhile, the new T18 envelope path (`signals.rs:1459`) already passes `envelope.metadata.compromise_at` (set by M4 to equal `effective_at` at the producer, but the field is independent and a future M4 revision can populate it from revocation-request metadata). The new path is unaffected by `derive_compromise_at`.

## What to do when re-engaging

**Trigger:** the release cycle in which `RecoveryV2Signal::KeyRevocationEffective` is removed (the back-compat window closes per the T18 spec doc at `genesis/docs/superpowers/specs/2026-05-15-dna-signal-as-epr-envelope.md`).

**At that point:**
- `derive_compromise_at` in `holochain_app_signal.rs:289` goes away along with the legacy translation path entirely. T6 becomes moot.
- If the project decides to keep the legacy path alive longer than one release cycle (e.g. for a long migration window), re-engage T6 then:
  1. Add `db::key_revocations::get_created_at_by_id(conn, &revocation_id) -> Option<String>` helper.
  2. Replace stub body with: `if let Some(ts) = lookup { parse_iso(&ts) } else { effective_at_fallback }`.
  3. Add two tests: lookup returns earlier time when row exists; falls back when row absent.
  4. ALSO update `signals.rs::handle_recovery_v2_signal` (around line 1180–1240) to compute the derived compromise_at and pass it to the sweep callsite at :1220 — otherwise T6 doesn't tighten the sweep window.
  5. Estimated effort: ~1–2 hours for the full pair.

## Why this is not a sprint blocker

Sprint acceptance per kickoff prompt §"Acceptance for the sprint as a whole":
- ✅ A.8 sweep substrate filled (T5 — load-bearing).
- ✅ RevocationAttestation IntegrityNotify arm (T7 — load-bearing).
- ✅ 5/17 @wip scenarios lifted (T9 done; T10 retained-with-rationale because federation step-defs absent).
- ⏳ Cross-stack soak (T11 — operator-driven, post-merge).
- ⏸️ Plan-tracking debt for W2A/W2B-KeyRotation (T1, T2 — done).

T6's deferral does not block sprint close. Related: [[project-w2-agent-peer-binding-deferred]] (mirror deferral pattern for W2D AgentPeerBinding arm pending iroh Phase 12), [[project-epr2b-recovery-m4-convergence]] (T18 EPR-envelope substrate context).
