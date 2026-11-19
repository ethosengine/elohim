---
id: "backlog-brit-rebase-cost-gitlinks-and-stranding"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "brit rebase spools gigabytes for a two-commit replay, refuses gitlinks, and can strand a repository in 'preparing'"
slug: "brit-rebase-cost-gitlinks-and-stranding"
written: "2026-11-19"
author: "claude-opus-5-5 (brit sprint 3, session 0e0924d5, steward of record human:matthew)"
status: "backlog"
priority: "high"
relatedNodeIds: []
tags: [brit, daily-driver, rebase, switch, submodule, shallow, performance, perf-io, perf-latency, perf-scale, unit-history, lane-operator, open-question]
cites:
  - genesis/data/timeline/backlog/brit-sprint-3-handoff-2026-11-19.md
shift_objective: |
  In this order (decided 2026-11-19, after the cost report): (1) the stranding fixes (cost report
  section 7 a-d) plus a preflight that refuses before any spooling when completion is impossible;
  (2) the redesign (cost report section 6), where retained work scales with what the operation
  can lose. The steward decides first whether the retained-manifest v2 format change may apply
  to every new copy. Until then rebase stays a declared git fallback on both devices.
---

# brit rebase: cost, gitlinks and stranding

## Measured on shem

brit 0.2.6, monorepo not shallow, a linked worktree, two docs-only commits replayed over two
upstream commits:

- `brit rebase origin/dev` ran 6 minutes at 95% of one core, RSS 160 MB;
- it wrote 29.1 GB under `/tmp/.brit-checkout-*` (29.09 GB cancelled) and read 3.5 GB;
- it then exited 1 with "Directory staging is not supported; name explicit files", leaving no state;
- `git rebase --autostash origin/dev` did the same replay in under 2 s;
- old empty `/tmp/.brit-checkout-*` directories were left behind.

## Found by reading and scratch runs (session 0e0924d5)

- Any gitlink difference between HEAD and upstream refuses ("affected gitlinks are unsupported").
- A gitlink present with its submodule directory present refuses with the directory-staging message.
- With the submodule directory ABSENT (uninitialised), `brit rebase` creates `.git/brit-rebase` in
  phase `preparing`, fails with "An object with id … could not be found" (the gitlink id treated as a
  local object), and then neither `--abort` nor `--continue` can clear it, and `brit switch` refuses
  "pending replay". The repository is stranded. The monorepo is protected only because rebase refuses
  shallow repositories first.
- Separately, a `brit switch` killed by SIGTERM just after taking the guard leaves an empty
  `<gitdir>/worktrees/<name>/brit-switch` directory. Every writer in every worktree then refuses
  "pending switch or pull", and `switch --status` / `--abort` fail with ENOENT. Cure today: remove the
  empty directory by hand.

## Shallow rebase (superseded in part, see the 2026-11-19 cost report below)

The selection condition "every boundary commit either walk reaches is reached by both, plus the
linear-chain rule" is necessary and sufficient. What is missing is the retained copy's format.
Design ready: a `shallow_floor` list in the journal state and a retained-manifest v2 written only
when the floor is non-empty, every history walk stopping at the recorded floor.

## Order of work (decided earlier on 2026-11-19; superseded by the cost report below)

1. Every refusal decidable from the index and two trees moves ahead of any spooling.
2. The stranding fixes, and a rule that a journal still in `preparing` can always be aborted.
3. A cost model where what is preserved scales with the replayed range and the paths it touches.
4. Then shallow floors and gitlink moves.

Until (3), rebase stays a declared git fallback on both devices. No lane has started.

## Cost at real-repository scale: scout report, 2026-11-19 ~11:20Z

This section supersedes the order of work above. Marks: [M] measured, [R] read in code, [I] inferred.
File:line references are in the brit repository (`gitoxide-core/src/repository/…` unless a crate is
named), read on the integration worktree.

### Bottom line

brit rebase is not usable at this scale, and bounded tuning will not fix it; it needs a different cost
model. On a full clone of brit (no gitlinks), a one-commit rebase took about 26 minutes to publish
where `git rebase` takes 0.21 s. Cleanup then ran for 4.5 hours and could never have finished.

