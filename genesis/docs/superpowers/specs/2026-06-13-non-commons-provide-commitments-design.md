---
id: non-commons-provide-commitments-design
status: design
created: 2026-06-13
class: substrate
artifact_kind: spec
written: 2026-06-13
# cites: TODO(integrator) — cite-seal is the finishing step (seal/describe/propagate).
#   The four design inputs below are the cite targets; fingerprints are integrator-run.
#   - 2026-06-08-epr-acquisition-slice2b-provide-loop-design.md | the commons provide loop this generalizes — replicates-commons action, ProvideReconciler, provide_projection_for; the direct ancestor | path: genesis/docs/superpowers/specs/2026-06-08-epr-acquisition-slice2b-provide-loop-design.md
#   - 2026-06-12-init-authoring-native-seeding-design.md | the per-corpus author-steward routing + story-derived collective graph this design's eligibility-and-visibility model must align with | path: genesis/docs/superpowers/specs/2026-06-12-init-authoring-native-seeding-design.md
#   - resilience-protocol-spec | Part V's three-class stewardship surface (encrypted / social / commons); the non-commons provide row lights the "social" class | path: genesis/docs/content/elohim-protocol/resilience/README.md
#   - reach-vocabulary-frontend-strand | the reach vocabulary is in known multi-way drift; this design references the drift, does NOT canonize a reconciliation | path: genesis/data/timeline/backlog/reach-vocabulary-frontend-strand.md
---

# Non-commons provide commitments — commitment-backed counting past commons reach

**Date:** 2026-06-13
**Status:** Design (decided shape; no production code)
**Owner surfaces:** the `replicates-commons` provide loop (`services/provide_reconcile.rs`),
the side-projection (`mishpat_projection::provide_projection_for` →
`db/rea_commitments::record_provide_from_commons_commitment`), the resilience snapshot
reader (`services/household_resilience.rs::snapshot`), the receiver-side consent gate
(`p2p/reach_authorization.rs::classify_pre_authorization`), and the Mishpat `Commitment`
coordinator/integrity validators.

