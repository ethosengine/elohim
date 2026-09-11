---
title: Memory-kit replacement — finish to parity, then clear the artifact
id: memory-kit-replacement-finish
status: proposed
class: devflow
serves: dev-system-equilibrium
date: 2026-09-10
cites:
  - "unified-memory-loop-design | Unified Memory Loop | sha256:07e941a325cc49c2 | path: genesis/docs/superpowers/specs/2026-06-01-unified-memory-loop-design.md"
  - "collective-memory-integration | Collective memory | sha256:00249865b48968ac | path: genesis/docs/superpowers/plans/2026-09-09-collective-memory-integration.md"
  - "ceremony-efficacy | Memory ceremony efficacy | sha256:23eb372f4ba66135 | path: genesis/docs/analysis/2026-09-09-memory-ceremony-efficacy.md"
  - "verification-as-memoized-derivation-guidestar | Verification as a memoized derivation | sha256:b4a0a2e087e67c12 | path: genesis/docs/superpowers/specs/2026-08-22-verification-as-memoized-derivation-guidestar.md"
  - "middot-measure-primitive-design | Middot | sha256:336ab2b4619b9144 | path: genesis/docs/superpowers/specs/2026-08-04-middot-measure-primitive-design.md"
---

# Memory-kit replacement: finish to parity, then clear the artifact

The unified-memory-loop design (operator clarification, 2026-09-09) retires memory-kit
into two owners: the native-agent memory collective for shared knowledge, and EPRFS
objects governed by `.epr-meta` for files, documents, algorithms and code. Retirement is
earned by replacement coverage, not by renaming. The 2026-09-09 sprints delivered the
collective-memory verbs, the paired burden measure and the ceremony entry. This plan
finishes the replacement and ends with the kit deleted.

The parity inventory taken 2026-09-10 is the ledger this plan drains:
`genesis/docs/superpowers/plans/memory-kit-replacement/parity-inventory-2026-09-10.md`.
Of 36 scripts, 0 are natively covered, 20 partial, 14 missing, 2 retire. The report
tier holds about 10 MB, of which `gap-items/` is the only load-bearing subdirectory.
The keystone is `placement-audit.py`: it feeds the SessionStart headline and dispatches
four other gates, and nothing native reads document placement yet.

## The pattern the replacement demonstrates

Every headline signal, benchmark and watermark the kit surfaces is a Mishpat-shaped
bound over the codebase substrate: a subject, a measure, a soft and hard watermark, and
stakes. The kit hides 35 such thresholds as constants inside scripts and keeps each
accumulator's history in a private JSON file. The substrate already has the home for
them: `.claude/epr-meta/measures.yaml` (middot: teeth-free, version-pinned measures) and
its `lenses:` section plus `policies.yaml` ceiling rows (soft/hard watermarks with an
enforcement class), both declaring a lift path into Mishpat Precedent entries. Today
that registry covers only clippy.

So the reusable pattern is: **bound declared in epr-meta → outcome witnessed by the
native report → history as folds in the flows sidecar → headline as a projection.** One
check per bound, three-valued (`passed | failed | skipped`), keyed subject × measure@version
× env, as the guidestar requires. Everything the kit computed becomes either a declared
bound evaluated this way, a lens relocated under its owner and declared as a foreign
measure, a native verb, or a retirement with a reason. Nothing survives as a third
authority, cadence or queue.

## Owner routing for every kit capability

