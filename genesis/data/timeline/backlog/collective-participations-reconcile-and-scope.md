---
id: "backlog-collective-participations-reconcile-and-scope"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "collective_participations never heal cross-peer (no reconcile/federation arm) and are written/read under mismatched AppContexts (qahal vs lamad)"
slug: "collective-participations-reconcile-and-scope"
written: "2026-08-22"
author: "doorway-breaker triage (CollectiveCommitted/MembershipCommitted msgpack decode fix follow-ups)"
status: "fixed-pending-runtime-proof"
priority: "medium"
tags: [dataplane, qahal, collectives, projection-reconcile, view-federation, app-context, bounded-code-fix]
---

# Participation rows: two queued follow-ups from the collective-signal triage

The msgpack decode cure (HoloHashB64-typed `ImagodeiSignal` mirror in
`elohim/elohim-storage/src/signals.rs` + direct typed decode in
`src/reconcile/holochain_app_signal.rs`) makes `CollectiveCommitted` /
`MembershipCommitted` project locally again. The triage exposed two adjacent
defects it deliberately did NOT fix:

> **STATUS 2026-08-22 — (a) and (b) LANDED, locally verified, awaiting runtime
> proof on the mesh. (c) partly landed: the humans/participations arm of it is
> in; the collectives-metadata half is precisely bounded below.** Verification
> so far is `cargo test --lib` (2947 pass, 8 new), `--test schema_contract`
> (228 pass, no wire shape changed), `clippy -D warnings` clean, `cargo fmt`.
> Nothing here is proven on a live peer yet: the participations arm has never
> run against a real conductor, and the migration has never executed against a
> peer's on-disk projection. The runtime probe that closes this row is named at
> the bottom.

## (a) No participations arm in projection_reconcile / view_federation

`projection_reconcile` and `view_federation` carry a **collectives** arm but
NO **collective_participations** arm. A participation row authored by a peer's
`post_commit` projection exists only on that peer — it never heals cross-peer.
Concretely: the triad assertions against `:8090` need jessica's and james's
participation rows visible from matthew's storage, and today they can never
arrive. Close by adding the participations arm to both surfaces (same
`dht_anchor_hash` idempotency key the local projector uses).

**LANDED.** A fourth arm, `p2p/participations_reconcile.rs`, alongside the
existing three:

- **Responder** — `PROJECTION_INVENTORY_TABLE_PARTICIPATIONS`
  (`"collective_participations"`) in `p2p/view_federation.rs`, served from
  `db::collectives::list_participation_anchor_inventory`. NULL/empty-anchor rows
  are excluded (a locally-authored participation has no DHT identity to
  reconcile on — the same ruling the collectives arm makes about NULL-cid rows).
  BOTH wire slots carry the Membership ActionHash: the diesel row id is a
  peer-local UUID (two peers projecting one Membership mint different ones), so
  it never reaches the wire.
- **Requester** — `discover_participations` / `heal_participations` on the
  shared `GapTracker` + `MissLedger` rails, own leg budget
  (`PARTICIPATIONS_LEG_BUDGET`, 30s), ordered LAST so it cannot starve REA or
  content. NO divergence class: the anchor IS the row key, so "present with a
  different identity" is not representable and every admitted gap is healable.
- **Conductor read** — `conductor_writes::get_membership_by_anchor` →
  `imagodei::get_membership_by_action` (an EXISTING coordinator fn; no DNA
  change, no hash move). It returns `ExternResult<Record>`, so its not-found is
  a wasm error string — classified in exactly one place
  (`is_membership_not_found`) rather than leaking "the conductor is broken" into
  the heal loop. An `UnknownFunction` answer (coordinator not yet hot-swapped)
  sheds the whole leg instead of burning a round-trip per gap.
- **ONE mapping** — `db/memberships.rs::project_membership`, shared with
  `ReconcileController::on_membership_projected` (which is now a thin caller).
  Anchor idempotency, `collective_id` resolution, `human_id` resolution and the
  `humans.agent_pub_key` / `humans.household_id` backfill therefore cannot drift
  between the signal arm and the reconcile arm.
- Metrics/logs ride the existing per-stream label surface as
  `stream="participations"`.

## (b) AppContext scope mismatch: written "qahal", read "lamad"

Account-import writes participations under AppContext `"qahal"`
(`elohim/elohim-storage/src/http.rs:11191`) while the legacy
`GET /db/collectives/{id}/participants` route reads ctx `"lamad"` — the same
class as the HUMANS_HAPP_ID drift (`elohim/elohim-storage/src/db/context.rs:6-21`).
Rows written by one surface are invisible to the other. Pick the canonical
context (and migrate or dual-read the stragglers) rather than patching one
route.

**LANDED — canonical scope is `qahal`, and the signature now enforces it.**

