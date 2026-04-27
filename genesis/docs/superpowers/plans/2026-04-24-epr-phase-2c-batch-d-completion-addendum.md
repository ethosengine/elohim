# EPR Phase 2C — Batch D Completion Addendum

**Status:** Addendum to `2026-04-23-epr-phase-2c-libp2p-federation-plan.md`. Narrows scope to Tasks 15-18 now that Batches A + B + C and Task 19 (harness) have landed.
**Date:** 2026-04-24
**Author:** Matthew Dowell + Claude (retrospective from A+B delivery)

---

## Where we are

Batches A + B landed on dev (Apr 23). Batch C + Task 19 harness landed on
branch `feature/epr-2c-batches-cd` (ready to merge to dev once Tasks 15-18 or
equivalent test coverage clears).

| Batch | Tasks | Status | Commits |
|-------|-------|--------|---------|
| A — Codec | 1-6 | ✅ dev | `d4c87204`, `e01e266c`, `c19860fd`, `daba57df` |
| B — Behaviour | 7-8 | ✅ dev | `3c59f731` |
| C — Handlers | 9-14 | ✅ feature/epr-2c-batches-cd | `372dffc8`, `fdf6ca92`, `4f0da3d5`, `a98bdbaa`, `6727d648`, `c4a88c0b` |
| D — Tests | 15 | ⬜ not started | |
| D — Tests | 16 | ⬜ not started | |
| D — Tests | 17 | 🚧 in-progress | `wip/task-17-migration-debug` branch — someone hit a migration issue |
| D — Tests | 18 | ⬜ not started | |
| D — Harness | 19 | ✅ feature/epr-2c-batches-cd | `88cc4fdf` |

---

## Retrospective — findings from reviewing Batches A + B on dev

This addendum was drafted after a post-delivery review of my A+B work.
Findings that matter for the Batch D kickoff and for Phase 2B downstream:

### ✅ Interface held across A+B → C handoff

The stub handlers in `handle_epr_atom_request` (commit `3c59f731`) were
preserved byte-for-byte in signature by Batch C's real implementations. The
`async fn handle_epr_atom_request(&self, peer: libp2p::PeerId, request:
EprAtomRequest) -> EprAtomResponse` shape survived; Batch C extended
functionality via `self.identity_map`, `self.db_pool`, and new service/helper
calls without touching the call site. This is the right outcome — stubs were
shape-correct, not just placeholders.

**Lesson for Phase 2B planning:** design decisions made during A+B (tag-based
discrimination, `envelope_bytes` as `Vec<u8>` with `serde_bytes`,
`Option<Vec<u8>>` slot preservation for batch, four response variants
`Atom`/`AtomBatch`/`Announced`/`NotFound`/`Error`) are now wire-locked. Phase
2B's EprHead↔Envelope reconciliation has to live *alongside* this protocol,
not rename or restructure it. Version bump to `/elohim/epr-atom/2.0.0`
required for any wire-breaking change.

### ✅ Golden vectors are the version-pin

`tests/vectors/epr_atom_messages.json` contains 9 CBOR-hex strings covering
every variant + the `Some(bytes) / None / Some(bytes)` slot pattern. The
fixture explicitly says "Regenerate only if protocol version bumps." The test
`golden_vectors_stable` re-encodes and hex-compares. **This is the canonical
signal that anyone touching the wire format has broken it.**

### ⚠️ Dockerfile inline-version drift risk

The `elohim-storage` Dockerfile (fix `0b410841`) inlines workspace-inherited
dep versions via sed (serde 1.0, chrono 0.4, ts-rs 10.0, etc.) because the
workspace root `Cargo.toml` isn't in the build context. Same pattern that
exists for the `constitution` crate. If the workspace Cargo.toml bumps any of
these, the container build silently uses the old version. **Not a blocker
for Batch D**, but worth filing as a pre-existing tech-debt ticket. A proper
fix is to COPY `elohim/Cargo.toml` as the workspace root and drop the sed.

### ⚠️ Stub-era Announce response still leaks "not yet implemented"

