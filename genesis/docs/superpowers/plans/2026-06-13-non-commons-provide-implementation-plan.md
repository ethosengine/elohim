---
id: non-commons-provide-implementation-plan
status: plan
created: 2026-06-13
class: substrate
artifact_kind: plan
written: 2026-06-13
implements: non-commons-provide-commitments-design
cites:
  - non-commons-provide-commitments-design | the DECIDED design this plan implements — §9.2 Option A (integrity enforces structural reach), §4-§8 surfaces | sha256:55d67eec29f98580 | path: genesis/docs/superpowers/specs/2026-06-13-non-commons-provide-commitments-design.md
requires_env: []
---

# Non-commons provide commitments — implementation plan

**Implements:** `2026-06-13-non-commons-provide-commitments-design.md` (DECIDED).
**Decision recap:** §9.2 = **Option A** — the mishpat **integrity** zome enforces the structural
reach invariant (`reach ∈ enum`, `reach_ceiling ≥ reach`); this **moves the DNA hash**. Plus the
operator elected (2026-06-13) to **batch** the already-decided `ratifies-limit-gradient` integrity
defense-in-depth arm (per-substrate-limitarian spec §346-347, never landed — coordinator-only at
`c718cda6f`) onto this single hash move, as a **separate commit**.

**Autonomy boundary:** all work is **commit-only on the shift branch**. The DNA reinstall is an
**operator ceremony** (`ALLOW_DNA_REINSTALL`, adam+matthew both flagged) — NOT performed here.

---

## 0. The two stages (the hash line is the whole discipline)

| Stage | Hash | Surface | Lands as |
|-------|------|---------|----------|
| **A** | neutral | storage projection + eligibility + schema + their tests | commits A1–A4 (native cargo, RUSTFLAGS="") |
| **B** | **MOVES** | `mishpat_integrity/src/lib.rs` (reach arm + limit arm) + coordinator twin + sweettest + a2o | commits B1 (reach), B2 (limit arm), B3 (coordinator+tests+a2o) |

**The entire hash-moving surface is `elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs`.**
The coordinator (`commitments.rs`) is hash-blind (per `project_dna_hash_blind_to_coordinator_zomes`);
it ships in Stage B for logical coherence but contributes nothing to the hash.

**Proof-of-exactly-one-move (mandatory gate):**
```
cd elohim/holochain/dna/mishpat && just pack && hc dna hash workdir/mishpat.dna   # BEFORE (current dev tree)
# … apply Stage B edits …
just pack && hc dna hash workdir/mishpat.dna                                       # AFTER
```
Exactly one new hash. An **empty `mishpat_integrity` diff = silent Option B** (the rejected option) —
the verify phase MUST confirm the integrity diff is non-empty AND the hash changed. (RUSTFLAGS baked
into the justfile — do not override; DNA/WASM workspaces use plain cargo, no CARGO_TARGET_DIR.)

---

## 1. Shared sub-decision — the inlined reach enum (load-bearing)

There is **no reach enum/ordinal reachable from the mishpat zomes**: `CORE_REACH_LEVELS` lives in the
*elohim* DNA (`content_store_integrity/src/generated_enums.rs:315`), and `elohim-epr` is not
WASM-zome-safe (pulls `ed25519-dalek`, `cid`, `ts-rs`). **Decision: inline the 8-value ordered list +
positional ordinal in `mishpat_integrity`**, following the established mold (the coordinator already
inlines it at `commitments.rs:506-515`, `validate_acknowledges_reach_change`).

```rust
// most-restrictive → most-open; ordinal = index. Mirrors schemas/v1/enums/reach.schema.json (_ordinal:true)
// and elohim-DNA CORE_REACH_LEVELS byte-for-byte. Un-gated hardcode — see §4 gate note.
const REACH_LEVELS: [&str; 8] =
  ["private","self","intimate","trusted","familiar","community","public","commons"];
// reach ∈ enum  := REACH_LEVELS.contains(&r)
// ceiling ≥ reach := idx(reach_ceiling) >= idx(reach)
```
Caveat (record in code comment): `pnpm run schema:check-dna` verifies `content_store_integrity`, NOT
mishpat, so this const is an **un-gated** mirror. §6 says reference-not-canonize the reach drift; this
inline rides the coordinator's existing precedent rather than introducing a new canonical home.

