---
title: Governed discovery — stations 0 to 3 (split, reader lens, bootstrapping head, standing reader)
id: governed-discovery-stations-0-3-plan
status: proposed
class: devflow
serves: recall-reaches-authority
date: 2026-09-11
cites:
  - "governed-discovery-journey-lens-graduation-design | the spec this plan implements — its five seams are these tasks' module boundaries | sha256:77030654da24c3e0 | path: genesis/docs/superpowers/specs/2026-09-11-governed-discovery-journey-lens-graduation-design.md"
  - "bounded-recall-mastery-sprint | the sprint whose round-5 binary and 51 tests are the byte-identical baseline for station zero | sha256:d4c6f66d1685d124 | path: genesis/docs/superpowers/plans/2026-09-11-bounded-recall-mastery-sprint.md"
---

# Governed Discovery (stations 0–3) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Tasks carry the checkbox (`- [ ]`) — that is the commitment grain `epr flow project` mints and `epr flow claim` takes; steps are numbered and tracked inside the task.

**Goal:** Split the recall executor behind the spec's five seams with byte-identical output, then add a reader lens resolved from the actor sidecar, make the SessionStart headline and run-plane line projections of the recall `open`, and stand up the measured standing reader (question bank, `sample`, `judge`, weekly routine, rolling-window habit check).

**Architecture:** `elohim/eprfs/epr-cli/src/flow/memory/recall.rs` (5,073 lines) becomes a `recall/` module directory whose files are the spec's seams (`journey`, `discovery`, `providers`, `lens`, `render`, `refusal`, `measure`). Every role is an existing kind: `ProcessSpec`/`Bound` (recipe), `Intent` (need), `AttentionTending` (lens), `FlowEvent`+`Verdict`+`Observation` (sample), `ProjectionRequest` (view). Nothing mints a struct that the spec maps to an existing one. Hooks stay thin Python that shells to `epr`.

**Tech Stack:** Rust 2021 (crate `elohim-epr-cli`, deps `elohim-epr`, `elohim-epr-rea`, `eprfs-agent`, `serde_json`, `sha2`); Python 3.12 hooks under `.claude/hooks/`; cucumber-js a2o profiles `ceremony` and `collective-memory`; the measure registry `.claude/epr-meta/measures.yaml`.

**Spec:** `genesis/docs/superpowers/specs/2026-09-11-governed-discovery-journey-lens-graduation-design.md`

## Global Constraints

- Native gate, run after every task that touches Rust, EXIT echoed on its own line: `cd /projects/elohim && env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-gate-target CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 cargo test --manifest-path elohim/eprfs/Cargo.toml -p elohim-epr-cli --test flow_memory_recall --test flow_concerns_corrections --test flow_memory_footprint --test help_is_a_question --test flow_report; echo EXIT=$?` plus `cargo fmt --all --manifest-path elohim/eprfs/Cargo.toml -- --check` and `cargo clippy --manifest-path elohim/eprfs/Cargo.toml -p elohim-epr-cli --all-targets -- -D warnings`. Claim the cargo berth first: `BERTH_SESSION=<session> berth claim cargo --ttl 7200`.
- Integrated gate before any commit that touches hooks, a2o or the contract: `just gate memory-ceremony; echo EXIT=$?`.
- Honesty floor: every rendered view at every lens prints recipe CID, lens CID, selection rule, omissions and receipts (collapsed to one line at `minimal`, never dropped). Unfilterable content classes (corrections, counter-evidence, accountability, own-community facts) render regardless of lens.
- Nothing mints a kind the spec maps to an existing one. Use `elohim_epr_rea::{ProcessSpec, StageSpec, Bound, Intent, FlowEvent, AgentRef}`, `elohim_epr::verdict::{Verdict, Witness, CheckWitness, Decision}`, `eprfs_agent::memory::{ProjectionRequest, Feedback, FeedbackKind}`.
- The recall contract's `version` moves whenever its bytes change; every bump is named in the task's commit message and the a2o ceremony profile is re-run.
- Path-limited commits only (`git add -- <paths>`); the elohim-storage WIP tree is another lane's and is never staged. No pushes.
- Each task ends with an independently runnable check and a commit.

---

## File structure

| Path | Responsibility |
|---|---|
| `elohim/eprfs/epr-cli/src/flow/memory/recall/mod.rs` | public surface (`run`, `usage`, `Contract`, `Execution`, re-exports); dispatch only |
| `…/recall/journey.rs` | session state machine: `execute`, `prior_state`, `revalidate`, `refresh_selected`, `native_context`, `retain_evidence`, continuation |
| `…/recall/discovery.rs` | `discover`, `discover_scored`, `question_terms`, `outline_with_terms`, `best_section`, `first_screen`, `matching_habits`, `focus_area`, `frontmatter_header`, `find_close`, `trim_to_character_boundary` |
| `…/recall/providers.rs` | `Provider` trait; `LocalLexical` (today's traversal), `MemPalace` (today's `retrieve`/`bounded_process`), `ProviderResult` |
| `…/recall/lens.rs` | `ReaderRef`, `LensLevel`, `LensView`, `LensProvenance`, `ResolveLens`, the `role@model → LensLevel` table reader |
| `…/recall/render.rs` | `render`, `render_first_screen`, `render_outline`, `render_concerns`, `summary_line`, `render_hits`, `bullet`, `clip`, `thousands`, `RenderFloor` |
| `…/recall/refusal.rs` | `print_refusal`, `refusal_lines`, `remedy_for`, `one_line`, `accepted_flags` |
| `…/recall/measure.rs` | `measure`, `sample_balance`, `external_sample`, `compare_samples`, `parse_measure_scope` |
| `…/recall/receipts.rs` | `excerpt`, `read_line_bounded`, `save_receipt`, `load_receipt`, `adopt_receipts`, `split_receipt_name`, `contained`, `refuse_private_import` |
| `elohim/eprfs/epr-cli/tests/flow_memory_recall.rs` | existing 51 tests, unchanged in station 0 |
| `elohim/eprfs/epr-cli/tests/flow_memory_recall_lens.rs` | station 1 tests |
| `elohim/eprfs/epr-cli/tests/flow_memory_recall_sample.rs` | station 3 tests |
| `.epr-meta/elohim/algorithms/recall-contract.json` | the recipe; gains `lens_table` (station 1) and `question_bank` pointer (station 3) |
| `.epr-meta/elohim/algorithms/recall-questions.json` | the question bank (station 3) |
| `.claude/hooks/load-project-context.py`, `.claude/hooks/run-projection.py` | become thin renderers of `epr flow memory recall open --lens minimal|simple` (station 2) |
| `.claude/hooks/.epr-meta` | the bootstrapping-head rule (station 2) |
| `.claude/epr-meta/measures.yaml` | `recall-journey-window@1` lens row (station 3) |
| `.epr-meta/recall-reaches-authority.habit.md` | check 3 re-pointed at the rolling window (station 3) |

