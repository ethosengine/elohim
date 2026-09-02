---
id: ark-s0-launcher-plan
status: active
cites:
  - "compute-envelope-tevah | Tevah | sha256:25153362aae54306 | path: genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md"
---

# Tevah S0 — the `ark` launcher on the household mesh — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The household mesh launches each conductor under an `ark` process that is its parent, and a SIGKILLed conductor leaves a death witness in the peer's own spool within ten seconds — station 1 of `@concern:death-witness`, the first measured step of habit `runtime-death-witnessed`.

**Architecture:** Three crates under `elohim/ark/`, members of the `elohim/` workspace. `ark-core` is pure (no I/O, no runtime): the `RuntimeManifest`/`Berth` declaration, exit classification, the ring, the death tally and the `RestartGovernor` (an `elohim_compute::Governor`), the witness, passport, and intent records, and the lifecycle state machine. `ark-supervisor` is I/O with no network: the `Driver` trait and the `Native` driver (`std::process`, never `tokio::process`), the reaper (`waitid(WNOWAIT)` → `/proc` → `wait4` rusage), the pipe readers, and the `amber-local` spool. `ark` is the binary every context execs: `run`, `describe`, `witness ls|show`, `hash`. The mesh script gains `MESH_CONDUCTOR_LAUNCH=ark`. Nothing in this slice touches the network, the DHT, custody, or `elohim-storage`'s code (storage delegates in S1).

**Tech Stack:** Rust 1.98 (`elohim/` workspace, edition 2021), `elohim-epr` (dag-cbor + `compute_cid`), `elohim-compute` (`Governor`, `Refusal`, `LimitOwner`, `BuildInfo`), `elohim-seam-contracts` (`Answer<T>`), `serde` + `serde_json` + `serde_ipld_dagcbor 0.6`, `sha2 0.10`, `nix 0.31` (`process`, `signal`) + `libc 0.2` (supervisor only), `clap 4.5` derive (binary only), `tempfile 3` (dev). Bash (`app/elohim-app/scripts/hc-mesh.sh`), Cucumber/TypeScript a2o (`genesis/a2o`).

**Spec:** `genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md` — §2 (primitive), §3 (`RuntimeManifest`/`Berth`), §5.1 (verdicts), §6 (witness path, the `amber-local` leg only), §8 (crate boundaries), §11 S0, §12 items 1, 5, 7, 15, 18, 19, 20, 24.

## Delegation contract (the operator's instruction: plan and orchestrate, delegate the legwork)