| Kit capability | Owner | Replacement | Station |
|---|---|---|---|
| placement-audit headline, ledger, focus, stasis, epr-meta census | EPRFS | `epr flow report` (bounds fold) + `epr flow report placement` | 1, 2 |
| cleanup-pressure, memkit-retention status, mempalace-currency status, scope-reconcile report | EPRFS bounds | measure + lens rows; folds via structured observations | 1 |
| placement-drift, map-currency-drift, claude-md-drift, memory-coherence-drift, memory-index-drift, sovereignty-guard-drift, context-coverage baseline | EPRFS bounds | hooks append `epr flow note --kind observation --measure` folds; JSON state files deleted | 1 |
| decompose, gap-items/ | EPRFS | checkbox-tasks extraction inside `epr flow project`; gap-items directory deleted | 2 |
| scope-reconcile apply/set/env, focus-baseline, state-machine-gen | EPRFS | `epr flow hold` + env-scope rule evaluated by `epr flow report scope`; state-machine-gen retires | 2 |
| cite-gen, cite-describe, cite-propagate, cites-migrate | EPRFS | native cite writer `epr flow cites seal|assign-id|describe|verify|stamp` in epr-cli, reader semantics transcribed from brit-epr (brit has no writer verb and its build is registry-credential-blocked, 2026-09-10); parity pinned to the ten recorded `cite-gen --seal` fixed-point digests; cites-migrate retires only after a dry run reports zero (225 ids + 10 conversions pending today) | 3b |
| memory-index-projector, memory-review, memory-coherence-audit, dedupe-memory-scan, cleanup-scan, cleanup-apply, path-update-scan, path-update-apply | agent memory | `.claude/memory` entries become contributions; MEMORY.md is `epr flow memory project --index`; the analyzers relocate under the memory owner as declared foreign measures | 4 |
| recall-ceremony, recall-packet, recall_runtime, recall_lenses, recall_providers, recall-contract.json | agent memory | native recall executor `epr flow memory recall` with budgets, receipts and session accounting; contract stays the algorithm artifact under `.epr-meta` | 5 |
| stale-record | Mishpat | `epr flow concerns --corrections` reads unresolved corrections by identity from flows | 5 |
| claude-md-audit, substrate-currency-audit, locus-drift, spec-coherence-index, prep-brainstorm, context-ratchet | EPRFS lenses | relocate under `.epr-meta/elohim/lenses/`, declare as foreign measures, invoked by the report; context-ratchet becomes bound rows | 4 |
| agent-audit, skill-audit | EPRFS packages | description-quality and overlap checks added to `package-projections.mjs verify`; scripts retire | 3 |
| story-coverage-audit, delivery-status-distribution | EPRFS lenses | relocate and declare; `delivery-scoreboard.py` reads the fold | 4 |
| dated report dirs, state-ledger.json, indexes | retire | derived; regenerable from folds | 6 |
| horizon-scans/, balance-sheets/, recall-executions/ | relocate | authored or live evidence; move under their owners with pins verified | 6 |

## Gate classification (p2p-design-gate)

| Entity | Class | Source of truth | Address |
|---|---|---|---|
| measure / lens / ceiling row | Operational (C), liftable Precedent | `.claude/epr-meta/{measures,policies}.yaml` | `<id>@<version>` |
| measure fold (one witnessed outcome) | Private (B) local record | `.eprfs/status/flows.jsonl` run-note event | content-derived CID |
| report | Ephemeral (C), regenerable from folds | never stored; printed or `--json` | none; method pin in the payload |
| memory contribution (from a `.claude/memory` entry) | as declared by the collective-memory plan | `.eprfs/status/flows.jsonl` | content-derived CID |

No DNA hash moves, no new EPR kind, no new entry type, no HTTP route.

## Delivery stations

- [ ] Every threshold the kit hard-codes is a declared bound, the native `epr flow report` folds each bound into a witnessed outcome, the SessionStart headline is a projection of that fold, and the drift-signal hooks append structured observations instead of writing JSON state.

Native owner: add `epr flow note --kind observation --measure <id@version> --subject <path-or-cid> --value <number> [--unit <u>] [--env <k=v>…]` so a numeric measurement is a structured run-note event (the design names this gap: observation notes are unit-only today). Add `epr flow report [--headline] [--json] [--bound <id>]` in `elohim/eprfs/epr-cli/src/flow/report.rs`: read `measures.yaml` and `policies.yaml`, evaluate each lens or ceiling row's soft and hard watermarks against the latest fold for its subject and env, emit one `CheckWitness`-shaped outcome per bound (`passed | failed | skipped`, with `summary` and `observed`), and print the headline lines the kit prints today (`memkit:`, `mempalace:`, `cleanup:`, `scope:`, memory budget) in the same order so CLAUDE.md's `cleanup:` and `scope:` triggers keep reading. A bound with no fold is `skipped` naming the missing measure, never zero. Accumulated counts are derived, not bridged: a lens row may declare `derive: count-since-reset` (or `distinct-subjects-since-reset`) over the measures it consumes, and the report computes the value from the folds appended since the latest reset observation for that lens (`<lens-id>-reset@1`), so `cleanup-pressure@1`, `placement-drift-due@1` and `map-currency-drift@1` no longer depend on the kit's JSON accumulators; until that lands the hooks write both fold and JSON. Method pin: the report's payload carries the CID of the measures and policies bytes it read.
Integration owner: declare the 35 thresholds as rows (`memory-index-bytes@1` soft 20000 hard 24000; `gospel-bytes@1` hard 12000; `memkit-report-tier-mb@1` soft 8; `cleanup-pressure@1` threshold 120; `decompose-threshold@1` 40; `entry-stale-days@1` 90; `package-description-chars@1` 60 and 80; `trigger-overlap@1` 3; `retention-head-days@1` 30 and `retention-tail-days@1` 90; and the rest, one row each, `default-authority: observation`) with the enforcement class the consuming lens carries today. Rewire `.claude/hooks/{placement-drift-signal,map-drift-signal,claude-md-drift-signal,claude-md-structural-signal,memory-coherence-signal,sovereignty-guard-signal}.py` to append observations through the note verb and stop writing `.claude/memory-kit/*.json`; rewire `load-project-context.py` and `delivery-gate.py` to `epr flow report --headline` with the kit as fallback only while the binary is absent. Tests: report reproduces today's headline slot-for-slot on a fixture where both are computed (value parity: same number, unit and warn/ok state per slot; bytes differ by design); trigger-count rows declare `compare: at-or-above` and the report honours it, so a kit constant meaning "fires at N" fires at N natively; a bound with no fold is `skipped`; an observation with an unknown measure id is refused naming the registry; two identical observations mint one CID.

