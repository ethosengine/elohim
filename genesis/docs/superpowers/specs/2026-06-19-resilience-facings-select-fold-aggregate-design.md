---
title: "Resilience Facings — select→fold→aggregate projection over the EPR holder-relation"
id: resilience-facings-select-fold-aggregate-design
status: Draft
class: protocol-canonical
domain: D5
topic: [resilience, facings, projection, fold, holder-relation, lens, epr, household, determinism, typed-view, dataplane]
refines:
  - genesis/docs/superpowers/specs/2026-06-07-lens-complete-epr-resolution-four-leg-coupling-design.md
informed-by:
  - genesis/docs/superpowers/specs/2026-06-07-lens-complete-epr-resolution-four-leg-coupling-design.md
  - genesis/docs/content/elohim-protocol/architecture/2026-06-04-qahal-epr-household-lattice-design.md
cites:
  - lens-complete-epr-resolution-four-leg-coupling-design | the §2 projection law this spec refines (URL-surfaces → audience-facings); §3 four-leg coupling, §4 typed self-certification, §5 closure-walk, §6 Operational-C verdict | sha256:79f821217c1c8e11 | path: genesis/docs/superpowers/specs/2026-06-07-lens-complete-epr-resolution-four-leg-coupling-design.md
  - qahal-epr-household-lattice-design | the household/dwelling/hub topology the holder-relation groups by; §7 zero-new-DHT-types verdict the facings inherit | sha256:ed5c1d3d2698b567 | path: genesis/docs/content/elohim-protocol/architecture/2026-06-04-qahal-epr-household-lattice-design.md
  - epr-acquisition-pull-queue-design | §5.1 ClusterClosure — the bounded typed-relation closure-walk the leg-neighborhood materialization reuses | sha256:24aad9240361c0a4 | path: genesis/docs/superpowers/specs/2026-06-07-epr-acquisition-pull-queue-design.md
  - resilience-card-lighting-plan | the closing card-lighting plan this spec is the composable substrate beneath; its §11.3 list→map typed accessor is the on-ramp | sha256:be6dfb65e5e8a433 | path: genesis/docs/superpowers/plans/2026-06-19-resilience-card-lighting-plan.md
  - genesis/data/timeline/backlog/resilience-card-self-cid-provide-loop-gate.md
  - elohim/elohim-storage/src/services/household_resilience.rs
requires_env: [household-nodes]
---

# Resilience Facings — select→fold→aggregate projection over the EPR holder-relation

> **One-line:** the resilience card, the shefa dashboard, the operator debug Lens, the
> reach/projection view and the EPR home are not five subsystems — they are **five named folds
> over one materialized relation** `(leg-neighborhood ∪ holder-relation)`, selected at an
> aggregation level. This spec makes that pipeline the substrate so a new facing is a registered
> fold, not a bespoke query path. It **refines** the lens spec's §2 ("everything is a projection
> of the one resolver") from *URL surfaces* to *audience facings*.

## 0. Provenance

Surfaced 2026-06-19 from the operator's framing while watching the live alpha resilience card read
near-zero (`stewardingCollectives 0`, `commitmentBackedCollectives 1`, `diversityScore 0`, `no
region data`, gap `contracts-short — 0 of 1`):

