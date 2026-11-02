---
id: brit-native-source-loop-2026-11-02
title: Brit's first native source-to-execution loop
status: Planned
date: 2026-11-02
serves: brit-native-daily-driver
steward: human:matthew
cites: []
graduation-trigger: >
  A disconnected Linux device captures A outside Git, keeps editing B, restores A,
  and explicitly runs a pinned native fixture against A with exact input/method
  identity, passing owner gates, independent review and successor rehearsal.
---

# Brit's first native source-to-execution loop

This is the execution binding for the operator-supplied sprint, not a replacement
architecture. It advances the native-storage section of
[the daily-driver backlog](../../../data/timeline/backlog/brit-daily-driver-handoff-2026-10-27.md)
and the filesystem/offline and fixed-candidate portions of
[shared working versions](../specs/2026-10-13-shared-working-versions-social-coding-design.md).
The broader habit remains open.

## Native stations

- [ ] Deliver the complete offline A/B journey with exact source and executable identities, independent review, owner gates and an isolated other-runtime successor rehearsal.
- [ ] Capture explicit literal files and symlinks through eprfs-local and `brit snapshot capture`, verifying and durably retaining the complete canonical tree closure before returning its root.
- [ ] Synchronize object custody and fresh restore in EPRFS, preserve existing candidates and destinations on interruption, and report retained staging residue.
- [ ] Run an explicit whole-tree local input through Ark's existing task admission and supervisor using `brit build run`, with declared confined outputs and no reusable derivation record.
- [ ] Wire the existing acceptance story, reconcile the owning backlog and covenant note, register decision points and append evidence to the existing habit atom.

The first station is the enclosing sprint promise; stations two through five are
the four ordered implementation commitments. Their order and original scope stay
fixed after projection. Fulfillment requires evidence, not checkbox changes.

## Capture and custody contract

`brit snapshot capture --source <directory> --store <private-store> -- <paths>...`
accepts explicit relative files and literal symlinks and infers ancestor directories.
Reject directories, globs, duplicates/ancestor aliases, escaping paths, metadata
components including `.git` and `.eprfs`, special files and source/store overlap.
Keep byte names, executable modes and literal symlink targets. Do not follow
symlinked ancestors or targets. Use descriptor-relative confined reads with bounded
pre/post stability checks and refuse observed changes. The root names the bytes
captured, not an atomic observation of an independently changing filesystem.

Filesystem I/O belongs in eprfs-local; canonical Tree v1 and storage traits remain
EPRFS-owned, with Git translation in brit-bridge. Write raw-codec leaves explicitly
through verified storage; keep legacy put_blob behavior. Equivalent captures,
including equivalent Git imports, have identical roots. Return only after complete
closure verification and durable storage, with selected-scope summary and
`published:false`. Editable work and authored candidates are durable private
material, with no new eviction/GC command or published-head claim.

Synchronize object files and containing directories. Restore only into a new,
absent directory under the existing trusted-parent Linux contract. Synchronize
staged files/directories before promotion and the destination parent afterward.
Required sync failures cannot report durable success. Preserve prior destinations,
source and candidates; report interrupted staging residue rather than silently
deleting uncertain state. Restore executes no hooks or other processes.

## Task contract

`brit build run --root <CID> --store <private-store> --request <request.json>
--artifact <executable> --run-dir <new-private-directory>` delegates to Ark.
Reuse TaskRequest, TaskSpec, TaskChild, admission, supervision and TaskOutcome.
Add a mutually exclusive local whole-tree input alongside material/Git mode.
Verify the whole closure before execution and admit actual verified/materialized
bytes. EPRFS owns the checked whole-tree conversion to ProjectedRoot and rejects
narrower views. A tree must never be disguised as KeyInput::Raw.

Create separate input/, output/ and home/ directories. Preserve the original spec
and method identity; expand only known `{input}`/`{output}` placeholders in the
runtime copy. Require explicit environment values, including PATH/HOME handling.
Keep brit-gate:/OwnEnvelope authorization; reject other methods without renaming
them. Reject native-mode Git/reuse options, wrong executable digests, unsupported
scope, unknown placeholders and non-whole views before spawn.

Capture only declared outputs with the same confinement/resource bounds. Missing
outputs, interruption or detected input mutation cannot succeed. Return the existing
outcome and output root without a reusable derivation record. This establishes one
exact-source execution, not a hostile-code sandbox, full hermetic closure,
independent acceptance, publication or a reusable attestation.

## Verification and delivery

The native fixture executable has no helper process/network dependency and a
synchronization point allowing the original source to change to B while restored A
runs. Its output must reflect A while B stays intact; B cannot inherit A's result.
Exercise text/binary/modes/byte names/symlinks, selection refusal, opening/read races,
concurrent captures, later edits, injected write/sync failures, interrupted
capture/restore, missing versus corrupt blobs, source/artifact/argv/env/output
identity changes, malicious hooks, output absence and input mutation.

Keep the 64 MiB object and 1 GiB closure limits. Preserve tree golden vectors,
existing snapshot/Git journeys and Ark material-mode tests. Run focused owner tests
and manifest-selected gates. Release EPRFS interfaces before updating consumers;
deliver no sibling-path dependency override. Keep broader @wip scenarios intact.

Baseline is d7dc62fdd in the retained brit-native-loop-20261102 worktree. The user
authorizes implementation and verification; gated release/integration is subject
to exact native acceptance and current authority. The bounded shift must pass
readiness after the existing launcher adaptation and gate-discovery prerequisites
are repaired. Never bypass those refusals. Use the existing shift journal for
costs, receipts, technical review and successor friction/relief. No second ledger.

The successor sequence remains in the native-storage backlog: root-pinned peer
hydration with separately selected serving stores; large-file custody/cache
lifecycle without raising 16 MiB peer frames; native working contexts/history;
publication/availability authority; then Git/LFS interoperability and reusable
builds with complete execution closures. This sprint adds no competing text
collaboration model; Automerge remains the shared-working-version direction.