- [ ] Document placement, gap extraction and env scope are native: `epr flow report placement` derives every file's placement state from epr-meta, cites and flows; `epr flow project` extracts checkbox stations from plans and specs without a gap-items directory; `epr flow report scope` and `epr flow hold` replace scope-reconcile and focus-baseline; `delivery-gate.py` and `scope-reconcile` consumers read native.

Native owner: port `placement-audit.py`'s state vocabulary (`ACTIVE`, `LINKED`, `MEM-UNLINKED`, `NEEDS-TRIAGE`, `CLAIMED-ONLY`, `SETTLED`, `SUPERSEDED`, `VERIFIED-STABLE`, `UNKNOWN-STATUS`) into `report.rs` as a derivation over the same inputs the flow plane already labels (`resources_labeled` in `epr flow project`), with `--ledger` and `--focus <subject>` renderings that match the kit's; port `decompose.py`'s `checkbox-tasks` method into `flow/project.rs` so a plan's `- [ ]` stations mint the same `plans__<slug>#N` gap ids directly, and delete `.claude/memory-kit/gap-items/` once `delivery-gate.py` and the placement report read the flow plane; implement `epr flow report scope` from `genesis/manifests/cluster-state.yaml` and the `@requires:` and `requires_env` resolvers (`_lib/env_scope.py` semantics, ported), and route `scope-reconcile --apply` moves through `epr flow hold`. Tests: placement states over a fixture corpus equal the kit's ledger counts; gap ids minted natively equal the ids in today's gap-items for three existing plans; scope report agrees with `scope-reconcile.py --report` on the current cluster-state.

- [ ] The package verifier absorbs the description-quality audits and the cite read-side parity is pinned: `agent-audit`/`skill-audit` checks live in `package-projections.mjs verify` reading their floors from declared middot rows, brit-epr's drift fingerprint is proven equal to the Python oracle over the corpus, the ten `cite-gen --seal` fixed-point digests are recorded as the writer parity baseline, and the `epr flow concerns --stamp` contract is written.

- [ ] The cite writer is native: `epr flow cites seal|assign-id|describe|verify|stamp <doc>` in epr-cli reproduces the recorded fixed-point digests byte-for-byte on the ten baseline docs, `stamp` implements the recorded contract and replaces `cite-propagate.py --apply` (72 edges pending today), hooks, commands and managed surfaces call the native verbs with the Python scripts as fallback only while the binary is absent, and the four cite scripts delete after a corpus dry run of `cites-migrate.py` reports zero pending.

Integration owner: add the brit binary to the tool pin (`MESH_TOOLS_DIR` sibling, built from the `elohim/brit` submodule pin, `RUSTFLAGS=""`), replace `cite-gen.py` invocations in `.claude/hooks/cite-seal-signal.py`, `.claude/commands/{brainstorm,plan,shift}.md`, `.claude/scripts/_lib/managed_surfaces.py` and the skills that name them; run brit's `cite_parity.rs` oracle test against the current corpus before the switch and record the digest; implement `cite-propagate`'s `status:`/`path:` stamping as `epr flow concerns --stamp <doc>` (native seat) so the stamp derives from the same verdict the concern view shows; add `descriptionMinChars` and trigger-overlap checks to the package verifier with the bound rows from station one; delete the four cite scripts and the two audit scripts. Tests: brit-sealed envelopes on ten existing docs are byte-identical to `cite-gen.py --seal`; verifier fails on a fixture agent with a 40-char description and passes at 80.

- [ ] Private memory entries are collective contributions and the memory analyzers live under the memory owner: every `.claude/memory/*.md` entry is imported as a contribution with git-author provenance and its original bytes pinned, `MEMORY.md` is projected by `epr flow memory project --index` under the declared byte bound, the PostToolUse hook is swapped, and memory-review, dedupe, coherence, cleanup and path-update relocate as declared foreign measures the report invokes.