---

## 2. Stage A — hash-neutral (native; `RUSTFLAGS="" CARGO_TARGET_DIR=<pool slot>`)

### A1 — Projection read-through (`elohim-storage/src/mishpat_projection.rs`)
- `provide_projection_for` (`:524`) returns `reach:"commons"` hard-coded at `:534-538`. **Trap:** reach
  is not on `row: &NewMishpatCommitment` (struct `db/models.rs:3581` has no reach field). Thread the
  parsed reach onto `ProvideProjection` (and the row, or re-parse from `row.bounds_json["reach_ceiling"]`).
  Prefer threading the parsed reach explicitly (the parse already has it at `:352-355`).
- `parse_replicates_commons` (`:336`) content-arm hard-reject `reach != "commons"` (`:356`) → structural
  read-through (`reach ∈ REACH_LEVELS`, `reach_ceiling ≥ reach`). **Capacity arm (`:431`) stays
  commons-pinned UNCHANGED** (commons-ratio-attested by design, §6/§10).
- Update the doc comment at `:505-507` ("always commons today").

### A2 — Projection rename + row write (`elohim-storage/src/db/rea_commitments.rs`)
- `record_provide_from_commons_commitment` (`:357`) → `record_provide_from_content_commitment`. The body
  already interpolates `content:{reach}` (`:365`) and `provide_projection_id` (`:324`) is already
  keyed by reach — **no logic change**, rename + doc-comment only. Once A1's caller passes the real
  reach, `content:<reach>` rows fall out for free.
- Sweep callers: `signals.rs:880-881` (live), tests `rea_commitments.rs:1496,1524,1532,1558`.

### A3 — Eligibility filter (`provide_reconcile.rs` + callers + `content_diesel.rs`)
- `derive_desired` (`:387-402`) stays **pure**; rename param `commons_head_refs` →
  `provide_eligible_head_refs`, generalize the `:395` filter to set-membership.
- The reach-aware classification happens at the **two callers** — `main.rs:1006-1008` and
  `p2p/mod.rs:6878-6880` — using `classify_pre_authorization` (pure, `reach_authorization.rs:198`;
  bound-tier branch = the §4.3 predicate). **Decouple the `main.rs:1007` double-use** of
  `commons_present` (caught-up proxy AND commons filter must become two sets).
- **DB gap:** `content_ids_with_reach` (`content_diesel.rs:820-836`) returns one-reach/no-pillar.
  Extend to return `(id → reach, pillar)` so callers build `content:<reach>` AND the topic
  `elohim/<pillar>/<reach>[/<collective>]` for the classifier. **Ride the ladder:** build the topic
  WITH pillar (+collective for community) now even though Stage-1 resolver discards past `parts[2]`;
  do NOT implement the Stage 2/3 graph walk (§9 item 5).
- **Test seam:** extract the call-site eligibility behind an injectable resolver (closure/trait,
  mirroring `CommitmentAuthor`/`MockAuthor`, `:84-130`) so admit/reject is unit-testable without a
  live pool. Note (flag, do NOT fix): `node_has_embodied_responsibility` fails OPEN (`:264,:277`).

### A4 — Schema rename + widen (`elohim/sdk/schemas/v1/commitments/`)
- `replicates-commons.schema.json` → `replicates-content.schema.json`. Widen `reach` (`:41`) and
  `reach_ceiling` (`:25`) from `const:"commons"` → the 8-value enum. **INLINE the 8 values, do NOT
  `$ref` reach.schema.json** — the mini-test uses `Ajv2020({strict:true})` with no `addSchema`, so a
  `$ref` throws MissingRef and `_ordinal` trips strict. `action` (`:11`) → `enum:["replicates-content",
  "replicates-commons"]` (wire-alias, mirrors the validator alias window).
- Rename `scripts/test-replicates-commons-schema.mjs` → `…-content-…`; flip the `reach:"community"`
  should-reject case (`:94-98`) to should-accept; update the package.json `schema:test` entry.