---

## Station 0 — the split (byte-identical)

### Task 0.1: Golden capture before any move

- [ ] Task 0.1: Golden capture before any move

**Files:**
- Create: `elohim/eprfs/epr-cli/tests/fixtures/recall-golden/README.md`
- Create: `elohim/eprfs/epr-cli/tests/flow_memory_recall_golden.rs`

**Interfaces:**
- Produces: `golden_open_focused()`, `golden_open_whole()`, `golden_refusal()` — each runs the shipped binary on the station-five fixture and asserts sha256 of stdout against a pinned constant.

1. **Step 1: Write the golden test that pins today's three renderings**

```rust
//! Station zero of governed discovery: the split must be byte-identical. Three renderings are
//! pinned by digest BEFORE any function moves; the constants are re-baselined only by a task
//! that says why.
use std::path::Path;
use std::process::Command;
use sha2::{Digest, Sha256};

mod common; // re-use tests/common.rs helpers: repo(), save_contract(), contract_value(), write()

fn run_text(root: &Path, args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_epr"))
        .args(["flow", "memory", "recall"]).args(args)
        .args(["--root", &root.to_string_lossy(), "--contract", "contract.json"])
        .output().expect("epr runs");
    String::from_utf8(out.stdout).expect("utf8")
}
fn digest(s: &str) -> String { format!("{:x}", Sha256::digest(s.as_bytes())) }

const GOLDEN_FOCUSED: &str = "<fill from first run>";
const GOLDEN_WHOLE: &str = "<fill from first run>";
const GOLDEN_REFUSAL: &str = "<fill from first run>";

#[test]
fn focused_open_is_byte_identical() {
    let dir = common::repo();
    common::begin_focused(dir.path(), "which command rebuilds the stale index");
    let text = run_text(dir.path(), &["open", "--session", "golden", "--need",
        "which command rebuilds the stale index", "--scope", "tooling"]);
    assert_eq!(digest(&text), GOLDEN_FOCUSED, "{text}");
}
#[test]
fn whole_open_is_byte_identical() {
    let dir = common::repo();
    let text = run_text(dir.path(), &["open", "--session", "golden-whole", "--need", "orient"]);
    assert_eq!(digest(&text), GOLDEN_WHOLE, "{text}");
}
#[test]
fn refusal_is_byte_identical() {
    let dir = common::repo();
    let text = run_text(dir.path(), &["read", "--session", "golden-refuse", "--path", "worktrees/x.md", "--lines", "1:2"]);
    assert_eq!(digest(&text), GOLDEN_REFUSAL, "{text}");
}
```

The fixture helpers (`repo`, `begin_focused`, `write`, `save_contract`, `contract_value`) exist in `tests/flow_memory_recall.rs`; move them to `tests/common.rs` as `pub fn` in this task so both test binaries share them (Rust integration tests share a `tests/common/mod.rs` — create `tests/common/mod.rs` and `mod common;` in both files).

2. **Step 2: Run once to capture the digests; paste them into the constants**

Run: `env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-gate-target cargo test --manifest-path elohim/eprfs/Cargo.toml -p elohim-epr-cli --test flow_memory_recall_golden -- --nocapture 2>&1 | grep -E "assertion|left|right" | head -6`
Expected: three failures printing the actual digest; copy each into its constant. Record the three digests and the binary sha256 in `tests/fixtures/recall-golden/README.md`.

3. **Step 3: Run again — all three pass**

Run: same command without `--nocapture`. Expected: `test result: ok. 3 passed`.

4. **Step 4: Commit**

```bash
git add -- elohim/eprfs/epr-cli/tests/flow_memory_recall_golden.rs elohim/eprfs/epr-cli/tests/common elohim/eprfs/epr-cli/tests/flow_memory_recall.rs elohim/eprfs/epr-cli/tests/fixtures/recall-golden
git commit -m "test(recall): pin three golden renderings before the station-zero split"
```

### Task 0.2: Turn `recall.rs` into `recall/mod.rs` and move the receipts seam

- [ ] Task 0.2: Turn `recall.rs` into `recall/mod.rs` and move the receipts seam

**Files:**
- Move: `elohim/eprfs/epr-cli/src/flow/memory/recall.rs` → `elohim/eprfs/epr-cli/src/flow/memory/recall/mod.rs`
- Create: `elohim/eprfs/epr-cli/src/flow/memory/recall/receipts.rs`

**Interfaces:**
- Produces: `pub(crate) fn excerpt`, `read_line_bounded`, `save_receipt`, `load_receipt`, `adopt_receipts`, `split_receipt_name`, `contained`, `refuse_private_import` in `receipts.rs`; `mod.rs` re-exports `pub use receipts::{excerpt, adopt_receipts, refuse_private_import, save_receipt};` so `tests/flow_memory_recall.rs` (which imports them by path) keeps compiling.

1. **Step 1: `git mv` the file and add the module**

```bash
cd elohim/eprfs/epr-cli/src/flow/memory && mkdir recall && git mv recall.rs recall/mod.rs
```

2. **Step 2: Cut the receipts functions into `receipts.rs` verbatim**

