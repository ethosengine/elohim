---
id: task-brit-readiness-2-report
gap: plans__2026-09-28-brit-governed-readiness#2
actor: agent:implementer@gpt-6
status: DONE_WITH_CONCERNS
commits: []
serves: dev-system-equilibrium
---
# Native actionable reconciliation

Added the ephemeral actionable view to the existing context evaluator. Feature
assertions and bounded habit references use the same reconciliation record snapshot;
human and JSON output share ordering, ownership, evidence state and suggested action.
Ordering follows declared covenant priority, available prerequisites, source order
and stable IDs. Unranked and unknown states remain visible. Rows carry feature text,
source path/CID, owners, blockers and reasons. Current repository identity and habit
reader errors are explicit rather than silently empty.

Habit entry follows declared references and existing accounted commitments without
a repository scan, with a 32-reference/read-scope limit and omission disclosure.
Source reads and historical labels are confined to the repository. Context never
claims work. Rejected or stale evidence retains native revalidation; technical
approval predating new production cannot suggest acceptance. The independent
acceptance evaluator is unchanged.

Scoped changes: `elohim/eprfs/epr-cli/src/flow/context.rs`, new
`context/actionable.rs`, and actionable assertions in `tests/flow_acceptance.rs`.
Other edits in the latter fixture belong to the portable repository task. No
commits or pushes; pre-existing dirty work preserved.

Gate evidence: `just gate eprfs` → `EXIT=0` (coordinated portable seat; 1142 passed,
0 failed, 0 ignored). The final additive assertion-preview change was then verified
with `env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-gate-target CARGO_BUILD_JOBS=1 cargo
test --manifest-path elohim/eprfs/Cargo.toml -p elohim-epr-cli --lib --test
flow_acceptance` → `EXIT=0` (281 library and 12 acceptance tests). Final strict
workspace/all-targets Clippy and workspace format check → `EXIT=0` each. Evidence
logs: `/tmp/epr-readiness-polish-tests.log`, `/tmp/epr-readiness-polish-clippy.log`,
`/tmp/epr-readiness-polish-fmt.log`; these transient paths are supplemented here by
recorded commands and results.

Regression evidence covers source-order/covenant ranking, unknown environment and
stale seals, existing owner preservation, revised source/historical linkage,
read-only record counts, habit-to-feature entry, external path confinement,
production/review/independent acceptance, changed evidence and conflicting decisions.
The first failed rejected-review test expected revision; native reconciliation
correctly required stronger revalidation. Its expectation was corrected without
weakening the evaluator.

Concerns: ranking is a suggestion over current target or declared habit references,
not acceptance authority or a new global scheduler. Cross-station prose dependencies
are not invented executable metadata. Habit context discloses source-scope omissions;
full standalone memory-bootstrap portability remains a separate declared station.
Parent integration owns public cold/warm timing and readiness habit DELTAs.