> *"we need clear dimensional handling of EPRs on the p2p-dataplane for the resiliency
> (user/hub-facing) / projection (collective-distributive/reach/doorway facing) /
> developer-operational (weave/tiers/PVC/storage) / REA (stewarded commitments & mutual compute
> agreements) / EPR-facing (what does all this look like from the content's perspective). What
> architecture helps us consistently model, compose, and humans drive those rich stories, and
> roll up that data deterministically into the resiliency cards, shefa dashboards, etc.?"*

This spec is the answer. It is **not** a fork of the lens spec — it extends its projection law to
the audience-facing read layer.

## 1. The gap: facings reach across legs + substrate with no shared model

The four-leg coupling law (lens spec §3: knowledge · value · governance · process) is enforced at
**compose**. But the **read** side — the cards and dashboards people actually look at — has no
shared projection model. Each facing hand-rolls its own Diesel against raw tables. Three concrete
symptoms, all live today in `elohim/elohim-storage/src/services/household_resilience.rs`:

- **Divergent duplicate joins.** `compute()` (lines 72–79) and `compute_regional_distribution()`
  (lines 458–472) run *the same* `shard_locations ⋈ humans` join twice, and the commitment side
  (174–198) is a third join — three reads of the holder graph, each shaped differently, none
  reconciled. (Edge-store divergence: the per-content stores even key differently —
  `mishpat_projection` keys on dwelling-hub-id, `peer_topology_view` + `household_resilience` on
  `household_id`.)
- **The per-household collapse throws away the per-peer dimension.** `compute()` collects straight
  into `HashSet<household_id>` (lines 83–84), so "how many distinct peers *within* a household hold
  this" — the intra-household resiliency the operator wants ("james ↔ matthew ↔ jessica") — is data
  that is *loaded and discarded*, not a missing query.
- **The `x of y` inconsistency.** The felt headline says "Held by N of the M households" (line 400)
  but only in `watching` state; the top-level `stewardingCollectives` is a bare count, and the
  placement-gap line independently reports "0 of 1 collectives". Two surfaces, two denominators,
  read from two stores → they disagree on screen.

The fold that *does* exist — `build_felt_status()` (lines 359–431), pure, no-DB, exhaustively
unit-tested (lines 528–696) — proves the cure already works. This spec generalizes it.

## 2. The pattern: select → fold → aggregate

**A facing is a pure fold over a materialized relation, selecting a subset of `(legs ∪
holder-relation)`, at an aggregation level.** Not an axis perpendicular to the legs — a *selection*
over them (Operational selects zero legs; REA a subset; EPR all). The pipeline:

```
materialize ( leg-neighborhood  ∪  holder-relation )   ONCE per request/snapshot
      │            (bounded closure-walk — lens §5: typed-relation set + depth cap + reach
      │             boundary — a cheap Rust walk; NO graph DB)
register named FOLDS    resiliency · reach · REA · operational · EPR  (each selects a subset)
      │
aggregate at a LEVEL    per-content  →  per-household  →  per-dashboard
```

This **refines the lens spec §2 table**. Where §2 says pillar mounts / `/raw` / claims-302 are
projections of the *resolver*, this spec adds: **audience facings are projections (folds) of the
same graph + the holder-relation.** Same law, read side.

| lens §2 projection | this spec's projection |
|---|---|
| `/epr/{cid}` = focal + all legs | **EPR facing** = fold over all four legs |
| `/lamad/path` = knowledge leg | **knowledge** sub-fold |
| `/epr/{cid}/raw` = graph neighborhood | the **materialized relation** itself (inspector) |
| (n/a — §2 is content-addressed) | **resiliency / reach / REA / operational** = folds over legs ∪ holder-relation, aggregated |

## 3. The materialized relations

**(a) The holder-relation (the new substrate primitive).** ONE query replacing Joins A + C,
keyed by the canonical agent identity, *retaining the per-agent dimension*:

```
HolderRow { hub_id, agent_id, region }   // v1 shipped shape; §11 names the framework target (adds content_cid, online measured-at-T)
```

- **Join key is `agent_cid`** (`uhCAk…`). Per `elohim-storage/CLAUDE.md` (Identity & Transport
  Coherence) and `household_resilience.rs:74`, `shard_locations.peer_id` *already holds* `agent_cid`
  (it is misnamed) and `humans.agent_pub_key` is `agent_cid` — so the join is namespace-coherent;
  the transport-id→agent_cid resolver is **NOT** needed (and is blocked anyway). The live empty
  card is a **NULL `humans.agent_pub_key`** data gap (§8), not a join bug.
- The grouping dimension is `hub_id` — the **hub abstraction** (household | dwelling | collective),
  the unifying edge-key that reconciles the divergent edge stores. v1 sources `hub_id` from
  `humans.household_id`; dwelling/collective grouping populates the same field without touching the
  folds (named-but-not-yet-wired).
- Folds derived from this relation: `households_stewarding` (distinct `hub_id`),
  **`intraHubPeers` (per hub, distinct `agent_id` — NEW, the discarded dimension recovered)**,
  `regional_distribution` (dedupe-by-hub, bucket by region — replaces the second join),
  `steward_collective_entries`.
- **v1 unifies the query DEFINITION, not yet the number of loads.** `snapshot()` loads the relation
  twice via the *same* `load_holder_relation` (once in `compute()` for the base folds, once for the
  intra/regional folds). The divergence (two differently-shaped joins) is gone; collapsing to a
  literal single load per request — threading the relation through `compute()` — is a noted follow-on,
  not done here. So "materialize once" (§2) is true of the query shape, not yet the call count.

**(b) The commitment relation (intent, distinct from observed).** `rea_commitments ⋈ humans` on
`agent_cid` (already correct, `household_resilience.rs:174–198`). Feeds `commitmentBackedCollectives`
(value leg) and the `contracts-short` gap, which honestly encodes **intent (committed) vs observed
(stewarding)** — a *feature*, not a bug.

**(c) The leg-neighborhood (for the EPR/REA/reach facings).** The bounded typed-relation
closure-walk from lens §5 (records-lifecycle vocab, depth cap, reach boundary). Knowledge/value/
governance/process legs as folds over the resource's anchored relations. v1 does not need this for
the resiliency facing (which folds (b)+(a)); it is the seam the other facings register against.

## 4. The five facings, resolved

| Facing | Audience | selects | aggregation level | surface |
|---|---|---|---|---|
| **EPR** | the content | all four legs (closure-walk) | per-content | `/epr/{cid}` lens-complete home |
| **Resiliency** | user / hub | commitment relation (b) + holder-relation (a) | per-content → per-household | resilience card |
| **Projection / reach** | doorway / collective | governance leg (reach) + distribution | per-content → per-collective | peer-topology, reach surfaces |
| **REA** | economic | value + process legs (commitments, `delegates-compute` mutual-compute agreements) | per-agent → per-household | shefa dashboard |
| **Operational** | developer | holder-relation (a) raw — weave/tiers/PVC/pantry | per-shard → per-node | operator debug Lens, placement-gaps |

Each facing names *which selection + which level*. Nothing reaches into a raw table ad-hoc again.

## 5. The determinism contract

"Rolls up deterministically" is true with one explicit caveat the spec MUST state, because the
cards advertise it:

- **Determinism is of the fold given a materialized snapshot.** Same materialized relation →
  identical card, every time (the `build_felt_status` property). Folds are pure; no ordering
  dependence, no per-metric divergent query.
- **Liveness is a measured-at-T input, not part of "same inputs → same card."** `onlinePeers.live`
  is time-varying; it enters the fold as an explicit `measured_at` field, never silently.
- **Honest-by-construction** carries from `build_felt_status`: `unmeasured → "not-yet-seen"`
  (never a fake verdict), floor-relative reassurance.
- **Rollup tier (T26 household→hub aggregator) is UNVERIFIED** as a deterministic fold-composition
  point. `project_node_metrics_vs_hub_aggregation_boundary` flags a per-node-vs-hub boundary. **Do
  not bank T26 as the rollup tier until a probe confirms it composes folds deterministically.**
  v1 aggregates per-content → per-household in-process; the hub tier is a follow-on gap.

## 6. Wire shape: typed view per facing (operator decision 2026-06-19)

Each registered fold produces a **typed ts-rs field/view** — not a generic `lenses: Vec<{name,
value}>` envelope. This honors lens §4 (`contentFormat` is in the CID; a resource self-certifies
what it is) and the `elohim-storage` boundary law (no `JSON.parse`, no case-conversion in TS;
camelCase at the wire). Cost: codegen per facing. Benefit: the card cannot silently drift, and the
schema-contract test catches shape changes. The generic envelope is the **rejected** option, with
lens §4 as the citation.

## 7. Engine: Diesel + Rust folds for v1 (Cozo deferred)

The closure-walk is bounded by construction (§3c), so the leg traversal is a cheap Rust walk — **no
second query engine.** The existing `#[cfg(feature = "graph-native")]` Cozo branch
(`api/resilience.rs:98–106`) returns *placeholders* for diversity/regional; adopting an incomplete
engine *to achieve determinism* is backwards. **One engine for v1: Diesel materialization + Rust
folds.** Reach for Cozo only if leg-traversal measures as a bottleneck.

## 8. P2P Design Gate output (run 2026-06-19)

**Verdict: clean read-projection — no new DHT entry type, no new identity, no new commitment.**

### Entity: holder-relation (materialized)
- **Classification:** Operational (C). One `Vec<HolderRow>` per request; reconstructable from
  `shard_locations` + `humans` (+ `collectives` for region). No `dht_anchor_hash`, no table.
- **Content Address:** rows carry `content_cid` (CID) + `agent_cid` (`uhCAk`); the relation itself
  is not addressed.
- **Join key:** `agent_cid` — both sides already hold it; never raw-compare against a transport id.
- **Anti-pattern caught:** cross-namespace string-equality (the all-zeros root). Corrected by
  documenting `agent_cid` as the one join key; resolver explicitly NOT used.

### Entity: `intraHubPeers` (new fold output)
- **Classification:** Operational (C) — a typed field on the resilience view, folded from the
  retained per-agent dimension. No new entity.

### Entity: facing views (resiliency/reach/REA/operational)
- **Classification:** Operational (C) read-views (lens §6 verdict stands; qahal-lattice §7: zero
  new DHT types). Typed ts-rs fields; extend existing endpoints, **no new `POST /api/v1/thing`**.

### Design constraints discovered (operator/security-owned — NOT autonomous fixes)
- **`humans.agent_pub_key` population is the live data gap AND a security gate.** It is written only
  by the seeder heal (gated on `/auth/me`, which 401s on server pods). The fix (boot self-session
  from the cell key) asserts "this pod IS human-X" *bypassing TOFU/portal trust*; the storage
  gospel **forbids consuming a self-asserted agent↔transport binding for economic attribution**
  (`commitmentBackedCollectives` IS economic attribution) until a cross-signed control proof lands
  (`2026-06-15-coherent-transport-identity-resolver-design.md`, blocked). → operator/security-owned;
  carries its own p2p-design-gate. This spec does NOT build it.
- **`shard_locations` has no seed write path.** `stewardingCollectives > 0` on live alpha needs
  real P2P `distribute_shards`, or the gated `PUT /admin/seed/shard-manifest` auditable-claim
  endpoint (operator-policy: which content × which stewards). This spec does NOT fabricate rows.

## 9. Slices (sequence)

- **Slice 1 — materialize the holder-relation + fold-refactor (proof test).** `load_holder_relation()`
  = ONE query (collapse Joins A + C), retaining `agent_cid`; refactor `households_stewarding` +
  `regional_distribution` into pure folds over it. Add the deterministic **proof-gate test** (the
  one `resilience-card-self-cid-provide-loop-gate.md` demands): seed coherent rows → assert
  `measured` + non-zero stewards + commitment-backed + named collectives + regional buckets. No
  behavior change to the lit values; this is the substrate + the executable proof.
- **Slice 2 — the intra-household fold (composability demonstration).** Add `intraHubPeers`
  (per household, distinct `agent_cid`) as a typed field + schema + ts-rs codegen, folded from the
  retained dimension. Test: two agents in one household → count 2. This is "a new lens = a new
  fold," proven.
- **Slice 3 — honest denominator consistency.** Surface the floor (`floor.wantsHouseholds`) as the
  single denominator across the top-line and the gap line, so `x of y` is consistent (kills the
  "0 vs 0 of 1" disagreement). View/UI touch.
- **Slice 4 — register the second facing (reach OR REA) against the materialized relation.** Proves
  the pipeline generalizes beyond resiliency. Follow-on.
- **Operator-owned (documented, not in these slices):** deploy+reseed `dev`→alpha; the `/auth/me`
  discriminating probe (settles the 2026-06-18 contradiction); the economic-attribution trust gate;
  the gated seed-shard-manifest policy; T26 hub-tier determinism probe.

## 10. Non-goals / boundaries

- Does **not** change what an EPR is, or the coupling law (lens §3) — read-projection only.
- Does **not** build the boot self-session / `agent_pub_key` auto-population (operator/security gate).
- Does **not** fabricate `shard_locations` or deploy/reseed (operator-owned).
- Does **not** adopt Cozo for v1, nor a generic untyped lens envelope.
- Does **not** bank T26 as the deterministic rollup tier until verified.

## 11. Lens Framework

This codifies the durable structure the child lens specs conform to: resiliency (the
proven reference, §1–§9 of this spec) plus four charter siblings —
`2026-06-19-reach-projection-facing-lens-design.md`,
`2026-06-19-rea-economic-facing-lens-design.md`,
`2026-06-19-operational-weave-facing-lens-design.md`, and
`2026-06-19-epr-content-perspective-facing-lens-design.md` (each refines this spec).
The decided shape: **a lens is a named pure free function that folds a materialized
relation into a typed view.** Not a trait, not a registry entry — a `pub fn`. The
purity boundary is made mechanical by a crate split, not left to convention.

### Module / crate structure

```
elohim-views   (exists; serde + ts-rs, NO diesel — verified)  ← typed wire views
   ▲
elohim-facings (NEW, pure)  — deny.toml: may depend ONLY on elohim-views, serde, chrono, std
   ▲
elohim-storage (server-only; the sole diesel touch — loaders take &conn)
```

`elohim-facings` is added to `deny.toml`'s graph and must **not** depend on
`elohim-storage` (else it inherits the server-only ban and the boundary collapses);
`elohim-views` must **not** gain diesel. **The one enforced rule:** every fold and
`assemble` takes materialized slices, **never `&mut SqliteConnection`**. Loading is
impure → storage-side; folding is pure → facings crate. Because the pure crate has no
diesel in its dependency graph, a `conn` *cannot* compile into a fold — **the compile**
makes the purity a fact (a `&mut SqliteConnection` won't resolve), not a code-review habit.
(cargo-deny is at most best-effort defense-in-depth: its `[bans.deny]` denies a crate
workspace-globally, which cannot express "facings may not use diesel" without breaking
storage's legitimate diesel — so verify it can scope per-crate before relying on it; the
Cargo.toml omission is the certain mechanism.) This is why the boundary is a crate split
rather than a module convention — and it matters more here because this is an
**agent-authored codebase**: convention-purity ("don't put a `conn` in a fold") erodes
under many authors over time; the diesel-free crate dependency graph is the only thing that
makes it stick.

### Canonical types + key signatures

The materialized-relation primitive carries liveness as a measured-at-T field (so the
fold stays pure given the snapshot). It uses §3's vocabulary — the join *value* is
`agent_cid` (`uhCAk…`), never a transport id; the grouping key is `hub_id` (household |
dwelling | collective), v1-sourced from `humans.household_id`:

```rust
// elohim-facings/src/relation.rs
pub struct HolderRow {
    pub content_cid: String,
    pub agent_id:    String,         // per-agent count key (v1: humans.id); the join VALUE is agent_cid (humans.agent_pub_key = shard_locations.peer_id), never a transport id (§3, §8)
    pub hub_id:      Option<String>, // grouping key; None drops the row (excludes null-hub)
    pub region:      Option<String>,
    pub online:      bool,           // liveness captured at materialization (measured-at-T)
}
```

There is exactly **one** generic layer — the fold combinators — and genericity stops
there:

```rust
// elohim-facings/src/fold.rs
pub fn bucket_by<R, K: Eq + Hash>(rows: &[R], key: impl Fn(&R) -> Option<K>) -> HashMap<K, Vec<&R>>;
pub fn distinct_count_by<R, K: Eq + Hash>(rows: &[R], key: impl Fn(&R) -> K) -> usize;
```

Per-facing assembly is a named free fn returning a typed `#[derive(TS)]` view from
`elohim-views`. `intra_hub_peers` becomes a composition of the combinators (the
composability proof) rather than a hand-rolled loop:

```rust
// elohim-facings/src/folds/resiliency.rs — pure; NO conn
pub fn assemble(snap: &Snapshot) -> ResilienceSnapshotView;
pub fn intra_hub_peers(rel: &[HolderRow]) -> HashMap<String, i32>; // bucket_by hub → distinct_count_by agent
```

The aggregation-level helper rolls per-content → per-hub via `bucket_by`. But per-hub →
per-dashboard rolls *derived verdicts*, which **cannot** go generic — it is a concrete,
hand-written `pub fn aggregate(views: &[ResilienceHubView]) -> DashboardView` per
facing. **Genericity ends at the relation layer; verdict-rollup is per-facing.** The
HTTP router dispatches by `match facing { … }`, never `Json<Value>` — snake_case never
leaves Rust.

### Add-a-lens recipe (the whole point — it's cheap)

1. Add `XRow` to `relation.rs` (plain struct; capture any liveness into a field).
2. Define typed `XView` in **`elohim-views`** (`#[derive(TS, Serialize)]`, camelCase);
   run `cargo test export_bindings`.
3. Write `pub fn load_x_relation(conn, …) -> Result<Vec<XRow>>` in
   `elohim-storage/src/db/` — **the only diesel touch.**
4. Write `pub fn assemble(rel: &[XRow], …) -> XView` in `elohim-facings/src/folds/x.rs`
   using the shared combinators; **unit-test with hand-built `Vec<XRow>`, NO DB.**
5. Add `aggregate()` in the same file *only if* the lens has a dashboard level.
6. Wire one HTTP route + one `is_service_path` arm (the doorway-shadow trap;
   `project_doorway_main_route_needs_is_service_path`) + the `INTERFACE_FILES` entry.

No trait to implement, no registry to register in, no matrix to fill.

### Migration of household_resilience.rs (behavior-preserving)

The 33 integration tests in `tests/household_resilience.rs` call only
`household_resilience::snapshot` (18×) and `household_resilience::compute` (6×). Both
take `&pool` — they *load*, so they are impure and **cannot move** into the pure crate.
They **stay in `household_resilience` as thin adapters with unchanged signatures**: load
rows (diesel) → call `elohim_facings::*` → return the view. That alone keeps all 33
green (the folds are `pub(crate)`, invisible to the separate test compilation unit — no
re-exports needed). Steps:

- **(a)** `git mv` the pure folds — `build_felt_status`, `floor_for_tier`,
  `intra_hub_peers`, and the `HolderRow` struct — into `elohim-facings`, flipping
  `pub(crate)` → `pub`. The 10 `felt_status_tests` move *with* `build_felt_status`,
  proving the DB-free boundary held.
- **(b)** Extract the two not-yet-pure folds into named pure fns: the inline stewarding
  `HashSet` collect in `compute()`, and the bucketing loop in
  `compute_regional_distribution`. The manifest/region *load* stays storage-side; only
  the bucket *loop* moves.
- **(c)** `load_holder_relation` + `count_online_peers_in_households` stay storage-side
  (they take `conn`).
- **(d)** Gate the cutover on **byte-identical JSON** before/after (mirror the
  sha256-the-generated-TS discipline). Green tests + identical bytes = behavior
  preserved.

The cross-crate ts-rs trap (CLAUDE.md's `../../../` breakage) **cannot bite**: no
`#[derive(TS)]` type moves — the views already live in `elohim-views`, which has no
diesel. C's deferred db-access trait (relocating `snapshot()` behind a trait) is **not**
done now; its trigger is a second non-storage consumer.

### What stays out (the rejected over-reification)

- **A `Lens` trait** — the primitive is a named free fn; a trait wraps it for zero
  polymorphism (dispatch is a `match`, not `dyn`).
- **A `dyn` registry / uniform dispatch** — not object-safe with a typed associated
  view; the only escape (`serde_json::Value`) violates §6's typed-view decision.
- **A `RelationSource` / db-access trait in v1** — speculative with one consumer;
  `&[Row]` slices already give injection + DB-free testing.
- **A facing×leg matrix** or a `Relations` god-struct — per-facing tuples only.
- **Cozo for the EPR closure-walk** (§7) — a bounded walk is a cheap Rust walk.
- **A generic aggregation helper for verdict-rollup** — genericity stops at the relation
  layer.

**Scope caveat:** this framework gives each facing a *home*; it does not author the
per-facing data gaps (reach vocabularies, REA's two-ledger split, EPR's 4th process leg,
operational's missing gauges). Those are loader-and-view work — see §8 and §10.

## 12. Doc-sync convergence is a CLIENT-COMPOSED sibling signal — NOT a server-fold lens (decision 2026-06-27)

When the Automerge content-sync plane was lit (`plans/2026-06-27-automerge-content-sync-plane-lighting-plan.md` — a producer fills the DocStore under `h_app_id="elohim"`; `/sync/v1` carried by doorway), the question arose: surface per-content doc-sync convergence on the resilience card as a §11 add-a-lens server fold? **Decided NO — it is a client-composed sibling signal, deliberately outside this framework's select→fold→aggregate.** Three reasons:

1. **Plane separation (anti-lie).** This framework folds the **blob-custody** plane (`shard_locations ⋈ humans`, RS-encoded holders). Doc-sync is the **Automerge CRDT** plane (`node:{contentId}` in the sled DocStore). Their verdicts genuinely diverge (a doc fully converged on ONE peer is not custody-resilient; a doc behind on heads can still be RS-distributed to 6 households). A green "synced" badge folded into the custody verdict would mask an at-risk custody verdict. The card renders it as a **separate, labeled row** ("Doc sync" / "document version"), never inside the resilience `<dl>` verdict.
2. **Determinism contract (§5/§7).** The facing contract is "same materialized Diesel relation → identical card; liveness is an explicit measured-at-T input." Doc-sync state lives in sled behind the **async** `SyncManager` — threading it through the synchronous `compute_base`/folds crosses both the sync→async boundary and the second-engine boundary §7 deliberately refused. Server-fold is strictly heavier (schema field + struct + schema_contract test + ts codegen + async-into-sync plumbing) **and** wronger.
3. **Already client-reachable, zero server change.** The data is per-content and the `/sync/v1` routes are already doorway-carried. The Angular `ContentDocSyncService` already polls them; the card just consumes the service. The "synced across M peers" count is NOT exposed (lives only in the unexposed `StreamTracker::StreamPosition`), so the indicator reads per-content **present (synced) / null (pending) + `updatedAt`**; cross-peer arrival is proven by test, not a peer-count field. Aggregate ("N of M converged") would need that unexposed tracker → a new route → net-new substrate, so it is a separate sibling facing if ever wanted (rejected here unless reframed).

This is the doc-sync analogue of "What stays out" above: the server-fold sibling facing is the rejected over-reification for this plane.