Use line ranges from `grep -n "^fn \(excerpt\|read_line_bounded\|save_receipt\|load_receipt\|adopt_receipts\|split_receipt_name\|contained\|refuse_private_import\)\b" mod.rs`. Each function moves with its doc comment and any private helper only it uses (`hex`, `libc_nofollow`, `modified_nanos`, `parse_range` stay in `mod.rs` if used elsewhere; otherwise move them too). Top of `receipts.rs`:

```rust
//! Receipts and bounded reads — the only code that touches source bytes on a reader's behalf.
use super::*;   // keeps every `use` the functions relied on; tighten in a later pass
```

In `mod.rs` add `mod receipts;` and `pub use receipts::{adopt_receipts, excerpt, refuse_private_import, save_receipt};` and change the `use` in `tests/flow_memory_recall.rs` only if a path changed (it should not: the module path `flow::memory::recall::…` is unchanged).

3. **Step 3: Build, run the golden + recall suites**

Run the native gate (Global Constraints). Expected: `EXIT=0`, golden `3 passed`, recall `51 passed`.

4. **Step 4: Commit**

```bash
git add -- elohim/eprfs/epr-cli/src/flow/memory/recall
git commit -m "refactor(recall): recall.rs becomes recall/mod.rs; receipts seam extracted verbatim"
```

### Task 0.3: Extract `refusal.rs`, `render.rs`, `measure.rs`

- [ ] Task 0.3: Extract `refusal.rs`, `render.rs`, `measure.rs`

**Files:**
- Create: `…/recall/refusal.rs` (`print_refusal`, `refusal_lines`, `remedy_for`, `one_line`, `accepted_flags`)
- Create: `…/recall/render.rs` (`render`, `render_first_screen`, `render_outline`, `render_concerns`, `summary_line`, `render_hits`, `bullet`, `clip`, `number_of`, `count_of`, `thousands`)
- Create: `…/recall/measure.rs` (`measure`, `sample_balance`, `external_sample`, `compare_samples`, `parse_measure_scope`, `prefixed`)

**Interfaces:**
- Produces: `pub(super) fn render(view: &Value) -> String` (unchanged signature); `pub(super) fn print_refusal(message: &str, session: &str, output_limit: Option<usize>) -> FlowResult<ExitCode>`; `pub(super) fn measure(args: &Args, contract: &Contract, state: &mut Value, phase: &str, method: &str) -> FlowResult<Value>`.

1. **Step 1: Move each group verbatim with `use super::*;` at the top of each new file; add `mod refusal; mod render; mod measure;` to `mod.rs`**
2. **Step 2: Run the native gate** — Expected `EXIT=0`, golden 3 passed.
3. **Step 3: Commit** — `git commit -m "refactor(recall): refusal, render and measure seams extracted verbatim"`

### Task 0.4: Extract `discovery.rs` and `providers.rs` with the `Provider` trait

- [ ] Task 0.4: Extract `discovery.rs` and `providers.rs` with the `Provider` trait

**Files:**
- Create: `…/recall/discovery.rs` (`discover`, `discover_scored`, `question_terms`, `habit_row`, `matching_habits`, `focus_area`, `first_screen`, `trim_to_character_boundary`, `frontmatter_header`, `find_close`, `outline`, `outline_with_terms`, `best_section`)
- Create: `…/recall/providers.rs`

**Interfaces:**
- Produces:

```rust
// providers.rs
pub(super) type ProviderId = String;
pub(super) struct ProviderResult { pub ranked: Vec<Value>, pub ranking_known: bool, pub method: Option<String>, pub usage: Value }
pub(super) trait Provider {
    fn id(&self) -> ProviderId;
    fn candidates(&self, terms: &[String], scope: &Path, contract: &Contract, session_root: &Path) -> FlowResult<ProviderResult>;
}
pub(super) struct LocalLexical;                 // wraps discovery::discover_scored; ranking_known = true, method = contract CID
pub(super) struct MemPalace { pub palace: PathBuf } // wraps retrieve()/bounded_process(); ranking_known = false, method = None
pub(super) fn providers_for(contract: &Contract) -> Vec<Box<dyn Provider>>;  // reads contract.ceremony.providers order
```

1. **Step 1: Write the failing unit test in `providers.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_lexical_declares_its_ranking_known_and_pins_the_contract_method() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.md"), "---\ntitle: alpha\ndescription: mempalace re-mine\n---\nbody\n").unwrap();
        let contract = Contract::from_value(crate::flow::memory::recall::tests_support::minimal_contract()).unwrap();
        let out = LocalLexical.candidates(&["mempalace".into()], dir.path(), &contract, dir.path()).unwrap();
        assert!(out.ranking_known);
        assert_eq!(out.method.as_deref(), Some(contract.method_cid().as_str()));
        assert_eq!(out.ranked.len(), 1);
    }
    #[test]
    fn mempalace_declares_its_ranking_unknown() {
        let p = MemPalace { palace: PathBuf::from("/nonexistent") };
        assert_eq!(p.id(), "mempalace");
        // without a binary the provider returns an empty, honest result rather than an error
        let contract = Contract::from_value(crate::flow::memory::recall::tests_support::minimal_contract()).unwrap();
        let out = p.candidates(&["x".into()], Path::new("."), &contract, Path::new(".")).unwrap();
        assert!(!out.ranking_known); assert!(out.ranked.is_empty());
    }
}
```

`tests_support::minimal_contract()` returns the JSON `contract_value(false)` already built in `tests/flow_memory_recall.rs`; move that builder into `mod.rs` under `#[cfg(test)] pub mod tests_support` and have the integration test call it too.

2. **Step 2: Run** `cargo test -p elohim-epr-cli --lib providers` — Expected: FAIL (types not defined).
3. **Step 3: Implement** by moving the discovery functions verbatim into `discovery.rs` and wrapping them in `LocalLexical::candidates` (call `discover_scored` and translate its `(candidates, omissions, usage)` into `ProviderResult`); wrap `retrieve` in `MemPalace::candidates`. The `search` operation in `journey.rs` (still `mod.rs` at this point) calls `providers_for(contract)` and iterates in declared order, stopping at the first provider that returns candidates — exactly today's "local, then optional MemPalace" order, so output is unchanged.
4. **Step 4: Run the native gate** — Expected `EXIT=0`, golden 3 passed, lib tests pass.
5. **Step 5: Commit** — `git commit -m "refactor(recall): discovery seam and Provider trait; local lexical and MemPalace as declared providers"`

