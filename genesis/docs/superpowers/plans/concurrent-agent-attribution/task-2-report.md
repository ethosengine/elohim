---
id: concurrent-agent-attribution-task-2-report
title: Concurrent native sidecar authors preserve accountable history
status: DONE
class: process-meta
gap: plans__2026-09-05-concurrent-agent-attribution-plan#2
actor: agent:implementer@gpt-6
commits: []
cites:
  - "concurrent-agent-attribution-task-2-brief | Coordinate concurrent actor and flow authors without losing evidence | sha256:191d2c4baa7daaf6 | path: genesis/docs/superpowers/plans/concurrent-agent-attribution/task-2-brief.md"
---

## Implementation and verification

Station 2 implementation is complete in the shared worktree. Review is a separate valueflow
verdict; this report does not assert one has already been recorded. No commit, push, installation,
model identity change or public attestation was performed.

Existing actor and flow sidecars now use native file locks, atomic create-new, coordinated
read/check/append transactions and per-record sync_all. The existing actor claim, flow claim,
note, both fulfillment paths and projection consume those transactions. Actor lookups try-lock
so busy attribution cannot stall governance; missing identity still grants no authority.
Existing atoms and the station 1 historical claim pins are unchanged. No dependency was added;
the filesystem helper is behind the existing sidecar feature (standard-library locks need
Rust 1.89+).

Gate evidence, parent-observed after the final fix (2026-09-05):

- `just gate elohim-epr` — `EXIT=0` (fmt, clippy, EPR and REA all-target tests).
- `just gate eprfs` — `EXIT=0` (fmt, workspace all-target clippy, workspace tests and doctests).
- `env RUSTFLAGS= CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim/dev cargo test --manifest-path elohim/Cargo.toml -p elohim-epr-rea --no-default-features --all-targets` — `EXIT=0`.
- Both changed seam registries validated against the existing JSON schema; scoped
  `git diff --check` passed (`EXIT=0`).

The real cargo berth was acquired by codex-concurrent-sidecar, renewed by the parent on takeover,
and released after verification (`EXIT=0`). Initial root-gate dependency resolution failed on
an uncached unrelated workspace dependency; an approved normal gate retry resolved it. Existing
ts-rs serde-attribute warnings were nonfailing. No alternative private lease or gate bypass was used.

## Runnable evidence and discovered constraints

`tests/support/concurrent_sidecars.rs` in epr-rea proves eight OS processes racing first open
and preserving 96 distinct records in each log, coherent flow reads across a paused transaction,
killed-writer lock release, and refusal of preserved interrupted tails. The first-open old-code
race was identified from source, not claimed deterministically reproduced.

CLI support modules concurrent_actors and concurrent_flows prove eight same-role workers have
one same-intent winner with the winner's exact claim pin; exact actor/claim/note/fulfillment
retries append once; concurrent projection retains deduplication; a held actor writer does not
block governance or fabricate a claim. Existing semantic EventKey regression tests remain in
the full gate; the new concurrent projection fixture alone does not prove differing-CID semantic
event deduplication.

The implementer reported a deterministic red probe against the installed pre-slice CLI:
complete JSON missing its newline was accepted as an idempotent claim (exit 0), while the new
regression expected refusal and failed (`EXIT=101`). The current CLI regression passes.

Parent verification found and reproduced a second defect in the initial implementation:
`just gate eprfs` returned `EXIT=1` because a sequential actor read saw WouldBlock after claim
returned. Closing a locked handle can leave its lock alive through a duplicate descriptor,
including the fork-to-exec window of another thread's spawned child. The deterministic
`dropping_transaction_unlocks_even_if_an_inherited_handle_remains_open` regression failed
with `EXIT=101` before explicit Drop unlocking and passed with `EXIT=0` after it. The full gates
above were rerun after this fix. This proves the descriptor-lifetime mechanism; the exact fork
interleaving of the initial gate failure was not instrumented.

The implementer measured the naive per-append validation prototype at 36.641 seconds for
1,000 records / 595,072 bytes; transaction-local validation reduced that to 1.653 seconds.
Read/check remained linear (103.442 ms before, 109.088 ms after). These are local development
measurements, not a universal performance bound or a speedup over the pre-slice unsynced store.
Validation is forgotten at lock release and invalidated before uncertain writes; no persistent
index, cache or alternate history was introduced.

## Honest limits

Locks coordinate cooperating native processes on the same filesystem, not peers. Per-record
fsync is not directory-creation durability, multi-record rollback or power-loss-atomic batches.
A crashed process releases its lock when all inherited descriptors close. Flow readers and
writers may wait for a live lock holder; actor attribution reads return an explicit I/O error
on contention, with the existing governance fallback. An incomplete, malformed or tampered log
is preserved and refuses further append; there is no automatic truncation or repair command.

Raw Python readers (habits-project.py and saga-status.py), derived labels.json reads and writes,
and noncooperating external file mutations are outside this native transaction guarantee.
No real Claude/Codex/Gemini lifecycle integration, peer witnessing, DID authentication, model-fit
consumer, FUSE mount or ark PVC behavior is claimed by these tests.

The delegated implementer exhausted its runtime allowance after focused verification; the
parent continued the same claimed work, recovered full gate evidence and fixed the lock-lifetime
regression. The first independent review dispatch also failed at runtime capacity, with no verdict;
any replacement review must be evidenced in the native valueflow before station 2 is discharged.

Initial review hold: the Codex reviewer could not run because of quota/capacity. Alternate installed
Claude dispatch was rejected before process creation by auto-review because sending repository
contents to that destination requires explicit user consent. Nothing was sent. Both full gates
are green, but no independent verdict or station 2 fulfillment is claimed.

Resolved review hold (2026-09-05): the operator subsequently authorized the scoped Claude
review. That launch selected API authentication and failed with HTTP 400, credit balance too
low, reporting zero tokens and zero cost; no review ran. The operator then explicitly selected
gpt-5.6-sol. That independent reviewer applied the planted valueflow-reviewer skill, found no
Important or Minor issues, and recorded exactly one approved verdict as
`agent:reviewer@gpt-5.6-sol`. It accepted the gate/red-state evidence and documented local
durability limits. No code edits, Cargo commands, commits, pushes or installs were performed by
the reviewer. This approval resolves the hold; station 2 can now be fulfilled against this report.
