---
title: "Lens version-DAG + EPR→policy dependency-declaration — which HEAD applies is declared, not inferred"
id: lens-version-dag-epr-policy-dependency-design
status: Draft
class: protocol-canonical
context-tier: disclosed
steward: cartographer
graduation-trigger: decompose-complete OR superseded-by-implementation
domain: D7
topic: [governance, mishpat, lens-market, versioning, dag, fork, revert, merge, policy-binding, dependency-declaration, lockfile, content-addressing, hash-neutral, household-nodes]
refines:
  - genesis/docs/superpowers/specs/2026-06-27-plural-mishpat-lenses-over-epr-design.md
  - genesis/docs/superpowers/plans/2026-06-27-plural-mishpat-lenses-service-layer-plan.md
cites:
  - plural-mishpat-lenses-over-epr-design | the charter spec; this design adds the version-DAG + head-selection layer the charter's version_parent back-pointer was reserving for | path: genesis/docs/superpowers/specs/2026-06-27-plural-mishpat-lenses-over-epr-design.md
  - plural-mishpat-lenses-service-layer-plan | the service-layer slice; PR4 was DEFERRED to THIS design (find_lenses left surfacing all heads on purpose); PR3 shipped the revocation primitive a revert/yank reuses | path: genesis/docs/superpowers/plans/2026-06-27-plural-mishpat-lenses-service-layer-plan.md
---

# Lens version-DAG + EPR→policy dependency-declaration

**Origin (2026-06-27, architect call).** The lens-market code review flagged "v1 and v2 of one school both
surface" as a recency bug whose obvious fix is "newest-in-chain hides the older." **That bakes in the wrong
model.** The architect's reframing, sealed here:

> The school could change heads — **fork, revert, or merge**. Which HEAD of a policy applies to an EPR becomes
> like a **`package.json` dependency declaration**. That would be ideal.

So PR4 was deferred (not implemented); `find_lenses_governing_epr` was left surfacing **all** heads, on
purpose, until this declaration layer lands. Background memory:
`[[project_versioned_entity_head_is_declared_dependency]]`.

---

## 1. The model

### 1.1 Versions are a DAG, not a line

A lens is an immutable `Mishpat::Commitment` (`action='author-lens'`), addressed by `cid == entry_hash`. A new
version is a new `create` carrying a `version_parent` back-pointer (commitments are immutable — never an
update). Edges point child→parent. This is git-shaped:

| Op | Mechanism | Mutates the DAG? |
|----|-----------|------------------|
| **fork** | a new `author-lens` with `version_parent = [cid_of_branch_point]` — a second child of a node | adds a node + edge |
| **merge** | a new `author-lens` with `version_parent = [head_a, head_b]` — **two** parents, reconciling branches into one head | adds a node + 2 edges |
| **revert** | **a new `binds-policy` declaration re-pinning the EPR to an earlier `cid`** — the DAG is *untouched* | no — declaration-level |

The **chain identity** is the **root cid** (the `version_parent = []` node) — content-addressed, immutable,
stable across the whole DAG. fork/merge only ever *add* nodes; nothing is mutated or deleted, so every prior
version stays canonical and fetchable by `cid` (the "cid is canonical" guarantee that motivated this design).

**Merge requires `version_parent` to become a SET.** Today's `author-lens.schema.json` declares
`version_parent` as `string|null`. Widening it to `array<cid>` (a node may have ≥1 parents) is a **payload-only
change → DNA-hash-neutral** (the payload lives in `payload_json`; no integrity bytecode moves). A single-parent
chain is the degenerate one-element-array case; a `null`/absent value stays the root.

### 1.2 Which HEAD applies is a DECLARED dependency (the `package.json` model)

Whoever binds policy to an EPR **declares** *"EPR E depends on policy chain P at constraint C"*. The constraint
is exactly `package.json` semantics:

| Constraint | Meaning | Analogy |
|-----------|---------|---------|
| `pin: <exact cid>` | this precise version, forever | **`package-lock.json`** — content-addressed, reproducible |
| `latest` | the current head of chain P (newest live descendant of the root) | `"*"` / `"latest"` |
| `range: <expr>` | the newest head satisfying an expression over the chain | `"^2"` semver range |