### Task 0.5: Extract `journey.rs`; `mod.rs` is dispatch only

- [ ] Task 0.5: Extract `journey.rs`; `mod.rs` is dispatch only

**Files:**
- Create: `…/recall/journey.rs` (`execute`, `prior_state`, `revalidate`, `refresh_selected`, `native_context`, `native_concerns`, `evidence_continuation`, `retain_evidence`, `relevant_findings`, `selected_evidence`, `receipt_stamp`, `continuation_page`, `doc_repair`, `orientation`, `edge_identity`, `edges`, `selected_edge`, `evidence_key`, `last_one`, `without_content`, `cite_desc`, `charge_native`)
- Modify: `…/recall/mod.rs` keeps `Contract`, `Execution`, `Args`, `parse_args`, `run`, `usage`, `command`/`action`/`push_*` helpers, `encode`, `truncate`, `take_flag`.

1. **Step 1: Move verbatim; `mod journey;` and `use journey::execute;` in `mod.rs`.**
2. **Step 2: Assert the split with a size test** in `mod.rs`:

```rust
#[cfg(test)]
mod shape {
    #[test]
    fn no_recall_module_exceeds_the_soft_line_ceiling() {
        for f in ["mod.rs","journey.rs","discovery.rs","providers.rs","render.rs","refusal.rs","measure.rs","receipts.rs"] {
            let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/flow/memory/recall/").to_string() + f).unwrap();
            assert!(text.lines().count() <= 1800, "{f} is over the module ceiling");
        }
    }
}
```

3. **Step 3: Run the native gate** — Expected `EXIT=0`, golden 3, recall 51, lib green, clippy clean.
4. **Step 4: Record the split as a rule**: append to `elohim/eprfs/epr-cli/src/.epr-meta` (create if absent, per the `elohim-epr-metafile` skill) a `class: inject` rule `recall-seams-stay-split` firing on writes under `src/flow/memory/recall/` whose text names the seam a function belongs to.
5. **Step 5: Commit** — `git commit -m "refactor(recall): journey seam extracted; mod.rs is dispatch; seam rule declared"`

### Task 0.6: Re-declare the contract as a ProcessSpec with Bounds (no behaviour change)

- [ ] Task 0.6: Re-declare the contract as a ProcessSpec with Bounds (no behaviour change)

**Files:**
- Modify: `.epr-meta/elohim/algorithms/recall-contract.json` (add `process_spec`, `bounds`; keep `composition`/`limits` for one version as aliases)
- Modify: `…/recall/mod.rs` (`Contract` gains `process_spec() -> ProcessSpec` and `bounds() -> Vec<Bound>`)
- Test: `tests/flow_memory_recall.rs` (new test at the end)

**Interfaces:**
- Produces: `impl Contract { pub fn process_spec(&self) -> elohim_epr_rea::ProcessSpec; pub fn bounds(&self) -> Vec<elohim_epr_rea::Bound>; }`

1. **Step 1: Failing test**

```rust
#[test]
fn the_contract_is_a_process_spec_whose_stages_equal_its_composition() {
    let contract = Contract::load(&repo_root().join(recall::CONTRACT_REL)).unwrap();
    let spec = contract.process_spec();
    let names: Vec<&str> = spec.stages.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["scope","discover","filter","group","select","read","judge"]);
    let bounds = contract.bounds();
    let body = bounds.iter().find(|b| b.unit == "body_scan_bytes").expect("declared");
    assert_eq!(body.limit, 65536.0);
    assert!(matches!(body.sense, Some(elohim_epr_rea::Sense::Ceiling)));
}
```

2. **Step 2: Run** — FAIL (`process_spec` not defined).
3. **Step 3: Implement**: in the JSON add

```json
"process_spec": { "id": "bounded-evidence-recall", "version": 10,
  "stages": [ {"name":"scope","artifact_kind":"Scope"}, {"name":"discover","artifact_kind":"Window"}, {"name":"filter","artifact_kind":"Window"}, {"name":"group","artifact_kind":"Window"}, {"name":"select","artifact_kind":"Candidate"}, {"name":"read","artifact_kind":"Receipt"}, {"name":"judge","artifact_kind":"Verdict"} ],
  "edges": [] },
"bounds": [ {"limit":65536,"unit":"body_scan_bytes","threshold_pct":100,"sense":"ceiling","source":"declared"}, {"limit":20000,"unit":"source_bytes","threshold_pct":100,"sense":"ceiling","source":"declared"} ]
```

and bump `"version": 10`. In `Contract`, `process_spec()` deserializes `value["process_spec"]` into `ProcessSpec` (fallback: build from `composition` so an older contract still loads); `bounds()` deserializes `value["bounds"]` (fallback: build from `limits`). `Contract::limit_usize` stays; nothing else reads the new fields yet.

4. **Step 4: Run the native gate and the a2o ceremony profile** (contract bytes moved: sessions need `adopt`; the fixture tests write their own contract so they are unaffected). Expected `EXIT=0`; `cucumber-js --profile ceremony` 6 scenarios pass.
5. **Step 5: Commit** — `git commit -m "feat(recall): the contract declares itself as a ProcessSpec with Bounds (v10); behaviour unchanged"`

---

## Station 1 — reader lens

### Task 1.1: `lens.rs` — ReaderRef, LensLevel, LensView, ResolveLens from the actor sidecar

- [ ] Task 1.1: `lens.rs` — ReaderRef, LensLevel, LensView, ResolveLens from the actor sidecar

**Files:**
- Create: `…/recall/lens.rs`
- Modify: `.epr-meta/elohim/algorithms/recall-contract.json` (add `lens_table`, bump to v11)
- Test: `elohim/eprfs/epr-cli/tests/flow_memory_recall_lens.rs`

**Interfaces:**
- Produces:

```rust
pub(super) enum LensLevel { Minimal, Simple, Standard, Detail, Debug, Trace }   // graphos vocabulary, same spelling
pub(super) enum Scaffold { LocateAndHandOneCommand, RankAndLetChoose }
pub(super) enum ReaderRef { Agent(elohim_epr_rea::AgentRef), Human { agent_cid: String, tier: String }, Unknown }
pub(super) struct LensProvenance { pub stated: Vec<String>, pub revealed: Vec<String>, pub defaults: String }
pub(super) struct LensView { pub level: LensLevel, pub choice_count: u8, pub density_bytes: usize, pub scaffold: Scaffold,
    pub cid: String, pub provenance: LensProvenance, pub expires_at: Option<String>, pub tended_at: Vec<String> }
pub(super) fn resolve(reader: &ReaderRef, contract: &Contract, requested: Option<LensLevel>, root: &Path) -> LensView;
pub(super) fn reader_from_session(root: &Path, session: &str) -> ReaderRef;   // reads .eprfs/status/actors.jsonl via SidecarActorStore::current(session)
```

Contract addition (`lens_table`, declared, CID-pinned like everything else):

```json
"lens_table": {
  "defaults": {"level":"standard","choice_count":6,"density_bytes":6000,"scaffold":"rank-and-let-choose"},
  "stated": { "claude-haiku-4-5":"minimal", "claude-sonnet-5":"simple", "claude-opus-5":"standard", "claude-fable-5-1":"detail", "gpt-5.6-sol":"standard" },
  "levels": {
    "minimal":  {"choice_count":1,"density_bytes":1500,"scaffold":"locate-and-hand-one-command"},
    "simple":   {"choice_count":3,"density_bytes":3000,"scaffold":"locate-and-hand-one-command"},
    "standard": {"choice_count":6,"density_bytes":6000,"scaffold":"rank-and-let-choose"},
    "detail":   {"choice_count":12,"density_bytes":12000,"scaffold":"rank-and-let-choose"},
    "debug":    {"choice_count":12,"density_bytes":24000,"scaffold":"rank-and-let-choose"},
    "trace":    {"choice_count":24,"density_bytes":32768,"scaffold":"rank-and-let-choose"} },
  "revealed_rule": "a reader tier whose last 3 journeys reached authority at level L is offered the next level; a tier with a mistaken assertion in its last 3 is offered the previous level",
  "expiry_days": 90
}
```

1. **Step 1: Failing integration tests**

```rust
mod common;
use common::*;
#[test]
fn a_sonnet_claim_resolves_to_simple_with_provenance_printed() {
    let dir = repo();
    claim_actor(dir.path(), "agent:reader@claude-sonnet-5", "lens-a");   // helper: runs `epr actor claim --as … --session …` against the fixture root
    let v = view_in(dir.path(), "lens-a", &["open", "--need", "which command rebuilds the stale index", "--scope", "tooling"]);
    assert_eq!(v["lens"]["level"], "simple");
    assert!(v["lens"]["provenance"]["stated"][0].as_str().unwrap().contains("claude-sonnet-5"));
    assert!(v["lens"]["cid"].as_str().unwrap().starts_with("bafk"));
}
#[test]
fn an_unknown_reader_gets_the_defaults_labelled_stated_none() {
    let dir = repo();
    let v = view_in(dir.path(), "lens-b", &["open", "--need", "orient"]);
    assert_eq!(v["lens"]["level"], "standard");
    assert_eq!(v["lens"]["provenance"]["stated"], serde_json::json!(["none"]));
}
#[test]
fn a_requested_wider_lens_is_never_refused_and_is_recorded_as_requested() {
    let dir = repo();
    claim_actor(dir.path(), "agent:reader@claude-haiku-4-5", "lens-c");
    let v = view_in(dir.path(), "lens-c", &["open", "--need", "orient", "--lens", "detail"]);
    assert_eq!(v["lens"]["level"], "detail");
    assert_eq!(v["lens"]["provenance"]["stated"][0], "requested: detail (stated claude-haiku-4-5 → minimal)");
}
```

2. **Step 2: Run** `cargo test -p elohim-epr-cli --test flow_memory_recall_lens` — FAIL (`--lens` unknown; no `lens` key).
3. **Step 3: Implement** `lens.rs` per the interfaces; in `mod.rs` parse `--lens <level>`; in `journey::execute` compute `let lens = lens::resolve(&reader, contract, args.lens, &args.root)` once per operation and put `view["lens"] = lens.to_value()`; the lens CID is `BlobCid::compute_raw(canonical JSON of {level, choice_count, density_bytes, scaffold, provenance})`. Revealed evidence: read the last three `recall-journey` folds for this reader from `.eprfs/status/flows.jsonl` (the `SidecarFlowStore` already reads them; filter `env.reader == reader label`); apply `revealed_rule`.
4. **Step 4: Run the lens tests, then the golden tests.** The golden digests will now FAIL because a `lens:` line is added: re-baseline the three constants in this task and say so in `tests/fixtures/recall-golden/README.md` ("station 1: one `lens:` line added after `Guiding context`").
5. **Step 5: Native gate EXIT=0; commit** — `git commit -m "feat(recall): reader lens resolved from the actor sidecar; lens CID and provenance on every view (contract v11)"`

### Task 1.2: `Render` per lens with the honesty and content floors

- [ ] Task 1.2: `Render` per lens with the honesty and content floors

**Files:**
- Modify: `…/recall/render.rs`
- Modify: `…/recall/lens.rs` (add `RenderFloor`)
- Test: `tests/flow_memory_recall_lens.rs`

**Interfaces:**
- Produces: `pub(super) struct RenderFloor { pub unfilterable: Vec<&'static str>, pub always_printed: [&'static str; 5] }` with `unfilterable = ["correction","counter-evidence","accountability","own-community"]` and `always_printed = ["recipe","lens","selection","omissions","receipts"]`; `pub(super) fn render(view: &Value, lens: &LensView, floor: &RenderFloor) -> String`.

1. **Step 1: Failing tests**