During the A+B window, a peer running stub handlers returning
`Announced { accepted: false, reason: Some("handler not yet implemented
(Batch C)") }` would emit that literal string to newer peers. Once Batch C
ships and dev is updated, all alpha-network peers will upgrade, but
mixed-version runs during rollout will see these placeholder reasons. Not a
bug — expected transient behaviour. **Worth flagging in Phase 2B's monitoring
design** so these reasons don't pollute operator dashboards silently.

### ⚠️ Announcement fanout is Phase 2B territory, not Batch D

AnnounceAtom as delivered in Batch C takes wire bytes, verifies, persists via
`EprService`. That shape is fanout-agnostic — *what triggers an Announce
call* is a producer-side concern, not receiver-side. Batch D integration
tests assume direct peer-to-peer `send_request` invocation. **Don't attempt
to test fanout strategy here.** That belongs in Phase 2B's signal-harness
migration block.

### ℹ️ `TODO(phase-2c)` markers are Phase 2B, not Batch C/D

The five `TODO(phase-2c)` comments in
`elohim/elohim-storage/src/services/epr_store.rs` (lines ~192, 195, 221, 230,
261) describe `FederatedEprStore` swarm-handle wiring. **These are not Batch
C or D tasks.** They belong to Phase 2B's projector/write-through block.
Relabel to `TODO(phase-2b)` when rebasing.

---

## Scope for this addendum — Tasks 15, 16, 17, 18

**Files:** all modifications to `elohim/elohim-storage/tests/epr_atom_federation_integration.rs` (the file created by Task 19 harness).

Per the base plan (`2026-04-23-epr-phase-2c-libp2p-federation-plan.md`
lines 1449-1775):

### Task 15 — Round-trip integrity (P0)
Peer A announces an atom. Peer B fetches it by CID. Assert byte-identical
envelope round-trip + verified signature.

### Task 16 — Reach gate parity (P0)
Peer A announces a private atom (reach=private, signer=A's agent key). Peer B
(different agent) fetches — expect `NotFound`. Peer A (same agent) fetches
— expect `Atom` with bytes. Parity with `handle_epr_request`'s gate logic.

### Task 17 — Batch semantics (P1) — ⚠️ ACTIVE WIP
Branch `wip/task-17-migration-debug` — someone hit a migration issue.
**Pick up that branch first** and either finish or capture the blocker.
Task 17's expectation: FetchBatch of [public_cid, private_cid_not_author,
unknown_cid] returns `[Some(bytes), None, None]` in that order (slot
preservation, leak-free denial).

### Task 18 — Validation rejection (P1)
Peer A announces an atom with a corrupted signature. Peer B's handler should
reject via `verify_incoming_epr` (`fdf6ca92`) and return
`Announced { accepted: false, reason }`. Atom should NOT be persisted.

---

## Out of scope (defer to Phase 2B)

These are flagged so the fresh subagent doesn't expand scope:

- Kademlia provider records for announced atoms
- LRU/bloom dedup for repeated announcements
- Announcement fanout strategy / signal-harness triggers
- TypeScript wire types for browser/Tauri peer
- EprHead ↔ Envelope reconciliation
- `FederatedEprStore` swarm-handle wiring (the 5 `TODO(phase-2c)` markers)
- Projector from `epr_atoms` → pillar tables

---

## Kickoff prompt for fresh subagent

```
Execute Batch D Tasks 15-18 of the EPR Phase 2C plan.

Primary plan: genesis/docs/superpowers/plans/2026-04-23-epr-phase-2c-libp2p-federation-plan.md
(Task descriptions at §Task 15 / §Task 16 / §Task 17 / §Task 18)

Addendum (START HERE): genesis/docs/superpowers/plans/2026-04-24-epr-phase-2c-batch-d-completion-addendum.md
(Scope narrowing, retrospective findings, WIP branch callout)

Base branch: `feature/epr-2c-batches-cd` @ 88cc4fdf (Batch C + harness from
Task 19 already there).

FIRST ACTION: Check `wip/task-17-migration-debug` branch — someone got partway
into Task 17 and hit a migration issue. Decide: rebase + continue, cherry-pick
what's usable, or start fresh with notes from that branch. Report that
decision before writing Task 17 test code.

Then execute Tasks 15, 16, 17, 18 in order using the existing harness
(`two_peer_swarm()` helper from commit 88cc4fdf). Each task:
  - Read the §Task N section of the primary plan
  - Write the test against the harness
  - Run `RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test epr_atom_federation_integration`
  - Commit with `test(epr-2c): <task desc>` format matching prior Batch D style
  - Move to next task

Do NOT expand scope. The addendum's §Out of scope list lists 7 deferrals that
are Phase 2B, not Batch D.

Use `superpowers:executing-plans` or `superpowers:subagent-driven-development`.

Done = Tasks 15-18 all pass locally on feature/epr-2c-batches-cd; branch ready
for merge to dev.
```

