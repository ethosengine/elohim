---
name: agentic-developer
description: Run a bounded autonomous delivery shift through the shared Claude/Codex host launcher, durable recovery, exact-session adapters, independent native acceptance, gated publication and executable successor learning.
metadata:
  runtime: antigravity
  sourceRuntime: claude
  master: package
  sourcePath: .epr-meta/elohim/packages/skills/agentic-developer.json
  packageKind: SkillPackage
governance: "epr:elohim-agent/skills/agentic-developer"
---
# Shared autonomous delivery shifts

Use `just dev shift start|resume|status|stop|recover` for a bounded task that needs
interruption recovery. The shared host implementation is `genesis/agentic/shift.py`;
its operator contract and declaration example are in `genesis/agentic/SHIFT.md`.
Claude and Codex use the SAME private `.claude/shifts/<id>/` in the owning retained
worktree. This directory is an execution projection, not an acceptance authority.

## Before launching

Use an isolated retained worktree and preserve the shared checkout's edits. Resolve
original user build authorization and separate gated landing/push authorization
from the existing conversation; do not ask again when it already authorizes them.
A covenant variance cannot provide either authorization. Freeze a JSON or YAML
Objective, path scope, original native intent and commitment, original scope CID,
served habit, baseline revision/evidence, verification commands, and absolute UTC
deadline. The objective ceiling is 2,400 minutes; downtime never extends it.

Use `epr flow claim --on <gap> --as <actor> --session <session> --brief <brief>
--serves <habit>` and preserve the resulting native references. Do not substitute
a sprint or conversation ID for the intent's existing scope CID. Classify private
execution records under the existing shift convention; do not create a second
acceptance or stock ledger.

The launcher checks tools, actual runtime authentication, Linux PID namespaces,
resource leases, manifest-selected gates, the measured baseline, and successful
Claude/Codex tool canaries under their actual sandbox/approval settings. Missing
readiness is a named refusal. Neither an echoed marker nor a numeric-looking blank
is a canary or measurement. Inspect the retained readiness attempt before retrying.

## Execution and recovery

Start with `just dev shift start <contract.json|yaml> --runtime codex|claude`.
Use `status <id>` for current native acceptance and durable evidence. Use
`resume <id>` for the exact saved session; `resume <id> --runtime <other>` starts
a NEW conversation from the same durable evidence. Never use latest-session flags.
Legacy journals are readable but cannot execute without a complete contract.

The host guardian holds exclusive execution ownership and the original deadline.
Ark supervises a contained controller; its heartbeat is 15 seconds. A controller
confirmed stopped for 60 seconds is recovered. Quiet builds are not stalled merely
because their transcripts are quiet. Losing the guardian kills the contained PID
namespaces, including daemonized descendants. Controller failure does not discard
a measurement the host still owns. Namespaced tools delegate lease checks to the
host guardian; namespace-local PID numbers never become host process identities.

Persist operation identity before dispatch. On recovery, inspect receipts, task
and build records, commits, and native flow records before retrying. Arbitrary
shell side effects can be uncertain: retain an explicit unresolved frontier rather
than claiming exactly-once execution. `stop <id>` persists a marker that automatic
recovery cannot clear. `recover` scans Git's registered retained worktrees and
refuses stopped, expired, corrupt, or unauthorized execution. Startup configuration
adoption is separate from exercising this command; never call that a host reboot.

## Delivery and learning

Agents implement within the frozen scope. Deterministic host operations own scoped
commit, ff-only landing, push, household lifecycle, and verification. No blanket
permission or hook bypass is available. Recheck current authority before dispatch
and publication. Run the exact manifest gates and household oracle against the
committed revision; bind the household's running binary to its build receipt.

Follow claim → immutable implementation report/fulfillment → technical review →
appointed INDEPENDENT acceptance → native reconciliation. The acceptor must run or
inspect the declared oracle at the exact delivered revision. Preserve the original
fulfillment bytes; later verification creates NEW evidence/acceptance reports and
explicitly supersedes old acceptance when needed. Historical fulfillment or a
checkpoint saying done never overrides current contrary native evidence.

Completion requires stable objective measurements, required gates, matching
household proof, current native acceptance, authorized dev publication, and
inspection of downstream CI at that revision. Use the story-harvest skill and its
isolated story review. Append a scoped habit DELTA and re-project the habit register;
never flip a broader habit green from a narrower regression result.

When the task requests learning proof, run the successor in the OTHER runtime with
no conversational history. Recover durable evidence and exercise the new regression
against a deliberately broken disposable fixture. Restore and verify the delivered
version. This is a verification rehearsal, not another product fulfillment; do not
inject a defect into the shared household or accepted production record.

Record delivered revision, acceptance and household receipts, both runtime sessions,
exercised safeguard, elapsed time, operator interventions and remaining obligations
in the owning sprint result. At deadline, stop and retain the unfinished frontier.

## Maintenance

`just gate agentic-shift` exercises state, adapters, containment/recovery, objective
validation, and lease delegation. Edit this planted SkillPackage first, project
only `SkillPackage:agentic-developer`, and run package-projection verification.