- **Executor** is named per task: **Codex** (`codex exec -s workspace-write -C /projects/elohim "<task text>"`, model default) or **Opus** (the `rust-architect` agent via the Agent tool, `model: opus`). The orchestrator (this session) dispatches, never implements, and reviews every task's diff before the next task starts.
- **Reviewer** is the other executor, read-only, with the task's own Interfaces block as the rubric: does the diff produce exactly the named symbols, do the named tests exist and pass, does the boundary test still pass, is anything outside `Files:` touched. Reviewer prompts carry the admissibility clause: only tree-fixable findings are admissible (no history rewrites, no TDD-evidence demands).
- Every executor prompt is the task text verbatim plus the **Global Constraints** section, plus: "Commit-only, path-limited `git add` of exactly the task's files, never push, never `git add -A`, never run `kubectl`, never start mesh processes inside a background tool task."
- Tasks 1 → 6 are sequential on `ark-core` (each builds on the last's types). Tasks 7 and 8 may run in parallel after 6 (disjoint files). 9 follows both. 10 follows 9. 11 follows 10. 12 follows 11. 13 is the orchestrator's.

## Global Constraints

- **Names (spec §12 item 20):** prose says tevah; code says `ark`. Packages: `elohim-ark-core` (lib `ark_core`, dir `elohim/ark/core`), `elohim-ark-supervisor` (lib `ark_supervisor`, dir `elohim/ark/supervisor`), `elohim-ark` (binary `ark`, dir `elohim/ark/cli`). No crate, type, module, or path may contain `tevah`, `pod`, or `seed`.
- **Purity boundary of `ark-core` (spec §8):** dependencies are exactly `elohim-epr`, `elohim-compute`, `elohim-seam-contracts` (path deps), `serde`, `serde_json`, `serde_ipld_dagcbor`, `cid`, `chrono`, `thiserror`, `sha2`, `hex`. A test (`boundary::no_runtime_or_io_deps`, Task 1) reads `Cargo.toml` and refuses `tokio`, `diesel`, `libp2p`, `iroh`, `nix`, `libc`, `reqwest`, `hyper`, `axum`, `rusqlite`, `hdk`, `hdi`. `ark-supervisor` adds `nix`, `libc` only. No `tokio` anywhere in S0 (the supervisor is threads + `std::sync::mpsc`; tokio arrives with the admin socket in S2).
- **Reaping (spec §12 item 19):** children are spawned with `std::process::Command` and reaped by the supervisor's own reaper — `waitid(P_PID, WEXITED|WNOWAIT)` to learn of death without consuming it, then `/proc/<pid>/status`, then `libc::wait4` for `rusage`. `tokio::process` is refused. The supervisor calls `prctl(PR_SET_CHILD_SUBREAPER, 1)` at start. `PR_SET_PDEATHSIG` is never set on a conductor.
- **The passport hashes what it runs (spec §12 item 7):** before spawn, the driver sha256s the resolved artifact file; a mismatch against `ArtifactRef::Pinned.sha256` is exit code 66, never a warning.
- **Write-ahead intent (spec §6):** every spawn, restart, stop, and give-up is appended to `intents.log` BEFORE the action; a witness is written to disk BEFORE any restart decision is acted on. Row-before-blob: write to `<name>.tmp`, `fsync`, rename.
- **Identity math is never re-derived (`elohim/.epr-meta`):** CIDs come from `elohim_epr::cid::compute_cid` over `serde_ipld_dagcbor::to_vec` of the record. No local sha-to-cid code.
- **CIDs inside records are strings** (base32 `bafy…`) in S0. The manifest becomes an `Epr` payload with real dag-cbor links in S1; this is recorded in the crate's `CLAUDE.md` as a declared S0 simplification.
- **Build environment:** `cd /projects/elohim/elohim && CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim/dev RUSTFLAGS="" cargo <cmd> -p <pkg>`. Never judge a cargo run from piped output — end every cargo command with `; echo EXIT=$?` on its own line. `cargo nextest` is not installed; use `cargo test`.
- **Gate:** `just gate elohim-ark` (Task 1 wires it) = `cargo fmt --check && cargo clippy -p elohim-ark-core -p elohim-ark-supervisor -p elohim-ark -- -D warnings && cargo test -p … --all-targets`. Clippy `-D warnings` is the bar for every task.
- **Exit codes of `ark run`:** 0 clean stop; 3 every process reached `GiveUp`; 64 usage; 65 manifest or berth invalid; 66 artifact hash mismatch; 67 spool unwritable.
- **Source of truth for every record in this plan (the P2P design gate ran in spec §7, C0–C14):** the death witness is Notarized (A) at INCIDENT grain and rides `issue-report` + `metadata_json.kind: death-witness` once anchored (S1); the passport is C live + one `node-context` atom per applied transition (S1); the manifest is a Manifest-kind EPR (`runtime-manifest`). Everything the S0 spool holds — witnesses, incidents, intents, tally, passport — is the `amber-local` leg: a content-addressed LOCAL projection whose CIDs are the identities the DHT anchors later. No new DHT entry type, route, or diesel table is created by this plan.
- **Spool layout** (`Berth.data_root/ark/`, mode 0700): `intents.log` (append-only JSON lines), `witnesses/<cid>.cbor` + `witnesses/<cid>.json`, `incidents/<incident_cid>.json`, `passport.json`.
- **Mesh:** `MESH_CONDUCTOR_LAUNCH=ark` is additive; `hc` and `direct` modes are untouched. Never start mesh processes inside a background tool task; the operator or the orchestrator runs `just mesh start` in the foreground.
- **Commits:** path-limited `git add` of exactly the listed files. Never `git add -A`. Never push. Trailer on every commit: `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` (Codex commits carry the same trailer so the batch reads as one lane).

---

## File structure

```
elohim/Cargo.toml                       modify: members += ark/core, ark/supervisor, ark/cli; [workspace.dependencies] += clap, nix, libc, sha2 (sha2/hex exist), serde_ipld_dagcbor, cid, tempfile
elohim/ark/CLAUDE.md                    the crate family's guidance (boundaries, S0 simplifications, how to run on the mesh)
elohim/ark/build-manifest.json          gate.projects.elohim-ark → recipe _gate-elohim-ark
justfile                                modify: + _gate-elohim-ark
elohim/ark/core/Cargo.toml
elohim/ark/core/seam-registry.yaml      birth rule: the decision predicates minted here (Task 13)
elohim/ark/core/src/lib.rs              module tree + re-exports + boundary test
elohim/ark/core/src/ring.rs             RingBuffer (lifted from process_manager.rs)
elohim/ark/core/src/exit.rs             ExitClass, ReadinessOutcome, classify_readiness_outcome
elohim/ark/core/src/manifest.rs         RuntimeManifest, ProcessSpec, ArtifactRef, ChildPolicy, Probe, …, manifest_cid()
elohim/ark/core/src/berth.rs            Berth, Template resolution, PassphraseSource
elohim/ark/core/src/tally.rs            DeathRecord, DeathTally, same_cause_key
elohim/ark/core/src/verdict.rs          Verdict, GiveUpReason, RestartGovernor: Governor
elohim/ark/core/src/sample.rs           ProcessSample
elohim/ark/core/src/witness.rs          DeathWitness, Incident, witness_cid()
elohim/ark/core/src/passport.rs         Passport, ProcessPassport, EffectiveTier
elohim/ark/core/src/intent.rs           Intent, IntentAction
elohim/ark/core/src/sink.rs             WitnessSink trait, Clock trait
elohim/ark/core/src/lifecycle.rs        ChildState, Event, Action, step()
elohim/ark/supervisor/Cargo.toml
elohim/ark/supervisor/src/lib.rs
elohim/ark/supervisor/src/driver.rs     Driver trait, Started, Fingerprint
elohim/ark/supervisor/src/native.rs     NativeDriver (std::process::Command)
elohim/ark/supervisor/src/reaper.rs     wait_nowait(), proc_status(), reap_with_rusage()
elohim/ark/supervisor/src/pipes.rs      spawn_line_reader() → ring + log + readiness matcher
elohim/ark/supervisor/src/spool.rs      Spool: WitnessSink over the amber-local layout; ls/show readers
elohim/ark/supervisor/src/supervisor.rs Supervisor: the loop, signal contract, incident bookkeeping
elohim/ark/supervisor/tests/native_reap.rs
elohim/ark/supervisor/tests/supervise_death.rs
elohim/ark/cli/Cargo.toml
elohim/ark/cli/src/main.rs              clap: run | describe | witness ls|show | hash
elohim/ark/cli/tests/cli_smoke.rs
app/elohim-app/scripts/hc-mesh.sh       modify: MESH_CONDUCTOR_LAUNCH=ark (start + join-peer), ARK_BIN, write_ark_declarations(), mesh_conductor_pid(), status role
genesis/a2o/steps/mesh/death-witness.steps.ts
genesis/a2o/features/resilience/death-witness.feature   modify: station 1 loses @wip
elohim/elohim-storage/.epr-meta/runtime-death-witnessed.habit.md   modify: DELTA line (Task 13)
genesis/manifests/habits.yaml           re-projected (Task 13)
```

---

### Task 1: Workspace scaffold, gate, and the purity boundary

**Executor:** Opus (rust-architect). **Reviewer:** Codex.

**Files:**
- Modify: `elohim/Cargo.toml` (members + workspace.dependencies)
- Create: `elohim/ark/CLAUDE.md`, `elohim/ark/build-manifest.json`
- Modify: `justfile` (after `_gate-elohim-epr`, line ~401)
- Create: `elohim/ark/core/Cargo.toml`, `elohim/ark/core/src/lib.rs`
- Create: `elohim/ark/supervisor/Cargo.toml`, `elohim/ark/supervisor/src/lib.rs`
- Create: `elohim/ark/cli/Cargo.toml`, `elohim/ark/cli/src/main.rs`

**Interfaces:**
- Produces: three compiling crates; `just gate elohim-ark` runs; `ark_core::boundary` test module.

- [ ] **Step 1: Workspace membership and shared deps.** In `elohim/Cargo.toml` add to `members`: `"ark/core"`, `"ark/supervisor"`, `"ark/cli"`. Add to `[workspace.dependencies]`:

```toml
# ark — the compute envelope (tevah). Pure core / I/O supervisor / binary.
clap = { version = "4.5", features = ["derive"] }
nix = { version = "0.31", features = ["process", "signal", "fs"] }
libc = "0.2"
serde_ipld_dagcbor = "0.6"
cid = { version = "0.11", features = ["serde-codec"] }
tempfile = "3"
```

- [ ] **Step 2: `elohim/ark/core/Cargo.toml`.**

```toml
[package]
name = "elohim-ark-core"
version = "0.1.0"
edition = "2021"
license = "Apache-2.0"
authors = ["Elohim Protocol"]
description = "ark-core — the pure half of the elohim compute envelope (tevah): RuntimeManifest/Berth, exit classes, the death tally, the restart governor, the witness/passport/intent records, and the lifecycle state machine. No I/O, no runtime — the dependency graph is the purity boundary."

[lib]
name = "ark_core"
path = "src/lib.rs"

[dependencies]
# Pure only. Adding tokio/nix/libc/diesel here fails `boundary::no_runtime_or_io_deps`.
elohim-epr = { path = "../../epr" }
elohim-compute = { path = "../../elohim-compute" }
elohim-seam-contracts = { path = "../../../crates/seam-contracts" }
serde = { workspace = true }
serde_json = { workspace = true }
serde_ipld_dagcbor = { workspace = true }
cid = { workspace = true }
chrono = { workspace = true }
thiserror = { workspace = true }
sha2 = { workspace = true }
hex = { workspace = true }
```

- [ ] **Step 3: `elohim/ark/core/src/lib.rs`** with the module tree (modules are created empty-but-compiling here and filled by Tasks 2–6) and the boundary test:

```rust
//! ark-core — the pure half of the elohim compute envelope (tevah).
//!
//! Spec: genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md §3, §5.1, §6, §8.
//! No I/O and no async runtime live here; `ark-supervisor` implements the traits in `sink`.

pub mod berth;
pub mod exit;
pub mod intent;
pub mod lifecycle;
pub mod manifest;
pub mod passport;
pub mod ring;
pub mod sample;
pub mod sink;
pub mod tally;
pub mod verdict;
pub mod witness;

pub use berth::{Berth, PassphraseSource};
pub use exit::{classify_readiness_outcome, ExitClass, ReadinessOutcome};
pub use intent::{Intent, IntentAction};
pub use manifest::{ArtifactRef, ChildPolicy, Probe, ProcessKind, ProcessSpec, RuntimeManifest};
pub use passport::{EffectiveTier, Passport, ProcessPassport};
pub use ring::RingBuffer;
pub use sample::ProcessSample;
pub use sink::{Clock, WitnessSink};
pub use tally::{DeathRecord, DeathTally};
pub use verdict::{GiveUpReason, RestartGovernor, Verdict};
pub use witness::{DeathWitness, Incident};

#[cfg(test)]
mod boundary {
    /// The purity boundary, read from this crate's own manifest: a runtime or I/O
    /// dependency arriving here would let a pure decision do I/O.
    #[test]
    fn no_runtime_or_io_deps() {
        let toml = include_str!("../Cargo.toml");
        const DENIED: &[(&str, &str)] = &[
            ("tokio", "ark-core is plain data + decisions; the supervisor owns the runtime"),
            ("nix", "syscalls are the supervisor's"),
            ("libc", "syscalls are the supervisor's"),
            ("diesel", "persistence is a storage concern"),
            ("rusqlite", "persistence is a storage concern"),
            ("libp2p", "the envelope has no swarm in v1 (spec §12 item 11)"),
            ("iroh", "the envelope has no swarm in v1 (spec §12 item 11)"),
            ("reqwest", "no network in the envelope"),
            ("hyper", "no network in the envelope"),
            ("axum", "no network in the envelope"),
            ("hdk", "the envelope is below the DNA line"),
            ("hdi", "the envelope is below the DNA line"),
        ];
        for (pkg, why) in DENIED {
            let needle = format!("\n{pkg} ");
            let needle_eq = format!("\n{pkg}=");
            assert!(
                !toml.contains(&needle) && !toml.contains(&needle_eq),
                "elohim-ark-core declares `{pkg}` — {why}"
            );
        }
    }
}
```

Each module file for this task is a one-line `//! filled by Task N` stub so the crate compiles; the `pub use` lines are added in the task that creates each type (leave `lib.rs` re-exports commented with `// Task N` markers until then, so the scaffold compiles).

- [ ] **Step 4: `elohim/ark/supervisor/Cargo.toml` and `lib.rs`.**

```toml
[package]
name = "elohim-ark-supervisor"
version = "0.1.0"
edition = "2021"
license = "Apache-2.0"
authors = ["Elohim Protocol"]
description = "ark-supervisor — the I/O half of the elohim compute envelope (tevah): drivers, the reaper, pipe readers, the amber-local spool, and the supervision loop. No network."

[lib]
name = "ark_supervisor"
path = "src/lib.rs"

[dependencies]
elohim-ark-core = { path = "../core" }
elohim-epr = { path = "../../epr" }
serde = { workspace = true }
serde_json = { workspace = true }
serde_ipld_dagcbor = { workspace = true }
chrono = { workspace = true }
thiserror = { workspace = true }
sha2 = { workspace = true }
hex = { workspace = true }
nix = { workspace = true }
libc = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
```

`lib.rs`: `pub mod driver; pub mod native; pub mod pipes; pub mod reaper; pub mod spool; pub mod supervisor;` with stub files.

- [ ] **Step 5: `elohim/ark/cli/Cargo.toml` and `main.rs`.**

```toml
[package]
name = "elohim-ark"
version = "0.1.0"
edition = "2021"
license = "Apache-2.0"
authors = ["Elohim Protocol"]
description = "ark — the launchable unit of the elohim compute envelope (tevah): run a RuntimeManifest in a Berth, describe the passport, list and show death witnesses."

[[bin]]
name = "ark"
path = "src/main.rs"

[dependencies]
elohim-ark-core = { path = "../core" }
elohim-ark-supervisor = { path = "../supervisor" }
clap = { workspace = true }
serde_json = { workspace = true }
```

`main.rs` for this task: a clap `Cli` with the four subcommands declared (`Run { manifest: PathBuf, berth: PathBuf }`, `Describe { berth: PathBuf }`, `Witness { #[command(subcommand)] cmd: WitnessCmd }` with `Ls { berth }` / `Show { berth, cid: String }`, `Hash { file: PathBuf }`) each returning exit 64 with "not yet wired (Task 10)" on stderr.

- [ ] **Step 6: Gate.** `elohim/ark/build-manifest.json`:

```json
{
  "manifestVersion": "1.0",
  "pipeline": "elohim-ark",
  "description": "ark — the elohim compute envelope (tevah): core + supervisor + binary",
  "steps": {
    "gate-ark": {
      "description": "fmt + clippy -D warnings + tests for the three ark crates",
      "inputs": { "sources": ["elohim/ark/**", "elohim/Cargo.toml", "elohim/Cargo.lock"], "buildProcess": [] },
      "outputs": { "artifacts": [], "verify": null },
      "depends": [],
      "executor": { "stage": "Gate Ark", "function": null }
    }
  },
  "gate": {
    "projects": {
      "elohim-ark": {
        "dir": "elohim/ark",
        "steps": ["gate-ark"],
        "run": { "kind": "root-just", "recipe": "_gate-elohim-ark", "cargo": { "workspace": "elohim", "profile": "dev", "rustflags": "" } }
      }
    }
  },
  "deployment": {}
}
```

`justfile`, after `_gate-elohim-epr`:

```make
_gate-elohim-ark:
    cd elohim && cargo fmt --check -p elohim-ark-core -p elohim-ark-supervisor -p elohim-ark && cargo clippy -p elohim-ark-core -p elohim-ark-supervisor -p elohim-ark -- -D warnings && cargo test -p elohim-ark-core -p elohim-ark-supervisor -p elohim-ark --all-targets
```

- [ ] **Step 7: `elohim/ark/CLAUDE.md`** (≤ 40 lines): the three crates and their purity rule; the S0 simplifications (CIDs as strings; threads not tokio; Native driver only, effective tier `None`); the build line from Global Constraints; the mesh line `MESH_CONDUCTOR_LAUNCH=ark ARK_BIN=… just mesh start`; the spec path; "tevah in prose, ark in code".

- [ ] **Step 8: Verify.** `cd elohim && CARGO_TARGET_DIR=… RUSTFLAGS="" cargo test -p elohim-ark-core -p elohim-ark-supervisor -p elohim-ark; echo EXIT=$?` → EXIT=0 with `boundary::no_runtime_or_io_deps` passing; `just gate elohim-ark; echo EXIT=$?` → EXIT=0. `python3 genesis/orchestrator/gate-runner.mjs --list 2>/dev/null | grep elohim-ark` or `just gate elohim-ark` resolving proves discovery.

- [ ] **Step 9: Commit** — `git add elohim/Cargo.toml elohim/Cargo.lock elohim/ark justfile && git commit -m "feat(ark): scaffold the compute-envelope crate family — ark-core (pure), ark-supervisor (I/O), ark (binary); gate wired; purity boundary test"`.

---

### Task 2: `ring.rs` + `exit.rs` — the lifted ring and classifier, plus `ExitClass`

**Executor:** Codex. **Reviewer:** Opus.

**Files:**
- Modify: `elohim/ark/core/src/ring.rs`, `elohim/ark/core/src/exit.rs`, `elohim/ark/core/src/lib.rs` (uncomment re-exports)
- Reference (read only, do not edit): `elohim/elohim-storage/src/conductor/process_manager.rs:115-150, 245-285, 901-960`

**Interfaces:**
- Produces: `pub struct RingBuffer { pub fn new(capacity: usize) -> Self; pub fn push(&mut self, line: String); pub fn last_n(&self, n: usize) -> Vec<String>; pub fn len(&self) -> usize; pub fn is_empty(&self) -> bool }`; `pub enum ReadinessOutcome { ChildExited, Retry, GiveUp }`; `pub fn classify_readiness_outcome(child_exited: bool, attempt: u32, max_retries: u32) -> ReadinessOutcome`; `pub enum ExitClass { Exited { code: i32 }, Signaled { signal: i32, core_dumped: bool }, OomKilled, Unknown }` with `Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug`, `#[serde(rename_all = "kebab-case", tag = "class")]`; `impl ExitClass { pub fn from_raw_wait_status(status: i32) -> Self; pub fn is_clean(&self) -> bool /* Exited{0} */; pub fn same_cause_token(&self) -> String /* "exited:1" | "signaled:9" | "oom" | "unknown" */ }`.

- [ ] **Step 1: Write failing tests** in `ring.rs` (`#[cfg(test)] mod tests`): copy `ring_buffer_keeps_the_last_n_and_drops_the_oldest` from `process_manager.rs:901` verbatim against `RingBuffer::new(3)`. In `exit.rs`: copy `readiness_outcome_prefers_child_death_over_attempt_budget` from `:925` with `Option<()>` replaced by `bool`; add

```rust
#[test]
fn exit_class_decodes_posix_wait_status() {
    // POSIX wait status encoding: exit code in bits 8..16; signal in bits 0..7; core in bit 7.
    assert_eq!(ExitClass::from_raw_wait_status(0), ExitClass::Exited { code: 0 });
    assert_eq!(ExitClass::from_raw_wait_status(1 << 8), ExitClass::Exited { code: 1 });
    assert_eq!(ExitClass::from_raw_wait_status(9), ExitClass::Signaled { signal: 9, core_dumped: false });
    assert_eq!(ExitClass::from_raw_wait_status(11 | 0x80), ExitClass::Signaled { signal: 11, core_dumped: true });
    assert!(ExitClass::Exited { code: 0 }.is_clean());
    assert!(!ExitClass::Signaled { signal: 9, core_dumped: false }.is_clean());
    assert_eq!(ExitClass::Signaled { signal: 9, core_dumped: false }.same_cause_token(), "signaled:9");
    assert_eq!(ExitClass::OomKilled.same_cause_token(), "oom");
}

#[test]
fn exit_class_serde_is_tagged_kebab() {
    let j = serde_json::to_string(&ExitClass::Signaled { signal: 9, core_dumped: false }).unwrap();
    assert_eq!(j, r#"{"class":"signaled","signal":9,"core_dumped":false}"#);
}
```

- [ ] **Step 2: Run** `cargo test -p elohim-ark-core; echo EXIT=$?` → fails to compile (types missing).
- [ ] **Step 3: Implement** `RingBuffer` (lift the `VecDeque` implementation verbatim, make it `pub`, add `len`/`is_empty`, doc-comment "lifted from elohim-storage process_manager.rs 264ce8ce4; storage delegates here in S1"), `ReadinessOutcome` + `classify_readiness_outcome` (lifted, `bool` instead of `Option<()>`), `ExitClass` with `from_raw_wait_status` decoding `WIFEXITED`/`WEXITSTATUS`/`WTERMSIG`/`WCOREDUMP` by hand (bit ops, no libc), `OomKilled` is never produced by the decoder (the supervisor promotes `Signaled{9}` to `OomKilled` when `/proc` or the cgroup says so — S3).
- [ ] **Step 4: Run** tests → PASS; `cargo clippy -p elohim-ark-core -- -D warnings; echo EXIT=$?` → 0.
- [ ] **Step 5: Commit** — `git add elohim/ark/core && git commit -m "feat(ark-core): RingBuffer + readiness classifier lifted from process_manager; ExitClass decodes POSIX wait status"`.

---

### Task 3: `manifest.rs` + `berth.rs` — the declaration and its placement

**Executor:** Codex. **Reviewer:** Opus.

**Files:**
- Modify: `elohim/ark/core/src/manifest.rs`, `elohim/ark/core/src/berth.rs`, `lib.rs` re-exports

**Interfaces:**
- Produces (all `Serialize, Deserialize, Clone, Debug, PartialEq`, `#[serde(rename_all = "snake_case")]`, with `#[serde(default)]` on every optional/defaultable field so a hand-authored JSON manifest may omit them):

```rust
pub const MANIFEST_SCHEMA: u32 = 1;
pub const MANIFEST_KIND: &str = "runtime-manifest";   // EprKind::Manifest schema key (spec §12 item 25)

pub struct RuntimeManifest {
    pub schema: u32,                       // == MANIFEST_SCHEMA
    pub kind: String,                      // == MANIFEST_KIND
    pub supersedes: Option<String>,        // CID string of the previous manifest in the lineage
    pub reach: String,                     // elohim_epr::Reach as its kebab string ("trusted" for a household in S0)
    pub processes: Vec<ProcessSpec>,
}
pub struct ProcessSpec {
    pub name: String,
    pub kind: ProcessKind,                 // Native only in S0; others deserialize but NativeDriver refuses them
    pub artifact: ArtifactRef,
    pub argv: Vec<String>,                 // templates: {data_root} {name} {port.<key>} {artifact}
    pub env: BTreeMap<String, String>,     // templates allowed in values
    pub env_scrub: bool,                   // default true
    pub stdin: StdinSource,                // Null | Passphrase
    pub readiness: Vec<Probe>,             // ladder, in order
    pub policy: ChildPolicy,
    pub listen: Listen,
}
pub enum ProcessKind { Native, InProcess, Wasm, Delegated }
pub enum ArtifactRef {
    Channel { channel_id: String },                                 // resolved in S1 (register 24); NativeDriver refuses in S0 with a named reason
    Pinned { cid: Option<String>, sha256: String, bytes: Option<u64> }, // the lockfile form; sha256 hex is mandatory
}
pub enum StdinSource { Null, Passphrase }
pub enum Probe {
    StdoutLine { contains: String, patience_ms: u64 },
    TcpListen { port_key: String, patience_ms: u64 },              // port_key resolves against Berth.ports
}
pub struct ChildPolicy {
    pub restart: Restart,                  // default Permanent
    pub shutdown: Shutdown,                // default { signal: 2 (SIGINT), grace_ms: 20_000 }
    pub intensity: Intensity,              // default { max_deaths: 5, window_s: 300 }
    pub backoff: Backoff,                  // default { min_s: 1, max_s: 60, steps: 6 }
    pub same_cause_limit: u32,             // default 3
}
pub enum Restart { Permanent, Transient, Temporary }
pub struct Shutdown { pub signal: i32, pub grace_ms: u64 }
pub struct Intensity { pub max_deaths: u32, pub window_s: u64 }
pub struct Backoff { pub min_s: u64, pub max_s: u64, pub steps: u32 }
pub struct Listen { pub ring_lines: usize /* 200 */, pub tail_lines: usize /* 40 */ }

impl RuntimeManifest {
    pub fn from_json(s: &str) -> Result<Self, ManifestError>;      // validates: schema, kind, ≥1 process, unique names, non-empty argv, Pinned.sha256 is 64 hex
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, ManifestError>; // serde_ipld_dagcbor::to_vec
    pub fn cid(&self) -> Result<String, ManifestError>;              // elohim_epr::cid::compute_cid(&canonical_bytes).to_string()
    pub fn process(&self, name: &str) -> Option<&ProcessSpec>;
}
#[derive(thiserror::Error, Debug)] pub enum ManifestError { Json(String), Schema(String), Kind(String), Invalid(String), Encode(String) }
```

and in `berth.rs`:

```rust
pub struct Berth {
    pub manifest: String,                          // CID string; `ark run` refuses a berth whose manifest != the loaded manifest's cid()
    pub node: Option<String>,                      // agent CID; None until S1 binds identity
    pub data_root: PathBuf,
    pub passphrase: PassphraseSource,              // Empty | Literal(String) | File(PathBuf)
    pub ports: BTreeMap<String, u16>,
    pub artifacts: BTreeMap<String, PathBuf>,      // process name → resolved local path (the S0 pinned-local resolver, register 24)
    pub incarnation: u64,                          // monotone; `ark run` bumps and persists it in passport.json
}
pub enum PassphraseSource { Empty, Literal(String), File(PathBuf) }
impl Berth {
    pub fn from_json(s: &str) -> Result<Self, BerthError>;
    pub fn resolve_template(&self, process: &str, artifact_path: &Path, template: &str) -> Result<String, BerthError>;
    // {data_root} → data_root display; {name} → process; {artifact} → artifact_path; {port.admin_ws} → ports["admin_ws"]; unknown key → BerthError::UnknownTemplate(key)
}
```

- [ ] **Step 1: Failing tests** (in each module): `manifest_json_round_trips_and_defaults_apply` (a minimal JSON with one process, only `name`, `artifact`, `argv` set → `policy == ChildPolicy::default()`, `env_scrub == true`, `listen.ring_lines == 200`); `manifest_cid_is_stable_and_order_insensitive_to_json_whitespace` (two JSON strings differing only in whitespace → same `cid()` string starting with `bafy`); `manifest_refuses_wrong_kind_duplicate_names_and_bad_sha` (three JSONs → `Err(ManifestError::Kind(_))`, `Err(Invalid(_))`, `Err(Invalid(_))`); `berth_resolves_templates_and_names_unknown_keys` (`{data_root}/{name}/conductor-config.yaml` resolves; `{port.admin_ws}` resolves to "4444"; `{port.nope}` → `Err(BerthError::UnknownTemplate("port.nope"))`).
- [ ] **Step 2: Run** → compile failure. **Step 3: Implement.** **Step 4: Run** → PASS, clippy clean.
- [ ] **Step 5: Commit** — `git add elohim/ark/core && git commit -m "feat(ark-core): RuntimeManifest (kind runtime-manifest, dag-cbor CID) and Berth with template resolution and the pinned-local resolver map"`.

---

### Task 4: `tally.rs` + `verdict.rs` — the death tally and the restart governor

**Executor:** Codex. **Reviewer:** Opus.

**Files:**
- Modify: `elohim/ark/core/src/tally.rs`, `elohim/ark/core/src/verdict.rs`, `lib.rs`

**Interfaces:**
- Consumes: `ExitClass`, `ChildPolicy`, `Intensity`, `Backoff`, `Restart` (Task 2/3); `elohim_compute::{Governor, Refusal, LimitOwner, RefusalCode}`.
- Produces:

```rust
pub struct DeathRecord { pub at_epoch_s: u64, pub class: ExitClass, pub uptime_ms: u64, pub first_stderr_line: Option<String> }
pub struct DeathTally { pub deaths: Vec<DeathRecord> }          // persisted by the spool; survives ark restarts
impl DeathTally {
    pub fn record(&mut self, d: DeathRecord);
    pub fn deaths_within(&self, now_epoch_s: u64, window_s: u64) -> u32;
    pub fn same_cause_run(&self) -> u32;                          // trailing run length of equal same_cause_key
    pub fn reset_on_ready(&mut self);                             // readiness RESETS the intensity window (spec §3 ChildPolicy)
}
pub fn same_cause_key(d: &DeathRecord) -> String;                 // "<class token>|<first structured stderr line or ''>|<fast:bool uptime<5000ms>"

pub enum Verdict { Restart { after_s: u64, attempt: u32 }, GiveUp { reason: GiveUpReason }, Stop }
pub enum GiveUpReason { SameCause { key: String, count: u32 }, IntensityExceeded { deaths: u32, window_s: u64 }, PolicyTemporary, TransientCleanExit }
pub struct RestartRequest { pub process: String, pub death: DeathRecord }
pub struct RestartGrant { pub bounded_by: BoundedBy, pub policy: ChildPolicy }
pub enum BoundedBy { ManifestPolicy, Commitment { cid: String } }   // S0 always ManifestPolicy (LimitOwner::Operator); S1 mints the self-contract → Commitment
pub struct RestartContext { pub now_epoch_s: u64, pub tally: DeathTally }
pub struct RestartGovernor;
impl Governor for RestartGovernor {
    type Request = RestartRequest; type Grant = RestartGrant; type Context = RestartContext; type Effect = Verdict;
    // authorize: Restart::Temporary → Refusal::gate(owner, "policy-temporary", …); Transient + clean exit → Ok but render yields Stop
    // gate: same_cause_run(after this death) >= same_cause_limit → Refusal::gate(owner, "same-cause", elevate text naming key+count)
    //       deaths_within(window) > max_deaths → Refusal::gate(owner, "intensity", …)
    // render: Restart{ after_s: min(max_s, min_s * 2^min(attempt, steps)), attempt } — attempt = deaths_within(window)
}
impl RestartGovernor {
    /// The whole decision as a Verdict — a Refusal becomes GiveUp (the witness carries both).
    pub fn verdict(&self, req: &RestartRequest, grant: &RestartGrant, ctx: &RestartContext) -> (Verdict, Option<Refusal>);
}
```

`LimitOwner` mapping: `BoundedBy::ManifestPolicy → LimitOwner::Operator`, `BoundedBy::Commitment → LimitOwner::Commitment`.

- [ ] **Step 1: Failing tests.** `three_identical_fast_deaths_give_up_by_same_cause` (three `Signaled{9}` at uptime 1000 ms → third verdict `GiveUp{SameCause{count:3}}` and the refusal's `code == RefusalCode::GateRefused("same-cause".into())`, `limit_owner == Operator`); `intensity_window_counts_only_recent_deaths_and_readiness_resets_it` (6 deaths spread over 600 s with window 300 → only those within 300 count; after `reset_on_ready` → 0); `backoff_doubles_and_caps` (attempts 0..8 with min 1, max 60, steps 6 → 1,2,4,8,16,32,60,60,60); `transient_clean_exit_stops_without_restart`; `temporary_never_restarts`; `commitment_bounded_refusal_names_commitment_owner`.
- [ ] **Step 2: Run** → compile failure. **Step 3: Implement.** **Step 4: Run** → PASS, clippy clean.
- [ ] **Step 5: Commit** — `git add elohim/ark/core && git commit -m "feat(ark-core): death tally, same-cause rule, and RestartGovernor as an elohim_compute::Governor — verdicts refuse-and-elevate"`.

---

### Task 5: `sample.rs` + `intent.rs` + `witness.rs` + `passport.rs` + `sink.rs` — the records and the sink

**Executor:** Codex. **Reviewer:** Opus.

**Files:**
- Modify: those five modules, `lib.rs`

**Interfaces:**
- Consumes: `ExitClass`, `Verdict`, `DeathRecord` (Tasks 2, 4).
- Produces:

```rust
pub struct ProcessSample { pub max_rss_bytes: Option<u64>, pub rss_bytes: Option<u64>, pub user_us: Option<u64>, pub system_us: Option<u64>, pub fds: Option<u32>, pub threads: Option<u32>, pub io_read_bytes: Option<u64>, pub io_write_bytes: Option<u64>, pub oom_score_adj: Option<i32> }

pub struct Intent { pub at_epoch_ms: u64, pub incarnation: u64, pub process: String, pub action: IntentAction, pub reason: String }
pub enum IntentAction { Spawn, Restart { attempt: u32, after_s: u64 }, Stop { signal: i32, grace_ms: u64 }, Kill, GiveUp }

pub const WITNESS_KIND: &str = "death-witness";
pub struct DeathWitness {
    pub schema: u32, pub kind: String,                       // 1, "death-witness"
    pub incident: String,                                    // Incident.id CID string
    pub process: String, pub incarnation: u64, pub pid: u32,
    pub artifact_sha256: String, pub artifact_path: String,
    pub started_at_epoch_ms: u64, pub died_at_epoch_ms: u64, pub uptime_ms: u64,
    pub exit: ExitClass,
    pub last_stderr: Vec<String>, pub last_stdout: Vec<String>,
    pub sample: Option<ProcessSample>,
    pub last_intent: Option<Intent>,                         // the envelope's own last decision about this child
    pub passport: Passport,                                  // as it stood at the moment of death
    pub verdict: Option<Verdict>,                            // filled AFTER the write-ahead witness is on disk (a second write, same incident)
}
impl DeathWitness { pub fn canonical_bytes(&self) -> Result<Vec<u8>, WitnessError>; pub fn cid(&self) -> Result<String, WitnessError>; }
pub struct Incident { pub id: String, pub process: String, pub opened_at_epoch_ms: u64, pub incarnation_at_open: u64, pub witnesses: Vec<String>, pub closed: Option<IncidentClose> }
pub enum IncidentClose { ReadyAgain { at_epoch_ms: u64 }, GaveUp { at_epoch_ms: u64, reason: GiveUpReason }, Stopped { at_epoch_ms: u64 } }
impl Incident { pub fn open(process: &str, at_epoch_ms: u64, incarnation: u64) -> Self /* id = cid over (process, opened_at, incarnation) via dag-cbor */; pub fn is_open(&self) -> bool; }

pub enum EffectiveTier { Enforced, Bounded, Delegated, Intrinsic, None }
pub struct ProcessPassport { pub name: String, pub artifact_sha256: String, pub artifact_path: String, pub pid: Option<u32>, pub started_at_epoch_ms: Option<u64>, pub ready: bool, pub effective_tier: EffectiveTier, pub deaths_in_window: u32 }
pub struct Passport { pub schema: u32, pub kind: String /* "runtime-passport" */, pub manifest: String, pub node: Option<String>, pub incarnation: u64, pub ark_version: String, pub processes: Vec<ProcessPassport>, pub last_verdict: Option<Verdict>, pub updated_at_epoch_ms: u64 }

pub trait Clock { fn now_epoch_ms(&self) -> u64; }
pub trait WitnessSink {
    fn intent(&mut self, i: &Intent) -> Result<(), SinkError>;                   // append-only, before the action
    fn witness(&mut self, w: &DeathWitness) -> Result<String /* cid */, SinkError>;
    fn incident(&mut self, i: &Incident) -> Result<(), SinkError>;
    fn passport(&mut self, p: &Passport) -> Result<(), SinkError>;
    fn tally(&mut self, process: &str, t: &DeathTally) -> Result<(), SinkError>;
    fn load_tally(&self, process: &str) -> Result<Option<DeathTally>, SinkError>;
    fn load_passport(&self) -> Result<Option<Passport>, SinkError>;
}
```

- [ ] **Step 1: Failing tests.** `witness_cid_changes_when_any_field_changes_and_is_stable_otherwise`; `witness_json_carries_kind_death_witness_and_tagged_exit` (assert the JSON has `"kind":"death-witness"` and `"exit":{"class":"signaled",…}`); `incident_id_is_content_derived`; `passport_json_kind_is_runtime_passport`. Add an in-memory `struct MemorySink` under `#[cfg(test)]` in `sink.rs` implementing the trait (Vec-backed) — Task 9's supervisor tests reuse it via `pub mod testing` behind `#[cfg(any(test, feature = "testing"))]`.
- [ ] **Step 2–4:** compile-fail → implement → PASS + clippy.
- [ ] **Step 5: Commit** — `git add elohim/ark/core && git commit -m "feat(ark-core): DeathWitness/Incident/Passport/Intent records with content-derived CIDs; WitnessSink and Clock traits"`.

---

### Task 6: `lifecycle.rs` — the pure state machine

**Executor:** Codex. **Reviewer:** Opus.

**Files:** `elohim/ark/core/src/lifecycle.rs`, `lib.rs`.

**Interfaces:**
- Produces:

```rust
pub enum ChildState { Idle, Spawning { attempt: u32 }, Booting { pid: u32, rung: usize }, Live { pid: u32 }, Dying { pid: u32, since_epoch_ms: u64 }, Dead, GaveUp }
pub enum Event { SpawnRequested, Spawned { pid: u32 }, RungPassed { rung: usize, of: usize }, RungTimedOut { rung: usize }, Died { class: ExitClass }, StopRequested, GraceExpired, VerdictReached { verdict: Verdict } }
pub enum Action { RecordIntent(IntentAction), Spawn, OpenIncident, WriteWitness, Decide, SleepThen(u64 /* seconds */), SendSignal(i32), Kill, CloseIncident(IncidentCloseKind), MarkReady, Exit }
pub enum IncidentCloseKind { ReadyAgain, GaveUp, Stopped }
pub fn step(state: ChildState, event: Event) -> (ChildState, Vec<Action>);
```

Transition table (every row is a test): Idle+SpawnRequested → Spawning, [RecordIntent(Spawn), Spawn]. Spawning+Spawned → Booting{rung 0}, []. Booting+RungPassed{rung, of} → Booting{rung+1} or (last) Live, [MarkReady, CloseIncident(ReadyAgain)]. Booting+RungTimedOut → Dying, [RecordIntent(Kill), Kill]. Booting|Live+Died → Dead, [OpenIncident (only if none open — the supervisor filters), WriteWitness, Decide]. Dead+VerdictReached{Restart{after_s,attempt}} → Spawning{attempt}, [RecordIntent(Restart), SleepThen(after_s), Spawn]. Dead+VerdictReached{GiveUp} → GaveUp, [RecordIntent(GiveUp), CloseIncident(GaveUp)]. Dead+VerdictReached{Stop} → Idle, [CloseIncident(Stopped)]. Live|Booting+StopRequested → Dying, [RecordIntent(Stop), SendSignal(policy signal — carried in the event by the supervisor as `StopRequested`; `step` emits `SendSignal(0)` placeholder? NO — make it `StopRequested { signal: i32 }`)]. Dying+Died → Idle, [CloseIncident(Stopped), Exit]. Dying+GraceExpired → Dying, [Kill]. Any illegal pair → unchanged state, `[]` (and a test proves `Idle+Died` is a no-op).

- [ ] **Step 1:** write one test per row (`#[test] fn idle_spawn_requested_records_intent_then_spawns()` …) plus `illegal_transitions_are_no_ops`.
- [ ] **Step 2–4:** compile-fail → implement `step` as one `match (state, event)` → PASS + clippy.
- [ ] **Step 5: Commit** — `git add elohim/ark/core && git commit -m "feat(ark-core): lifecycle state machine — spawn → boot ladder → live → dying → dead, verdict-driven, as a pure step function"`.

---

### Task 7: `driver.rs` + `native.rs` + `reaper.rs` — the Native driver and the reaper

**Executor:** Opus (rust-architect). **Reviewer:** Codex.

**Files:** `elohim/ark/supervisor/src/{driver,native,reaper}.rs`, `elohim/ark/supervisor/tests/native_reap.rs`.

**Interfaces:**
- Consumes: `ProcessSpec`, `Berth`, `ArtifactRef`, `StdinSource`, `PassphraseSource`, `ExitClass`, `ProcessSample`.
- Produces:

```rust
pub struct Fingerprint { pub hostname: String, pub kernel: String, pub cgroup_v2_delegated: bool /* false in S0: read-only probe of /sys/fs/cgroup/cgroup.subtree_control writability */, pub effective_tier: EffectiveTier /* None in S0 */ }
pub struct Started { pub pid: u32, pub stdout: std::process::ChildStdout, pub stderr: std::process::ChildStderr, pub artifact_sha256: String, pub artifact_path: PathBuf, pub started_at_epoch_ms: u64 }
pub trait Driver {
    fn fingerprint(&self) -> Fingerprint;
    fn start(&self, spec: &ProcessSpec, berth: &Berth) -> Result<Started, DriverError>;
    fn signal(&self, pid: u32, signal: i32) -> Result<(), DriverError>;
    fn stats(&self, pid: u32) -> Option<ProcessSample>;           // /proc/<pid>/{status,io,fd}
}
pub struct NativeDriver;
#[derive(thiserror::Error, Debug)] pub enum DriverError { UnsupportedKind(ProcessKind), ChannelUnresolvedInS0 { channel_id: String }, ArtifactMissing(PathBuf), ArtifactHashMismatch { expected: String, actual: String, path: PathBuf }, Template(String), Spawn(String), Signal(String) }

// reaper.rs
pub enum WaitEvent { StillRunning, Exited { class: ExitClass, sample: ProcessSample } }
pub fn wait_nowait(pid: u32) -> Result<WaitEvent, ReapError>;    // waitid(P_PID, WEXITED|WNOWAIT|WNOHANG): learns of death without consuming
pub fn proc_status_sample(pid: u32) -> Option<ProcessSample>;     // best-effort /proc read while the zombie still exists
pub fn reap_with_rusage(pid: u32) -> Result<(ExitClass, ProcessSample), ReapError>; // libc::wait4 → ExitClass::from_raw_wait_status(status) + ru_maxrss (KiB→bytes), ru_utime/ru_stime
pub fn become_subreaper() -> Result<(), ReapError>;               // prctl(PR_SET_CHILD_SUBREAPER, 1)
```

`NativeDriver::start`: refuse `kind != Native`; refuse `ArtifactRef::Channel` with `ChannelUnresolvedInS0`; resolve path = `berth.artifacts[spec.name]` else `ArtifactMissing`; sha256 the file (streamed, 1 MiB chunks), compare to `Pinned.sha256` (lowercase hex) else `ArtifactHashMismatch`; build `Command` with resolved argv/env templates, `env_clear()` when `env_scrub`, always pass `PATH` and `HOME` through if present in the parent (documented), `stdin` = piped (write passphrase + `\n`, then drop) or null, `stdout`/`stderr` piped, `current_dir(berth.data_root)`; `pre_exec` sets nothing (no PDEATHSIG).

- [ ] **Step 1: Failing integration test** `tests/native_reap.rs`:

```rust
// A real child: /bin/sh is the artifact (its sha256 computed in the test), argv echoes then sleeps.
#[test]
fn sigkilled_child_is_witnessed_as_signaled_9_with_rusage() {
    let dir = tempfile::tempdir().unwrap();
    let sh = std::path::PathBuf::from("/bin/sh");
    let sha = ark_supervisor::native::sha256_file(&sh).unwrap();
    let spec = ProcessSpec { name: "child".into(), kind: ProcessKind::Native,
        artifact: ArtifactRef::Pinned { cid: None, sha256: sha, bytes: None },
        argv: vec!["{artifact}".into(), "-c".into(), "echo booted; sleep 30".into()], ..Default::default() };
    let berth = Berth { manifest: "x".into(), data_root: dir.path().into(), artifacts: [("child".to_string(), sh)].into(), ..Default::default() };
    let started = NativeDriver.start(&spec, &berth).unwrap();
    assert!(matches!(wait_nowait(started.pid).unwrap(), WaitEvent::StillRunning));
    NativeDriver.signal(started.pid, 9).unwrap();
    // poll ≤ 2 s for the death to become visible without consuming it
    let mut seen = false;
    for _ in 0..40 { if let WaitEvent::Exited { .. } = wait_nowait(started.pid).unwrap() { seen = true; break; } std::thread::sleep(std::time::Duration::from_millis(50)); }
    assert!(seen, "waitid(WNOWAIT) never saw the death");
    let (class, sample) = reap_with_rusage(started.pid).unwrap();
    assert_eq!(class, ExitClass::Signaled { signal: 9, core_dumped: false });
    assert!(sample.max_rss_bytes.unwrap_or(0) > 0, "rusage carried no maxrss");
}
#[test] fn hash_mismatch_refuses_to_spawn() { /* sha256 = "00"*32 → Err(DriverError::ArtifactHashMismatch{..}) and no process exists */ }
#[test] fn channel_artifact_is_refused_in_s0_by_name() { /* Err(DriverError::ChannelUnresolvedInS0{channel_id}) */ }
```

(`ProcessSpec: Default` and `Berth: Default` are added in this task if Task 3 did not derive them.)

- [ ] **Step 2: Run** → compile failure. **Step 3: Implement** with `nix::sys::wait::waitid(Id::Pid, WEXITED|WNOWAIT|WNOHANG)`, `nix::sys::signal::kill`, `libc::wait4`, `libc::prctl`. **Step 4: Run** → PASS; clippy clean (`unsafe` blocks carry a `// SAFETY:` line each).
- [ ] **Step 5: Commit** — `git add elohim/ark/supervisor && git commit -m "feat(ark-supervisor): Native driver (std::process, env scrub, passport-grade artifact hash) and the reaper — waitid(WNOWAIT) then wait4 rusage; subreaper"`.

---

### Task 8: `pipes.rs` + `spool.rs` — line readers and the amber-local spool

**Executor:** Codex. **Reviewer:** Opus.

**Files:** `elohim/ark/supervisor/src/{pipes,spool}.rs`, unit tests inline.

**Interfaces:**
- Produces:

```rust
// pipes.rs
pub struct StreamTap { pub ring: Arc<Mutex<RingBuffer>>, pub matched: Arc<Mutex<Vec<String>>> /* readiness needles seen */ }
pub fn spawn_line_reader<R: Read + Send + 'static>(name: &'static str, r: R, ring_lines: usize, log: Option<File>, needles: Vec<String>) -> StreamTap;
// a std::thread per stream: BufRead::lines → push to ring → append "<line>\n" to log → if any needle is contained, push the needle to `matched`

// spool.rs
pub struct Spool { root: PathBuf /* <data_root>/ark */ }
impl Spool {
    pub fn open(data_root: &Path) -> Result<Self, SpoolError>;    // mkdir -p 0700 root, witnesses/, incidents/; refuses if not writable → SpoolError::Unwritable
    pub fn list_witnesses(&self) -> Result<Vec<WitnessSummary>, SpoolError>;  // newest first, from witnesses/*.json
    pub fn read_witness(&self, cid: &str) -> Result<DeathWitness, SpoolError>;
    pub fn list_incidents(&self) -> Result<Vec<Incident>, SpoolError>;
}
pub struct WitnessSummary { pub cid: String, pub incident: String, pub process: String, pub died_at_epoch_ms: u64, pub exit: ExitClass, pub verdict: Option<Verdict> }
impl WitnessSink for Spool { /* intent → append line to intents.log (O_APPEND, fsync); witness → canonical dag-cbor to witnesses/<cid>.cbor + json sidecar, tmp+fsync+rename; incident/passport/tally → json tmp+rename */ }
```

- [ ] **Step 1: Failing tests.** `line_reader_fills_ring_and_matches_needle` (feed `"a\nConductor ready.\nb\n"` through an `io::Cursor`, join thread → ring `last_n(10)` = 3 lines, `matched == ["Conductor ready."]`); `spool_writes_witness_row_before_blob_and_lists_newest_first` (two witnesses with different `died_at` → `list_witnesses()[0]` is the later one; `witnesses/<cid>.cbor` bytes == `w.canonical_bytes()`; no `*.tmp` remains); `spool_intent_log_is_append_only_json_lines`; `spool_refuses_unwritable_root` (chmod 0500 tempdir → `Err(SpoolError::Unwritable)`; skipped when running as root).
- [ ] **Step 2–4:** compile-fail → implement → PASS + clippy.
- [ ] **Step 5: Commit** — `git add elohim/ark/supervisor && git commit -m "feat(ark-supervisor): pipe line readers with readiness needles; the amber-local spool as a WitnessSink (row-before-blob, 0700)"`.

---

### Task 9: `supervisor.rs` — the loop and the signal contract

**Executor:** Opus (rust-architect). **Reviewer:** Codex.

**Files:** `elohim/ark/supervisor/src/supervisor.rs`, `elohim/ark/supervisor/tests/supervise_death.rs`.

**Interfaces:**
- Consumes: everything above.
- Produces:

```rust
pub struct Supervisor { /* manifest, berth, driver: Box<dyn Driver>, sink: Box<dyn WitnessSink>, clock: Box<dyn Clock>, shutdown: Arc<AtomicBool> */ }
pub struct RunOutcome { pub exit_code: i32 /* 0 | 3 */, pub passport: Passport }
impl Supervisor {
    pub fn new(manifest: RuntimeManifest, berth: Berth, driver: Box<dyn Driver>, sink: Box<dyn WitnessSink>, clock: Box<dyn Clock>) -> Self;
    pub fn shutdown_flag(&self) -> Arc<AtomicBool>;   // SIGTERM/SIGINT handler sets it (installed by the binary, Task 10)
    pub fn run(self) -> Result<RunOutcome, SupervisorError>;
}
```

Behaviour (one thread per process, each driving `ark_core::lifecycle::step`; the main thread joins them and polls `shutdown`): on start `become_subreaper()`, bump `berth.incarnation` (load passport → +1), write passport. Per process: `Action::Spawn` → `driver.start` → `spawn_line_reader` ×2 (log files `<data_root>/ark/logs/<name>.{stdout,stderr}.log`, needles from `Probe::StdoutLine`) → boot ladder: for each rung poll every 100 ms up to `patience_ms` (`StdoutLine` → `matched` contains; `TcpListen` → `std::net::TcpStream::connect_timeout(127.0.0.1:port, 200ms)` succeeds) while also `wait_nowait` each poll (a dead child during boot is `Event::Died`, never a rung timeout — the classifier's rule). Live: poll `wait_nowait` every 250 ms. On `Died`: `proc_status_sample` → `reap_with_rusage` → build `DeathWitness` (rings' `last_n(tail_lines)`, `last_intent` = the last intent this process recorded, passport as it stands) → `sink.witness` FIRST → `RestartGovernor.verdict` (grant `BoundedBy::ManifestPolicy`) → write the witness a second time with `verdict` filled (same incident; the first CID is the write-ahead record) → `sink.incident` → `Event::VerdictReached`. `SleepThen(s)` honours `shutdown` (wake early). `StopRequested{signal}` on shutdown: `driver.signal(pid, policy.shutdown.signal)`, wait `grace_ms` polling `wait_nowait`, then `Kill`. All processes `GaveUp` → `exit_code 3`; clean shutdown → 0. Passport rewritten on every state change.

- [ ] **Step 1: Failing integration test** `tests/supervise_death.rs` using `/bin/sh` as in Task 7, `Spool` on a tempdir, a real clock:

```rust
#[test]
fn a_sigkilled_child_leaves_a_witness_then_restarts_then_gives_up_on_same_cause() {
    // manifest: one process "child", argv: sh -c 'echo booted; exec sleep 300', readiness: StdoutLine{"booted", 5000},
    // policy: same_cause_limit 3, intensity {max_deaths 10, window 300}, backoff {min 0, max 0, steps 1} (no sleep in tests)
    // run the Supervisor on a thread; wait until passport.json says processes[0].ready == true and read its pid;
    // kill -9 that pid; within 2 s Spool::list_witnesses() has 1 entry: exit Signaled{9}, incident open, verdict Some(Restart{..});
    // read intents.log: sequence contains Spawn, then Restart{attempt:1};
    // wait for ready again (new pid) → incident closed ReadyAgain;
    // kill -9 the new pid twice more (each after ready) → after the third death the verdict is GiveUp{SameCause{count:3}}? NO: readiness resets the window,
    //   so use a child that DIES ON ITS OWN fast: second manifest process "flapper": sh -c 'exit 7' with readiness [] → three deaths uptime<5s, same class →
    //   verdict GiveUp{SameCause{key contains "exited:7", count 3}}; supervisor exits 3 once "child" is also stopped via the shutdown flag.
    // Assert: witnesses for flapper == 3, last has verdict GiveUp; passport.last_verdict == GiveUp; no zombie: `wait_nowait(pid)` → Err (no such child).
}
#[test]
fn shutdown_sends_policy_signal_then_kills_after_grace() { /* child traps INT and sleeps: sh -c 'trap "" INT; echo booted; sleep 300'; grace_ms 300 → after shutdown the child is gone within 1 s and intents.log ends with Stop then Kill */ }
```

- [ ] **Step 2–4:** compile-fail → implement → PASS + clippy. Both tests must run under `cargo test -p elohim-ark-supervisor --all-targets` in < 15 s.
- [ ] **Step 5: Commit** — `git add elohim/ark/supervisor && git commit -m "feat(ark-supervisor): the supervision loop — boot ladder, witness-before-verdict, RestartGovernor, incidents, the SIGINT-then-SIGKILL stop contract"`.

---

### Task 10: The `ark` binary

**Executor:** Codex. **Reviewer:** Opus.

**Files:** `elohim/ark/cli/src/main.rs`, `elohim/ark/cli/tests/cli_smoke.rs`.

**Interfaces:**
- `ark run --manifest <path.json> --berth <path.json>`: loads both (65 on error), refuses `berth.manifest != manifest.cid()` (65, message names both CIDs), opens the `Spool` (67), installs SIGTERM+SIGINT handlers (`nix::sys::signal::sigaction` on a `static AtomicBool` shared with `Supervisor::shutdown_flag`), runs, exits with `RunOutcome.exit_code`. Prints one line per state change to stderr as JSON (`{"ark":"state","process":…,"state":…}`) so the mesh log stays greppable.
- `ark describe --berth <path>` → `passport.json` pretty-printed to stdout (exit 0; 65 if absent).
- `ark witness ls --berth <path>` → JSON array of `WitnessSummary`; `ark witness show --berth <path> <cid>` → the witness JSON.
- `ark hash <file>` → lowercase sha256 hex on stdout (the mesh script pins with it).
- `ark manifest cid --manifest <path>` → the CID string (the mesh script writes it into the berth).

- [ ] **Step 1: Failing test** `cli_smoke.rs` using `env!("CARGO_BIN_EXE_ark")`: `hash_matches_sha256sum` (write a tempfile, compare against `sha2` computed in-test); `run_refuses_mismatched_berth_manifest_with_65`; `witness_ls_on_empty_spool_prints_empty_array`; `run_then_kill_child_then_witness_ls_shows_one` (spawn `ark run` as a child process with the Task 9 manifest, read passport for the pid, `kill -9`, poll `ark witness ls` ≤ 5 s → one entry, then SIGTERM the ark process → exit 0).
- [ ] **Step 2–4:** compile-fail → implement → PASS + clippy.
- [ ] **Step 5: Commit** — `git add elohim/ark/cli && git commit -m "feat(ark): the binary — run/describe/witness ls|show/hash/manifest cid; signal handlers hand the supervisor its stop contract"`.

---

### Task 11: `hc-mesh.sh` — `MESH_CONDUCTOR_LAUNCH=ark`

**Executor:** Opus (rust-architect, with the mesh traps memory in the prompt). **Reviewer:** Codex (read-only diff review) — then the **orchestrator runs the mesh in the foreground** (never a background task).

**Files:** Modify `app/elohim-app/scripts/hc-mesh.sh`: header env doc (after the `MESH_CONDUCTOR_LAUNCH` block, line ~80), a new `ARK_BIN` doc line, `write_ark_declarations()` near `record_mesh_pid` (line ~258), the start launch site (line ~1610), the join-peer launch site (line ~1884), the status roster (line ~1823), and `mesh_conductor_pid()`.

**Interfaces:**
- `ARK_BIN` — the `ark` binary (default: `/projects/.cargo-target-pool/family/dev/elohim/dev/debug/ark`, then `command -v ark`; refuse to start if neither is executable, naming the build line `cd elohim && CARGO_TARGET_DIR=… RUSTFLAGS="" cargo build -p elohim-ark`).
- `write_ark_declarations <name> <config_path>` writes `$LOCAL_DEV_DIR/$name/ark/manifest.json` and `berth.json`:

```json
{ "schema": 1, "kind": "runtime-manifest", "reach": "trusted",
  "processes": [ { "name": "conductor", "kind": "native",
      "artifact": { "pinned": { "sha256": "<$ARK_BIN hash $hc_bin>" } },
      "argv": ["{artifact}", "--piped", "--structured=Log", "--config-path", "{data_root}/conductor-config.yaml"],
      "stdin": "passphrase",
      "readiness": [ { "stdout_line": { "contains": "Conductor ready.", "patience_ms": 120000 } },
                     { "tcp_listen": { "port_key": "admin_ws", "patience_ms": 30000 } } ],
      "policy": { "shutdown": { "signal": 2, "grace_ms": 20000 } } } ] }
```

and `berth.json` = `{ "manifest": "<$ARK_BIN manifest cid --manifest …>", "data_root": "$LOCAL_DEV_DIR/$name", "passphrase": {"literal":"test"}, "ports": {"admin_ws": <admin port for $name>}, "artifacts": {"conductor": "$hc_bin"}, "incarnation": 0 }` (incarnation is read back from an existing `passport.json` if present so restarts keep it monotone — `ark run` does the bump).

- Launch (both sites): `setsid nohup "$ARK_BIN" run --manifest "$LOCAL_DEV_DIR/$name/ark/manifest.json" --berth "$LOCAL_DEV_DIR/$name/ark/berth.json" >> "$LOCAL_DEV_DIR/.sandbox_run_log.$name" 2>&1 &` then `record_mesh_pid ark "$name" "$!"`. `assert_toolchain_parity` is skipped exactly as `direct` skips it (the CLI never rewrites the config).
- `mesh_conductor_pid <name>` → `jq -r '.processes[] | select(.name=="conductor") | .pid' "$LOCAL_DEV_DIR/$name/ark/passport.json"`; `status` shows `conductor(ark) $name pid=<child> incarnation=<n> ready=<bool>`; `stop_all` needs no change (it terminates recorded pids; ark's SIGTERM handler stops its child with SIGINT + grace).
- The per-conductor log line the spin detector greps (`holochain --piped` in `ps`) is unchanged because the child's argv is the same.

- [ ] **Step 1:** implement the script changes; `bash -n app/elohim-app/scripts/hc-mesh.sh; echo EXIT=$?` → 0; `shellcheck -S warning app/elohim-app/scripts/hc-mesh.sh` no NEW findings versus `git stash`-free baseline (compare counts).
- [ ] **Step 2 (orchestrator, foreground):** build `ark` (`cd elohim && CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim/dev RUSTFLAGS="" cargo build -p elohim-ark; echo EXIT=$?`), then `just mesh stop; MESH_CONDUCTOR_LAUNCH=ark just mesh start` and confirm: `just mesh status` lists three `conductor(ark)` rows; `ss -tln` shows the three admin ports; `just mesh prologue` still passes.
- [ ] **Step 3 (orchestrator):** `kill -9 "$(mesh_conductor_pid jessica)"` (source the script for the helper), then within 10 s `"$ARK_BIN" witness ls --berth elohim/holochain/local-dev/jessica/ark/berth.json` shows one witness with `"signaled",9`; `ark describe` shows `incarnation` unchanged (the ark did not die) and the conductor `ready: true` again after the restart.
- [ ] **Step 4: Commit** — `git add app/elohim-app/scripts/hc-mesh.sh && git commit -m "feat(mesh): MESH_CONDUCTOR_LAUNCH=ark — each household conductor runs as the child of an ark; manifest+berth written per peer; mesh_conductor_pid reads the passport"`.

---

### Task 12: Station 1 on the household lane

**Executor:** Codex. **Reviewer:** Opus. **Run:** orchestrator, foreground, on the running ark-launched mesh.

**Files:**
- Create: `genesis/a2o/steps/mesh/death-witness.steps.ts`
- Modify: `genesis/a2o/features/resilience/death-witness.feature` (remove `@wip` from station 1 ONLY)
- Reference (read only): `genesis/a2o/steps/conductor-spin.steps.ts` (how the mesh is reached: `spawnSync` of `hc-mesh.sh` helpers, `loadHouseholdMeshFixture`), `genesis/a2o/src/framework/fixtures/household-mesh.ts`.

**Interfaces (step texts, verbatim from the feature):**
- `Given the household mesh is three storage peers: Jessica, Matthew, and James` — reuse if an equivalent step exists in the household fixture; else implement via `loadHouseholdMeshFixture()` and assert three peers.
- `And each peer's conductor is running as a child of that peer's envelope` — for each peer: `passport.json` exists under `elohim/holochain/local-dev/<peer>/ark/`, `processes[0].ready === true`, and `/proc/<pid>/status` `PPid:` equals the pid recorded in `/tmp/elohim-local-mesh/pids/ark-<peer>`. Fails with the sentence "mesh is not ark-launched: start it with MESH_CONDUCTOR_LAUNCH=ark" when absent (the precondition the feature's comment names).
- `When Jessica's conductor is killed with SIGKILL` — `process.kill(pid, 'SIGKILL')` on the passport pid; remember the pid and the witness count before.
- `Then within 10 seconds Jessica's peer lists a death witness for a new incident` — poll `ark witness ls --berth …` (ARK_BIN from env, same default as the script) every 500 ms; assert count increased by one and the new entry's `incident` is not among the earlier entries' incidents.
- `And the witness names the signal, how long the conductor ran, and its last stderr lines` — `ark witness show`: `exit.class === 'signaled' && exit.signal === 9`, `uptime_ms > 0`, `last_stderr.length > 0` (the conductor logs to stderr under `--structured=Log`; if it is empty on this build, assert `last_stdout.length > 0` and note the stream in the failure message).
- `And the witness carries the envelope's own last decision about that conductor` — `last_intent.action` is `spawn` or `restart`.
- `And the witness names the hash of the conductor program the envelope actually started` — `artifact_sha256` equals `ark hash <artifact_path>`.
- `And the witness carries Jessica's passport as it stood at the moment of death` — `passport.processes[0].pid === killedPid`.

- [ ] **Step 1:** write the steps; `cd genesis/a2o && pnpm exec tsc --noEmit -p .; echo EXIT=$?` → 0.
- [ ] **Step 2 (orchestrator):** `cd genesis/a2o && npx cucumber-js --config /dev/null --tags '@concern:death-witness and @station-1' features/resilience/death-witness.feature` (the profile-merge trap: never `-p local` with a positional) with the act1-household lane env (`owned-substrate: true` — `cluster-state.act1-household.yaml`) → 1 scenario passed. Stations 2–4 stay `@wip` and are not run.
- [ ] **Step 3: Commit** — `git add genesis/a2o/steps/mesh/death-witness.steps.ts genesis/a2o/features/resilience/death-witness.feature && git commit -m "test(a2o): death-witness station 1 — the envelope that held the pipes witnesses the death (household lane, ark-launched mesh)"`.

---

### Task 13: The habit delta, the seam registry, and the projection (orchestrator)

**Files:** `elohim/ark/core/seam-registry.yaml` (create), `elohim/elohim-storage/.epr-meta/runtime-death-witnessed.habit.md` (DELTA), `genesis/manifests/habits.yaml` (re-project), this plan (checkboxes).

- [ ] **Step 1:** `seam-registry.yaml` conforming to `elohim/sdk/schemas/v1/manifest/seam-registry.schema.json` (copy the header shape from `crates/seam-contracts/seam-registry.yaml`), registering three decision points with their test names: `classify_readiness_outcome` (exit.rs), `same_cause_key` (tally.rs), `RestartGovernor::verdict` (verdict.rs). Validate with the schema tool the seam-contracts registry uses (`grep -rn seam-registry .claude/scripts | head` names it).
- [ ] **Step 2:** habit DELTA line: `DELTA <date>: station 1 GREEN on the household mesh (ark-launched, MESH_CONDUCTOR_LAUNCH=ark) — <run id / scenario output line>; stations 2–4 remain @wip; status stays red until station 4.` Then `python3 .claude/scripts/habits-project.py` and `--check`.
- [ ] **Step 3: Commit** — `git add elohim/ark/core/seam-registry.yaml elohim/elohim-storage/.epr-meta/runtime-death-witnessed.habit.md genesis/manifests/habits.yaml genesis/docs/superpowers/plans/2026-09-02-ark-s0-launcher-plan.md && git commit -m "chore(habits): runtime-death-witnessed — station 1 green on the ark-launched household mesh; ark-core seam registry born"`.

---

## Self-review against the spec

- **§8 crate table:** `ark-core` owns manifest/berth/exit/ring/tally/verdict/witness/passport/sample/lifecycle (Tasks 2–6) ✓; `ProcessSample` is new, `ResourceSnapshot` untouched ✓; `ark-supervisor` owns driver/reaper/pipes/spool/loop (Tasks 7–9) ✓; the `Driver` trait carries `fingerprint/start/signal/stats` — `recover`, `wait`, `stdio` from the Nomad shape are folded into the reaper and pipes for S0 and named in `CLAUDE.md` as the S2 growth points ✓; the binary's `notify` (sd_notify) is S2 and absent here on purpose ✓.
- **§11 S0 receipt:** station 1 (Task 12) ✓; the ring + classifier lift with both tests (Task 2) ✓; `direct` → `ark run` as an argv port (Task 11) ✓; `hc sandbox generate` still owns config + install ✓; witness to spool + intent log + ExitClass + tally + same-cause GiveUp (Tasks 4, 5, 8, 9) ✓; artifact ref in the closure/channel shape with the pinned-local resolver only (Task 3 `ArtifactRef`, Task 7 refusal of `Channel`) ✓.
- **§12 items:** 1 (own crate family) ✓, 5 (liveness = own reaper) ✓, 7 (passport hashes what it runs) ✓ Task 7, 15 ✓, 18 (ark's own binary pinned-only — `ARK_BIN` is a path, no channel) ✓, 19 (no tokio::process; no PDEATHSIG) ✓, 20 (names) ✓, 24 (update-loop shape) ✓.
- **Gaps deliberately left for S1** and stated in `CLAUDE.md`: no custody, no attestation, no `node` identity in the berth, no self-contract commitment (`BoundedBy::ManifestPolicy` only), CIDs as strings, effective tier `None`, storage's `process_manager` untouched (duplicate ring for one slice).
- **Type consistency check:** `ExitClass::from_raw_wait_status` (Task 2) is what `reap_with_rusage` (Task 7) calls; `Probe::StdoutLine{contains}` (Task 3) is what `spawn_line_reader` needles (Task 8) and the ladder (Task 9) read; `WitnessSink::witness → cid` (Task 5) is what `Spool` (Task 8) and the supervisor (Task 9) use; `Berth.artifacts` (Task 3) is what `NativeDriver::start` (Task 7) resolves; `Passport.processes[].pid` (Task 5) is what `mesh_conductor_pid` (Task 11) and the steps (Task 12) read.