**The inversion this seals: the BINDING decides the head, the infrastructure does not.** A read-time
"newest-in-chain auto-hides older" filter in `find_lenses_governing_epr` would usurp a decision that belongs to
the declaration. The pin case is the load-bearing one — an exact-`cid` pin is a lockfile, the natural fit for a
content-addressed substrate, giving reproducible governance (an EPR governed by *exactly* this audited reading,
immune to a later upstream version it never reviewed).

### 1.3 Plurality is orthogonal to versioning

Distinct **schools** (georgist + beerian) over one EPR always co-surface — no collapse (the charter's headline).
This design governs *one school's own version DAG* and which head an EPR binds **per school**. The market still
shows the plural set: for each governing school, the head its binding resolves to. `latest` on a school's chain
that has forked into ≥2 live branch heads is **ambiguous** → either the binding must `pin` a branch, or the
ambiguity surfaces as **contention** on that school (feeding the existing `contention_index`/`regime` signal —
"this school is mid-fork, unresolved").

---

## 2. Entities (P2P design-gate output)

### 2.1 `PolicyBinding` — the EPR→lens dependency declaration (the one new thing)

- **Classification:** **Notarized (A)** — a `Mishpat::Commitment`, `action='binds-policy'`. Existing Commitment
  entry type, **new action discriminator → DNA-hash-neutral** (coordinator hot-swap, exactly the `author-lens`
  precedent). Which deterministic policy governs an EPR is a witnessed governance act peers must agree on; it
  carries an author (accountability) and is revocable — so it rides the Commitment substrate, **not** an A2 link.
- **Content address:** **Content-Derived (CID)** — `cid == entry_hash` (immutable). It *references* the chain by
  `chain_root_cid` + the constraint. **NOT a relational FK to a lens row** (anti-pattern): the reference is a
  content-addressed root + a constraint the resolver evaluates against the DAG.
- **Payload (`binds-policy` schema):**
  ```jsonc
  {
    "action": "binds-policy",
    "epr_scope": "epr:lamad-spa",        // the EPR slug-id this binding governs (plan A3)
    "school": "georgist",                // which school's chain (plurality axis — one binding per school)
    "chain_root_cid": "bafyrei…",        // the version-chain identity (root author-lens cid)
    "constraint": {                       // package.json-style
      "kind": "pin" | "latest" | "range",
      "value": "bafyrei…" | null | "<range-expr>"
    }
  }
  ```
- **Source of truth:** DHT (Mishpat Commitment) → projection `lens_policy_bindings` (**A-class, `dht_anchor_hash
  NOT NULL`**, `cid` PK = entry_hash; anchor-preserving upsert; fail-closed). One **live** binding per
  `(epr_scope, school)` — a new `binds-policy` supersedes the prior (revert = re-pin), revocation via the
  existing `revokes-commitment` (already wired to both `mishpat_commitments` and `lenses` by PR3 — extend to
  `lens_policy_bindings`).
- **Coordinator:** `mishpat::create_commitment{action:'binds-policy'}` + `validate_binds_policy` (closed-
  coordinator default — reject malformed; **no integrity arm** → hash-neutral). Mirrors `validate_author_lens`.
- **HTTP:** no new GET — the existing `GET /api/v1/epr/{scope}/lens-market` serves the *resolved* market. Write
  is `POST` create-commitment through the conductor bridge.
- **Anti-pattern check:** ✅ no CID-as-FK · ✅ source-of-truth declared (DHT) · ✅ no new entry type · ✅ id is
  `entry_hash` not UUID · ✅ no cross-namespace identity compare · ✅ no sovereignty framing (community-grounded).

### 2.2 Lens version-DAG — the EXISTING lens entity, extended (not new)

- **Classification:** Notarized (A), already built (`author-lens` Commitment → `lenses` table). Extension:
  `version_parent` → `array<cid>` for merge. **Payload-only → hash-neutral.** `validate_author_lens` gains a
  parent-shape check (each entry a well-formed cid; a child may not list itself; cycles rejected at the
  resolver, see §3).

### 2.3 `EffectiveHead` resolution — Operational (C)

- **Classification:** **Operational (C)** — reconstructable from the notarized `lens_policy_bindings` + the
  `lenses` DAG. No `dht_anchor_hash`. `find_lenses_governing_epr` consults it: for each `(epr_scope, school)`
  with a live binding, resolve the constraint against the chain and surface that head; with **no** binding,
  fall back to surfacing all live heads (today's behavior — the honest default).
- **Reconstruction:** recompute at read time from the two source-of-truth projections; nothing to persist.

---

## 3. The resolver (operational, read-time)

`resolve_effective_heads(epr_scope) -> Vec<Lens>`:
1. Load live `lens_policy_bindings` for `epr_scope` (fail-closed: notarized + non-revoked).
2. For each `(school, chain_root_cid, constraint)`:
   - `pin` → the lens row at `constraint.value` (if live; a revoked/absent pin is fail-closed → school drops,
     `warn!` — never empties the market, the EprRouter lesson).
   - `latest` → walk the chain from `chain_root_cid` over `lenses.version_parent`, take the live descendant with
     no live child. **≥2 such heads ⇒ ambiguous fork:** surface none for that school and raise its contention
     (a "mid-fork" signal), OR (config) surface all branch heads. Cycle/oversize-DAG guard: bound the walk depth.
   - `range` → newest head satisfying the expression (grammar TBD — start with `latest`-of-a-named-branch).
3. Schools **without** a binding: surface all their live heads (back-compat, honest default).
4. Plurality preserved: union across schools; per-school the binding picks the head.

This is the ONLY place "which head" is decided — in the operational resolver, driven by the notarized
declaration. `find_lenses_governing_epr` calls it instead of the current flat scan; the flat scan stays as the
no-binding fallback.

---

## 4. Hash-neutrality ledger

| New/changed thing | Class | Verdict |
|-------------------|-------|---------|
| `binds-policy` action + `validate_binds_policy` | coordinator action | **HASH-NEUTRAL** (hot-swap) |
| `version_parent` → array (merge) in `author-lens` payload | payload-only | **HASH-NEUTRAL** |
| `parse_binds_policy` + projection + `lens_policy_bindings` table | storage | **HASH-NEUTRAL** |
| resolver in `lens_facing` / `db::lenses` | storage (operational) | **HASH-NEUTRAL** |
| extend `revokes-commitment` arm to `lens_policy_bindings` | storage | **HASH-NEUTRAL** |

No integrity bytecode touched ⇒ no DNA hash move ⇒ deploy by coordinator hot-swap, provable on household-nodes.

---

## 5. Phased implementation (bottom-up, TDD; mirrors the lens-market slice)

- **V1 — declaration substrate:** `binds-policy` validator (teeth) + schema; projection + `lens_policy_bindings`
  A-class DAO (anchor-preserving upsert, fail-closed); extend `revokes-commitment` to the new table.
- **V2 — the resolver:** `resolve_effective_heads` (pin first — the lockfile case, highest value + simplest);
  wire into `find_lenses_governing_epr` with the no-binding flat fallback. `latest` second; `range` last.
- **V3 — merge:** widen `version_parent` to array in the schema + `validate_author_lens` + projection + the DAG
  walk; ts-rs/codegen.
- **V4 — surfacing:** the market view exposes the resolved head per school + a "mid-fork/ambiguous" flag feeding
  contention/regime; a `LensBindingView.boundConstraint` field so the UI can show "pinned to vX" vs "tracks latest".

Per-task gates: sweettest (validator), storage `RUSTFLAGS=""` nextest/clippy/fmt with `CARGO_TARGET_DIR`,
`export_bindings`, `schema_contract`, then a live household-nodes render. CI-green ≠ binding-correct.

---

## 6. Open questions (resolve in V-series brainstorms)

- **Authority:** WHO may author a `binds-policy` for an EPR? (the EPR author? a qahal vote? a steward role?) —
  ties to the Mishpat authority-arc (`sets-authority-arc` commitments). Likely a B2 attestation / governance
  gate, not open-to-anyone. **This is the highest-leverage open question** — the binding is a power.
- **`range` grammar:** start minimal (named-branch `latest`); avoid reinventing semver.
- **Ambiguous-fork policy:** surface-none-and-flag vs surface-all-branch-heads — config or per-binding.
- **Standing across a chain:** does affinity earned under v1 transfer to v2 on the same chain, or stay
  per-version? (The deferred Option-B question — answer once selection-write-path lands.)
- **GC / DAG bound:** depth/size cap on the version walk (operational guard against a pathological chain).
