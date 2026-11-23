---
epr-meta-version: 1
id: genesis-agentic-loop-tooling
covers: subtree
purpose: >
  Utility scripts and reference data behind the agentic developer loop: the command-palette
  matcher and allowlist generalizer, the pre-shift readiness check, the safety taxonomy and
  anti-pattern catalogues, the cargo target pool and its resource guards (bin/: cargo-pool,
  ram-guard, io-guard, berth, pool pre- and post-flight), pool-policy.json, and the delegated
  compute worker (compute/).
rules:
  - id: agentic-record-py
    policy: record-and-judge-are-native@1
    when:
      write: "*.py"
      contains-any: ["appendFileSync(", "write_json_atomic", "state.json", "deadline =",
                     "monotonic() >=", ">= bound", "> bound", "age <= ", "age > "]
  - id: agentic-record-mjs
    policy: record-and-judge-are-native@1
    when:
      write: "*.mjs"
      contains-any: ["appendFileSync(", "write_json_atomic", "state.json", "deadline =",
                     "monotonic() >=", ">= bound", "> bound", "age <= ", "age > "]
  - id: agentic-record-sh
    policy: record-and-judge-are-native@1
    when:
      write: "*.sh"
      contains-any: ["appendFileSync(", "write_json_atomic", "state.json", "deadline =",
                     "monotonic() >=", ">= bound", "> bound", "age <= ", "age > "]
  - id: agentic-held-py
    policy: held-state-is-the-incident@1
    when:
      write: "*.py"
      contains-any: ["tool_use", "approval-evidence", "pending-approval", "held-state", "held_state",
                     "\"held\"", "'held'", ".held"]
  - id: agentic-held-mjs
    policy: held-state-is-the-incident@1
    when:
      write: "*.mjs"
      contains-any: ["tool_use", "approval-evidence", "pending-approval", "held-state", "held_state",
                     "\"held\"", "'held'", ".held"]
  - id: agentic-held-sh
    policy: held-state-is-the-incident@1
    when:
      write: "*.sh"
      contains-any: ["tool_use", "approval-evidence", "pending-approval", "held-state", "held_state",
                     "\"held\"", "'held'", ".held"]
---
# genesis/agentic — agentic-loop tooling

What belongs here: the scripts the `agentic-developer` skill and the cargo pool call, the data
they read, and a `*.test.mjs` (or `*_test.py`) beside each script, run with
`node --test genesis/agentic/*.test.mjs`. There is no `package.json`; the scripts use the root
workspace's dev dependencies. `pool-policy.json` is read by the pre-push hook and the disk-guard
PreToolUse hook, so a change to its watermarks or cargo overrides changes what every gate does.

`covers: subtree` is a considered ownership claim with no edit-time rule. Shift working state
(objectives, journals, sprint results) belongs in `.claude/shifts/`, not here.

## One home for a record, many implementors

Authoritative, not gate-enforced in v1, except that two advisory `inject` bindings above nudge a script that stores a record or judges a bound by hand (`record-and-judge-are-native@1`, `held-state-is-the-incident@1`). They never block.

**The principle.** The record converges and the implementor varies. A record that another device or agent must interpret has one native home. "Retire" applies only to a second copy of a record, or of the rule that judges it. A local sensor, guard, device policy, harness adapter or push transport is an edge implementor and stays; it senses, adapts and calls. The existing rule `durable-primitives-are-native-py` in `.claude/scripts` and `.claude/hooks` says this for `*.py`; lifting it into the policy is a follow-up.

**The four homes** (items (b) and (c) are the steward's lean of 2026-11-23, not yet a ruling, and no rule enforces them):
- (a) Delegated authority is a delegation recorded in the flow that crosses devices, with provider, recipient, scope, bound, window and withdrawal. The nearest existing form is the compute delegation in mishpat (`Commitment`, action `delegates-compute`, in `elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs`) and the epr-rea `Commitment`. A covenant clause is not a delegation, and a mooring is not one either. A stewardship grant's capability flags do not authorize affirmative acts (`genesis/docs/superpowers/specs/2026-10-04-commons-pool-as-collective-party-design.md`).
- (b) A bounded ask names its asker, its bound and a fallback that stays inside authority already held. A timeout never grants authority.
- (c) Held state is the Incident in `elohim/eprfs/device-runtime/src/session_stasis.rs`, folded by `epr session stasis fold`. Liveness never establishes standing, and a held process is not a death witness.
- (d) A window or limit is judged by `elohim_epr_rea::Bound`. An expiring grant is a bound that should be sensed approaching; the runner grant expired during a landing on 2026-11-23 (arrival backlog row 36).

**The contract-test rule.** A contract is proven by a test at least two implementors pass, one of them a fake (peers plan section 9, `carrierContract` in `genesis/orchestrator/gate-carry.test.mjs`).

**What may vary.** Without leave: sensors, adapters, transports, local thresholds and device policy. By declared variation: a second judge or record, declared with its contract difference (`epr flow sense rule --measure limit-models@1 --subject <file> --class intentional --why <the contract difference>` for a limit judge; no measure id exists yet for a second record, so state the difference in the file header). The same everywhere: canonical bytes and CIDs, wire kinds, `Bound` semantics, and the interpretation of a crossing.

**Three ways a reuse rule can flatten variety by accident, and the guard for each.**
- Counting bespoke files rewards deleting implementors. Count unresolved duplicate records and judges instead.
- An advisory nudge can harden into a block. These stay `inject`; a block needs a ratified policy and a lived, recurring drift.
- Generalizing a grant can erase a distinct witness or a private ceremony. Before graduating anything, compare inputs, errors, authority, persistence and trust boundary.

**Known gaps.** A basename glob cannot match the extensionless scripts in `bin/`, so the bindings do not see them; only `*.py`, `*.mjs` and `*.sh` are bound. The bindings match literal tokens, so a drift written in other words passes silently.
