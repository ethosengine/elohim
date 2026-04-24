# Wave 1 — Sweettest Bodies (Per-DNA Baselines)

**Date:** 2026-04-24
**Branch:** `wave1-sweettest-bodies` (off `dev`)
**Kickoff:** `genesis/docs/plans/2026-04-24-rno-wave1-sweettest-bodies-kickoff-prompt.md`
**Wave 1 plan:** `genesis/docs/plans/2026-04-21-rno-lessons-wave-1-execution-plan.md` §2.1.3
**Done signal:** Jenkins-green `DNA Integration (bootstrap-steward)` across all 5 DNAs **with `#[ignore]` removed**, handoff doc §0 #3 flipped 🟡 → ✅ with build number.

---

## §0. Pre-flight resolutions of the six known unknowns

### Q1 — mishpat coordinator surface

Surface is rich (governance proposals, challenges, precedents, votes, opinion statements, gate decisions). Baseline picks `create_proposal` + `get_proposal_by_id` + `query_proposals` — a representative create/read pair.

- Coordinator extern: `create_proposal(input: CreateProposalInput) -> ExternResult<ProposalOutput>`
- Read by id: `get_proposal_by_id(id: String) -> ExternResult<Option<ProposalOutput>>`
- Read by query: `query_proposals(input: QueryProposalsInput) -> ExternResult<Vec<ProposalOutput>>`
- Validator (`mishpat_integrity::validate_proposal`): shape-only, no author-restricted gating beyond what coordinator enforces.

### Q2 — lamad content_store CID handling

Coordinator zome lives at `dna/elohim/zomes/content_store/` (the `lamad-v1` directory is the prior-DNA-version healing-export shim). Surface picked: `create_content` + `get_content_by_id` + `get_content` (by `ActionHash`).

- `create_content(input: CreateContentInput) -> ExternResult<ContentOutput>`
- `get_content_by_id(input: QueryByIdInput) -> ExternResult<Option<ContentOutput>>`
- `get_content(action_hash: ActionHash) -> ExternResult<Option<ContentOutput>>`

Baseline = "publish content, retrieve by id, retrieve by action hash, cross-agent visibility." Erasure / chunked-blob round-trip is **out of scope**.

### Q3 — node_registry register_node inputs

`register_node(registration: NodeRegistration) -> ExternResult<ActionHash>` takes the full `NodeRegistration` integrity entry struct (~26 fields including identity, capacity, location, capabilities, participation, health, metadata, claim_status, signature). Test fixture must construct one — recommended as a `common::fixtures::node_registration()` factory with sane defaults parameterized on `node_id`/`agent_pub_key`.

Read paths: `get_nodes_by_region(region) -> Vec<NodeRegistration>`, `get_nodes_by_tier(tier) -> Vec<NodeRegistration>`, `get_available_custodians(filters) -> Vec<NodeRegistration>`.

### Q4 — infrastructure self-registration baseline

`register_doorway(input: RegisterDoorwayInput) -> ExternResult<DoorwayOutput>` — coordinator always sets `operator_agent` to `agent_info().agent_initial_pubkey`, so self-registration is the only path through the coordinator surface.

Validator rule (`infrastructure_integrity::validate_doorway_registration` lines 360-368): `doorway.operator_agent != action.author` returns `ValidateCallbackResult::Invalid("Doorway operator_agent must match the author (self-registration only)")`. The same rule applies to `validate_doorway_update` (lines 398-406) and `validate_health_attestation` (line 489).

Practically testable scenarios via the coordinator:
- (a) Agent A self-registers → succeeds.
- (b) Cross-agent visibility: agent B retrieves A's doorway via `get_doorway_by_id` after `mirrors::settle_dht`.
- (c) Operator-only enforcement: agent B calls `update_doorway` for A's id → coordinator returns "Only the doorway operator can update this registration" (line 354). This proves the operator binding without bypassing the coordinator.

### Q5 — Cross-agent assignment per DNA

Each DNA gets exactly one cross-agent scenario using `two_agent_conductors()` + `mirrors::settle_dht`:

| DNA | Cross-agent scenario |
|---|---|
| imagodei | Existing `second_agent_is_not_bootstrap_steward` (already cross-agent). New scenario 3 may stay single-agent if it BLOCKS. |
| mishpat | Steward creates proposal; second agent reads via `get_proposal_by_id` after settle. |
| lamad | Steward creates content; second agent reads via `get_content_by_id` after settle. |
| node_registry | Steward registers node; second agent queries via `get_nodes_by_region` after settle. |
| infrastructure | Agent A self-registers doorway; agent B reads via `get_doorway_by_id` after settle, then attempts `update_doorway` (rejected). |

### Q6 — Ignore-flip strategy

**Staged flip, per DNA, each behind its own Jenkins-green proof.** One commit per DNA-flip:

- `test(sweettest-imagodei): unignore — jenkins green on <build>`
- `test(sweettest-mishpat): unignore — jenkins green on <build>`
- ...

Cleaner than a single flip-all commit because if any one DNA regresses on Jenkins, only its commit reverts. Memory: `feedback_shift_measure_jenkins.md` — measures live in Jenkins.

---

## §1. Pre-dispatch setup edits

### 1.1 — fixtures.rs path resolution for node-registry

**Problem discovered:** Jenkinsfile pack step (line 386) produces `dna/node-registry/node-registry.dna` — flat layout, hyphenated, **NOT in `dna/elohim/workdir/`** like the four happ-bundled DNAs. Current `fixtures::dna_path("node_registry")` only searches:

1. `dna/<name>/workdir/<name>.dna` → `dna/node_registry/workdir/node_registry.dna` (no — dir is hyphenated)
2. `dna/elohim/workdir/<name>.dna` → `dna/elohim/workdir/node_registry.dna` (no — pack puts it elsewhere)

**Fix:** Add a third path candidate to `fixtures::dna_path` — `dna/<name-with-hyphens>/<name-with-hyphens>.dna`. A single-line transformation `name.replace('_', '-')` covers node-registry without breaking anything else.

This is a `common/` edit, in scope per kickoff "helpers naturally belong in `common/`."

### 1.2 — Helper for NodeRegistration construction

Add `common::fixtures::node_registration(node_id: &str, agent: &AgentPubKey) -> NodeRegistration` returning a sane-default registration. Used only by the node_registry test for now, but a natural fixture given the 26-field struct.

This may evolve later when other DNAs need similar factories; resist over-abstracting on first use (3rd duplication > premature helper).

---

## §2. Per-DNA scopes (subagent dispatch references)

Each subagent receives §0 + §1 + their own §2.x section. Shared rules:

- **Out of scope:** all DNA-source files (zomes/**/*.rs, dna.yaml, Cargo.toml in zome dirs). If a needed extern is missing, **report BLOCKED**.
- **In scope:** `tests/sweettest/src/tests/<dna>.rs` and additions to `tests/sweettest/src/common/`.
- **No git revert/reset** on pre-existing commits (memory `feedback_subagent_scope_guardrails.md`).
- **Forbid `#[ignore]` removal** — that's a later staged step.
- **Schema-first:** if you add a wire shape or fixture struct, write the type first (memory `feedback_schema_first_ioc.md`).
- Use `cargo check -p elohim_sweettest` from `tests/sweettest/` to validate compile.
- Vocabulary: stewardship / bootstrap-steward (memory `project_no_sovereignty_stewardship_over_ownership.md`).

### 2.1 — imagodei

Current state: `bootstrap_steward_is_identifiable` and `second_agent_is_not_bootstrap_steward` already bodied.

**Add scenario 3 attempt:** "integrity validation rejects bootstrap-only actions from non-steward agents." Investigation note from §0: `bootstrap_steward.rs` lines 28-37 explicitly state authority is graduated — `is_bootstrap_steward` is identity, **not** a capability gate. No existing integrity validator in `imagodei_integrity` rejects an action *because the author is not the bootstrap steward*.

**Most likely outcome:** subagent BLOCKS scenario 3 with explanation that the bootstrap-steward authority frame design (`genesis/docs/superpowers/specs/2026-04-21-bootstrap-steward-authority-frame-design.md`) deliberately avoids gating any DHT action on bootstrap-only identity. The proper rejection scenario would require a `StewardshipGrant`-gated extern, which is a later sprint.

