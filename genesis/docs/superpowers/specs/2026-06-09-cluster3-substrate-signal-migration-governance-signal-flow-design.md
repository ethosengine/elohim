---
title: substrate_signal Migration & Self-Referential Governance Signal-Flow
id: cluster3-substrate-signal-migration-governance-signal-flow-design
status: design
created: 2026-06-09
cluster: 3 of 3 (attention-substrate program)
substrate_scope: household-nodes (v1; no shem dependency)
companion: 2026-06-09-per-substrate-limitarian-governor-design.md (Cluster #1)
sibling: 2026-06-09-cluster2-sacredness-surface-firewall-anti-capture-design.md (Cluster #2)
note: >
  Inline file:line are draft pointers against feat/native-content-graph-seam.
  cite-seal (cite-gen.py --seal) is the finishing step. Corrected against an adversarial
  review that found the original migration plan targeted a fixture-only code path
  (verdict: does-not-hold) — fixes marked [adversarial-fix].
---

# Cluster #3 — `substrate_signal` Migration & Self-Referential Governance Signal-Flow

> Cluster #1 designed *what the limitarian governor governs*; this doc designs **the data spine it rides**
> and **the signal plumbing that lets governance sense its own state** — the reflexive property, proven for
> content-attention, extended to governance itself.

> **Provenance:** synthesized from a grounded multi-agent pass, then **corrected** against three adversarial
> reviews. The headline correction: the migration must land through the **production write path**
> (`upsert_with_anchor` / `NewEconomicEvent`), not the fixture-only generic projector. Fixes are marked
> **[adversarial-fix]**.

---

## 0. The honest one-paragraph version

Two seams disagree silently. (1) `substrate_signal` is declared in the enum schema and the manifest layer but **has never crossed into the Rust event substrate** — no integrity field, no validator, no View, no column. (2) A passed governance tally writes a SQL status and **emits nothing**, so governance signals cannot flow or aggregate. This doc lands the field through the **real production path** and adds a single **emit-on-transition** seam that simultaneously closes Cluster #1's dead ratification seam and lets governance ride the same k-anon aggregation machinery that senses content-attention. **The genuine fork it forces** (and the original synthesis missed): a first-class `substrate_signal` *column* changes the DNA hash (alpha-pair partition risk) — while a `metadata_json` *key* would ride every existing path with **zero DNA-hash change**. That choice, and the governance "votes auditable-vs-private" choice, are the two decisions only the operator can make.

---

## 1. The two silent seams

**Seam 1 — `substrate_signal` is three artifacts that disagree** (verified):
- Enum schema declares the 7-value vocabulary + names a `_dna` constant (`substrate-signal.schema.json:7,14-19`).
- Codegen emits `CORE_SUBSTRATE_SIGNALS`/`ALL_SUBSTRATE_SIGNALS` (`generated_enums.rs:372,383`), but the integrity zome's re-export block (`content_store_integrity/src/lib.rs:66-71`) **never aliases them** — both constants are dead (referenced by zero validators; contrast `CORE_ENGAGEMENT_TYPES as ENGAGEMENT_TYPES` at `:68`, which *is* wired — proving the pattern exists, substrate-signal was just never plumbed).
- `EconomicEvent` carries no `substrate_signal` in the integrity struct (`lib.rs:1115`), the validator (`validate_economic_event:4392`), or the View (`elohim-views/src/shefa.rs:18`).

**Seam 2 — the signal pipeline has three legs, only one wired** (verified):

| Leg | path | state |
|---|---|---|
| 1. DHT→SQL | `CommitmentCommitted` (`signals.rs:608`), `ReaEconomicEventCommitted` (`rea_projection.rs:415`) | **wired** |
| 2. vote→tally | `AttestationProjector → tally_projector::recompute` (`tally_projector.rs:18-25`) | **wired to the SQL row, then terminal — emits nothing** |
| 3. private→aggregate | `aggregate_and_emit` (`aggregator.rs:143`) | **built, zero production callers** |

The self-referential blocker is exactly **leg 2 has no emission**: a tally crossing threshold is a derived SQL transition with no post-commit hook, so nothing downstream can flow.

---

## 2. `substrate_signal` migration — corrected to the production path

### 2.1 [adversarial-fix] The production write path, not the generic projector

The original plan landed `substrate_signal` via `shefa_economic_event_column_mapping` (`mapping.rs:176`) + the generic `project()` (`mod.rs:263`). **That path is fixture-only:** `with_shefa_economic_event` (`mapping.rs:221`) is referenced exclusively in `sweep.rs:253` + mapping-module tests. **Production** `ReaEconomicEventCommitted` events flow:

```
ReaEconomicEventCommitted (rea_projection.rs:415)
  → build CreateEconomicEventInput from the DHT event   (rea_projection.rs — the event→input extraction)
  → economic_events::upsert_with_anchor                 (rea_projection.rs:492)
  → NewEconomicEvent (typed Diesel Insertable)          (models.rs:388)  → INSERT
```

So the real landing sites — **none of which the original 6-step plan named** — are:
1. **`CreateEconomicEventInput`** (`economic_events.rs:21`) — add `substrate_signal: Option<String>`.
2. **`NewEconomicEvent`** (`models.rs:388`) — add the column to the `Insertable`.
3. **`upsert_with_anchor`** (`economic_events.rs:~40`) — wire `input.substrate_signal` into the `NewEconomicEvent`.
4. **The `event → input` extraction in `rea_projection.rs`** — read `substrate_signal` off the DHT event into the input.

**A silver lining of the typed path:** on a Diesel `Insertable`, an unmapped field **cannot silently NULL — it fails to compile** until added to `NewEconomicEvent`. So the "silent-field-NULL" hazard the original spec hardened against lives on the *fixture* `project()` path, not production; production fails loud. The validator-first ordering still matters (DNA reject-at-write), but the silent-NULL risk is confined to the test projector. **The headline green test must drive `ReaEconomicEventCommitted` end-to-end, never `project()`** — a `project()`-based test would pass while production rows stay unpopulated (the dead-seam reborn).

### 2.2 [adversarial-fix] The genuine fork: first-class column vs `metadata_json` key

`EconomicEvent.metadata_json` already exists (`lib.rs:1148`); the validator already substring-greps it (`bounded_by` at `:4414`); `bounded_by` is already extracted from it in `rea_projection` (`:452`); `EconomicEventView.metadata` already surfaces it parsed (`shefa.rs:44`). **A `substrateSignal` key in `metadata_json` rides every existing path with zero DNA-hash change** — no integrity-struct edit, no truth-layer migration, no alpha-pair partition. This **dissolves the "hardest tradeoff"** the original spec agonized over.

The case for a first-class column anyway: the governor needs `GROUP BY substrate_signal` to compute a per-substrate concentration distribution (Cluster #1), and grouping on a JSON-extracted key is slower and un-indexable. **Recommendation: first-class column** — but the doc must *argue* it (queryability) rather than assume it, and the operator owns the call (§Decision 1). If the answer is `metadata_json`, the whole DNA-hash/alpha-pair section below is moot.

### 2.3 [adversarial-fix] The validator: a typed field validates clean; metadata_json does not

A first-class typed `substrate_signal: Option<String>` lets `validate_economic_event` do a **direct whitelist-membership check** against `SUBSTRATE_SIGNALS` (cheap, exact — match the field, accept `None`, accept members, reject non-members). The original spec's worry — that the substring idiom can't express whitelist membership without enumerating 7 accept-patterns plus an inexpressible reject-all clause — **only applies to the `metadata_json` route.** This is a third argument for the first-class column: it makes the validator *simpler and correct*, not harder. (Keep the substring idiom only for genuinely-in-metadata fields.)

### 2.4 Ordering (first-class-column branch)

The DNA wall (validator) lands first, the field second, the production wiring last:
1. **Wire the constant** — `pub use generated_enums::CORE_SUBSTRATE_SIGNALS as SUBSTRATE_SIGNALS;` in `lib.rs:66-71` (the dead const gets its first consumer; `check-dna.mjs:199-258` already passes since it verifies `CORE_/ALL_` arrays). Decision 1-adjacent: alias `CORE_` for block consistency (CORE == ALL today).
2. **Validator floor** — direct whitelist check in `validate_economic_event` (`:4392`): accept absent, accept member, reject non-member. Whitelist admits all 7 (forward-compat) though v1 only emits `attention`.
3. **Integrity struct field** — `EconomicEvent.substrate_signal: Option<String>` (`lib.rs:1115`); old entries deserialize `None`. `just pack` (not `just build`, per `project_sweettest_native_build_env`) refreshes the `.dna`. **DNA hash changes here** (§2.6).
4. **View field** — `EconomicEventView.substrate_signal` (`elohim-views/src/shefa.rs:18`, camelCase + `skip_serializing_if`) + the `From` impl in `views.rs`. `cargo test export_bindings` **in `elohim-views`**; **sha256-verify** the generated TS diff is exactly one optional field.
5. **Production write path** — the four sites in §2.1 (`CreateEconomicEventInput`, `NewEconomicEvent`, `upsert_with_anchor`, the `rea_projection` extraction) + a Diesel migration `economic_events.substrate_signal TEXT NULL` (watch the timestamp-collision trap, `feedback_diesel_migration_timestamp_collision`).
6. **Fixture parity (optional)** — add the `shefa_economic_event_column_mapping` line so the test projector matches production; clearly secondary, never the landing.

### 2.5 [adversarial-fix] Backfill: COALESCE or real backfill, not NULL-by-convention

NULL-means-attention-by-convention **collides with `GROUP BY substrate_signal`** — NULLs bucket separately from `'attention'`, so the concentration aggregate under-counts attention on exactly the historical rows. Pick one: **backfill `'attention'` for real** (keyed on `lamad_event_type` content-view → attention), or **`COALESCE(substrate_signal, 'attention')` at every aggregate read**. The convention cannot live only in app prose. Recommendation: COALESCE at the aggregate read in v1 (cheap, reversible), real backfill when the governor goes live.

### 2.6 DNA-hash blast radius (first-class-column branch)

Adding the integrity field changes the DNA hash. Per CLAUDE.md, a DNA-content change does not reach running conductors on a normal edge redeploy (persistent PVC, role-structure-only stale-check); forcing reinstall is gated behind `ALLOW_DNA_REINSTALL` (mints a new agent key). **The alpha genesis pair (adam+matthew, `project_alpha_topology_bootstrap_pair`) must both get the flag or partition onto different DHTs.** v1-lock is household-only — prove on the M/J/J mesh (`feedback_household_nodes_is_the_stable_floor`) where the operator controls all peers; the alpha-pair reinstall + its pre-field lineage decision are a deliberate later operator step (§Hardest tradeoff).

---

## 3. Self-referential governance signal flow & aggregation

### 3.1 Seam A — emit-on-transition in `tally_projector::recompute`

`recompute` (`tally_projector.rs:18-25`) is terminal today. Add an **edge-triggered** emit:

```rust
let prev = governance_action_tally::get_by_parent_cid(conn, parent_cid)?.map(|r| r.computed_status);
let tally = compute_tally(conn, parent_cid)?;
upsert(conn, &tally)?;
if prev.as_deref() != Some("reached-quorum") && tally.computed_status == "reached-quorum" {
    emit_governance_threshold_reached(conn, &tally)?;   // NEW — clones signals.rs:1306 writeback mold
}
```
For a `ratify-limit-gradient` action, `emit_governance_threshold_reached`:
- **writes the dead `responsibility_demand_configs.ratified_by/_at/dht_anchor_hash` columns** (`up.sql:703`, today `None` at `api/token.rs:302-304`) — this **is** Cluster #1's ratify-writeback projector; and
- **fires a `GovernanceThresholdReached` signal** so leg 3 (aggregation) and the re-ratify loop can subscribe.

The zome admits the new kind: add `ratify-limit-gradient` to `GOVERNANCE_ACTION_KINDS` (`governance_action.rs:148,263`) and the `child_attestation_kind_for_governance_action` map (`:389-400`) → child kind `limit-gradient-approval` (Cluster #1's A2 vote entity).

**[adversarial-fix] Idempotency — the down-then-up flip.** `compute_tally` is latest-per-issuer with no immutability, so a voter can flip *after* quorum: reached-quorum→pending→(re-flip)→reached-quorum re-arms the edge → a **second `ratified_at` write**. The edge-compare alone does not catch this. Guard the *writeback* (not just the emit): make `emit_governance_threshold_reached` a no-op if `ratified_at` is already non-NULL for that config CID (first-ratification-wins latch), or make the writeback idempotent by CID. State which (recommend first-ratification-wins; a re-ratification is a *new* config CID, not a re-fire on the old one).

### 3.2 Seam B — governance-state aggregate on the existing group-and-suppress machinery

`aggregate_and_emit` (`aggregator.rs:143`) is substrate-agnostic (group-by-string-key, k=5 Suppress). Point siblings at governance rows. Two candidates, both cloning the firewall mold (Cluster #2):
- **`GovernanceParticipationCandidate`** — groups ratification participation by `governance_kind`. Output `{governance_kind, participating_pct, context_window_seconds}`. No per-voter, no per-collective identity below k.
- **`ConcentrationSnapshot`** (Cluster #1 C entity) — groups per `(substrate_signal, layer)` over `economic_events`. Output `{signal, layer, concentration_value, computed_at}`. No per-agent term.

v1 (reflexive-sensing, no clock): driven by `POST /api/v1/governance/aggregate-tick` (AppScopedDb-gated) + the test harness — not a scheduler.

### 3.3 [adversarial-fix] The firewall is fenced around the wrong object — the genuine governance decision

The original spec reassured that "the tally row holds counts, never per-voter rows." **True for the row, false for the surface.** `compute_tally` (`governance_action_tally.rs:78-115`) reads the **full `attestations` table**, where every vote persists as a row with `issuer_cid + vote_value + parent_governance_action_cid` in clear, queryable SQL. `SELECT issuer_cid, vote_value FROM attestations WHERE parent_governance_action_cid = ?` **reconstructs who-voted-how.** The firewall guards the emitted aggregate (which never leaked) and leaves intact the precise table a capturing operator queries to track dissenters — the *same* `list_by_signer` hazard Cluster #2 flags for tending, missed here and worse (it is wired, not unused).

This forces a decision that cannot be papered (§Decision 4 — **operator-owned**):
- **(a) Votes auditable-by-design.** Many governance models *want* roll-call votes; one-agent-one-vote *requires* attribution at source. Then **say so plainly** — the protocol's governance privacy model is "votes are auditable, not private" — and stop claiming k-anon protects voters (it protects nothing the `attestations` table already discloses). Add a read-path guard anyway: the tally is the *only* sanctioned reader filtered by `parent_governance_action_cid`; any per-`issuer_cid`-group query is forbidden/unwired (grep-guarded like `list_by_signer`).
- **(b) Votes private.** Then `issuer_cid` cannot persist in clear in an operational projection — the tally machinery needs the HMAC-pseudonym treatment Cluster #2 specifies for tending (vote dedup by HMAC, not raw key).

**Recommendation: (a) auditable-by-design with the read-path guard** — it matches the existing data shape, is honest, and many sociocratic models prefer accountable votes. But it is a governance-philosophy choice, not an engineering default.

### 3.4 [adversarial-fix] k-of-what, and the governance-power-concentration impossibility

- **k=5 was inherited from attention and never re-derived.** Name the suppression unit per aggregate: `GovernanceParticipationCandidate` suppresses below 5 *ratifications-of-a-kind* (not 5 voters) — so **rare high-stakes constitutional kinds (limit-gradient ratifications, infrequent by design) are suppressed indefinitely** and cannot be participation-sensed. `ConcentrationSnapshot`'s k unit is *economic events per (signal,layer)*. State each; accept that the rarest, highest-stakes governance cannot be k-anon-sensed (route to the parked wisdom cluster).
- **"Concentration of governance power" is unsatisfiable under the firewall.** It means "are the same few agents winning every ratification" — a function over `attestations.issuer_cid × governance_actions`, i.e. a **per-agent term the firewall forbids in the aggregate.** The original spec silently substituted substrate-*usage* concentration (a different quantity) and called it governance-power concentration. **Name the substitution and its gap:** v1 senses substrate-usage concentration as a *proxy*; true governance-power concentration needs either the auditable-votes read-path (decision a) computed by a sanctioned reader, or it is unmeasurable k-anonymously. Do not conflate them.

### 3.5 [adversarial-fix] Sense, not act, in v1

The FeedbackSignal receive-side governance projector is deferred; in v1 the signal lands on the standing/reach surface (`api/standing.rs`), not a governance projector. So v1 is **reflexive observation without reflexive control** — "governance can be *shown* its own state; *acting* on it is deferred." Frame it that way; "governance senses itself" oversells a v1 that senses-and-displays but cannot yet act.

---

## 4. Data-arch homes & p2p-gate summary

(unchanged from the strong synthesis — see the entity/signal table: `substrate_signal` field on A; `ratify-limit-gradient` GovernanceAction A; `limit-gradient-approval` A2; `governance_action_tally` C + emit seam; `GovernanceThresholdReached` C/signal; `GovernanceParticipationCandidate`/`ConcentrationSnapshot` C/k≥5 aggregates; ratify-writeback into `responsibility_demand_configs`). **Gate addition [adversarial-fix]:** the gate must be run on the **read paths into `attestations`**, not only the entities — per-voter reconstruction is the surveillance surface, and it is gated by Decision 4, not by the entity classification.

---

## 5. Dependencies

- **Hard prerequisite for Cluster #1:** no `substrate_signal` field ⇒ no per-substrate distribution ⇒ no `ConcentrationSnapshot`. Land §2 fully first. §3.1's emit seam IS Cluster #1's ratify-writeback projector.
- **Hands the firewall obligation to Cluster #2:** the two aggregate candidates clone `candidate_struct_has_no_peer_identity` with the governance forbidden set (`signed_by`/`issuer_cid`/`voter`), same-commit-as-struct. Cluster #2 also owns the `attestations` read-path guard (Decision 4) and the anti-capture property test.
- **Shared DNA-hash change** with Cluster #1's `validate_ratifies_limit_gradient` arm; same alpha-pair implication; both contained by household-only v1.

---

## 6. Open decisions for operator

1. **[headline] `substrate_signal` home: first-class column vs `metadata_json` key.** Column = queryable `GROUP BY` (governor needs it) but a DNA-hash change + alpha-pair partition; metadata_json = zero DNA-hash change but un-indexable. *Recommend column (queryability), argued not assumed.*
2. **Constant alias** `CORE_` vs `ALL_` (CORE==ALL today; recommend CORE for block consistency).
3. **Backfill** real `'attention'` vs `COALESCE`-at-read (recommend COALESCE v1).
4. **[headline] Governance votes: auditable-by-design vs private** (§3.3). *Recommend auditable-by-design + read-path guard — but it's a governance-philosophy call.*
5. **Re-ratification routing target** ("the governing collective" addressing for the deferred receive-side projector).
6. **aggregate-tick authority** (any authed agent vs steward-only; cross-tenant sensing is forbidden by the AppScopedDb boundary by design).
7. **Idempotency latch** (first-ratification-wins on the writeback — §3.1).
8. **k-of-what acceptance** (rare constitutional kinds un-sensable; route to wisdom cluster).

---

## 7. v1 slice (household-nodes, no shem; the green tests)

**LANDS:** `substrate_signal` end-to-end through the **production path** (the four §2.1 sites), single value `attention`; the validator floor; the emit-on-transition seam + ratify-writeback; the two governance aggregates with firewall tests (Cluster #2); the `attestations` read-path guard per Decision 4.

**The one green test [adversarial-fix — drives production, not the fixture projector]:** an end-to-end test driving `ReaEconomicEventCommitted` with `substrateSignal:"attention"` → assert `economic_events.substrate_signal == "attention"` on a real row via `upsert_with_anchor`; an event without it → `None`, no error; a `"garbage"` value → **rejected by the DNA validator** (the wall). Plus: emit-on-transition fires exactly once on the quorum edge and does **not** double-write `ratified_at` on a down-then-up flip; the ratify-writeback closes the dead seam (columns go non-NULL); a 4-ratification cohort emits nothing (below-k suppress).

**HELD:** the FeedbackSignal receive-side governance projector (sense-not-act); the alpha-pair `ALLOW_DNA_REINSTALL` + pre-field lineage; cross-tenant (shem) aggregation.

---

## 8. Hardest unanswered tradeoff

**If the answer to Decision 1 is "first-class column," the v1 household landing forks the substrate:** household peers carry `substrate_signal`; the alpha pair cannot until a deliberate `ALLOW_DNA_REINSTALL` that mints new agent keys (prod lineage cost). The unanswered part is the **reconciliation path** when the alpha pair finally reinstalls — backfill their pre-field events to `attention`, or genesis a fresh chain? A lineage decision the operator owns, invisible until the household→alpha promotion is attempted. **The `metadata_json` route makes this tradeoff vanish entirely** — which is exactly why Decision 1 is the headline, not a detail.
