---
id: placement-integration-report
title: Placement interfaces — Codex recovery and integration evidence
status: DONE_WITH_CONCERNS
class: architecture
gap: plans__2026-10-20-placement-contract-architecture-plan#22
actor: agent:implementer@codex
commits: []
habits: [materialize-what-is-read]
cites: []
---

This report closes recovery of the interrupted C1–C6 interface and first-adapter workflow and
records H1's habit ratchet. It does not graduate the broader placement architecture plan.
The integration phase remained uncommitted under the original architecture-before-landing ruling.
The operator subsequently authorized `/shift` to land this result; the landing phase follows below.
The household mesh stays stopped; identities were preserved and no household proof was run.

## Integration evidence, 2026-10-21

Receipts below live under `genesis/local-dev/placement-codex-handoff/`. Counts are successful
tests, with existing ignored tests separately identified. Native gates ran serially with their
declared toolchain, Cargo pool, resource limits and RUSTFLAGS. Optional brit evidence attachment
was unavailable because brit was not on the configured binary path; the gates themselves ran.

| Gate | Result | Receipt |
|---|---|---|
| seam-contracts | EXIT=0; 186 passed, 1 ignored; fmt, clippy, WASM build | `seam-contracts-gate.log` |
| eprfs | EXIT=0; 1,326 passed; fmt and workspace/all-target clippy | `eprfs-gate-final.log` |
| elohim-placement | EXIT=0; 129 placement tests plus 21 compute tests with default features disabled | `elohim-placement-gate-final.log` |
| elohim-ark | EXIT=0; 155 unit/integration tests plus 3 compile-fail doctests | `elohim-ark-gate-final.log` |
| elohim-compute | EXIT=0; 50 tests | `elohim-compute-gate-final.log` |
| elohim-epr | EXIT=0; 355 tests | `elohim-epr-gate-final.log` |
| elohim-storage | EXIT=0; 5,728 passed, 59 ignored; final-source build and both clippy configurations | `c5-gate-3.log` |
| k8s-bridge | EXIT=0; 26 tests, fmt and locked all-target clippy | `k8s-bridge-gate-final.log` |
| doorway | EXIT=0; 1,835 passed, 2 ignored; fmt and clippy | `doorway-gate-final.log` |
| epr-ts | EXIT=0; 43 tests, build and typecheck | `c6-epr-ts-final.log` |
| eprfs-ts | EXIT=0; 27 tests, build, typecheck and purity rail | `c6-eprfs-ts-final.log` |
| storage-client-ts | EXIT=0; 131 tests plus plain Node ESM public-package consumer | `c6-storage-client-ts-final.log` |
| elohim-app | EXIT=0; 234 files / 4,717 tests, Angular build, standalone service-worker typecheck, lint and architecture rails | `c6-app-gate-final.log` |
| schema generation and contracts | EXIT=0; 270 generated interfaces fresh and 78 schema tests | `c6-codegen-verify.log`, `schema-test-3.log` |

Both seam registries validate against the authoritative JSON schema. A frozen offline pnpm
install preceded the final TypeScript gates. The k8s lockfile adds the required path dependencies
and async-trait edge and removes unreachable compute-runtime dependencies, with no existing
package version changes. The offline resolver's unrelated downgrades were discarded first.

## Integration corrections

- Probe readings expose getters over private fields; callers cannot rewrite evidence stage or
  answer fields. Compile-fail contracts run in the seam-contracts gate.
- Pending tasks reserve the trusted envelope and measured view before earlier classes can
  allocate the same capacity. Missing measurements retain offered room. Tests cover both orders.
- Unadmitted requesters receive identical complete private-root and unknown-root refusals across
  read/list/diff/batch, with denied presence and zero store reads.
- Storage metadata failures remain unreachable rather than false absence. The ranking shim
  preserves prior contract vectors; production and dependency rails cover missed forms.
- TypeScript consumers use built public exports and the shared address verifier. Cache read
  failures remain typed absences. The app gate now typechecks its standalone service worker.
- Positive schema fixtures supply existing required trust/tags fields; requirements were not weakened.

C1–C6 reports retain original review history and add current Codex evidence. The interfaces
spec's reconciliation section records implemented signatures and open boundaries. The handoff's
rust-analyzer errors are absent from fresh compiled gates; their fixes were already in the
inherited tree when recovery began.

## Evidence limits and remaining boundaries

The first eprfs gate failed the existing
`migration_provider_is_the_session_claim_when_one_exists` test: its provider matched but the
steward read as the repository rather than the expected human. It then passed 21 focused runs,
and the final full suite passed all 50 memory-import tests. Neither production attribution code
nor assertions changed. Its intermittent cause remains unestablished. Evidence is retained in
`eprfs-gate.log`, `eprfs-migration-focus-1.log` and `eprfs-migration-repeat.log`.

The habit stays **red and inactive**. Runtime adoption, first-acceptance wiring, remote HTTP
batch/signer integration, live service-worker root/head discovery, resident views, persistence,
broadcasts and live probes remain future work. Membership-by-inclusion does not enforce an
independent per-object reach restriction; that policy remains open before remote serving.
The unpublished AGPL dependency and Apache-licensed eprfs release boundary still needs a release
and licensing decision. No package was published.

C5's handoff failure is explicitly a Claude HTTP429 weekly limit. The reason for C6's historical
Codex-to-Claude fallback was not established and is not claimed as a confirmed rate limit.
Current technical verdicts do not erase historical review records or appoint acceptance authority;
this report claims neither full-plan acceptance nor runtime graduation.

## Story-harvest station for the runtime follow-on

- Chain: `placement-floor-converges` → task placement on a berth.
- Between: an existing pending task commitment → allocation of an earlier-class task.
- Missing runtime station: pending commitments retain their trusted measured footprint before
  any new allocation. Given a 4,097-byte berth, a pending task with a 4,096-byte measured view
  and a public requirement reduced to one byte, an earlier-class task cannot take that room.
  An unknown view measurement must not be interpreted as free capacity.
- Probe: `a_pending_task_reserves_its_trusted_view_before_earlier_classes` in Ark's task contract.
- Current state: green at the native contract boundary, unmeasured through a production runner.
  Carry this station into the runtime task-placement a2o slice; the interface pass does not
  widen the story lane or claim a live measurement.

## H1 ratchet

Exactly one evidence DELTA was appended to the served habit on 2026-10-21, preserving red/inactive.
The declaration now names the native view/tier/refusal checks and the TypeScript package gates.
`habits-project.py` projected 50 habits and `habits-project.py --check` returned EXIT=0; receipts
are `h1-habits-project.log` and `h1-habits-check.log`. Independent review confirmed the declaration
and projection agree and the active-habit count remains two. H1 is complete within that scope;
its terminal flow record names this report. The broader plan body and intent identities remain intact.

## Landing authorization and boundary

After the integration gates passed, the operator explicitly requested `/shift` to land the result
while unattended. This supersedes the prior no-commit/no-push constraint for this workflow.
A separate checkout isolates the placement changes from unrelated shared-worktree changes.
Only placement-owned paths and the placement hunk of the generated habit projection belong in
the landing. Agent-doc/covenant work, telemetry, unrelated artifacts and submodule changes are
excluded. Commit/push and fresh CI outcomes are recorded in the shift journal, not implied by
these local integration results. The broader architecture remains Active.
