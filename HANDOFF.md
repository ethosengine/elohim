# HANDOFF — Acquisition family: spec LANDED → next is /plan slice 1

_Last updated: 2026-06-07 (post-brainstorm) · Author: Claude Opus · Branch: `dev` (local commits ride the next dispatch) · Session mode: **orchestrating → implementing handoff** (the brainstorm is done; the next session plans/implements)._

_The previous handoff (acquisition-family brainstorm) is **RESOLVED**: the brainstorm ran 2026-06-07 — full pre-step ceremony (lexical + semantic lenses, MAP D5 + roadmap orientation), 13-reader evidence workflow over canon + code anchors, p2p-design-gate passed, operator adjudicated 7 decisions, spec written/sealed/decomposed._

---

## Goal

Implement the **EPR acquisition family** per the landed spec:
`genesis/docs/superpowers/specs/2026-06-07-epr-acquisition-pull-queue-design.md`
(id: `epr-acquisition-pull-queue-design`, Draft, protocol-canonical, D5, refines the route-claims
conformance spec's Appendix E).

**The design in one breath:** dual pins (device pin = airplane-mode-durable local want, zero hub
zero DHT; provide tier = minted `provide-content` Commitment at sync-back; dwelling tier =
consented co-stewardship, a dial, designed-not-built) · a sibling **acquisition reconcile stream**
on rails extracted from the replication loop (priority lanes; sha256-byte-arrival done-signal;
tri-state completion with concrete-count guard; unified vocab `{total,fetched,pending,failed,caughtUp}`)
· typed-relation **closure** (depth/size caps in pin body; `contentToSync` finally gets both ends)
· **striping seamed not built** (`ShardRangeRequest` on shard_protocol; bitswap is serve-only —
NOT the seam) · commons-only pinning v1 (capability-by-hash quarantined, §14).

## Current state (verified)

- Spec landed + cite-sealed (4 envelopes + relationship hints) + decomposed: **13 OPEN gap-items**
  in `.claude/memory-kit/gap-items/specs__2026-06-07-epr-acquisition-pull-queue-design.json`
  (hand-authored — the mechanical pass had latched onto §12 gate bullets; replaced).
- Backlog seed marked ELEVATED (`epr-routing-complementary-captures.md`).
- All normative legs household-nodes-testable; held legs tagged inline
  (`@requires:alpha-cluster-6peer` trust-weighted scoring + WAN cold-fetch; `@requires:shem` scale).
- CI triage (same session, background): `elohim-genesis#1104` 3 fingerprints = ONE concern,
  stewardship-allocation seeder existence-read truncated at limit=10000 → paginated fix
  `ec5937287`, ledger triaged, backlog `ci-genesis-stewardship-allocation-seed-truncation.md`
  (`in-progress`, awaits ≥3-green disappearance).

## What worked (carry forward)

- The evidence workflow (13 parallel readers + synthesis barrier) resolved every contradiction by
  direct file read before the operator saw a question — the 6 decisions were sharp because each
  carried verified `file:line` evidence. Workflow script (resumable):
  `/projects/.claude-config/projects/-projects-elohim/f3f3f3f8-1f77-4511-8570-8fa43f8c9f8f/workflows/scripts/acquisition-family-evidence-wf_7e2189d1-f90.js`
- Semantic lens (historian/MemPalace) defeated vocabulary drift: identity-driven-replication
  (2026-04-06) IS the pull-queue prior art; epr-body-plane's `resolve_and_fetch` is the named
  striping extension point; resilient-html5-delivery has the 6-tier scoring rubric. All cited in
  the spec.
- Operator reframe to capture: **pin is device-durable first** (airplane mode), dwelling backup is
  a distinct consented tier, "hubbiness is a role with a dial" — this restructured three gate
  decisions at once and became spec §1 (the dual-pin model).

## What didn't work / gotchas

- The handoff's `2026-04-19-self-healing-p2p-dataplane-design.md` cite was STALE — archived in the
  2026-05-15 compaction (recover via `git show 53190a234^:...` if ever needed); live successors are
  tiered-quilt + blob-custody-reconciliation. Lesson: handoff cites rot; the spec's sealed cites don't.
- `decompose.py` requirement-bullets heuristic grabbed the §12 gate-record bullets (adjudications,
  not gaps) — hand-authoring was needed. Watch for this on gate-record-bearing specs.
- MemPalace MCP is per-subagent only (dispatch historian for semantic lenses; ToolSearch finds nothing).

## Next steps (ordered)

1. **Operator reviews the spec** (it is Draft; user-review gate of the brainstorm was reached but
   the written-file review is the formal step). Then `/plan` → **Slice 1**: `reconcile_rails`
   extraction + acquisition stream + DevicePin + rungs 2–3 + `.pull` wire + `wait-for-pull`
   (gap-items #1–#6; all household-nodes).
2. Slice 2 = `provide-content` mint + scorer arm + sync-back + rung 4 (gaps #7–#10; sweettest for
   the zome leg — zome-sweettest-sync).
3. Slice 3 = closure resolver + rung 5 + `contentToSync` both ends (gaps #11–#13).
4. Follow-on specs captured in §14: striping implementation (the seam is normative); dwelling
   escalation UX (needs consent surface); capability-by-hash adjudication (blocks gated pinning).

## Still-pending tails (inherited, not blocking)

1. **Shell DI fix verification**: once operator's rebuild-all deploys a shell newer than
   `main-2OW3WZQR.js`, run `E2E_DEVICE_MODE=playwright E2E_DOORWAY_ALPHA=https://doorway-alpha.elohim.host pnpm exec cucumber-js --tags '@deep-link and not @wip'` (genesis/a2o) — expect 9/9.
2. **Nexus/Harbor incident closure**: ≥3-green rebuild streak (`ci-nexus-harbor-pvc-jam-incident.md`).
3. **Genesis seeder fix confirmation**: next `elohim-genesis` build > #1104 — the three allocation
   assertions should pass (UNSTABLE may persist from the unrelated degraded-substrate condition).
4. Local commits riding next dispatch: `b15a16ee3`, `6d1b6024d`, `dbf15fc91`, `ec5937287` + this
   session's spec commit. One dispatcher at a time; verify runs SPAWN.

---

_Open this file in a fresh conversation to continue: review the spec, then `/plan` Slice 1._