> **Companion to** the Slice-2b provide-loop spec (`2026-06-08-epr-acquisition-slice2b-provide-loop-design.md`)
> — that spec landed `replicates-commons` and the `content:commons` provide row; this spec
> closes the *reach generalization* it explicitly deferred ("Dwelling-tier escalation …
> distinct action, implemented when the consent surface exists", §13). The consent surface
> now exists (`classify_pre_authorization`), so the deferral resolves.

---

## 1. The gap, stated precisely

The resilience snapshot's `commitment_backed_collectives` counts distinct households holding
an `active` `provide` commitment whose `resource_classified_as` equals `content:<reach>`,
where `<reach>` is read from the content row:

```rust
let content_reach: String = content::table…select(content::reach)…;   // household_resilience.rs:160
let scope = format!("content:{}", content_reach);                       // :166
…filter(rea_commitments::resource_classified_as.nullable().eq(&scope)) // :179
```

Every `content:<reach>` provide row in production is minted by exactly one path: the
side-projection in `signals.rs:872` fires `provide_projection_for`, which hard-codes
`reach = "commons"` (`mishpat_projection.rs:507,535`) because `replicates-commons` is
reach-gated to commons at the DNA validator (`commitments.rs:233,252`) and at parse time.

**Consequence:** content authored at `household` / `intimate` / `community` / any
non-commons reach can *never* show a commitment-backed collective. The snapshot's
`diversity_score` for such content collapses to `min(households_stewarding, 1) / 7`
(the `commitment_backed_collectives.max(1)` floor at `:198`) — it counts raw stewarding
peers with no notarized backing. Non-commons content is structurally invisible to the
commitment-backed half of the resilience verdict.

This is the **last open strand** of the commitment-backed counting work. Commons is done;
the init-authoring spec's per-corpus routing (`§b`) means most genesis content is *not*
commons — the FCT corpus routes to the church collective at `community` reach, household
content sits at `household` reach. Those are exactly the rows that render
`commitment_backed_collectives: 0` today.

---

## 2. Why commons was the easy case (and what changes off-commons)

The commons provide loop works without a consent question because **commons is openly
providable by anyone**. The receiver-side pre-authorization gate
(`reach_authorization.rs`) always pre-authorizes Public/Commons of any pillar
("integrity topics + Public/Commons of any pillar are always pre-authorized") —
*every node carries the commons*. So "may this peer provide commons content X?" is
unconditionally yes, and the only eligibility predicate the reconciler needs is
`pin.caught_up && content.reach == "commons"` (`provide_reconcile.rs:7-8,394-395`).

Off-commons, two new questions appear that commons never had to answer:

- **(Q-eligible) Who is *allowed* to provide this content?** A household-reach learning
  record is not openly providable; a peer offering to serve it must have *embodied
  responsibility* for that scope — membership in the household/collective, a stewardship
  relationship, a delegation. This is precisely what `node_has_embodied_responsibility`
  already decides for receiver-side pre-authorization. A provide commitment a peer is not
  eligible to make is a privacy violation, not just a no-op.

- **(Q-visible) Who may *see* the provide commitment itself?** The commitment is metadata
  about a private resource ("agent A provides household-Eden content X"). At commons reach
  the commitment is public by nature. At household/intimate reach, the *existence and
  shape* of the commitment is itself reach-restricted data — leaking it leaks the social
  graph. The commitment's visibility must not exceed the reach of the content it provides.

These two questions — not the projection mechanics — are the real content of this design.
The projection generalizes trivially (§5); the consent model is the substrate decision (§4).

---

## 3. P2P design gate (invoke explicitly)

Per the mandatory gate, answered before any route or table is touched.

| # | Gate question | Answer |
|---|---|---|
| 1 | **Entity classification** | The provide commitment is **Notarized (A)** — existing Mishpat `Commitment`, action-discriminated. The `rea_commitments` provide row it projects is **Operational (C)** projecting A (a read-cache of the notarized fact). The eligibility decision is **Operational (C)** — recomputed on read from the graph, never persisted as its own truth. No new entity is notarized. |
| 2 | **Does a DHT entry type already exist?** | **Yes — reuse, do not mint.** Mishpat `Commitment` (`mishpat_integrity/src/lib.rs:275`) is the single entry type; the `action` field discriminates. Mishpat carries **9 entry types** (Precedent, Discussion, GovernanceState, GraduatedFeedback, OpinionStatement, Place, ChallengeOutcome, Commitment, StringAnchor) — abundant headroom (~9/100), but headroom is *irrelevant*: the REA compute-commitment primitive principle is one substrate primitive instantiated across contexts. A new provide entry type would be the cardinal sin. The decision (§4) is whether the new reaches are a **parameterized `replicates-commons`** (reach in payload) or a **new action** — and it lands on parameterize, renamed (§4.1). |
| 3 | **Identity scheme** | `Commitment` CID = **`entry_hash`** (never `action_hash` — the bounds-gate/fetch key; returning action_hash silently breaks every bounds check yet passes per-task tests, per `project_mishpat_commitment_cid_is_entry_hash`). The projected provide row keys on `provide_projection_id(provider, reach)` = `provide-{provider}-content:{reach}` — already parameterized by reach, already idempotent per `(provider, reach)`. |
| 4 | **Coordinator fn + projecting signal** | Coordinator: the existing `create_commitment` (Mishpat `commitments.rs`), with the validator generalized (§4.2). Signal: the existing `CommitmentCommitted` post-commit signal → `signals.rs:858` `parse_commitment_payload` → `provide_projection_for` → `record_provide_from_commons_commitment` (renamed §5.1). **Zero new coordinator fns, zero new signals, zero new tables, zero new entry types.** |

Gate verdict: **clean pass on reuse.** The whole design is action-payload generalization +
a consent predicate + a projection rename. The one integrity-zome change (§4.2, §7) is a
DNA-hash-moving change and is called out for governance.

---

## 4. The core decision: parameterize the existing action, gated by consent

### 4.1 One action, reach in the payload — NOT a new action per reach

**Decided: the provide commitment stays a single action carrying `reach` in its payload,
not a family of distinct actions (`replicates-household`, `replicates-community`, …).**

Rationale, one line each:
- **The REA primitive is reach-agnostic by design** — Part V: "the same protocol shape …
  the only thing that changes is who the counterparties are and what reach the Resource
  carries." Minting an action per reach fragments one primitive into N.
- **The validator already variant-dispatches** — `validate_replicates_commons` branches on
  `variant` (content|capacity); adding reach is one more field, not one more dispatch arm.
- **The projection already parameterizes by reach** — `provide_projection_id(provider, reach)`
  and `record_provide_from_commons_commitment(…, reach, …)` take reach as an argument; the
  only reason it's always `"commons"` today is the hard-code in `provide_projection_for`.
- **The bounds-validator's `reach_ceiling` is already the generalization seam** — bounds
  carry `reach_ceiling`; the commons validator pins it to `"commons"`; generalizing means
  *accepting a non-commons ceiling under a consent precondition*, not a new code path.

**Naming consequence (DECIDED):** the action name `replicates-commons` becomes a misnomer
once it carries non-commons reach. **Rename the action to `replicates-content`** (the
reach-general content-provide action), keeping `replicates-commons` as an accepted alias at
the validator for one migration window so in-flight commons commitments still validate.
The action is the *what* (replicates content at a declared reach); the reach is a payload
field, not the action identity. `replicates-dwelling` (the household-hub replication
instance) stays distinct — it is a different REA shape (co-stewardship, two-party consent),
not a reach variant of content provide.

> Rename is a coordinator+integrity+schema+projection touch. It is the bulk of the DNA work
> and the reason this is staged (§7). If the operator prefers to keep `replicates-commons`
> as the literal action string and *only widen its reach gate*, that is a smaller change with
> a permanently-misleading name — flagged as an OPEN operator choice in §9.

### 4.2 The reach gate generalizes from "must be commons" to "must be consent-cleared"

Today the DNA validator hard-rejects non-commons:

```rust
if payload["reach"].as_str().unwrap_or("") != "commons" {        // commitments.rs:252
    return Err("replicates-commons content reach must equal 'commons'".into());
}
if bounds["reach_ceiling"]… != "commons" { return Err(…) }       // :233
```

The generalized validator (DNA-side) **cannot do the consent check** — it has no graph
access (HDI validators cannot `get_links`, per `project_hdi_no_get_links_in_validators`,
and the embodied-responsibility walk is a coordinator/storage-side graph traversal). So the
DNA-side rule weakens to a **structural** check only:

- `reach` ∈ the schema reach enum (a known value — see the vocabulary-drift caveat §6);
- `reach_ceiling` ≥ `reach` (a commitment may not promise wider reach than its content) —
  ordinal comparison against the schema enum;
- commons stays openly admissible (no precondition);
- **non-commons is admissible at the DNA layer but carries the consent burden downstream.**

The **consent enforcement** lives where the graph lives: the **storage-side eligibility
predicate** in the provide reconciler (§4.3), defense-in-depth at *author time* (the
reconciler refuses to author a commitment the node is not eligible to make) — mirroring the
commons loop's "rejected at mint AND at emit" discipline, except here "mint" is gated by
graph-eligibility rather than a constant. This is the substrate-floor / elohim-ceiling line:
the DNA floor checks structure (well-formed, ceiling-bounded); the eligibility *discernment*
(does this node have embodied responsibility for this scope?) is a graph judgment one layer
up. It is deterministic graph membership, not policy — so it stays in the substrate
reconciler, not the elohim ceiling.

### 4.3 Who authors it, and when — the non-commons eligibility predicate

The commons reconciler's desired set is `caught_up_head_refs ∩ commons_head_refs`
(`provide_reconcile.rs:394-395`). The generalization replaces the `commons_head_refs`
filter with a **reach-aware eligibility filter**:

> A caught-up pin on content at `reach R` is provide-eligible iff **either** `R == commons`
> (openly providable, unchanged) **or** this node has **embodied responsibility** for the
> content's scope at reach `R` — i.e. `node_has_embodied_responsibility(pillar, R, scope)`
> returns pre-authorized (`reach_authorization.rs::classify_pre_authorization`).

The predicate reuses the *exact* function that already decides receiver-side
pre-authorization — the symmetry is the point: **a node may provide a scope iff it was
already pre-authorized to receive/steward/validate that scope.** You can only offer what you
have standing to hold. Concretely (Stage 1, structural floor):

- the node has a local agent with a `peer_identity_bindings` row attesting standing in the
  scope (household membership, collective membership, stewardship relationship), per
  `reach_authorization.rs` Stage 1;
- Stage 2/3 (the graph walks — qahal household memberships, imagodei attestations) deepen
  this exactly as they deepen receiver-side pre-authorization; no separate ladder.

This grounds eligibility in the **story-derived collective graph** the init-authoring spec
names (`§b.1`): matthew/jessica/james each have embodied responsibility for
`household-dowell` content; adam for `household-eden`; the church members for
`community-local-church`. A Dowell node is eligible to provide a household-Eden record
**only** if a cross-collective stewardship edge exists (adam *is* matthew's recovery
custodian — `§b.1` — so that edge is real for recovery shares, absent for arbitrary
household content). The eligibility predicate makes the social graph load-bearing for
provide, the same way it is load-bearing for receive.

### 4.4 Who may see the commitment — visibility bounded by content reach

(Q-visible from §2.) The Mishpat `Commitment` is a DHT entry — gossiped, visible to the
DHT. For commons that is correct. For a household-reach provide commitment, the commitment's
*payload* (provider, head_ref of a private content) must not leak past the content's reach.

**Decided: the non-commons provide commitment carries no plaintext private identifiers in
the gossiped entry.** The commitment references the content by its **EPR CID (head_ref)**,
which is already a content-addressed opaque hash, not a title or human name; the `reach`
field declares the class but not the membership; the provider is the author's agent key
(public by nature — the source-chain author is always visible). What must NOT appear:
counterparty human identifiers, household labels, or content titles. The commitment says
"agent A replicates CID X at reach household" — A and X are already public-safe; "household"
is a class, not a member list. The *binding of X to a household* lives behind the reach gate
on the content entry, not on the commitment.

This means the commitment entry shape is **identical across reaches** — only the `reach`
value differs — and no new privacy machinery (encryption, selective disclosure) is needed
for v1. The receiver-side pre-authorization gate already prevents a non-eligible node from
*resolving* CID X to its content; the commitment leaks only the opaque CID + reach class,
which is the same information a Kad provider record already exposes. **Open** (deferred §9):
whether reach `intimate`/`private` provide commitments should be on a private source-chain
entry rather than gossiped at all — that is a stronger privacy posture than v1 needs for the
counting gap, and it changes the projection path (private entries don't gossip to other
peers' projections).

---

## 5. The projection — how `provide_projection_for` generalizes

The side-projection is **already reach-parameterized**; only the hard-code is removed.

### 5.1 `provide_projection_for` (mishpat_projection.rs:524)

```rust
// today:
reach: "commons".to_string(),            // :507 doc, :535 value — hard-coded

// generalized:
reach: row_reach,                        // read the commitment's declared reach
```

The reach is read from the parsed commitment payload (the `content` variant already carries
a `reach` field — `commitments.rs:241` requires it; today it must equal `"commons"`, after
§4.2 it may be any consent-cleared reach). `provide_projection_for` returns `Some` for any
`replicates-content` (renamed) **content** variant with a non-empty recipient; the capacity
variant still returns `None` (a byte pledge, no per-content offer — unchanged).

The function name and its sibling `record_provide_from_commons_commitment` are renamed to
drop "commons" (`record_provide_from_content_commitment`); the doc comments lose the
"always `commons` today" qualifier. **No signature change** — `reach` is already a parameter
of `record_provide_from_commons_commitment` (`rea_commitments.rs:361`).

### 5.2 What the snapshot counts per reach — UNCHANGED

The snapshot reader needs **no change**. It already reads `content::reach`, builds
`content:<reach>`, and counts distinct `household_id` over `provide` rows at that scope
(`household_resilience.rs:160-188`). The moment non-commons provide rows exist, the existing
query counts them. This is the elegance of the parameterized-id scheme: the snapshot was
*always* reach-general; only the producer was reach-pinned.

`provide_projection_id(provider, reach)` already yields one row per `(provider, reach)`, so a
provider stewarding both household and community content gets two provide rows — counted
independently per content's reach, exactly as the snapshot scopes. No double-count: a
provider with many household pins collapses to one `provide-{provider}-content:household` row
(`insert_or_ignore`, `rea_commitments.rs:392`).

### 5.3 The substrate-owned junction gap (named, not papered)

The snapshot's count joins `rea_commitments.provider` → `humans.agent_pub_key` and filters
`humans.household_id IS NOT NULL` (`:172,184`). Per `project_resilience_snapshot_humans_junction`,
**no HTTP create surface sets `humans.household_id`** — it is Epic-B ingestion work. So a
non-commons provide row lights the count **only** when the provider's `humans` row carries a
`household_id`. This design *produces the provide rows*; it does not invent the junction
write path. The init-authoring spec's `§b` decision (which agent authors which corpus, which
collective's junctions light) is the upstream dependency: when Epic-B ingestion derives the
`household_id` junctions from the story-derived graph (`§b.1`), these provide rows become
countable. Until then, a non-commons provide row is *correct-but-dormant* — it projects
honestly and counts zero until the junction lands, never a fake number (the
correct-but-dormant discipline; never wire a guaranteed no-op, but a correct projection
awaiting its upstream producer is honest and unblocks the consumer the moment data arrives).

---

## 6. Reach vocabulary — reference the drift, do not canonize

The reach value the commitment carries, the `content::reach` the snapshot reads, and the
`Reach` enum the consent gate consumes (`elohim_epr::Reach`) are in **known multi-way drift**
— the schema-8, the Rust services enum, the resilience-epic Part V vocabulary, and the
TypeScript geographic-8 are mutually inconsistent (`reach-vocabulary-frontend-strand.md`;
resilience README roadmap item 13). **This design does not reconcile them.** It makes two
narrow commitments that survive whatever reconciliation lands:

- The commitment's `reach` value is whatever `content::reach` already holds for the content
  being provided — **read-through, never re-vocabularized.** The provide row's
  `content:<reach>` scope is a string interpolation of that same value; producer and consumer
  use the identical vocabulary by construction (both read `content::reach`), so the provide
  row can never drift from the content it backs even while the *cross-layer* vocabularies
  drift.
- The DNA-side `reach_ceiling ≥ reach` ordinal check (§4.2) needs an ordering. It uses the
  **schema enum's** ordinal (`reach.schema.json`, matched by `elohim/epr/src/reach.rs`) —
  the DNA-notarized vocabulary — because that is the one the DNA already validates content
  reach against (`content_store_integrity` validates against the schema-8). The check is
  scoped to the one vocabulary the DNA owns; it makes no claim about the others.

When roadmap item 13 reconciles the vocabularies, this design's read-through stance means the
provide loop inherits the reconciliation for free (it never hard-coded a reach beyond
`commons`, which is a fixed point of every vocabulary).

---

## 7. Staging — storage-side-only vs DNA-hash-moving

The DNA-hash governance line (`2026-06-11-dna-upgrade-governance.md`) splits the work cleanly.

### Stage A — storage-side only, NO DNA change (ships first)

Everything here is coordinator-blind and DNA-hash-neutral:
- Remove the `"commons"` hard-code in `provide_projection_for` (§5.1) — read the payload reach.
- Rename `record_provide_from_commons_commitment` → `…_content_commitment` (and
  `provide_projection_for`'s doc) — pure storage rename, sweep callers crate-wide.
- Add the reach-aware eligibility filter to the provide reconciler's desired-set computation
  (§4.3), calling the existing `classify_pre_authorization` — storage-side graph read.
- **No snapshot change** (§5.2).

> **But:** Stage A alone produces non-commons provide rows **only if** a non-commons
> `replicates-content` commitment is already landing in the projection — which it cannot,
> because the DNA validator still rejects non-commons reach (`commitments.rs:252`). So Stage A
> is necessary-but-not-sufficient; it makes storage *ready* and is independently testable with
> a synthesized non-commons commitment row (test_util), but the end-to-end path needs Stage B.

### Stage B — Mishpat DNA change, MOVES THE DNA HASH (governance-gated)

The integrity-zome change is the reach-gate generalization (§4.2):
- Coordinator `validate_replicates_commons` (rename to `validate_replicates_content`):
  replace the `reach == "commons"` hard-reject with the structural `reach ∈ enum` +
  `reach_ceiling ≥ reach` check; keep `replicates-commons` as an accepted action alias for
  one migration window.
- Integrity `mishpat_integrity` defense-in-depth substring arm: accept the renamed action,
  drop the commons-only reach assertion (structural only — no graph access in HDI).
- Schema `replicates-commons.schema.json` → `replicates-content.schema.json`: `reach` enum
  widens from `const: "commons"` to the schema reach enum; `reach_ceiling` likewise.

This is an **integrity-zome change → the DNA hash changes → a network event** (peers on
different hashes are different DHTs → partition). It cannot ride a normal edge redeploy
(`ALLOW_DNA_REINSTALL` gating, per root CLAUDE.md). On alpha, the genesis bootstrap pair
(adam + matthew) must both get the flag or they land on different DHTs. **This is the gating
governance decision and is operator-owned** — the spec stops at naming it.

The coordinator/integrity split matters here: the validator generalization is in the
*coordinator* zome (`commitments.rs`) — a coordinator-only change hot-swaps via
`update_coordinators` (no re-key, no DHT churn, `sync_coordinators` / `ALLOW_COORDINATOR_UPDATE`,
per `project_dna_hash_blind_to_coordinator_zomes`). The **integrity** defense-in-depth arm
change is what moves the hash. **OPEN (§9.2):** if the integrity arm can be left commons-only
— relying on the coordinator validator + storage eligibility to gate non-commons — Stage B is
**coordinator-only and hot-swappable**, no DNA-hash move, no reinstall ceremony. That is a real
architecture fork worth the operator's call: defense-in-depth (integrity also enforces, hash
moves) vs. hot-swappable (integrity stays permissive on reach, coordinator + storage enforce).

### Stage C — Epic-B junction dependency (not this spec)

The provide rows count only when `humans.household_id` is populated (§5.3). That ingestion is
Epic B, driven by the init-authoring spec's `§b`/`§b.1` story-derived graph. Named dependency,
out of scope.

---

## 8. Testing (CI-aware)

- **Storage unit (local + the §2.1-of-slice2b CI gap caveat applies — no CI stage runs
  `cargo test` on `elohim-storage`):** `provide_projection_for` yields the content's reach
  (not hard-coded commons) for household/community content;
  `record_provide_from_content_commitment` writes `content:household` scope; the eligibility
  filter admits a node with embodied responsibility and rejects one without (mock the
  `classify_pre_authorization` seam).
- **Snapshot:** seed a non-commons provide row + a provider `humans` row with a `household_id`;
  assert `commitment_backed_collectives` counts it at the content's reach; assert a provider
  *without* `household_id` counts zero (correct-but-dormant honesty).
- **DNA sweettest (CI-covered, `--run-ignored all`):** a `replicates-content` commitment at
  `reach=household` commits and projects with non-null `dht_anchor_hash`; `reach_ceiling < reach`
  is rejected; the `replicates-commons` alias still validates (migration-window compat).
- **a2o:** one scenario — *non-commons content shows commitment-backed protection*
  (`genesis/a2o/features/resilience/non-commons-provide-counting.feature`): author household
  content, an eligible peer provides it, the snapshot reads a non-zero commitment-backed
  count. Household floor by default (M/J/J); cross-region breadth `@requires:shem`.
- **Per-item failure isolation preserved** — one bad commitment skips with a logged reason,
  never aborts projection (the EprRouter poisoned-row lesson; the side-projection is already
  non-fatal at `signals.rs:890`).

---

## 9. Open questions / operator decisions

1. **Action rename vs gate-widen-only (§4.1).** DECIDED-pending-operator: rename
   `replicates-commons` → `replicates-content` (clean name, more churn) vs keep the literal
   action string and only widen its reach gate (smaller change, permanently misleading name).
   The spec recommends the rename with a one-window alias; the operator owns the churn/clarity
   trade.
2. **Stage B: integrity change vs coordinator-only (§7).** Can the non-commons reach gate live
   entirely in the coordinator + storage eligibility (hot-swappable, no DNA-hash move), leaving
   the integrity defense-in-depth arm commons-only? If the integrity arm must enforce
   non-commons structurally, the hash moves and the alpha bootstrap-pair reinstall ceremony is
   required. This is the single biggest cost fork — operator call.
3. **Private/intimate provide commitments: gossiped vs source-chain-private (§4.4).** v1
   gossips the commitment (opaque CID + reach class only, which is provider-safe). A stronger
   posture puts intimate/private provide on a private source-chain entry — but private entries
   don't project to *other* peers' storage, which changes how the count federates. Deferred;
   needed only if leaking "agent A provides CID X at reach intimate" is judged too much.
4. **`reach_ceiling ≥ reach` ordinal source (§6).** Uses the schema enum's ordinal because the
   DNA already validates content reach against it. If roadmap item 13 reconciles to a different
   canonical ordinal, this check inherits it — but until then it is scoped to the one vocabulary
   the DNA owns. Not a blocker; a note for the reconciler.
5. **Eligibility at Stage 1 is structural** (`peer_identity_bindings` row). The Stage 2/3 graph
   walks (qahal memberships, imagodei attestations) are the same deferral the consent gate
   itself carries (`reach_authorization.rs` "Stage 2/3 — open question O2"); this design rides
   that ladder, does not get ahead of it.

---

## 10. Out of scope

- **Reach vocabulary reconciliation** (roadmap item 13) — referenced, never canonized (§6).
- **`humans.household_id` junction ingestion** — Epic B, init-authoring `§b`/`§b.1` (§5.3).
- **`replicates-dwelling` (household-hub co-stewardship)** — a distinct REA shape (two-party
  consent), not a reach variant of content provide; its own action, already designed.
- **Stronger commitment privacy** (encryption / selective disclosure / private source-chain
  entries) — §9.3, deferred; v1's opaque-CID-only gossip is sufficient for the counting gap.
- **Capacity-pledge non-commons** — the capacity variant is a byte pledge with no per-content
  offer and no reach counterparty; it stays commons-ratio-attested, unchanged.
- **The cross-node serve mechanics** (does an eligible provider actually serve non-commons
  bytes to an authorized peer?) — that is the receiver-side resolution path, gated by the same
  pre-authorization; this spec closes the *counting* gap (the commitment-backed verdict), not
  the serve path, which is its own slice.