```rust
#[test]
fn minimal_prints_the_five_floor_fields_on_one_line_and_at_most_one_choice() {
    let dir = repo(); claim_actor(dir.path(), "agent:reader@claude-haiku-4-5", "lens-d");
    let text = text_in(dir.path(), "lens-d", &["open", "--need", "which command rebuilds the stale index", "--scope", "tooling"]);
    assert!(text.len() < 1600, "{}", text.len());
    assert!(text.contains("recipe bafk") && text.contains("lens bafk") && text.contains("selection:") && text.contains("omissions:") && text.contains("receipts:"));
    assert_eq!(text.matches("\n  epr flow memory recall ").count(), 1, "one handed command at minimal");
}
#[test]
fn a_correction_candidate_renders_at_every_lens() {
    let dir = repo(); write(dir.path(), "tooling/correction.md", "---\ntitle: correction\ndescription: correction of the stale index claim\ncontent_class: correction\n---\nthe index command changed\n");
    for (model, level) in [("claude-haiku-4-5","minimal"),("claude-fable-5-1","detail")] {
        let s = format!("lens-e-{level}"); claim_actor(dir.path(), &format!("agent:reader@{model}"), &s);
        let text = text_in(dir.path(), &s, &["open", "--need", "stale index", "--scope", "tooling"]);
        assert!(text.contains("tooling/correction.md"), "{level}: {text}");
    }
}
```

2. **Step 2: Run — FAIL.**
3. **Step 3: Implement**: `render` takes the lens; at `Minimal`/`Simple` it emits the orientation as two lines, then the floor line `recipe <cid> · lens <cid> · selection: <rule> · omissions: N · receipts: N`, then candidates truncated to `choice_count` (candidates whose frontmatter `content_class` is in `floor.unfilterable` are kept regardless of count and marked `[floor]`), then exactly `choice_count` Linked choices. `Standard` and above keep today's rendering plus the floor line. `discovery::discover_scored` reads `content_class` from frontmatter into the candidate so the floor can see it.
4. **Step 4: Native gate EXIT=0; re-baseline golden constants (say why); commit** — `git commit -m "feat(recall): render per lens with the honesty and content floors"`

### Task 1.3: Fresh-reader sample on the lens + rule

- [ ] Task 1.3: Fresh-reader sample on the lens + rule

1. **Step 1:** Run one context-reset Sonnet reader against the new binary with the standing question, `--lens` unset; fold the four recall-journey measures with `--env reader=fresh-reader-4-sonnet --env contract=v11`.
2. **Step 2:** Append `DELTA` to `.epr-meta/recall-reaches-authority.habit.md` with the numbers; `python3 .claude/scripts/habits-project.py`.
3. **Step 3:** Commit — `git commit -m "habit(recall-reaches-authority): lens station measured"`

---

## Station 2 — the bootstrapping head

### Task 2.1: `open --purpose bootstrap` renders the session's top red as a ProjectionRequest

- [ ] Task 2.1: `open --purpose bootstrap` renders the session's top red as a ProjectionRequest

**Files:**
- Modify: `…/recall/journey.rs`, `…/recall/discovery.rs`, `…/recall/mod.rs`
- Test: `tests/flow_memory_recall_lens.rs`

**Interfaces:**
- Produces: `--purpose bootstrap` on `open`: the intent becomes the register's top red habit's check line (read from `genesis/manifests/habits.yaml`: first `status: red` habit in declared order, or `active: true` first), `scope` `.`, and the view carries `view["projection"] = {"purpose":"bootstrap","audience":"private","inputs":[<habits.yaml cid>, <flows.jsonl cid>],"omissions":[…]}` — the `ProjectionRequest` shape from `eprfs_agent::memory`.

1. **Step 1: Failing test**

```rust
#[test]
fn bootstrap_purpose_carries_the_top_red_as_intent_and_declares_its_inputs() {
    let dir = repo();
    write(dir.path(), "genesis/manifests/habits.yaml", "habits:\n- id: alpha\n  status: red\n  active: true\n  checks: ['a2o @concern:alpha']\n  invariant: alpha holds\n");
    let v = view_in(dir.path(), "boot", &["open", "--purpose", "bootstrap"]);
    assert!(v["orientation"]["intent"].as_str().unwrap().starts_with("alpha"));
    assert_eq!(v["projection"]["purpose"], "bootstrap");
    assert_eq!(v["projection"]["audience"], "private");
    assert!(v["projection"]["inputs"].as_array().unwrap().len() >= 1);
}
```

2. **Step 2: Run — FAIL.** **Step 3: Implement** (`--purpose` parsed in `mod.rs`; `journey::execute` builds the intent from the register when `purpose == bootstrap` and no `--need`; `discovery::matching_habits` already reads the register — reuse it). **Step 4: Gate; commit** — `git commit -m "feat(recall): open --purpose bootstrap — the session's top red as a ProjectionRequest"`

### Task 2.2: The headline and run-plane hooks become projections of `open`

- [ ] Task 2.2: The headline and run-plane hooks become projections of `open`

**Files:**
- Modify: `.claude/hooks/load-project-context.py` (`get_habits_status`, `get_memory_budget` → one call), `.claude/hooks/run-projection.py`
- Modify: `.claude/hooks/__tests__/drift_observation_test.py` (headline assertions), add `.claude/hooks/__tests__/bootstrap_projection_test.py`
- Modify: `.claude/hooks/.epr-meta` (rule `bootstrapping-head-is-recall-open`)

**Interfaces:**
- Consumes: `epr flow memory recall open --session bootstrap-<session-id> --purpose bootstrap --lens minimal` (headline) and `… --lens simple` (run-plane), both `--json` for the hooks and text for the terminal.
- Produces: the SessionStart block prints, after `epr flow report --headline`, the `minimal` rendering (≤1,500 bytes); the per-turn run-plane prints the `simple` rendering's first 6 lines. Both carry `recipe <cid> · lens <cid>`.

1. **Step 1: Failing Python test**

