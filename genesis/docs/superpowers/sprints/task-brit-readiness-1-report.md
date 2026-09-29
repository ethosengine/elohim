---
id: task-brit-readiness-1-report
gap: plans__2026-09-28-brit-governed-readiness#1
actor: agent:implementer@gpt-6
serves: dev-system-equilibrium
date: 2026-09-28
status: DONE_WITH_CONCERNS
commits: []
---

# Portable repository attribution and native habit census

Implemented explicit local repository scope through `.epr-meta/repository.yaml`
(`version: 1`, `agent: repo:<namespace/name>`). New production flow operations
resolve this declaration from their explicit root; missing or malformed declarations
refuse instead of attributing a foreign checkout to Elohim. Root Elohim declares its
historical identity, preserving existing scope CIDs. Historical no-root helpers remain
compatibility APIs; production call sites use the root-aware functions. No participant,
network authority, DHT entity, or historical record was introduced or rewritten.

Native habit reading now discovers directory-local atoms from the Git index plus
untracked, nonignored files, without traversing gitlinks. Archive fallback skips nested
checkouts and fixture/build trees. The reader validates declarations and covenant order,
returns the declaration path and optional priority, and reports malformed governance.
A declared modern repository cannot resurrect deleted atoms from a stale generated
register. Undeclared historical archives can still read their legacy projection.
Projection propagates an invalid declared covenant/census rather than silently dropping
the WIP fence. Existing synthetic test fixtures now explicitly declare their historical
repository scope and author valid native habit atoms where needed.

Memory collective default attribution and the affiliation omission rule also use the
local repository declaration. Context-free affiliation signature/wire validation stays
independent of local scope; both admission and writes apply the same scoped omission
rule. Two-repository, historical-CID, idempotent projection, stale-register, invalid
census, nested-checkout and scoped affiliation regressions cover these boundaries.

Gate evidence: `CARGO_BUILD_JOBS=1 just gate eprfs` — `EXIT=0`.
The owning gate passed formatting, strict workspace/all-targets Clippy and 1,142 tests
with zero failures or ignores (74 suite results). Log: `/tmp/epr-readiness-gate.log`.
Earlier red runs identified undeclared legacy fixtures and the old generated-register
source assertion; these were migrated without relaxing production checks.

After task 2's additive assertion-text view polish, verification used
`env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-gate-target CARGO_BUILD_JOBS=1` with:

- `cargo test --manifest-path elohim/eprfs/Cargo.toml -p elohim-epr-cli --lib --test flow_acceptance` — `EXIT=0`; 281 library tests and 12 acceptance tests.
- `cargo clippy --manifest-path elohim/eprfs/Cargo.toml --workspace --all-targets -- -D warnings` — `EXIT=0`.
- `cargo fmt --manifest-path elohim/eprfs/Cargo.toml --all -- --check` — `EXIT=0`.

Logs: `/tmp/epr-readiness-polish-{tests,clippy,fmt}.log`. Scoped changes remain
uncommitted in the shared tree by dispatch instruction; nothing was pushed.

Concern: this qualifies the shared native context route consumed by developers and
harnesses. The existing bounded recall bootstrap recipe still names the historical
`genesis/manifests/habits.yaml` input and is not a fully standalone Brit bootstrap
installation. Its separate governed budget/recipe contract remains unchanged. No
daily-driver readiness or habit-green claim is made by this implementation report.