Native owner: `epr flow memory import <dir>` (batch contribute, idempotent by content, `steward:` from git author, `display` the file's `name:`), and `epr flow memory project --index --budget <measure-id>` rendering the index rows the projector renders today, refusing at the hard watermark and reporting unloaded rows as a fold. Integration owner: move the analyzers to `.epr-meta/elohim/lenses/memory/` unchanged, declare each in `measures.yaml` with `family: memory-lens`, `procedure` naming the script and `unit` naming its JSON shape; the report invokes them and folds their outputs; swap `.claude/settings.json:206` to the native index projection; retire `cleanup-apply` (archival relocation is not net removal per the design) and route accepted cleanups through `epr flow hold`. Tests: import of the current 76 entries is idempotent on a second run; projected `MEMORY.md` equals the projector's output on the same inputs; the index bound fails at 24,001 bytes.

- [ ] The recall executor is native and corrections close by identity: `epr flow memory recall` carries budgets, receipts, method pins and session accounting; `epr flow concerns --corrections` lists unresolved corrections by identity with closure evidence; `recall-ceremony.py` and its libraries delete; the ceremony skill and packages name only native verbs.

Native owner: port `recall_runtime.py` (budget counters, bounded reads, method pinning), `recall_lenses.py` (projection receipts) and `recall_providers.py` (recipe-selected providers, deterministic local traversal default) into `flow/memory/recall.rs`; the contract stays `.claude/scripts/memory-kit/recall-contract.json` relocated to `.epr-meta/elohim/algorithms/recall-contract.json` and read as the algorithm artifact whose CID every receipt pins; receipts move from `.claude/memory-kit/recall-executions/` to `.eprfs/status/recall/` with existing receipts relocated and their CIDs re-verified. Implement `--corrections` on `epr flow concerns` from `run:correction` events not followed by a closure note naming the same target CID, replacing `stale-record.py`'s filename-date heuristic (efficacy error one). Integration owner: memory-ceremony and memory-kit packages re-authored package-first to the native verbs, projected, verified 1919/0 or the new count; the `_gate-memory-ceremony` recipe drops the Python recall suite once the native tests cover the same 60 cases. Tests: the 60 recall unit cases pass against the native executor; a correction with a closure note is not listed; one without is; a receipt's method CID equals the contract file's CID.

- [ ] Parity is proven by observation and the artifact is cleared: two isolated readers answer the fixed fresh-start question set before and after against the same tree, the parity report shows every inventory row as native, relocated or retired with evidence, and `.claude/scripts/memory-kit/` and `.claude/memory-kit/` are removed with authored evidence relocated and pins verified.

Integration owner: the question set is the efficacy analysis's five (correct authoritative source reached; mistaken assertions avoided; files, bytes and tokens consumed; implementation rework avoided; unresolved inflow) asked as concrete repository questions and answered by a context-reset reader once through the kit's `open` and once through `epr flow report` plus `epr flow memory recall`, receipts retained, judgment recorded in the timeline chronicle rather than a new dashboard. Then `epr flow report parity --inventory <path>` renders the inventory rows against the tree: a row is `passed` when its replacement exists, its tests pass and no consumer references the kit path. Only at all-passed: relocate `horizon-scans/` to `genesis/docs/analysis/horizon-scans/`, `balance-sheets/` to `.eprfs/status/balance/`, delete dated report dirs and state files, `git rm -r` the kit, remove the memory-kit skill package and its projections, update the four memory agents' packages, the root `CLAUDE.md` bounded-recall and trigger sections, `genesis/build-manifest.json`, `justfile` and `.claude/workflows/memory-stasis-loop.js`. Append one DELTA to `.epr-meta/dev-system-equilibrium.habit.md` and reproject. The habit flips only if the fresh-reader comparison shows the after run reaching the authoritative source with fewer mistaken assertions at equal or lower bytes; otherwise it stays RED with the numbers.

## Execution boundaries

Stations one and three can run in parallel; station two follows one; station 3b (native
cite writer) follows two on the native seat; four and five follow two; six is last and
is the only station that deletes the directory. At most two seats at a time, scoped write sets: the native seat owns
`elohim/eprfs/**` and `.eprfs/status/**`; the integration seat owns `.claude/hooks/**`,
`.claude/epr-meta/**`, `.claude/settings.json`, `.epr-meta/elohim/**`, commands, skills,
`genesis/build-manifest.json`, `justfile`, and the habit atom. No commits or pushes in
this sprint; existing dirty edits are preserved. Native gate: `env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-gate-target CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 cargo test --manifest-path elohim/eprfs/Cargo.toml --workspace`
plus fmt and clippy `-D warnings`, `EXIT=$?` echoed on its own line, cargo berth claimed
first. Integrated gate: `just gate memory-ceremony`. Every station ends with independent
technical review; stations four and six add a fresh-agent observation with context reset.
Reports land in `genesis/docs/superpowers/plans/memory-kit-replacement/task-N-report.md`
linked through the native lifecycle. The kit is never deleted piecemeal: a script is
deleted only in the station whose replacement covers it, and the whole directory only in
station six.