```python
class BootstrapProjectionCase(unittest.TestCase):
    def test_headline_block_is_the_minimal_lens_of_recall_open(self):
        out = subprocess.run([sys.executable, HOOKS / "load-project-context.py"], input="{}", capture_output=True, text=True, env=self.env(), timeout=60).stdout
        block = out.split("BOOTSTRAP:", 1)[1]
        self.assertLess(len(block.encode()), 1600)
        self.assertIn("recipe bafk", block); self.assertIn("lens bafk", block); self.assertIn("top red:", block)
        self.assertEqual(block.count("\n  epr flow memory recall "), 1)
    def test_run_plane_is_the_simple_lens_and_names_no_second_renderer(self):
        out = subprocess.run([sys.executable, HOOKS / "run-projection.py"], input="{}", capture_output=True, text=True, env=self.env(), timeout=60).stdout
        self.assertIn("lens bafk", out)
        self.assertNotIn("re-derived this turn from habits.yaml", out)   # the bespoke renderer is gone
```

2. **Step 2: Run** `EPR_BIN=/tmp/eprfs-gate-target/debug/epr python3 -m unittest discover -s .claude/hooks/__tests__ -p 'bootstrap_projection_test.py'` — FAIL.
3. **Step 3: Implement**: both hooks call the binary via `_observation.resolve_bin()` with a 6-second budget and print its stdout under `BOOTSTRAP:` (headline) or as the run-plane block; on timeout or non-zero exit they print one line `bootstrap: skipped — <reason>` (honest absence, never a fallback renderer). Delete `run-projection.py`'s own habits.yaml/flows.jsonl derivation and cache (`cache_key`, `read_cache`, `write_cache` go; the binary is the cache). Keep `epr flow report --headline` as the first block (it is the bounds fold, a different derivation).
4. **Step 4: Declare the rule** in `.claude/hooks/.epr-meta`: `class: inject`, `when: {write: "run-projection.py|load-project-context.py"}`, text: "the bootstrapping head is `epr flow memory recall open --purpose bootstrap`; a hook renders it at a lens and never derives a second orientation".
5. **Step 5: Integrated gate** `just gate memory-ceremony; echo EXIT=$?` — Expected 0 (the drift test's headline assertions updated to the new block). **Step 6: Commit** — `git commit -m "feat(hooks): SessionStart headline and run-plane are projections of recall open at minimal/simple; bespoke renderers retired"`

### Task 2.3: Orchestrator bootstrap sample

- [ ] Task 2.3: Orchestrator bootstrap sample

- Start a fresh session (or a context-reset general-purpose agent) with only the SessionStart block, ask it to name the concern's habit, last delta and first action, and fold `recall-screens-to-shape@1` with `--env reader=<model>-bootstrap`. Append the DELTA; reproject; commit.

---

## Station 3 — the standing reader

### Task 3.1: The question bank as Intents

- [ ] Task 3.1: The question bank as Intents

**Files:**
- Create: `.epr-meta/elohim/algorithms/recall-questions.json`
- Modify: `.epr-meta/elohim/algorithms/recall-contract.json` (`"question_bank": ".epr-meta/elohim/algorithms/recall-questions.json"`, bump v12)
- Test: `tests/flow_memory_recall_sample.rs`

**Interfaces:**
- Produces: a JSON array of `{ "id": "q-remine", "intent": { "action": "consume", "resource_spec": {"description": "…"}, "in_scope_of": "<recipe process cid>", "raised_by": {"label":"agent:steward@repo"} }, "reached_when": { "path": ".claude/skills/memory-ceremony/SKILL.md", "assertion": "names the three mempalace commands and the stamp-only-if-not-lock-blocked rule" }, "scope": ".claude/skills" }` — six questions: re-mine discipline; where a correction is closed (`epr flow concerns --corrections`); which habit is top red and its check; how a hook resolves the binary; what `body_scan_bytes` bounds; where journey folds live.

1. **Step 1: Failing test**

```rust
#[test]
fn the_question_bank_loads_as_intents_in_scope_of_the_recipe() {
    let contract = Contract::load(&repo_root().join(recall::CONTRACT_REL)).unwrap();
    let bank = contract.question_bank().unwrap();
    assert!(bank.len() >= 6);
    for q in &bank { assert_eq!(q.intent.in_scope_of.to_string(), contract.method_cid()); assert!(q.reached_when.path.exists_relative_to(&repo_root())); }
}
```

2. **Step 2: Run — FAIL.** **Step 3: Implement** `Contract::question_bank() -> FlowResult<Vec<Question>>` where `Question { id, intent: elohim_epr_rea::Intent, reached_when: ReachedWhen { path, assertion }, scope }`. **Step 4: Gate; commit** — `git commit -m "feat(recall): question bank as Intents in scope of the recipe (contract v12)"`

### Task 3.2: `sample` and `judge` verbs

- [ ] Task 3.2: `sample` and `judge` verbs

**Files:**
- Modify: `…/recall/mod.rs` (operations), create `…/recall/sample.rs`
- Test: `tests/flow_memory_recall_sample.rs`

**Interfaces:**
- Produces:
  - `epr flow memory recall sample --question <id> --reader <agent:role@model> --session <id> [--lens L]` → runs the journey non-interactively: `open` (focused, question scope) → take the first located candidate → `read` its range → `finish`; writes a `FlowEvent { action: Consume, provider: reader, receiver: AgentRef("agent:steward@repo"), resource: <intent cid>, quantity: Magnitude::Count{value: metered_bytes, unit: "bytes"}, process: Some(recipe cid), fulfills: [intent cid] iff the receipt's bytes contain the `reached_when.assertion` terms }` to the flows sidecar and folds `recall-metered-bytes@1`, `recall-screens-to-shape@1`, `recall-unmetered-bytes@1 = 0` with `--env reader=… question=… recipe=… lens=…`.
  - `epr flow memory recall judge --event <cid> --as <seat> --mistaken <n> --reason <text>` → refuses if `seat == event.provider`; writes `Verdict { axis: "recall-journey", subject: Some(event cid), decision: Permit|Refuse, witness: Witness{checks:[CheckWitness{check_id:"mistaken-assertions", outcome, summary, observed: n}]} }` as a `verdict` note on the plan and folds `recall-mistaken-assertions@1`.

1. **Step 1: Failing tests**

```rust
#[test]
fn sample_reaches_authority_on_the_fixture_and_folds_its_measures() {
    let dir = repo_with_bank();   // fixture: one question whose reached_when names tooling/skill.md
    let v = ok_in(dir.path(), "smp", &["sample", "--question", "q-fixture", "--reader", "agent:reader@claude-sonnet-5"]);
    assert_eq!(v["event"]["action"], "consume");
    assert_eq!(v["event"]["fulfills"].as_array().unwrap().len(), 1);
    let folds = flows(dir.path()).into_iter().filter(|r| r["measure"] == "recall-metered-bytes@1").count();
    assert_eq!(folds, 1);
}
#[test]
fn a_reader_cannot_judge_its_own_journey() {
    let dir = repo_with_bank();
    let v = ok_in(dir.path(), "smp2", &["sample", "--question", "q-fixture", "--reader", "agent:reader@claude-sonnet-5"]);
    let cid = v["event"]["cid"].as_str().unwrap();
    let r = run_in(dir.path(), "smp2", &["judge", "--event", cid, "--as", "agent:reader@claude-sonnet-5", "--mistaken", "0", "--reason", "self"]);
    assert_eq!(r.status, 2); assert!(r.stdout.contains("refused: a reader never judges its own journey"));
}
#[test]
fn a_second_seat_verdict_folds_mistaken_assertions() {
    let dir = repo_with_bank();
    let v = ok_in(dir.path(), "smp3", &["sample", "--question", "q-fixture", "--reader", "agent:reader@claude-sonnet-5"]);
    let cid = v["event"]["cid"].as_str().unwrap();
    let j = ok_in(dir.path(), "smp3-seat", &["judge", "--event", cid, "--as", "agent:seat@claude-opus-5", "--mistaken", "1", "--reason", "misread the stamp rule"]);
    assert_eq!(j["verdict"]["witness"]["checks"][0]["observed"], 1);
    assert_eq!(j["verdict"]["decision"], "refuse");
}
```

2. **Step 2: Run — FAIL.** **Step 3: Implement** `sample.rs` composing the existing operations in-process (call `journey::execute` with synthesized `Args` for `open`, `read`, `finish`); reuse `flow::note` for the folds and `flow::note --kind verdict` for the Verdict. **Step 4: Gate; commit** — `git commit -m "feat(recall): sample and judge — a journey as a FlowEvent, a second-seat Verdict, folds on the recall-journey measures"`

### Task 3.3: Rolling window bound and the habit check

- [ ] Task 3.3: Rolling window bound and the habit check

**Files:**
- Modify: `.claude/epr-meta/measures.yaml` (lens `recall-journey-window-ceiling@1`: `derive: rate-over-window`, `consumes: [recall-mistaken-assertions@1, recall-unmetered-bytes@1]`, `window_days: 91`, `hard: 0.2` — fraction of journeys in the window with any mistaken assertion or unmetered bytes)
- Modify: `elohim/eprfs/epr-cli/src/flow/report.rs` (support `derive: rate-over-window` — count folds in the window whose value > 0 over all folds in the window; `skipped` when the window holds fewer than 3 folds)
- Modify: `.epr-meta/recall-reaches-authority.habit.md` (check 3 → `epr flow report --bound recall-journey-window-ceiling`)
- Test: `tests/flow_report.rs`

1. **Step 1: Failing test** in `tests/flow_report.rs`: a fixture with 5 folds (one with value 1) over 10 days reads `passed … 0.20 within hard 0.2`; with 2 folds reads `skipped — fewer than 3 journeys in window`.
2. **Step 2: Run — FAIL.** **Step 3: Implement** in `report.rs` beside `count-since-reset`. **Step 4: Gate; reproject habits; commit** — `git commit -m "feat(report): rate-over-window derive; recall habit reads a rolling window"`

### Task 3.4: The weekly routine and the reader tiers

- [ ] Task 3.4: The weekly routine and the reader tiers

**Files:**
- Create: `.claude/workflows/recall-standing-reader.js` (Workflow script: for each reader tier the actor sidecar has seen in 30 days × each question: `agent(prompt=<context-reset reader brief>, model=tier)`, then one `judge` per sample by a seat of a different tier; folds land via the verbs)
- Modify: `genesis/build-manifest.json` (`gate.projects.memory-ceremony.inputs.sources` gains the workflow and the question bank)

1. **Step 1:** Write the workflow with `meta`, two phases (`Sample`, `Judge`), reader brief identical to the 2026-09-11 fresh-reader brief (entry-only, no direct reads), judge brief given the `reached_when.assertion`.
2. **Step 2:** Dry-run once with one tier and one question; confirm one FlowEvent, one Verdict, five folds.
3. **Step 3:** Schedule weekly via `/schedule` (operator-visible routine) — document the routine id in the habit atom.
4. **Step 4:** Commit — `git commit -m "feat(workflows): weekly standing reader across tiers; judged by a different tier"`

### Task 3.5: Close station 3

- [ ] Task 3.5: Close station 3

- Append DELTA to the habit with the first window's reading; reproject; `just gate memory-ceremony` EXIT=0; commit `habit(recall-reaches-authority): standing reader live; window reading recorded`.

---

## Self-review

- **Spec coverage:** Station 0 → seams (0.2–0.5) and ProcessSpec/Bounds (0.6). Station 1 → ReaderRef/ResolveLens/lens table (1.1), Render floors (1.2), sample (1.3). Station 2 → `--purpose bootstrap` ProjectionRequest (2.1), hooks as projections + rule (2.2), sample (2.3). Station 3 → Intents bank (3.1), FlowEvent/Verdict verbs (3.2), rolling window (3.3), routine across tiers (3.4). Stations 4 and 5 are out of scope by the operator's direction.
- **Placeholder scan:** the only `<fill …>` strings are the three golden digests captured by Task 0.1 step 2, by design.
- **Type consistency:** `LensView`, `LensLevel`, `ReaderRef`, `RenderFloor`, `Provider`, `ProviderResult`, `Question`, `ReachedWhen` are defined once and used by the later tasks under the same names.
