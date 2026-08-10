---
title: Algedonic Phase 1 — the discipline at the EPR level, local-first
id: algedonic-phase1-epr-local-first
status: Draft
cites:
  - algedonic-feedback-signal | the D2 spec this plan implements phase-1 of — kinds, evidence contract, slices, non-goals | sha256:d0b1b524dc7240fc | path: genesis/docs/superpowers/specs/2026-08-10-algedonic-feedback-signal-design.md
  - algedonic-slice1-delivery-flow | predecessor plan; its Tasks 3-5 are re-homed by this plan (3-4 to phase 2, 5 reshaped as local consumer) | sha256:66054e651d33f3a4 | path: genesis/docs/superpowers/plans/2026-08-10-algedonic-slice1-delivery-flow-plan.md
  - epr-meta-compose-gate | the mechanism spec for the measure/verdict layer Task 4 graduates to typed evidence | sha256:6052ce071bfec509 | path: genesis/docs/superpowers/specs/2026-06-25-epr-meta-compose-gate-design.md
domain: protocol
sprint: operator-directed-2026-08-10
---

# Algedonic Phase 1 — EPR-Level Discipline, Local-First

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Operator steering (2026-08-10):** establish the local-first patterns of algedonic signal handling on epr-meta/epr-rea FIRST — exercise the discipline, close the loops, set the limits, express validation and enforcement in idiomatic Rust, canonize the do/do-nots of algedonic signal design — and graduate that working pattern in the devspace **before** the network level (CI/CD-produced signals). Phase 2 (network) re-homes slice-1's held Tasks 3-5; phase 3 burns down concerns the pattern subsumes.

**Goal:** the complete algedonic loop — producer → stock/limit → addressed signal → consumer — typed, validated, and demonstrably closed inside the devspace with real pain (the two open god-file findings), zero CI involvement.

**Relationship to slice-1:** slice-1 landed its plane-agnostic footholds before the steering arrived (concern-route helper `8a05236a7`; ci-harvest concern+no-measure `11f334120` — inert until harvests run; the two wire schemas). Slice-1 Tasks 3-5 are HELD and re-homed to phase 2 by this plan; Task 5 (renderer join) returns here reshaped as the local consumer (Task 5 below). The wire schemas in `elohim/sdk/schemas/v1/feedback-signals/algedonic-{approach,breach}.schema.json` are this plan's field-name contract.

**P2P design gate:** inherited from the cited spec's audit — the algedonic kinds are instances of the EXISTING `FeedbackSignal` DHT entry (Category A); source of truth is the DHT. This plan adds NO entry type, table, or route: Rust validation vocabulary, REA model surface, devspace ledger fields, and doc canon only.

## Global Constraints

- **Commit-only, path-limited**: never push; the tree carries other sessions' modifications — commit exactly your files with `git commit -m "..." -- <paths>`.
- **Native cargo discipline**: `RUSTFLAGS=""` for native builds; set `CARGO_TARGET_DIR` from `cargo-pool key` run in the crate directory; never trust piped exit codes — echo `EXIT=$?` on its own line after every cargo run; `cargo test`, not `cargo check`, verifies.
- **Wire parity is law**: Rust field names (after serde rename) must match the two schema files' `required`/`properties` exactly; a parity test reads the schema JSON from the repo — never a copied constant.
- **Additive ledger fields only**: `architecture-findings.jsonl` fingerprints are byte-stable; new fields never enter fingerprint inputs; readers use `.get(...)`.
- **Honest absence**: a signal/finding with no resolvable concern address omits the key — never guess.
- **The canon is enforced, not narrated**: every do/do-not in Task 2's canon must point at the code construct or test that makes it structural (unrepresentable or validation-refused) — a canon line with no enforcing construct is a TODO, not a rule.
- **Stdlib-only** in `.claude/scripts` Python; plain-script tests (`python3 <file>` exits 0).
- **No new instruments** beyond the files each task names.

---

### Task 1: `elohim/epr` — the algedonic vocabulary, invariants unrepresentable-or-refused

**Files:**
- Create: `elohim/epr/src/algedonic.rs`; register `pub mod algedonic;` in `elohim/epr/src/lib.rs`
- Test: in-file `#[cfg(test)]` per crate convention (verify convention by reading `validation.rs` first)

**Interfaces:**
- Produces: `AlgedonicKind {Approach, Breach}`, `AlgedonicEvidence`, `AlgedonicSignal`, `validate(...) -> Result<...>`, `open_signal_key(...)` — consumed by Tasks 3-4 conceptually (shape), Task 2 (canon anchors).