**If a real bootstrap-only validator is found in `imagodei_integrity`:** body a third test exercising it cross-agent. Otherwise, leave the existing scenarios unchanged and emit a BLOCKED report citing the design doc.

### 2.2 — mishpat

Current state: `bootstrap_steward_is_configured` (single agent) only.

**Add cross-agent proposal round-trip:**

```rust
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNA artifact"]
async fn proposal_round_trips_across_agents() -> Result<()> {
    // Two conductors, steward = a1
    // a1: create_proposal(...) — capture proposal id
    // settle_dht(&[&cell1, &cell2])
    // a2: get_proposal_by_id(id) — assert Some(_) returned, fields match
    // a2: query_proposals(...) — assert proposal in result set
}
```

`CreateProposalInput` shape comes from `mishpat::CreateProposalInput`. Pick minimal viable input; if the input requires referential fields (e.g., a precedent_id, challenge_id) that you'd have to fabricate, BLOCK and explain.

### 2.3 — lamad

Current state: bare install only (`content_store_is_reachable`).

**Add publish + retrieve + cross-agent:**

```rust
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNA artifact"]
async fn content_publishes_and_retrieves_by_id() -> Result<()> {
    // single agent
    // create_content(CreateContentInput { id: "test-1", content_type: "concept", ... })
    // get_content_by_id(QueryByIdInput { id: "test-1" }) — assert Some, fields match
    // get_content(action_hash) — assert Some, fields match
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNA artifact"]
async fn content_visible_across_agents() -> Result<()> {
    // two agents
    // a1: create_content(...) — capture id
    // settle_dht
    // a2: get_content_by_id(id) — assert Some, fields match
}
```

`CreateContentInput` shape lives in `dna/elohim/zomes/content_store/src/lib.rs` (search for `CreateContentInput`). Pick the minimum viable field set. If validator requires fields you'd have to invent (e.g., a content hash that ties to chunked blob), BLOCK.

DNA artifact lands at `dna/elohim/workdir/lamad.dna` — fixture path resolution already finds this via the `dna/elohim/workdir/<name>.dna` fallback.

### 2.4 — node_registry

Current state: `node_registry_has_bootstrap_steward` (single agent).

**Pre-fix from §1.1 must be in place** so fixture finds `dna/node-registry/node-registry.dna`. Verify by running `cargo check -p elohim_sweettest` first; if path resolution still fails at runtime, report BLOCKED with the exact path-search output.

**Add helper + tests:**

```rust
// in common/fixtures.rs
pub fn node_registration(node_id: &str, agent: &AgentPubKey) -> NodeRegistration { ... }

// in tests/node_registry.rs
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNA artifact"]
async fn register_node_round_trips() -> Result<()> {
    // single agent
    // register_node(node_registration("alpha", &agent)) — capture ActionHash
    // get_nodes_by_region("us-west") — assert vec includes "alpha"
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNA artifact"]
async fn admission_visible_across_agents() -> Result<()> {
    // two agents
    // a1: register_node(...)
    // settle_dht
    // a2: get_nodes_by_region(...) — assert includes a1's registration
}
```

`NodeRegistration` integrity struct lives at `dna/node-registry/zomes/node_registry_integrity/src/lib.rs` lines 56-97. Re-export through `node_registry_coordinator::NodeRegistration` (already exported per coordinator lib.rs lines 7-10).

### 2.5 — infrastructure

Current state: `infrastructure_installs_without_bootstrap_steward` (single agent).

**Add self-registration + visibility + operator-only update:**

```rust
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNA artifact"]
async fn doorway_self_registers() -> Result<()> {
    // single agent
    // register_doorway(RegisterDoorwayInput { id: "alpha", url: "...", ... })
    // get_doorway_by_id("alpha") — assert Some, operator_agent == agent.to_string()
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNA artifact"]
async fn doorway_visible_across_agents_and_operator_only_can_update() -> Result<()> {
    // two agents
    // a1: register_doorway(... id="alpha")
    // settle_dht
    // a2: get_doorway_by_id("alpha") — assert Some, fields match
    // a2: update_doorway(... id="alpha", url="hijacked") — assert Err with "Only the doorway operator..."
}
```