The monorepo is protected today only because rebase refuses shallow repositories. A full clone of
brit itself is exposed.

### Measurement

- **Clone** [M]: full, not shallow; 17,972 commits (all refs), 418,781 packed plus 2,990 loose
  objects, 297 MB of packs.
- **Closure** [M]: the tip reaches 17,793 commits; the rebase's object set is 218,090 objects,
  2.19 GB decompressed (blobs 2.10 GB, trees 70 MB, commits 21 MB). The index has 3,846 entries.
- **Shape:** branch at tip~50 (first-parent), add 1 or 3 small commits, then `rebase tip`.
- **git rebase** [M]: 0.21 s for one commit, 0.25 s for three, 39 MB RSS.
- **brit rebase** [M] (release brit 0.2.6, `brit -t rebase tip`, one commit):

| Time | Phase | Bytes written so far |
|---|---|---|
| t≈300 s | — | ≈25 GB |
| t=547 s | `pre-approved` | 49 GB |
| t=742 s | `replaying` (first checkout decision) | 64.7 GB |
| t=748 s | `step-proposed` | — |
| t=1,282 s | `ready-final` | 110 GB |
| ≈1,578 s | `completed`, "Rebase published" | — |
| 1,578–17,972 s | cleanup copy: 77,778 of ~218k members, 1.003 GB on disk | — |

- **At the stop** [M]: wall time 4:59:33, peak RSS 624 MB; file-system output 350,763,400 blocks,
  about 179.6 GB, of which 162.9 GB was cancelled; the trace root reads `user: 1393s | sys: 15845s`,
  so system calls dominate. Measured by the coordinator at the SIGTERM stop (process 17,972 s old):
  wchar 167,250,602,854; write_bytes 179,590,852,608; cancelled_write_bytes 162,890,326,016; 77,780
  numbered files in the retained copy (77,778 counted after the stop).
- **Journal** [M]: 303 MB in 415 files. `state.json` is 37.2 MB; each decision adds two 37.4 MB files
  (`decision-N.json` and `transition-N-*/plan.json`); `retention-0.json` is 29.8 MB. The payload is a
  JSON array of decimal numbers, about 4 bytes on disk per byte of data, wrapped around a JSON list of
  all 218k object ids.

### Mechanism

- **One full pass** [R]: every object reachable from both tips, down to the roots, is hashed. Each blob
  goes through `verify_object` (`replay/validation.rs:92-125`), which creates a fresh Budget, i.e. a new
  `/tmp/.brit-checkout-*` TempDir, per blob (`:99`). It then calls `find_stream`
  (`gix-odb/src/store_impls/dynamic/stream.rs:68-200`), which spools the delta base and a full
  reconstruction for every delta level into `blob-*` files and hashes the result; `verify_object`
  hashes it a second time. Bytes per pass ≈ Σ blob size × (delta depth + 1): 2.10 GB × 6.0 = 12.6 GB
  per pass for this clone [M], against about 12.5 GB per pass observed, at roughly 250 s per pass. On
  the monorepo the shem measurement above (29.1 GB over 3.72 GB of blobs) implies an average
  reconstruction depth of about 7.8 [I]. It scales with history and delta depth, not with tracked
  content.
- **Passes before publish: 4 + 2k for k replayed commits** [R]: the `Selection::prepare` history walk
  (`replay/mod.rs:88-89`); `complete_preparation` re-verifying the closure (`replay/journal.rs:395-403`);
  `apply_decision` re-verifying the whole closure on every decision (`journal.rs:2785-2788`: initial
  checkout, each step, final); `finish_proposal` walking the full history again per step
  (`journal.rs:1463`). For k=1, 6 passes × ~260 s ≈ 1,560 s, matching the measured ≈1,578 s [M/I]. A
  three-commit rebase would publish in ≈43 min [I; not run].