- **Free facts (no work):** `generated-ts/` is gitignored (zero committed consumers, not in
  `INTERFACE_FILES`); `codegen-rs` scans `enums/` only — neither ripples. **Disambiguation:** the
  ts-rs `ReplicatesCommonsPayload` (committed; Rust source `elohim-views/src/replicates_commons.rs`) is
  a SEPARATE type from the gitignored JSON-schema `ReplicatesCommonsCommitment`. The ts-rs rename (if
  taken) is `cargo test export_bindings` + sha256-diff discipline — keep it in Stage A, A2-adjacent.

### A-tests (local-only — no CI stage runs `cargo test` on elohim-storage; sweettest is the CI backstop)
- `provide_projection_for` yields the content's reach (NET-NEW — none exists).
- `record_provide_from_content_commitment(..., "household", ...)` writes `content:household`.
- Eligibility filter: embodied-responsibility node admitted, non-eligible rejected (injected resolver).
- Snapshot: non-commons content + provide row + provider WITH `household_id` → count ≥ 1; provider
  WITHOUT `household_id` → 0 (correct-but-dormant, §5.3). Snapshot reader itself is UNCHANGED (§5.2 —
  exact-eq `content:<reach>` already generalizes).

---

## 3. Stage B — hash-moving (DNA; plain cargo, RUSTFLAGS baked, `just pack`)

### B1 (commit) — reach generalization in `mishpat_integrity/src/lib.rs` (THE hash move)
- Arm at `:836` (`"replicates-commons" => {…}`). **Today checks only `variant` + `reach_ceiling=="commons"`
  — never top-level `reach`.** Edit:
  - Accept `"replicates-content" | "replicates-commons"` (alias window).
  - Branch on `variant`: **content** → `reach ∈ REACH_LEVELS` (when present) AND
    `idx(reach_ceiling) ≥ idx(reach)`; **capacity** → keep `reach_ceiling == "commons"` UNCHANGED.
  - Drop the unconditional `reach_ceiling == "commons"` reject (`:841-843`).
  - Add the §1 `REACH_LEVELS` const; update comment/error strings.
- **NET-NEW integrity unit tests** (none exist for this arm): well-ordering accept (`household`/
  `household`), `ceiling<reach` reject, non-enum reach reject, alias `replicates-commons` still validates,
  capacity still pins commons.

### B2 (commit) — batch: `ratifies-limit-gradient` integrity defense-in-depth arm
- Add a `"ratifies-limit-gradient" => {…}` arm to `commitment_action_requirements` mirroring the
  coordinator's structural checks (the coordinator arm shipped at `c718cda6f`; integrity arm never did,
  though the limitarian spec §346-347 names it). Structural/substring only (HDI — no graph access).
- Integrity unit tests for the arm. **Separate commit** so the hash-moving diff is reviewable per-purpose.

### B3 (commit) — coordinator twin (hash-blind) + sweettest + a2o
- `commitments.rs`: rename `validate_replicates_commons` → `…_content` (`:213`); dispatch (`:194`)
  accepts BOTH action strings → same validator; internal action assert (`:214`) accepts both. **Do NOT
  blanket-replace the shared `reach_ceiling=="commons"` at `:233`** (it runs for capacity too) — MOVE
  the ordinal check INTO the content branch (`:251-254`), keep capacity's ceiling pin. Use the inline
  reach list precedent (`:506-515`).
- Action-string blast radius — both strings honored on read/filter paths during the window:
  `mishpat_projection.rs:154,414-415,474-475`, `commitment_fetcher.rs:95`, `db/mishpat_commitments.rs:161`,
  `services/replication_prioritizer.rs:115,189`, `services/provide_reconcile.rs:425-426`.
- **Inverting tests** (assertion flips commons-reject → non-commons-accept): coordinator
  `commitments.rs:887`; sweettest `replicates_commons_substrate_correct_test.rs:162`.
- **Sweettest generalization** (CI-covered, `--run-ignored all`):
  `replicates_commons_round_trip_test.rs:81` → add `reach=household` variant asserting 32-byte
  action_hash + peer readback; `replicates_commons_substrate_correct_test.rs:162` →
  `reach=household` accepted, `ceiling<reach` rejected, ≥1 `replicates-commons` alias commit.