---

## What follows Batch D

Once Batch D is green:

1. ✅ Merged to `dev` (through commit `e9e2806a` on gossipsub foundation).
2. ✅ **Phase 2B designed** — see:
   - Kickoff prompt: `genesis/docs/plans/2026-04-24-epr-phase-2b-brainstorm-kickoff-prompt.md`
   - Design spec: `genesis/docs/superpowers/specs/2026-04-24-epr-phase-2b-design.md` (8 coupling decisions resolved, 9 projector invariants, 15 p2p-design-gate classifications, three-arc framing: resiliency producer → 2B hinge → Phase 3–7 graph surface consumer)
   - First-draft plan: `genesis/docs/superpowers/plans/2026-04-24-epr-phase-2b-plan.md` (39 tasks across 4 batches A/B/C/D mirroring Phase 2C shape)
3. ✅ **Phase 2B executed** — all four batches landed on dev:
   - **Batch A** (12 tasks): `AgentPeerBinding` DNA entry type, `HolochainBackedPeerIdentityMap`, DNA signal stream (converges with Recovery M4), `ReconcileController`, libp2p handshake + gossipsub identity binding topic
   - **Batch B** (8 tasks): single elohim-storage projector (Principle P1), manifest-declared pillar mapping, `EprHead` as projector-derived (Category A2), shefa `EconomicEvent` first mapping
   - **Batch C** (7 tasks): `/api/v1/signal/emit`, conductor signing API, Angular signal harness migration, 4-layer write-through flag, integrity-always-on exception
   - **Batch D** (8 tasks): tiered fanout, Kad `start_providing`, gossipsub topic enumeration + reach-gated subscription, integrity direct-notify (D.5: `IntegrityNotify`/`IntegrityAck` wire variants), dedup LRU with `P2PStatusInfo` observability, `providers()` returning local + DHT providers
4. `TODO(phase-2b)` markers resolved via Z.1 close-out sweep (Batch D Z.1):
   - 5x `epr.rs` read-only routes → re-tagged `TODO(phase-3)` (dedup wiring deferred; `providers()` route already wired)
   - `epr_store.rs` module doc — updated to reflect Phase 2B reality (swarm wired, fanout live)
   - `epr_store.rs` line 293 cold-fetch → re-tagged `TODO(phase-3)` with pointer to Phase 3 kickoff prompt
   - `epr_atom_protocol.rs` TODO(Z.1) — removed; schema updated with `integrity_notify` + `integrity_ack` variants
   - `mod.rs` TODO(Z.1) dedup stats → resolved; `P2PStatusInfo` now carries `dedupUniqueLen` / `dedupTotalSeen`
5. Batch A's DNA signal stream contract converges with Recovery M4's fast-path revocation work — both epics share `dna-signal-stream.schema.json` as the coordination surface.
6. ✅ **Phase 3 kickoff prompt drafted** — see `genesis/docs/plans/2026-04-26-epr-phase-3-manifest-resolver-kickoff-prompt.md`

### TODO(phase-3) markers planted by Z.1 (grep starting points for Phase 3)

```
elohim/elohim-storage/src/api/epr.rs          — 5x TODO(phase-3): local_libp2p_peer_id dedup wiring (fetch ×3, verify ×1, list ×1)
elohim/elohim-storage/src/services/epr_store.rs — 1x TODO(phase-3): cold-fetch via swarm_handle.resolve_epr(cid)
elohim/elohim-storage/src/services/epr_kind.rs  — 1x FIXME(phase-3): replace pillar_for_kind_provisional with ManifestRegistry
```

Phase 3 execution begins at operator call per `genesis/docs/plans/2026-04-26-epr-phase-3-manifest-resolver-kickoff-prompt.md`.