- **Worktree witness** [R]: `worktree_witness` → `snapshot` (`authoring/conflict.rs:368-376, 436-476`)
  reads and hashes every file in the worktree, including untracked and ignored files, `target/`,
  `node_modules` and submodule checkouts; only entries named `.git` are skipped. It runs at prepare
  (`journal.rs:205`) and on every `validate_current` (`:491`). Monorepo linked worktree
  `a2o-refused-follow-visible`: 150,742 files, 1.74 GB [M]. Main checkout: 7.9 M files, 374 GB, because
  it contains `.worktrees/` [M].
- **Cleanup** (`journal.rs:1771-1806`) [R] holds the shared mutation guard for the whole call
  (`:1772`). `capture_current` (`:2064-2087`) makes a member for every closure object.
  `retained::capture` computes every member's fingerprint up front (`replay/retained.rs:369-382`), each
  `Source::Object` read building a new Budget and running `find_stream` again (`:96`): one full
  reconstruction pass. `validate_content` hashes every member (`:385-387`): a second pass. The
  per-member loop (`:423-431` → `persist_content :137-168`) does a third reconstruction, then a copy,
  re-read, fingerprint, fsync of the file and fsync of the directory.
- **The quadratic term is `check_growth`** [R + M]: the manifest is written once (`retained.rs:421`).
  Each member calls `switch::payload::check_growth(leaf, size)` (`:145`), which calls `usage()`
  (`switch/linux/payload.rs:139-159`), which `read_dir`s and `lstat`s every file already in the copy:
  ≈n²/2 lstats, about 3×10⁹ by member 77.8k and 2.4×10¹⁰ for all 218k. That explains the sys-time
  dominance and the falling rate (4.3 → 2.7 files/s). The 163 GB of cancelled writes is spool churn
  from the passes plus three reconstructions per member [I, consistent with the byte totals].
- **Why the whole closure is retained** [R]: the spec pins "the complete local object closure".
  `validate_release` must prove from the copy alone that every object reachable from every released
  ref is present, and the released refs include `upstream`, whose history is the whole repository
  (`replay/cleanup.rs:110-160`).

### Hard facts

- **Cleanup and abort can never finish for a closure over 1 GiB decompressed** [R]. `check_growth`
  compares against `SCRATCH_LIMIT = 1 GiB` (`gix-utils/src/stream.rs:13`); the copy was at 1.003 GB with
  about 140k members still to go [I]. `abort` needs the same `capture_current` before it restores
  anything (`journal.rs:1719`). That covers brit itself and the monorepo (3.9 GB).