`RegisterDoorwayInput` shape from `infrastructure_types` re-exports (coordinator lib.rs lines 22-26). Required fields: `id`, `url`, `capabilities_json`, `reach`, `version` plus optional `region`, `bandwidth_mbps`. Use `register_doorway` not `update_doorway` for the second agent's failure path — the validator rejects on `operator_agent != author` regardless, but the coordinator catches it first with a friendlier message.

---

## §3. Subagent dispatch template

Each parallel dispatch uses `rust-architect` subagent type. Brief structure:

> **Task:** Fill in the missing test bodies in `elohim/holochain/tests/sweettest/src/tests/<dna>.rs` per §2.<n> of `genesis/docs/superpowers/plans/2026-04-24-rno-wave1-sweettest-bodies.md`.
>
> **Branch:** `wave1-sweettest-bodies` (already cut off `dev`).
>
> **Read first (REQUIRED context):**
> - The kickoff doc at `genesis/docs/plans/2026-04-24-rno-wave1-sweettest-bodies-kickoff-prompt.md`
> - This plan doc, especially §0 (resolutions), §1 (pre-flight), §2.<n> (your scope), §3 (this brief), §4 (constraints)
> - The current test file `tests/sweettest/src/tests/<dna>.rs`
> - The DNA's coordinator zome at the path indicated in §2.<n>
>
> **In scope:** `tests/sweettest/src/tests/<dna>.rs` and additions to `tests/sweettest/src/common/`.
> **Out of scope:** ALL files outside `tests/sweettest/`. No DNA-source edits. No README touching (later step). No `#[ignore]` removal (later step).
>
> **No `git revert` or `git reset` on any commit you don't own.** Other autonomous work may be in flight. If you encounter unexpected state, BLOCK and report — do not "clean up."
>
> **BLOCK rather than expand scope** if a coordinator extern you need is missing, an integrity validator you'd test doesn't exist, or fixture construction needs a struct field you'd have to invent. Report BLOCKED with the specific gap.
>
> **Verification before reporting done:** `cd elohim/holochain/tests/sweettest && cargo check -p elohim_sweettest` must pass. Don't claim "tests pass" — Jenkins is the bar (memory `feedback_shift_measure_jenkins.md`).
>
> **Commit hygiene:** one commit per logical unit. Pattern: `test(sweettest-<dna>): <scenario>`. Push to `wave1-sweettest-bodies`. Do NOT bypass husky with `HUSKY=0`. Do not push if `cargo check` fails.

---

## §4. Constraints recap

- DNA-source-edit = sprint-scope violation. BLOCK instead.
- Schema-first: write types first (memory `feedback_schema_first_ioc.md`).
- Bootstrap-steward vocabulary, NOT progenitor in surface language (memory `project_no_sovereignty_stewardship_over_ownership.md`).
- Integrity validators cannot use `get_links` (memory `project_hdi_no_get_links_in_validators.md`) — affects what can be tested at validator-only level vs requiring coordinator gates.
- Stage-1 security gradient (memory `project_bootstrap_to_elohim_security_gradient.md`) — don't expect Stage-3 elohim-enforcement rigor in current validators.

## §5. Definition of done (mirrors kickoff)

- [ ] All five `src/tests/<dna>.rs` files have bodies for the TODOs OR a BLOCKED report explaining why.
- [ ] `cargo check -p elohim_sweettest` passes locally.
- [ ] Jenkins `DNA Integration (bootstrap-steward)` runs green with `--include-ignored` across all five DNAs.
- [ ] `#[ignore]` removed via per-DNA staged commits, each behind its own Jenkins-green proof.
- [ ] `tests/sweettest/README.md` §Pipeline integration updated.
- [ ] `git diff --name-only dev...HEAD` shows no DNA-source files modified.
- [ ] `genesis/docs/plans/2026-04-21-rno-lessons-roadmap-handoff.md` §0 #3 flipped 🟡 → ✅ with build number.