- **a2o** (§8): new `genesis/a2o/features/resilience/non-commons-provide-counting.feature` — template
  `resilience-dimensions.feature:76`. Author household content, eligible peer provides, snapshot reads
  non-zero commitment-backed count; honest-zero companion (provider w/o `household_id` → 0). Household
  floor `@e2e @resilience @resilience-p1`; cross-region breadth row `@requires:shem`.

---

## 4. Verification gates (the verify-workflow contract)

1. **Option-A proof:** `git diff` on `mishpat_integrity/src/lib.rs` is **non-empty** AND `hc dna hash`
   differs before/after by **exactly one** new hash. (Empty integrity diff ⇒ FAIL — silent Option B.)
2. **Coordinator hash-neutrality:** `commitments.rs`-only changes do NOT alter the hash (sanity: a build
   with only Stage-B-coordinator edits, no integrity edit, yields the SAME hash).
3. **`just pack` before sweettest** (not `just build`) — else sweettest installs stale DNA and greens
   against old behavior.
4. **Sweettest green** (`--run-ignored all`): household accept + ceiling<reach reject + alias validates.
5. **Native storage green** (A-tests): `RUSTFLAGS="" CARGO_TARGET_DIR=<pool slot> cargo test` (local-only).
6. **`schema:test` green** post-rename; `cargo test export_bindings` clean if the ts-rs payload renamed.
7. **Capacity unchanged:** capacity-variant tests stay green at both layers (commons-ratio-attested).
8. **Per-row isolation preserved:** `signals.rs:880-910` non-fatal side-projection intact.

---

## 5. Commit plan (commit-only, shift branch)

```
A1  feat(storage): provide projection reads reach from payload (drop commons hard-code)
A2  refactor(storage): record_provide_from_commons → _content_commitment (rename sweep)
A3  feat(storage): reach-aware provide eligibility via classify_pre_authorization
A4  feat(schema): replicates-commons → replicates-content; widen reach/reach_ceiling enums
B1  feat(mishpat-integrity): structural reach validation [DNA HASH MOVES]   ← the hash-moving commit
B2  feat(mishpat-integrity): ratifies-limit-gradient defense-in-depth arm   ← batched, same hash move
B3  feat(mishpat): coordinator reach generalization + sweettest + a2o scenario
```
B1 is the single auditable hash-moving commit for the reach work; B2 the batched limit arm; both touch
only `mishpat_integrity/src/lib.rs` so the operator reviews the immutable-floor change in two clean diffs.

---

## 6. Handoff — what "landed" means and does NOT mean

- **Landed = code committed + locally verified.** The DNA hash move requires an **operator reinstall
  ceremony** (`ALLOW_DNA_REINSTALL`; the alpha bootstrap pair adam+matthew must BOTH be flagged or they
  partition into different DHTs). Merge ≠ reinstall — committing/merging the hash-moving DNA is SAFE
  (install stale-check is role-structure-only; no auto-partition).
- **Substrate ready ≠ snapshot lit.** This makes non-commons counting *possible*, but the alpha snapshot
  will show non-commons counts only once `humans.household_id` is populated (Epic B / Stage C,
  out of scope — see `resilience-snapshot-humans-junction`). Do not read "landed" as "the snapshot now
  shows non-commons peers."
- **Closing the alias window later = a SECOND integrity change = a SECOND hash move** (the operator may
  prefer a permanent alias to avoid a second ceremony).

## 7. Resolved sub-decisions (defaults taken; flag if operator disagrees)
- **§9.1 rename + one-window alias** — TAKEN (spec recommendation): `replicates-content` with
  `replicates-commons` accepted at both validators + the wire schema for one window.
- **Schema `action` = enum (wire alias)** — TAKEN (coherent with the validator alias).
- **Inline reach const in `mishpat_integrity`** — TAKEN (coordinator precedent; §1).
- **ts-rs `ReplicatesCommonsPayload` rename** — TAKEN within Stage A (sha256-diff discipline); the
  JSON-schema `ReplicatesCommonsCommitment` is gitignored and renames freely.
