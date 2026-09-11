---
id: collective-memory-task-2-report
status: DONE_WITH_CONCERNS
gap: plans__2026-09-09-collective-memory-integration#2
actor: agent:implementer@gpt-6
commits: []
cites: []
---

# Bounded footprint and safe retention

Implemented the existing balance entry point as a Python-backed deterministic lens.
`genesis/scripts/memory_balance.py` exposes `method()`,
`snapshot(root, *, run_id, phase, scope, max_files=2000, max_bytes=20000000,
max_entries=10000, max_seconds=10, max_depth=64)`, and `compare(baseline, close)`.
Scope is ordered explicit relative paths, either strings (authored) or dictionaries
with exactly `path` and `category` (authored/projected/retained/operational).
First selected ownership wins; paths are visited once. Symlink traversal is refused.
Read bytes, elapsed work, content hashes, actual sampling windows, missing/changed
inputs and declared exclusions are visible. Optional external providers are not run.
No whole-tree default scan or latest-snapshot pairing remains.

Comparison requires explicit complete ordered baseline/close observations with equal
run, method bytes, root, scope and limits. Counts include all sampled run artifacts;
unique content bytes are distinguished from per-path footprint. Archive relocation
has a separate same-content witness and does not count as net deletion. Unknown
model tokens and private stores remain unmeasured. Snapshot serialization and later
closing artifacts fall after the declared cutoff and need separate accounting.

The shell entry point preserves `--json` and `--no-save`: presentation no longer
changes persistence semantics, and no-save creates no report directory. Explicit
output uses exclusive creation. Missing legacy archive is excluded from default
scope with a limitation; callers can select an actual archive explicitly.

Retention's unproved Git-history assumption is removed. Its legacy API has no
reliable pin/reference inventory or custodian disposition. It therefore holds every
proposed move/delete, preserving ignored, pinned and unknown evidence. This is a
conservative transitional safety repair, not completed reference-aware deletion or
a new pin registry. Nondated recall-executions remains preserved. Native custody
integration and eventual toolkit retirement remain required migration work.

## Baseline and attribution

Before these implementation edits, a trusted independent sequential read captured
`/tmp/collective-memory-source-baseline.json`: 110 files, 1,422,829 bytes, 34,422 lines,
all stat-stable during their reads. Its declared cohort includes native Rust and
Cargo manifests, measurement paths, recall implementations/tests, both capability
packages and this sprint's existing design/plan/scenario/briefs. Those earlier
planning edits already existed at sampling time. This is not a whole-session or
atomic baseline. Its method differs from the replacement collector: **do not compare
it as a same-method pair**. The integrated acceptance must establish its own new
same-method baseline and close. Shared-tree changes cannot be solely attributed to
this seat. Direct implementation reads and orchestration are outside sampler costs.

## Evidence

Gate-evidence: owning command `just gate memory-ceremony`; full integrated execution
is delegated to root after native rebuild, per explicit coordination (not run by this
seat; no gate EXIT claimed). The gate now includes the nine focused tests and watches
helper/shell/test/retention paths. No new command family was added.

Observed checks:

- `python3 -m unittest discover -s genesis/scripts/__tests__ -p memory_balance_test.py`
  — 9 tests, OK, `EXIT=0`.
- `just --show _gate-memory-ceremony` — expected composed recipe, `EXIT=0`.
- `python3 -m json.tool genesis/build-manifest.json` — `EXIT=0`.
- `node genesis/orchestrator/gate-runner.mjs --target memory-ceremony --print`
  — correct native pool/resource configuration, `EXIT=0`.
- Changed-file-list routing of helper/retention includes memory-ceremony, `EXIT=0`.

Tests exercise spaces/newlines/quotes, safe counting, overlapping categories,
consolidation versus relocation plus report overhead, incompatible pairs, missing
inputs, byte/depth/elapsed limits, declared exclusions, symlink refusal, concurrent
file mutation, CLI persistence/error flags and ignored evidence retention.

Existing routing limitation: `--target genesis/scripts/memory_balance.py` returns
Unknown gate project/path (`EXIT=1`); the owning project name works and changed-file
routing selects it. Native context still reports root-directory project ambiguity.
No runner redesign was introduced into this bounded seat.

No commits or pushes. Changed files: memory_balance.py, memory-balance.sh,
memkit-retention.py, memory_balance_test.py, genesis/build-manifest.json and justfile.
Accepted prior ceremony reports were not modified.
