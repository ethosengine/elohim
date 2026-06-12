---
id: "backlog-resilience-unmeasured-vs-zero-honest-denominators"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Resilience snapshot conflates never-measured with measured-zero; peer counts need live/known denominators"
slug: "resilience-unmeasured-vs-zero-honest-denominators"
written: "2026-06-12"
author: "overnight session (operator demo read: elohim-host-landing all zeros)"
status: "backlog"
priority: "high"
jobs: [elohim-genesis]
tags: [resilience, views, ux-honesty, distribution-plane, demo]
cites:
  - genesis/docs/superpowers/specs/2026-06-12-resilience-dimensions-proof-suite-design.md
  - genesis/data/timeline/backlog/dna-bridge-role-name-conformance.md
---

# Unmeasured ≠ zero: honest resilience denominators

Operator demo read (2026-06-12, `elohim-host-landing`): the snapshot shows
`at-risk · 0 collectives · 0 commitment-backed · 0% diversity · no region
data` — and live probing shows EVERY seeded content id returns the
identical shape. Root: **no seeded content has a `shard_manifests` row**
(the bulk-seed path writes content.db + blobs and never enters the
quilt/distribution plane), so `household_resilience::compute()` returns
the degenerate at-risk view before any household/peer/commitment logic
runs. The tell: `regionalDistribution.unknown == 0` (a measured content
with regionless stewards puts the steward count in `unknown`).

Two distinct fixes:

## 1. View honesty (bounded — schema-first wire change)
The degenerate "never measured" case renders identically to a genuinely
measured-and-unprotected content. That is a false claim on the demo
surface. Change:
- `ResilienceSnapshotView` gains `distributionState: "unmeasured" |
  "measured"` (manifest-missing → `unmeasured`). The progressive icon and
  snapshot panel render a distinct "not yet distributed" state — never a
  fake at-risk verdict.
- Peer counts become numerator/denominator pairs (operator: "0/0 num
  types to represent live peers vs known peers"): `onlinePeerCount` →
  `{live, known}` where known = stewarded_nodes across the stewarding
  households (the D2 join). Tooltip renders `2/3 peers live`; `0/0` only
  when genuinely nothing is known. Same shape consideration for
  `stewardingCollectives` once desired-count (RS target, currently the
  D4 constant 7) is exposed.
- Path: schema (`elohim/sdk/schemas/v1/views/`) → Rust struct →
  `schema_contract` test → `INTERFACE_FILES` codegen → snapshot
  component. Boundary tests extend the D1/D2 sections of
  `tests/household_resilience.rs` (the degenerate row already exists —
  it asserts the NEW state instead).

## 2. The demo actually lighting up (distribution-plane decision)
For seeded content to be measured at all, the seed/import path must
create shard manifests (enter the distribution plane), or the
distribution plane must adopt seeded content on first read (heal-on-read
analog: manifest-on-first-stock). This is a p2p-design-gate question
(who authors the manifest, what triggers stock for bulk-seeded content)
— NOT a view fix, and the view fix should land first so the gap is
honest while this is designed. Related junction gaps (humans.household_id,
provide rows, regions) remain workstream D as mapped in the
2026-06-11 pickup plan.

**Operator design direction (2026-06-12, pre-design-gate seed):** don't
backfill manifests — build CLI tooling that REPLAYS the real CRUD
lifecycle users would perform on an EPR over time (compose through the
reach gate → publish through put_epr's republish/bounds validators →
stock → supersede), so seeded content is indistinguishable from
user-authored content because it took the same path. The substrate
already votes for this shape: the 2026-06-11 anchor-gap self-healing
(`update_via_conductor` → create_content re-publish) is the retroactive
version of the same front-door principle. Companion frame: "what if
every EPR were a git artifact" — the substrate is structurally ~70% of
one already (CID = object hash; sealed supersedes/superseded_by = signed
parent-commit edges; `epr:x@2` = refs resolving through history; the
envelope = commit metadata; source chains = per-agent branches; the
three-legged coupling is the leg git LACKS, the merge primitive is what
EPR lacks — that case lives on the CRDT plane). The CLI's verb set
should therefore BE the git porcelain over the real routes (`epr
commit|push|log|tag|stock`), and the seed sprint becomes
`epr push --as trusted-issuer` in a loop — the provenance manifest is
the commit graph real usage would have produced. Bonus: every seed run
becomes an at-scale integration test of the whole CRUD gate surface.