The tiebreaker was not preference: the table's own DDL has always said
`h_app_id TEXT NOT NULL DEFAULT 'qahal'`
(`migrations/2026-01-08-000000_initial/up.sql:769`). `"lamad"` was the operating
content scope leaking into an identity-adjacent projection — the same shape
`HUMANS_HAPP_ID` cures. Three writers had drifted, not two: account-import
(`qahal`), the legacy `POST …/participants` route (`lamad`), and
`on_membership_projected` (`lamad`).

- `db::context::PARTICIPATIONS_HAPP_ID = "qahal"`, documented beside
  `HUMANS_HAPP_ID`, with a test pinning it to the column default.
- The five participation functions in `db/collectives.rs`
  (`get_participations_for_human`, `get_participants_of_collective`,
  `create_participation`, `update_participation_intimacy`,
  `depart_collective`) now take **no `AppContext` at all** and scope by the
  constant internally. This is the anti-drift shape: a caller cannot hand them
  the operating content scope by accident, so the fix is enforced by the
  signature rather than by remembering a filter. The three HTTP handlers lost
  their `ctx` parameter for the same reason.
- Stragglers migrate, rather than dual-read:
  `migrations/2026-08-22-153000_participations_canonical_qahal_scope`. It is
  UNIQUE-safe by construction — a straggler whose `(collective_id, human_id)`
  pair already exists under `qahal` has its DHT provenance
  (`dht_anchor_hash`, `member_cid`) merged onto the canonical row first, then is
  dropped; the rest are moved. Nothing is destroyed that the canonical row does
  not carry, and every row is re-derivable from the DHT by the (a) arm anyway.
- Side effect worth naming: `epr_service`'s `community`-reach authorization read
  participations under the caller's content scope, so it could not see the rows
  account-import wrote. It now reads the canonical scope — a live behavioural
  fix that fell out of the same cure.

## Watch item

When `ALLOW_SEED_SHARD_MANIFEST`'s 403 is lifted: if the costeward leg derives
consent from participants, it lands on this surface — both (a) and (b) become
its preconditions.

## (c) Same class, third instance: canonical `humans` rows never heal cross-peer (household-vocabulary split)

The saga ch10 card-tells-truth divergence (2026-08-22: doorway A said
`stewardingCollectives: 1`, doorway B said 2 for the SAME `elohim-host-landing`
custody facts) was this class wearing the resilience card. The custody plane
AGREED — both peers' `shard_manifests` + `shard_locations` held both holder
agents — but the slug-keyed fixture `humans` rows (`human-matthew-manager` →
`household-dowell`, seeded via doorway A) exist only on matthew: there is no
humans/membership reconcile arm, so jessica/james only ever get the
`identity_fill` CREATE fallback (`id = agent:{pubkey}`,
`household_id = collective:{action_hash}` verbatim). One physical household then
splits into two id vocabularies inside a single peer's fold.

**Read-side cure landed** (bounded, same branch):
`services/household_resilience.rs` now canonicalizes cid-form
`humans.household_id` values through the local `collectives.collective_cid`
alias (which the collectives arm DOES replicate — both peers already held
`household-dowell ↔ collective:uhCkkoQQ…`) in the holder relation, the
replication-commitment relation, and the commitment-backed-collectives count.
Counts now agree; the fold serves only local truth (verify-locally-then-serve
intact).

**Still open in this class:** the canonical slug `humans` rows themselves
(display names, and any peer whose `collectives` projection lacks the
`collective_cid` alias) never converge cross-peer — jessica renders label
`household-dowell` where matthew renders `Dowell Household`, and her placeholder
collectives row carries no region (regional-distribution buckets still
diverge). Closing that is the humans/participations reconcile arm this file
tracks — one arm shape, three projections (participations, humans, collectives
metadata refresh).

### (c) — what landed with (a), and where the boundary now sits

**LANDED: the identity-coherence half, because it composed naturally into the
same arm.** `project_membership` (the ONE mapping (a) introduced) already owned
the `humans.agent_pub_key` / `humans.household_id` backfill, so the reconcile
arm inherited it: a peer healing a Membership it never authored now ALSO stamps
that member's identity columns from DHT truth — the projection the local signal
arm could only ever populate for its own authored memberships. Two additions
make it land on the right row:

- **`human_id` resolves through `humans.agent_pub_key`.** A healed row lands
  under the canonical slug (`human-jessica-spouse`), not `agent:uhCAk…`. That is
  what the household-formation E2E's triad assertion reads
  (`participantHumanId`), and it is the (c) vocabulary split showing up one
  level down: one physical person carrying two id vocabularies inside a single
  peer's fold. Degrade-don't-guess is preserved — a member this peer has never
  met keeps the `member_cid`, exactly the pre-cure behaviour.
