---
id: task-brit-readiness-3-report
gap: plans__2026-09-28-brit-governed-readiness#3
actor: agent:implementer@gpt-6
status: DONE_WITH_CONCERNS
commits: []
---
# Brit readiness boundary implementation

Added installed-epr delegation at `brit context`, preserving OS arguments, inherited
stdio and evaluator exit codes without sibling lookup. Added standalone repository
identity, covenant, four local habits, recipe bindings, per-habit work plans and the
complete capability-family boundary map. Inherited gix code remains untouched by
this task. The parent implemented the exact CI qualifier/contract/negative fixtures;
this task connected its owning habit and added context journeys to inherited CI.

No commits or pushes. All changes are scoped uncommitted work. Pre-existing dirty
safe-authoring sprint and through-line documents were preserved.

Gate evidence: `just gate brit` → `EXIT=0` (`ethosengine/brit@bd915393df11 Tests pass
success`). This attests the existing committed parent pin only, not the dirty Brit
worktree. `(cd elohim/brit && epr check)` → `EXIT=0`, 39 changed paths, governance floor
coherent. This local governance check does not authorize remote reach.

Additional verification, RUSTFLAGS empty, CARGO_BUILD_JOBS=1, target
`/tmp/brit-cutover-cargo-pool/family/main/home__matthew__git__elohim__elohim__brit/dev`,
under claimed/released cargo berth `brit-readiness`:

- `cargo +1.98 build -p brit-cli --bin brit` → `EXIT=0`.
- `cargo +1.98 test -p brit-cli -p cli-journey --test context --bin brit` → `EXIT=0`, 3 unit and 2 journey tests passed, none skipped. Repeated after final executable rebuild.
- `cargo +1.98 clippy -p brit-cli -p gitoxide-core -p cli-journey --all-targets -- -D warnings` → `EXIT=0`. Existing inherited removed-lint warnings remain toolchain diagnostics.
- `python3 scripts/ci/test_qualify_readiness.py` → `EXIT=0`, 9 tests.
- Scoped rustfmt, YAML parse and `git diff --check` → `EXIT=0`.
- Public help and actual installed `epr flow context` JSON delegation both → `EXIT=0`.

Concerns: product daily-driver and maximal Git compatibility markers remain unwired;
feature reconciliation and inherited CI habits remain red. Native epr cross-component
projection, independent acceptance and cold/warm context measurements are parent
integration work. Context's synthetic executable forwarding fixture is Unix-only;
missing-evaluator refusal and parser tests are portable. Dependency prose deliberately
does not claim a new checkbox dependency scheduler. Local Git reference is v2.54.0
at 94f057755b7941b321fd11fec1b2e3ca5313a4e0, not a committed Brit gitlink. No remote
check result or release on this dirty revision is claimed.