- [ ] **Step 1 — survey (read-only):** read `kind.rs`, `validation.rs`, `verdict.rs`, `error.rs` for the crate's error/validation idiom; read both algedonic schema files (wire contracts only — source of truth: DHT `FeedbackSignal` entry); note the crate's workspace root and serde conventions.
- [ ] **Step 2 — failing tests first:** (a) evidence-mandatory: constructing/validating a signal without `stock`/`limit`/`bound_ref` refuses; (b) `Approach` requires `threshold_pct`, `Breach` forbids requiring it; (c) NO valence: assert the type carries no agree/disagree field (compile-time — the struct simply has none; the test asserts serialized JSON key-set equals the wire schema's `properties` ∩ required semantics — source of truth: DHT `FeedbackSignal` entry, schema is contract only); (d) wire parity: serialize a valid sample of each kind, load the matching schema JSON from `elohim/sdk/schemas/v1/feedback-signals/`, assert every schema-`required` field is present and no unknown keys emitted; (e) `open_signal_key` = `(declarer, target, kind)` — two signals differing only in evidence collide (dedupe key equality); (f) hysteresis predicate: `should_emit(prev: Option<&AlgedonicSignal>, new)` false inside the hysteresis band, true crossing it (band = `threshold_pct` for approach; any-open-signal suppression for same key).
- [ ] **Step 3 — implement idiomatically:** mirror the crate's existing error enum + validation style; `severity: info|warn|critical`; `standing_impact` fixed `advisory` (const fn / serialize-only constant — mirror how the wire schemas landed it); module rustdoc header states the loop (producer/stock/limit/consumer) in one paragraph and points to the canon (Task 2).
- [ ] **Step 4 — verify:** `RUSTFLAGS="" CARGO_TARGET_DIR=$(cargo-pool key) cargo test -p <epr-crate-name>` + `cargo clippy -p <epr-crate-name> -- -D warnings` + `cargo fmt --check`; echo `EXIT=$?` each.
- [ ] **Step 5 — commit** (algedonic.rs + lib.rs only).

---

### Task 2: The design canon — do/do-nots, each line anchored to its enforcing construct

**Files:**
- Modify: `genesis/docs/superpowers/specs/2026-08-10-algedonic-feedback-signal-design.md` — add §4b "Design canon (do / do-not)" between §4 and §5
- Modify (reseal): any doc whose cite envelope on the spec goes stale (`cite-gen.py --refresh` after confirming claims hold — expected: the slice-1 plan and this plan)

**Content (each line names its anchor):** DO — evidence-mandatory (`AlgedonicEvidence` non-optional fields; Task 1b test); hysteresis-bounded emission (`should_emit`; Task 1f); one open signal per `(declarer, target, kind)` (`open_signal_key`; Task 1e); honest absence for unresolvable addresses (`concern_routes.route` → None; slice-1 Task 1); lightest-class enforcement first (`ask` before `deny`, per the epr-meta ladder); addresses/metadata never in identity (fingerprint-stability constraints, slice-1 Task 2 + Task 4 below). DO-NOT — no valence on algedonic kinds (field absent by construction; Task 1c); no parallel pain pipeline (spec §6 restated: local instruments decide WHETHER, the addressed carrier is one); no unbounded re-fire (dedupe-to-one-open + debounce, the anti-tolling rule); no deny-first gates on pain paths (CounterEvidence floor stays open); no guessed addresses.

- [ ] **Step 1:** write §4b (≤35 lines), every line carrying its anchor in parentheses; where the anchor is a Task 1 construct, name the function/test.
- [ ] **Step 2:** `cite-gen.py --refresh` each citing doc AFTER confirming its claims still hold; `cite-gen.py --verify` both.
- [ ] **Step 3: Commit** (spec + resealed citing docs).

---

### Task 3: `epr-rea` — limits live on commitments; pain flows against the promise

**Files:**
- Modify: `elohim/epr-rea/src/model.rs` (+ `fold.rs`/`walk.rs` as the survey dictates)
- Test: crate convention (read existing tests first)

**Interfaces:**
- Consumes: the Task-1 shape (kind names, evidence field names — by convention, not by crate dependency, unless the crates already depend).
- Produces: a bound/limit surface on commitments and an algedonic event/observation that references `bound_ref` = the bounding commitment's CID; `walk`/`fold` projections expose open pain per commitment.

- [ ] **Step 1 — survey:** read `model.rs`/`fold.rs`/`walk.rs`; find the existing limit/bound vocabulary (grep hits exist) and the event/observation shapes; decide the MINIMAL additive surface (prefer extending an existing enum/struct over new types; if a bound already exists, reuse it — the task may reduce to one event variant + one fold projection).
- [ ] **Step 2 — failing tests:** a commitment with a bound + stock events crossing it yields an approach/breach in the fold; the projection keys pain by the commitment CID; no bound → no pain (honest absence).
- [ ] **Step 3 — implement minimally; Step 4 — crate gate (test/clippy/fmt, EXIT echoed); Step 5 — commit.**

---

### Task 4: epr-meta measure mint — typed evidence + concern (the live local producer graduates)

**Files:**
- Modify: `.claude/hooks/epr-meta-resolver.py` (`_file_arch_finding` ~line 78)
- Modify: `.claude/scripts/_lib/epr_meta.py` only if the measure verdict must carry structured fields to the hook (survey first)
- Modify: `.claude/epr-meta/policies.yaml` — optional `concern:` param permitted on measure bindings (validate_meta whitelist if params are validated)
- Test: extend `.claude/scripts/_lib/__tests__/epr_meta_resolver_test.py` (or sibling, same convention)

**Interfaces:**
- Consumes: `concern_routes.route` (slice-1 Task 1; explicit binding param wins — honest absence otherwise).
- Produces: architecture findings additively carrying `stock` (measured LoC), `limit` (the ceiling), `bound_ref` (`<policy-id>@<version>` + manifest id), `concern` (optional) — field names matching the wire schemas' evidence block; consumed by Task 5.

- [ ] **Step 1 — failing test:** synthesized hard-ceiling write (resolver-test payload shape, non-existent filename) with a binding carrying `params: {concern: "..."}` → the minted ledger entry (fixture ledger path, never the live one) carries `stock/limit/bound_ref/concern`; same write, no param → no `concern` key; **fingerprint identical with and without the new fields**.
- [ ] **Step 2 — implement additively; Step 3 — run resolver + eval + cascade tests; Step 4 — commit.**

---

### Task 5: The local consumer — habits renderer joins devspace pain (slice-1 Task 5, reshaped)

**Files:**
- Modify: `.claude/scripts/habits-status.py` (headline ~51, full ~80)
- Test: `.claude/scripts/_lib/__tests__/habits_status_pain_test.py`

Same contract as slice-1 Task 5 (module-level `LEDGERS` override; `.get("concern")`; missing ledger → empty; headline `· pain: N open @<concern>`; `--full` per-habit `pain:` line, ≤3 fps) with ONE change: `LEDGERS` = ci-findings + runtime-findings **+ architecture-findings** — the local plane joins first; the graduation proof is a real local breach rendering addressed on the session-start headline.

- [ ] **Step 1 — failing test (fixture covers a concern-carrying architecture finding, a concernless entry, an absent ledger); Step 2 — implement; Step 3 — test + live smoke (`python3 .claude/scripts/habits-status.py` must render); Step 4 — commit.**

---

### Task 6: Capture + re-home — the arc recorded, slice-1 residue routed

**Files:**
- Create: `genesis/data/timeline/backlog/2026-08-10-algedonic-phase2-network-phase3-dedupe.md` (dir conventions + `.epr-meta` law apply)
- Modify: `genesis/docs/superpowers/plans/2026-08-10-algedonic-slice1-delivery-flow-plan.md` — one status note under the title: Tasks 3-5 re-homed (3-4 → phase 2; 5 → phase 1 Task 5 reshaped); Tasks 1-2-6 landed; then `cite-gen.py --refresh/--verify` as needed

**Capture content:** phase 2 (network) — slice-1 Task 3 (runtime-harvest concern threading), Task 4 (pre-push tag/changeset deny), live CI validation of the ci-harvest wiring, CI/CD emitting typed algedonic signals tracked by EPRs/epr-meta (network→local-first); phase 3 (dedupe/burn-down) — vision-gap limit-governor stub supersession check, `rate-limit-exceeded` wire-schema alignment to the algedonic evidence shape (source of truth stays the DHT `FeedbackSignal` entry), C15 algedonic-channel minting, app-manifest `algedonicHandler` field, `.epr-meta` ask-policy on EprKind-birth surfaces (`elohim/epr/src/kind.rs`, `elohim/sdk/domains/*/manifest/`), zome whitelist + `CounterEvidence` floor routing (slice-2 spec items). Each item cites the spec §5.

- [ ] **Step 1 — write capture; Step 2 — slice-1 plan note + reseal; Step 3 — commit.**

---

## Self-Review (at authoring)

- **Steering coverage**: "exercise the discipline / close the loops" = Tasks 1+4+5 (typed loop closed on real pain); "set the limits" = Task 3 (limits on commitments); "validation and enforcement in idiomatic rust" = Task 1; "do and do-nots" = Task 2 (anchored canon); "then graduate to network" = Task 6 capture (phase 2); "dedupe/superseded burn-down" = Task 6 capture (phase 3).
- **Scope stays one rung**: no zome, storage, route, or CI change; the only Rust is two local crates; the only Python is the existing devspace instruments.
- **Known unknowns, stated honestly**: epr-rea's existing bound vocabulary (Task 3 Step 1 decides reuse-vs-add); epr crate test convention and workspace/package name (Task 1 Step 1); whether the measure verdict already carries stock/limit through to the hook (Task 4 survey).
