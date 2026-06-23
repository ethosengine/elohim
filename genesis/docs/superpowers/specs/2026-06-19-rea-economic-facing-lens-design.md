---
title: "REA / Economic Facing — the commitment-ledger lens (intent vs observed, mutual-compute)"
id: rea-economic-facing-lens-design
status: Draft
class: protocol-canonical
domain: D5
topic: [rea, economic, facings, lens, commitment, mutual-compute, valueflows, dataplane]
refines:
  - genesis/docs/superpowers/specs/2026-06-19-resilience-facings-select-fold-aggregate-design.md
informed-by:
  - genesis/docs/superpowers/specs/2026-06-19-resilience-facings-select-fold-aggregate-design.md
  - genesis/docs/superpowers/specs/2026-06-07-lens-complete-epr-resolution-four-leg-coupling-design.md
cites:
  - resilience-facings-select-fold-aggregate-design | the select→fold→aggregate lens framework (§11) this facing is a child of — its materialized-relation + pure-fold + typed-view substrate | sha256:93279fd25a0600d1 | path: genesis/docs/superpowers/specs/2026-06-19-resilience-facings-select-fold-aggregate-design.md
  - lens-complete-epr-resolution-four-leg-coupling-design | the four-leg coupling law + §2 projection law this lens facing ultimately descends from | sha256:79f821217c1c8e11 | path: genesis/docs/superpowers/specs/2026-06-07-lens-complete-epr-resolution-four-leg-coupling-design.md
requires_env: [household-nodes]
---

# REA / Economic Facing — the commitment-ledger lens

> **One-line:** the economic lens folds the ValueFlows commitment ledger(s) into a household-economic
> facing — who *promises* to hold/serve (intent), what was *realized* (observed value flows), and the
> *mutual-compute* (`delegates-compute`) agreements that bind reciprocal hosting — as one child of the
> lens framework (`2026-06-19-resilience-facings…` §11).

## Provenance
Refines `2026-06-19-resilience-facings-select-fold-aggregate-design.md` (the framework) and
`2026-06-07-lens-complete-epr-resolution-four-leg-coupling-design.md`; the resiliency facing
(`services/household_resilience.rs`) is the proven reference — its `commitment_backed_collectives`
count is the seed of this facing's intent fold.

## Materialized relation(s)
One spine relation, `CommitmentRow`, loaded **unified across both ledgers** with a `ledger` discriminator:
- `rea_commitments` (`db/rea_commitments.rs` → `ReaCommitmentView`) — `action / provider / receiver /
  resource_classified_as (JSON list) / state / in_scope_of`.
- `mishpat_commitments` (`db/models.rs`; `cid = entry_hash`) — `provider / recipient / bounds_json /
  valid_from..valid_until / state` — the compute-agreements (`delegates-compute`, `replicates-dwelling`).

Natural join key is `bounded_by` cid (= the commitment `entry_hash`). The observed side is a second
relation, `EventRow` over `economic_events` (`EconomicEventView`), linked to its governing commitment via
`economic_events.bounded_by`. The agent join VALUE is `agent_cid` everywhere (`provider`/`recipient` hold
`uhCAk…`, never a transport id). `resource_classified_as` is read only through `classifications_of`
(`rea_commitments.rs`), never scalar `.eq` (the U1 dark-card class, 2026-06-19).

## Folds (pure fns over the relation)
- **`commitment_backed(rel, scope)`** → distinct households with an active provide commitment whose
  classifications contain `content:<reach>`; lifts the proven `household_resilience.rs` logic
  (`bucket_by(hub)` + `classifications_of` membership). *Intent.*
- **`by_action(rel)`** = `bucket_by(rel, action)` → count per action (`provide`, `operate-doorway`,
  `project-epr`, `custody-blob`, `delegates-compute`, `replicates-dwelling`).
- **`mutual_compute(rel)`** → reciprocal `delegates-compute` pairs: `bucket_by` provider↔recipient, flag
  pairs present in both directions (the distinctive new fold; intent-only until a compute-event bridge exists).
- **`realized_value_flow(events, rel)`** → committed-vs-realized: per commitment cid, sum `economic_events`
  where `bounded_by = cid`; surfaces fulfillment ratio. *Observed.*

## Typed VIEW + HTTP surface
New `MishpatCommitmentView` (`#[derive(TS)]`, camelCase: `cid, action, scope, provider, recipient, bounds,
validFrom, validUntil, state, dhtAnchorHash, createdAt`) in `elohim-views/src/shefa.rs` — closes the
unread-surfaced gap (today only a write path + an internal bounds read). Lens output is
`ReaFacingView { commitmentBacked, byAction, mutualCompute, realizedFlows }`. HTTP: **extend existing** —
add a `facing=rea` arm + a GET/list route over `mishpat_commitments` (read-only; the write path already
exists, so **no new POST**). Both new routes need an `is_service_path` arm (the EPR-router-shadow trap).

## Aggregation levels
per-commitment → per-provider-household via `bucket_by(hub)`; per-household → dashboard via a hand-written
`aggregate(views: &[ReaHubView]) -> ReaDashboardView` (verdict rollup is per-facing; genericity stops at
the relation layer).

## P2P Design Gate output
**Operational, Category C — zero new DHT entry types.** Both ledgers are already notarized DHT entries
(`Mishpat::Commitment`, cid = `entry_hash`); this lens is a read projection over existing operational
tables. Identity is content-derived (commitment cid) + agent-composite (`agent_cid`). No new sync messages.
**Do NOT** consume unsigned/self-asserted `AgentPeerBinding` for economic attribution (open security item);
the lens joins on `agent_cid` directly.

## Slices (sequence)
**Blocked-until (lens, not fold):** `commitment_backed` already works in the resiliency facing, so its
*fold* is ready — but the rest of the REA lens is gated on real surface work: `MishpatCommitmentView` +
its GET route (the mishpat ledger is write-only/unread-surfaced today), `economic_events.bounded_by` seed
(for `realized_value_flow`), and the mishpat→rea `delegates-compute` bridge (for `mutual_compute` to appear
in provide vocabulary). This is a charter; the lit lens follows that data/view work.

**Slice 0 — proof gate (`commitment_backed`, one metric end-to-end).** It silently zeros — it exercises the
exact join + classification path where the all-zeros bug lives. Coherent-seed checklist (each independently
zeros the metric): (1) `state = 'active'` (POST inserts `'proposed'`); (2) `humans.agent_pub_key` populated
**and** `= commitment.provider`; (3) `humans.household_id` not null; (4) `resource_classified_as`
array-wrapped containing `content:<reach>`; (5) the `content` row's `reach` matches scope. Test-first,
DB-free fold over hand-built `Vec<CommitmentRow>`, then the storage adapter on seeded data.

**Then:** `by_action` view + `facing=rea` route → `MishpatCommitmentView` + GET route (+ `is_service_path`)
→ `mutual_compute` fold → `realized_value_flow` (needs `economic_events.bounded_by` seed) → `aggregate` dashboard.

## Non-goals / operator-owned
- The unified **mishpat→rea bridge for `delegates-compute`** — only the content-provide bridge exists
  (`record_provide_from_content_commitment`); the compute bridge is downstream data work.
- **Realized-compute events** — `delegates-compute` has no observed-side `economic_event` emitter yet;
  `mutual_compute` stays intent-only until one lands.
- **StewardshipAllocation** (recognition-share routing) — orthogonal to commitments; not folded here.
- **Reach-vocabulary reconciliation** (backlog item 13) — loader-and-view work.
- **The `Lens` trait / `dyn` registry / facing×leg matrix** — excluded per the framework constraint.