- **One person, one participation row.** Because `create_participation` upserts
  on `(collective_id, human_id)`, a healed Membership ADOPTS the row
  account-import already created for that human and stamps the anchor onto it,
  rather than minting a second participation. That also pulls the previously
  unreachable account-import row INTO the cross-peer inventory (it now carries
  an anchor), so it starts replicating.

**STILL OPEN — bounded, and deliberately not attempted here:**

1. **`humans` display-name / metadata reconcile.** No `humans` arm exists.
   `project_membership` fills `agent_pub_key` and `household_id` on an EXISTING
   row and never creates one, so a peer with no `humans` row for jessica still
   has no display name for her — `identity_fill`'s `agent:{pubkey}` CREATE
   fallback remains the only row-minting path. Closing it needs its OWN arm
   (inventory over `humans` keyed by `agent_pub_key`, healed from an imagodei
   `get_human_by_agent_key`-shaped conductor read), NOT another projection
   walked by the participations arm: the identity is a different key, the
   conductor read is a different extern, and `humans` is the `imagodei`
   partition rather than `qahal`. That is the balloon boundary — a fifth arm,
   sized like this one, not an extension of this one.
2. **`collectives` metadata refresh (region).** The collectives arm's
   `project_collective` refreshes `name` + `governance_layer` only; `region` is
   written by no DHT-fed path at all — `CollectiveWire` has no region field, so
   it would have to be derived from the charter, which is a DNA entry-shape
   question, not a reconcile-arm gap. Regional-distribution buckets therefore
   still diverge between peers.
3. The read-side canonicalization cure (commit `2cb043387`,
   `services/household_resilience.rs`) remains what makes the counts agree in
   the meantime. Unchanged by this work.

## Runtime observations (2026-08-22 wave-4 mesh roll, new binary serving)

**The arm heals cross-peer, live.** Within minutes of boot,
`GET /db/collectives/household-dowell/participants` showed jessica holding
matthew's AND james's rows, and james holding gertrude's and jessica's —
rows neither peer authored. Probe #1 below is substantially answered.
Partial convergence at measure time: matthew's own fold still held only his
own row (1/2/2 across the three peers, unchanged across a ~2min window) —
either the discovery cadence hadn't reached matthew's requester leg yet or
that leg warrants a look on the next pass.

**Probe #2's target has a vocabulary split the probes above didn't name:**
the household-formation scenarios read `family-dowell` (0 participation
rows on every peer, `collectiveCid` null there), while account-import and
the live healing all operate on `household-dowell`. The formation
ceremony's Memberships and the fixture participations land under different
collective slugs — same household-vocabulary class as (c). Until that
seam is picked (ceremony writes to family-dowell? scenario should read
household-dowell?), probe #2 cannot go green regardless of this arm.

## Runtime proof this row is still waiting on

Nothing below has run against a live peer (except as noted above).

1. On the mesh, `projection-reconcile[participations]: peer inventory received`
   appears with non-zero `entries` on a peer that did NOT author the membership,
   followed by `HEALED membership from own conductor`.
2. `GET /db/collectives/family-dowell/participants` on matthew's storage returns
   all three triad members (household-formation "All three members are affirmed
   participants"). NOTE the precondition this cure does NOT provide: each
   member's Membership must actually be committed on their own conductor. That
   scenario's own comment records matthew as unbindable in genesis #1489, never
   affirming — an identity-binding defect UPSTREAM of this projection. If the
   triad is still short after this lands, read the count: 2-of-3 means the
   binding; 0-of-3 meant this row.
3. The migration ran: no rows left with `h_app_id <> 'qahal'` in
   `collective_participations`.

## a2o changes this work implies (NOT made here — for the a2o owner)

- **None required for the two failing household-formation scenarios.** Both
  probe endpoints this cure serves, and the step definitions already accept the
  shapes produced (`humanId` for the triad; `collectiveCid` for the CID
  assertion).
- **One stale comment to refresh.** The feature-file comment above "The
  household collective is coherent" states that the wire shape of
  `GET /db/collectives/{id}` "carries no collectiveCid field at all". That is no
  longer true — `CollectiveView` carries `collective_cid`
  (`elohim/elohim-views/src/shefa.rs`), pinned against
  `collective-view.schema.json` by the schema-contract test (228 assertions,
  green). Cause (1) in that comment's two-cause diagnosis is now closed; only
  cause (2) — "projected but null on this peer" — remains, and is exactly what
  the collectives reconcile arm exists to fill.
- **Optional, small:** `CollectiveParticipationView` carries no `memberCid` /
  `dhtAnchorHash`, though `participantHumanId` already falls back to
  `memberCid`. Adding them (additive: schema + contract test + codegen) would
  let the E2E distinguish "healed from the DHT" from "locally imported" at a
  glance. Not needed for the assertions to pass.