- **After such a rebase publishes, the repository is unusable for brit** [M]: `rebase --abort` refuses
  ("this replay phase cannot be aborted…", correct since the decision is final); `rebase --continue`
  re-enters cleanup and cannot end; every brit mutation refuses "pending replay" (`switch tip`: "pending
  replay … recover its internal session"). Someone has to clear the journal by hand. The published
  branch itself is fine.
- **A long rebase locks itself out** [I]: journal open calls `check_growth(dir, 0)`
  (`journal.rs:2131`); at about 105 MB per step plus ~190 MB fixed, a brit-repo rebase of roughly 8 or
  more steps would produce a journal that cannot be opened at all.
- **Retained copies are never deleted** [R]: a copy lives at `.git/brit-rebase-retained/<nonce>/<seq>/`;
  no code path deletes it; the only removals are journal and tombstone members (`cleanup.rs:441,450`;
  `journal.rs:367,2441`). Every rebase would leave a permanent full-history copy.
- **The worktree witness hashes every file, ignored ones included** (above).

### Cost model

N = closure objects, Bz = closure blob bytes, d = mean reconstruction depth, k = replayed commits,
W = worktree bytes (ignored files included), V = number of `validate_current` calls.

- **Publish:** (4 + 2k) × [Bz·(d+1) spooled + a hash of every object] + V × hash(W).
- **Journal:** about 171 B per closure object per state image, times (1 + 2 × decisions), plus about
  140 B per object per step [M-derived].
- **Abort and cleanup:** a copy of every object's decompressed bytes, three reconstructions each,
  O(N²) lstats, and failure once the copy exceeds 1 GiB.
- **RSS:** O(N); 624 MB at N = 218k.
- **Monorepo, above its single boundary commit** [M]: 12,839 commits, 185,831 objects, 3.92 GB (blobs
  79,226 = 3.72 GB, trees 189 MB), 21,663 index entries, 5 gitlinks. A two-commit landing would take
  8 passes of about 29 GB and 6 min each: ≈48 min and ≈233 GB spooled, plus 1.74 GB of witness hashing
  per validation [I]; in the main checkout the witness alone is 374 GB per validation. Cleanup and abort
  can never complete.

### `/tmp/.brit-checkout-*` directories

- **Who creates them** [R]: every `Budget::new` (`gix-utils/src/stream.rs:52-64`); 47 call sites, most
  passing `std::env::temp_dir()`; replay creates one per object (`validation.rs:99`, `retained.rs:96`).
- **Why they survive** [R]: a `tempfile::TempDir` is removed only on drop. On a second SIGINT or
  SIGTERM, `gix/src/interrupt.rs:126-128` cleans only the `gix_tempfile` registry and then dies with the
  default handler; SIGKILL and OOM also skip drops.
- **Observed** [M]: a single SIGTERM unwound cleanly with no leak, but a SIGTERM'd 60 s `--continue`
  left one directory with 4 spool files. There were 13 leftovers on the host, 12 in triplets each
  holding one 15-byte spool, held by no live process, not attributed.
- **Cleanup rule proposed:** one Budget per operation, not per object; put it under
  `<git-dir>/brit-scratch/<owner>` with a recorded pid and start time; register it with the signal
  cleanup; have the next brit invocation sweep directories whose owner process is gone. Keep it under
  the git dir: on this host `/tmp` (overlay) and `/projects` (md127) are different filesystems [M], so
  rename-into-place cannot be used, the IO guard does not see `/tmp`, and a restart wipes `/tmp`.

### Refusals that can move ahead of any spooling [R]

Today `prepare_mode` runs `Selection::prepare` (one full pass, `journal.rs:182`), then preservation
capture (`:196`). Ahead of all spooling today: shallow repository (`replay/mod.rs:71/76`), signing and
merge drivers (`:68`). After at least one full pass, all of these could run first from the index and
two trees:

- multiple merge bases (`mod.rs:93`);
- merge-bearing, root or non-linear range (`:101` → `:194,:204`);
- moved gitlink, through footprint (`:112-117` → `:321-327`);
- external filters (`:116`);
- refStorage (`journal.rs:186-191`);
- dirty or out-of-scope footprint (`:202`);
- untracked collision (`:212-222`);
- branch occupied by another worktree (`:242-255`);
- in preservation capture: present gitlink with a directory (`authoring/conflict.rs:86-91` →
  `index/authoring.rs:69-70`; this is the 6-minute refusal measured on shem), unmerged index (`:74-77`),
  split index (`:81`).

Reorder `Selection::prepare` so it resolves both tips, computes the range and base, applies the chain
rule, then runs every check above; only then verify anything, and only what is actually read.

### Stranding fix (report §7) [R; stranding confirmed by running earlier]

- **(a) Dispatch.** `replay/command.rs:87-90` sends `--abort` on a `preparing` journal to
  `recover_preparation`, which calls `complete_preparation` (`journal.rs:2202-2203`), rolling forward
  through the verify that already failed, and even on success calls the full `abort()`, which builds a
  full retained copy. `abort_prepared` (`journal.rs:2253-2334`) verifies no objects, but only test code
  calls it (`switch/durability.rs:150`). Fix: route phases `preparing`, `prepared` and
  `cancelling-prepared` straight to `abort_prepared`, and drop the `recover_preparation` call.
- **(b) Gitlink ids treated as local objects.** `Preservation::original_object_ids`
  (`authoring/conflict.rs:144-149`) must skip mode-COMMIT entries (`Entry.mode`, `:25`); it feeds
  `complete_preparation` (`journal.rs:387-403`) and `capture_current` (`:2064-2071`).
  `capture_current`'s observed-index ids (`journal.rs:2072`) need the same filter. `capture_inner`
  (`conflict.rs:86-110`) must skip the worktree read for COMMIT entries, which fixes the
  present-gitlink refusal.
- **(c) The rule: preparation must be cancellable without its own success.** (1) Nothing live is
  changed before the phase leaves preparation (already holds). (2) Every artifact preparation creates
  is named in `state.json` before it is created (`payload-<hash>` via `payloads`, retention refs from
  the nonce and labels). (3) Cancellation reads only those named artifacts plus the pinned HEAD, index
  and branch fingerprints; it never verifies or walks objects, never re-captures preservation, never
  builds a copy, and is exempt from the journal-open quota (`journal.rs:2131`). (4) If the live state
  differs from the pinned fingerprints, still remove brit's own artifacts and report the difference.
- **(d) Test:** inject a failure at every preparation checkpoint (`replay-before-journal` …
  `replay-retained`) and with an uninitialised gitlink; assert that `--abort` exits 0 and leaves no
  journal, no retention refs and no scratch leftovers.

### Redesign (report §6)

Bounded fixes (one verified-id cache across passes, one Budget per operation, an incremental quota
counter instead of directory scans, a compact journal encoding) would cut constants by perhaps 10–50×,
but every pass stays O(history) and the copy stays O(history), so the 1 GiB ceiling still blocks
cleanup and abort. The proposed model: retained work scales with what the operation can lose — the
replayed range, the objects it introduces, and the paths it touches.

1. **Selection, read-only and bounded:** range and base with the painting merge-base and an `O ^U`
   rev-walk, cost proportional to the range (`merge_base` took 435 µs in the trace [M]); the chain rule
   (base p) and every index- and tree-only refusal here.
2. **Shallow:** if the bounded walks never read a boundary commit's parents, the result is exactly the
   full-history result; if a walk does, refuse and name the commit.
3. **Verify only what is read or written:** chain commits, the trees along changed paths for each
   commit and its parent, changed blobs, and U's trees along the footprint.
4. **Retention refs** for the original tip and the steps; the upstream ref stays protected by the
   user's own ref.
5. **Retained copy:** footprint index and worktree images, objects introduced by c₁..c_k over p,
   objects from abandoned steps, and p's trees along the footprint. The manifest records
   `floor = {p}`; `validate_release` stops at floor commits and treats a tree entry whose id equals the
   parent's same-path entry as inherited, staying copy-only and becoming O(introduced bytes).
6. **Journal:** the chain, footprint, floor and per-step introduced ids, with no closure list and a
   binary or base64 payload.
7. **Witness:** footprint paths and their ancestors only.
8. **The copy is durable data,** outside the 1 GiB scratch quota.

### From the earlier design report

Carried over, per the cost report: gitlinks treated as index-only (kept out of object sets and
witness); the refusal when both sides moved a submodule pointer to different commits; the manifest
floor field, generalised to the chain base for every rebase. The rest of the earlier design is
superseded, including the journal's `shallow_floor` and the boundary-change rules (walks never go below
p; the only remaining check is to refuse if a chain commit becomes shallow).

### Corrections

- The coordinator's first guess, a manifest rewritten per object, was wrong: the quadratic term is
  `check_growth`'s directory scan per member.
- This entry's earlier statement that the retained copy's shallow format was the main missing piece no
  longer holds: the monorepo's shallow boundary sits 12.8k commits deep, and shallow support does not
  help cost.

### Order of work (decided 2026-11-19, after the cost report)

1. The stranding fixes (§7 a–d above), plus a preflight that refuses before any spooling when
   completion is impossible.
2. The redesign (above).

Rebase stays a declared git fallback on both devices.

### Open decision for the steward

In the redesign, the retained-manifest format change (v2 with a floor) would apply to EVERY new copy,
not only shallow ones as scoped earlier (v2 only when the floor is non-empty). An older brit would
then fail closed on any copy a newer one wrote. This reverses the approved scope and needs the
steward's decision again.
