---
id: "backlog-brit-rebase-cost-gitlinks-and-stranding"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "brit rebase spools gigabytes for a two-commit replay, refuses gitlinks, and can strand a repository in 'preparing'"
slug: "brit-rebase-cost-gitlinks-and-stranding"
written: "2026-11-19"
author: "claude-opus-5-5 (brit sprint 3, session 0e0924d5, steward of record human:matthew)"
status: "wip"
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

## Recovery lane, 2026-11-19 (four rounds reviewed; merged locally, not pushed; round 5 running)

Lane `sprint3/recovery` (worktree `brit-s3-recovery`). Commits up to `0d23297e22`:

- `c8a824dd`: `--abort` cancels a failed preparation without rolling it forward. An eleven-checkpoint
  journey with INT and KILL. It also found a stranding the scout had not listed: KILL at
  `replay-journal-ready` left an empty `brit-rebase/` on which `--abort` failed with ENOENT.
- `0dda8356`: gitlink ids are never looked up as local objects. The absent-directory case is now an
  early named refusal; no refusal was removed.
- `041285ad`: a preflight refusing before any journal or spool when the retained copy cannot fit (1.0 s
  on the scout's 218,090-object clone, against about 250 s and 12.5 GB for one verification pass).
- `d6f06f8d`: a switch stopped before its first record is harmless and aborts.
- Journals written by 0.2.6 in the gitlink-stranded state are cancelled without loss. Formats
  unchanged; gix-* untouched.

**Independent review of that range.**

- The abort, gitlink and switch parts are SOUND: nothing outside brit's own artifacts was removed under
  symlink, foreign-member, crafted-nonce and concurrency attacks, and the invariant "no live write
  before a durable non-preparation phase" holds on every path by reading.
- The preflight is a MUST-FIX. For a file of S bytes changed by a replayed commit, the copy needs about
  7·S + 200 KB while the bound counts about 2·S + 50 KB, so it admits rebases from about 29% of the
  limit upward that then publish and stick at cleanup (at the real 1 GiB limit: one replayed commit
  touching roughly 120–480 MB of files). A worse variant, with the bound near 95% of the limit, exceeds
  the JOURNAL quota inside the first checkout decision (phase `pre-approved`, `decision-1.json`
  durable, worktree already changed), and both `--continue` and `--abort` fail.
- Found by the author and independently by the reviewer: a `prepared` journal holding only
  `index.pre-rebase` and an unstarted pre-rebase invocation falls to the full abort and strands over
  the quota.

**Decisions taken (coordinator, 2026-11-19).**

- Admission must be conservative and cover both quotas, proven by a boundary test over shapes.
- Recovery commands are never blocked by the quota.
- No state may exist in which every command refuses. A retire command (working name
  `--retire-cleanup` / `--retire`) moves the journal aside and keeps everything, for
  `completed`-with-cleanup-pending, `retaining-abort`, and a mid-transition journal (where it names the
  retained original-HEAD ref and the commands to return to it, and changes nothing live).
- Only the pure reorders D1 (refStorage and branch-occupied first), D2 (merge bases before the walk)
  and D5 (index-only pre-check for unmerged, split and both gitlink cases) are in this lane; D3, D4
  and D6 belong to the redesign.

**Round 2 reviewed: BROKEN.** Round 2 (commits `9f3e6373`, `073cbfb0`, `5b107d72`: three admission
gates, `--retire`, unlaunched-hook cancellation) was reviewed and broken in three ways, all confirmed
by running:

- **(M1) A conflict resolution strands.** `--stage` is not gated, and `--abort` itself passed through
  the forward reserve (`accept_discard_child`). With a 1 MB limit, a 100 KB resolution made
  `--continue`, `--abort` and `--retire` all refuse; at 200 KB `--retire` was accepted but left
  `.git/brit-authoring` behind, with every mutation refusing.
- **(M2) `--retire` in a COPIED repository deleted the ORIGINAL repository's git lock files.**
  `release_recorded_locks` removed recorded absolute paths by device and inode without checking that
  they lie under this repository.
- **(M3) `--retire` was accepted, and `--status` steered to it, where `--abort` would have restored
  everything.** Its printed advice `git checkout --force <branch>` then discarded the user's
  uncommitted work.

Sound in round 2: the admission gates' refusals leave nothing behind; the unlaunched-hook cancellation;
opening a journal without the quota.

Consequence stated by the reviewer and accepted: the conservative preflight charges about 5,900 bytes
per closure object, so any repository over roughly 182,000 objects, or with a closure over about
0.86 GB decompressed, is refused for every rebase in under a second, brit's own repository and the
monorepo included: "safe there by being unavailable".

**Decision (coordinator): simplify, do not patch.** Round 3, by a fresh implementer, under five rules:

- R1: admission refuses before anything changes.
- R2: a forward step may be refused before it writes anything, naming `--abort`.
- R3: `--abort` and cleanup are NEVER refused by any quota in any phase.
- R4: `--retire` only for this repository's own journal in a terminal or retaining phase, removing no
  lock files and printing no checkout advice.
- R5: a journal brit cannot verify as its own is reported truthfully and never touched.

Fallback if round 3 does not pass review in time: land only round 1's reviewed parts (abort of a failed
preparation, gitlink ids, the killed switch).

Rebase stays a declared git fallback for the monorepo and for brit either way.

**Round 3 reviewed: R1–R5 HOLD.** Round 3 (commits `b4183e93b9`, `e47ac0bbcd`, `a9fd9cc80a`)
implemented the five rules and was reviewed a third time:

- a conflict resolution of any size under a 1 MB limit is refused at `--stage` or `--continue`, and
  `--abort` restores exactly;
- a ten-checkpoint kill sweep restored HEAD, index, worktree and the user's uncommitted work exactly in
  every case but one;
- `--retire` refuses every live state and no longer touches another repository's locks;
- no false "foreign journal" for a linked worktree or a symlinked path;
- a 3-commit docs replay on a 1,171-object repository at the real limit is admitted and finishes in
  22.5 s.

Two strandings remain. Both are crash windows, neither is about quotas, both are at the real limit,
and by reading neither is changed by this lane (so probably present in 0.2.6):

1. A kill during conflict `--continue` after a complete `state.pending` is written and before its
   rename. Afterwards `--abort`, `--continue` and `--skip` fail "foreign pending replay state", because
   a retry builds a different successor and the equal-bytes rule for an existing pending record never
   matches.
2. A torn retained-copy manifest. `manifest.json` is not written atomically, and `persist_exact`
   refuses a differing manifest even in an incomplete copy, so a retry of `--abort` fails (outcome
   confirmed with a simulated torn file).

Also: `brit switch` recovery is still metered. A part-way switch whose journal is over the limit fails
`--status`, `--abort` and `--continue` (reached only by changing the limit between runs).

Admission's smallest admitted limit is now about 1.9–3.2 times the real copy. Large repositories (over
roughly 182,000 objects) are still refused in under a second, with wording that says brit rebase is not
available at that size yet.

Below, the first of these is M1 and the second M2. The third review also asked for unmetered
`brit switch` recovery and three status wordings.

**Round 4: done and reviewed; no must-fix on SIGKILL paths.** A fresh implementer, seven commits
`a9fd9cc80ad061c3f7868e91a335bc339dbe0975..c63ef6bfa4fa520364ebf4f1d8318aa307db68d9`:

- `1d24e1a87e83539893d629b7ce3a70900e9f8276` (M2): the manifest, `COPY_COMPLETE` and the cleanup
  receipt are published whole (spool, rename, sync). A manifest that is not a whole checksummed record
  in an incomplete copy is replaced; a whole differing manifest still refuses.
- `1b6f2f94f260b60f155630ba4aaea0a2a6ce72da`: switch `--status`/`--abort`, pull `--abort` and rolling a
  durable decision forward are unmetered; fault builds accept `BRIT_SWITCH_CHECKPOINT=name#N`.
- `0f30fb0c672d3579f794a5e3c7f0a9f334f84227` (M1): `Session::settle_pending` discards a pending record
  beside a valid `state.json` at the entry of every rebase command except `--status` (a
  preparation-phase successor excepted); torn decision records are discarded; a torn `plan.json` beside
  a whole owner record is rewritten; an abandoned never-decided plan's recorded locks are released.
  The last fixes an `--abort` killed at `lock-acquired` that refused on every retry, a stranding older
  than the lane. Also the status wordings.
- `6b3ade669030e6974de9222f4d783e5dad6a0f09`: journeys, including a sweep that kills four commands at
  every journal save at three points each.
- `8ef125b05e45186d2717f7d120e3e3333d4e3817`: docs.
- `1f7036bf3a6caa2d50ec79f4613a1c135580c9a5` and `c63ef6bfa4`: the author's own corrections after its
  core suites went red on two intermediate changes.

Reported by the author: every verification leg EXIT 0 on `c63ef6bfa4` (rebase journeys 46 with faults,
switch 28, pull 44), the same counts as uid 65534.

Fourth independent review (the reviewer of rounds 1–3; its scripts and outputs were session scratch
and are not kept, so the results are recorded here). Verdict: merge with the residuals documented.
Confirmed by running:

- **Lock release holds.** `release_undecided_locks` (`transition.rs:58`) goes through `reclaim_locks`,
  which checks per lock that the receipt path equals the resource's lock, that anchor and external lock
  have the same device, inode and checksum, and that the receipt is private and owned. An `--abort` was
  killed in repository A at `lock-acquired`, and separately at `lock-intent`; every rebase command was
  then run in a copy B and in a moved copy C. A's lock files, journal and refs were byte-identical
  afterwards; B and C refused everything except `--status`; A then aborted and restored exactly. With
  the owner alive and paused at `lock-acquired`, `--abort`, `--continue` and `--retire` refused naming
  the holder and left its locks untouched. (Lock deletion was what removed another repository's locks
  in round 2.)
- **M1 fixed.** After a kill at `replay-state-file-synced` in a conflict `--continue` or `--skip`,
  `--status` names the leftover and `--abort` restores exactly. With kills at saves #1–#4 of a conflict
  `--continue` and of the first run, `--abort` restores HEAD, index, worktree and the user's dirty file
  exactly, leaving no journal and no `brit-authoring`; from an identical crash `--continue` finishes
  with the right history (or, on the first run, stops at the expected conflict).
- **M2 fixed.** Killed at `replay-retained-member`, or at `replay-retained-manifest` with the manifest
  then truncated, the retried `--abort` restores exactly.
- **Switch quota fixed.** Under a 1,000-byte limit `switch --status` and `--abort` work and restore
  exactly; killed at `index-published`, `--continue` finishes. The changed `conflict_authoring` test
  is correctly retargeted (the oversized receipt is still refused, for its length).
- **No regressions** in the round-1 preparation-cancel attacks or the moved/copied-repository
  fixtures; the round-1 band case is now refused before anything changes.

**Residuals.** The reviewer produced each by killing at the nearest checkpoint and cutting the record
to half its length. A SIGKILL cannot realistically tear these small fsynced records; a crash or power
loss before the fsync can. None is new in this lane.

| Record | How produced | Outcome |
|---|---|---|
| `metadata-lock-0.json` in an abort transition | abort killed at `lock-acquired` | stranded: `--abort`, `--continue`, `--skip` and `--retire` all refuse; `--status` says "--abort finishes restoring", which is wrong |
| `metadata-lock-0.json` in the forward first checkout | rebase killed at `lock-acquired` | stranded: all four refuse; `--status` says "--continue carries on; --abort restores", wrong |
| `retention-0.json` | killed at `replay-intermediate-retained` | stranded: all four refuse; `--status` wrong the same way |
| `index.pre-rebase` | killed at `replay-decision-durable` | cleared by `--abort`, exact restore |
| `proposal-0.json` | killed at `replay-retention-recipe` | cleared: live state restored exactly, journal then retired |
| cleanup receipt (285 KB) | clean rebase killed at `replay-cleanup-receipt-durable` | stranded: the rebase is published; `--continue` refuses and every mutation is blocked. The receipt is now published whole, so this torn form comes only from an older brit or a power loss |

- **Should-fix 1.** After an `--abort` killed mid-copy, a stock `git add` of an unrelated dirty file
  followed by `git reset -- <file>` does not restore the index byte for byte. From then on `--abort`
  refuses "foreign replay metadata before abort", `--retire` refuses, and `--status` names no cure (the
  manual cure would be copying the retained copy's observed index back; nothing says so). At an
  ordinary conflict stop the same reset does let `--abort` work.
- **Should-fix 2.** `--status` gives the wrong cure in every stranded row above, and at a conflict stop
  where `git add` was run and not undone.
- **The author's own not-fixed list:** records still written in one non-atomic pass whose torn form
  makes a retry refuse: `metadata-lock-N.json` and `directory-*.json` (a lock receipt with a git lock
  held), `proposal-N.json`, `retention-*.json`, `index.pre-rebase`, `payload-license-*.json`; a
  leftover `*.new` of the authoring child, not checked; a torn cleanup receipt left by an older brit or
  a power loss still blocks every mutation; phase `published-foreign-state` prints no cure. The
  review's table tested five of these records: torn `index.pre-rebase` and `proposal-0.json` were
  cleared by `--abort`, not refused as the list says.
- **Not established by the review:** whether every `save` caller keeps nothing between "pending
  synced" and "renamed" (since settled by the integrator's audit, Phase 4g below); torn `directory-*.json`, `payload-license-*.json` and the authoring child's `*.new`
  (not produced); kills during an edit-stop `--amend`.
- **Earlier residuals, unchanged:** a process killed during any journal `save()` can leave
  `.git/.brit-checkout-*`; a `preparing` journal in a moved or copied repository cannot be cancelled
  (identity check; pre-existing).

**Decision (coordinator, after the fourth review): merge.** This superseded the round-1-only fallback.

**Phase 4g: merged locally, NOT pushed, and not to be pushed as it stands.** `sprint3/integrate` is
locally at `948a939f16e416d6357448e49eb2a4ac318e2f9d`: merge `b1010c4540` of `c63ef6bfa4` into
`080f305797`, then `b0770fdead` (docs/rebase.md section "States this version can still strand" and its
changelog entry) and `948a939f16` (the lane's changelog entries); commit details in the handoff entry,
"Where the work is". `origin run-ci/sprint3-integrate` is still `080f305797`.

- **The blocker** [M], found by the integration pass and by none of the four reviews, because none
  ran CI's memory step (ci.yml:480): `tests/cli-journey/tests/memory_replay.rs`, unchanged since the
  lane's base on both sides, gives 8 passed / 1 ignored on `080f305797` and 2 passed / 6 failed on the
  lane's own round-4 binary and on the merged head alike. The lane's gates refuse rebases that
  completed before:
  - four `sixty_four_mib_*` tests are refused by the preflight (`copy_bound.rs`): "brit rebase is not
    available on repositories of this size yet; its journal and retained copy can need up to
    1259171527 bytes … over the 1073741824-byte scratch limit". 1,259,171,527 is the modelled sum,
    about 1,007,337,221, times the 5/4 safety factor;
  - `sixty_four_mib_edit_amend_preserves_authored_object_custody` is refused by `Session::admit`
    ("with its exact records known … the 5/4 safety factor … exceeds");
  - `twenty_mib_edit_scope_expansion_preserves_original_and_authored_bytes` is refused at
    `--continue` by `Session::reserve`, with 797,595,909 bytes requested for the step.
- **Everything else in the phase verified** [M]: full cli-journey 495 passed / 6 failed
  (`memory_replay` only) / 73 ignored; journey faults 145 / 0; core faults 181 / 0 / 1 ignored;
  max-pure journeys 374 / 0 / 6 ignored; all clippy and doc legs. The test seams are absent from the
  default and max-pure binaries (0 occurrences of the variable names by `strings`; 12 with/without
  comparisons identical). The reviewer's round-4 fixtures re-run against the MERGED binaries give the
  same outcome lines. `rebase --status` writes nothing (measured), and is rightly not on the read-only
  list because it shares the path `rebase` with every writer, so `brit rebase --status | head` can still
  print "Broken pipe" and exit 1. The save-caller audit the review asked for found no write between
  "pending synced" and "renamed" (`publish_state` has exactly two callers).
- **Two discrepancies against the lane's stated rules** [R]: `--retire` and `rebase <upstream>` are
  dispatched before `settle_pending`; `--retire` also accepts phase `retaining-skip` with zero
  decisions, which the docs do not mention.
- **Not run for this head:** the non-root leg and the bash journeys.

**Decision (coordinator): tighten the gates, do not loosen them.** The gates stay sound upper bounds:
an admitted rebase must not meet the metered limit part-way. The lost capability comes back by
tightening them: the 5/4 multiplier is replaced with additive allowances that scale with decisions and
objects, and double counts shown by measurement are removed. Raising the limit and making the gates
advisory are both rejected.

**Round 5, started 19:29Z** in the integration worktree on top of `948a939f16`, with that brief and
`memory_replay.rs` untouched as its acceptance test. It stops and reports if a fixture still cannot be
admitted. It needs its own independent review, then the non-root leg, before push 10.

**Round 5 stopped at about 20:13Z as instructed, with the measurement** (head `a3b3ac71d5`: one docs commit,
code unchanged; `memory_replay` still 2 passed / 6 failed at the delivery limit, 8 passed with a wide
test limit). Measured on the merged faults build with a temporary probe at every gate and every
metered write (reported by the implementer; probe and tables in session scratch `recovery5/`, not
durable):
- None of the six fixtures writes a METERED retained copy. Every copy is an abort's or a cleanup's,
  which R3 leaves unmetered.
- After the tightening this brief allowed, four of the six are still refused, and in every case the
  refusing figure is the reservation for that unmetered copy. The copy model runs 1.6 to 2.4 times the
  measured copy: `sixty_four_mib_replay` admit-copy figure 1,594,532,078 as built and 1,208,516,799
  at best, against a measured copy of 671,418,512; `…_preserved_staged_and_dirty_neighbor`
  1,762,298,616 as built and 1,342,730,029 at best against 805,634,522. Limit 1,073,741,824.
- The journal figures fit with the 5/4 factor kept: at most 755,597,307 as built (604,477,846
  unscaled) against measured journals of 268,600,885 to 336,510,137.
- The `--continue` refusals at 797,656,311 and 2,617,968,476 bytes "requested" measured the state held
  in memory (a 20 MiB file inline, JSON-encoded, times three), not the record that is saved: the
  after-image is replaced by its fingerprint before the save, and no record on disk exceeded about
  85 MB. Reserving on the projected record gives 42,686,414 for the 20 MiB fixture (measured).
- The 5/4 factor is covering a real under-estimate: with `merge.conflictStyle=diff3` a conflicted
  payload was 7,680,032 bytes against a planned step of 5,220,000, because the conflict also writes
  the base version. Removing the factor would open the admitted-then-refused band for diff3 conflicts.
- One proven double count: the copy projection adds the initial checkout's upstream images twice.

**Decision (coordinator, about 20:15Z), superseding "tighten only" above:** the scratch limit meters the
journal. R3 already took an abort's and a cleanup's retained copy out from under it, so reserving a
projected copy under that limit refuses work that fits and protects nothing: an unmetered copy cannot
be refused part-way.
1. Journal gates stay exactly as built — sound upper bounds, WITH the 5/4 factor — at preflight,
   `admit` and `reserve`.
2. An unmetered retained copy is no longer projected. Its gates refuse only what is certain: at the
   preflight, when the closure's object bytes alone exceed the limit (this keeps the one-second refusal
   of a repository whose copy could never have fitted); at `admit` and `reserve`, when the copy an
   abort would write now already exceeds it. No factor, no projection.
3. A forward step that itself writes a metered copy (`--skip`) keeps its scaled reservation.
4. The reserve-on-the-saved-record fix is applied.
What this gives up, and the steward should know before brit `main`: an abort's or a cleanup's
retained copy can exceed the 1 GiB scratch limit (bounded by the closure, plus the journal the limit
bounds, plus the worktree and staged states it copies). That is the consequence of never refusing an
abort. Rejected alternatives: dropping the copy gate entirely (a small rebase on a very large
repository would then be admitted, run for many minutes and write a multi-gigabyte copy, where today
it is refused in a second); a tight copy model (needs blob reads at admission and left about 134 MB of
margin in the measured worst case); a larger separate limit for copies (a limit raise standing in for
a model).
The same implementer was resumed at about 20:15Z with this decision. Owed before any push: `memory_replay`
8 of 8 with the file untouched; journal soundness against measured peaks; a boundary journey and a
band search across limits, including a diff3 conflict; an abort whose copy exceeds the limit restoring
exactly; the reviewer's fixtures re-run; then a FRESH adversarial review, the non-root leg and full
verification.

**Implemented, 22:36Z (reported by the implementer; not yet independently reviewed).** Local head
`b122f0515d00062af9e025bc78f17a96fdf6eae2` on `sprint3/integrate`, not pushed: `c4cb4ca6` (code and
journeys), `cc9d7d94` (docs), `692a31a7` (journey margin), `b122f051` (the certain test at admission
counts the copy's member bytes; `quota_usage`'s figure had included an upper bound of the manifest and
exceeded a measured copy, 1,037,757 against 870,356).
- `memory_replay` at the delivery limit: 8 passed, 1 ignored; the file is byte-identical to
  `080f305797`'s. Full cli-journey on the default build 501 passed / 0 failed / 73 ignored; faults
  journeys 148; core faults 181 / 1 ignored; power_loss 4; replay unit tests 18; clippy for default,
  faults and max-pure; docs with `-D warnings`; fmt: every leg EXIT 0.
- Measured at the delivery limit, 64 MiB fixtures: scaled journal figures 420,137,516 to 839,723,917
  against measured journal peaks 268,604,247 to 336,519,779; certain-copy figures 134,218,923 to
  604,002,427 against measured copies 671,420,038 to 806,141,095. On 54 small rebases every admitted
  one had a scaled journal figure at least its measured peak (ratio 1.16 to 22.1), and the certain
  figure was at most 0.869 of the smallest copy written.
- Boundary journeys on the faults build: a clean replay is admitted from 3,356,531 bytes and
  completes; one byte less is refused before anything changes; a diff3 conflict is admitted from
  18,256,470. A rebase whose abort copy is larger than the limit is admitted, stops at its conflict,
  and `--abort` restores HEAD, index, worktree and the user's edit exactly.
- **Finding and second decision (coordinator, about 22:36Z).** As first built, `reserve` also refused
  a forward step when the abort's copy already exceeded the limit. The implementer's band search
  showed that this refuses PLANNED steps after admission on the diff3 fixture: the conflict stop at
  limits 18,256,470 and 18,541,728, and `--stage` from about 19.4M to 27,099,448 (each before writing,
  `--abort` restoring exactly). Decided: that check comes out of `reserve`. It stays at the preflight
  and at admission; `--skip`'s metered copy keeps its scaled reservation. After admission only the
  metered journal, or a metered skip copy, may refuse. The implementer is making that one change and
  turning the diff3 journey strict (no refusal after admission at any searched limit).
- Stated in `docs/rebase.md` and the changelog: the copy can exceed the limit by at most the closure
  plus the journal plus two index copies plus, for each footprint path, its before-image, worktree
  file and up to three staged versions, plus the manifest; measured 0.67 to 0.81 GB on the 64 MiB
  fixtures, 0.94 GB with a stop and a resolution. New refusal thresholds: history over 1 GiB, or about
  390,000 objects for a one-commit replay.
- A fresh adversarial reviewer started at 22:37Z on `a3b3ac71d5..b122f0515d`, first question:
  can a copy larger than the limit break an abort or a cleanup, or meet any other size cap. Still owed
  after it: the non-root leg, full verification on both builds, push 10 and CI.

**Final state of that round, 23:05Z.** Local head `99227182ad54991983d50d86f0ab11d799386f73` ("after
admission an unmetered copy never refuses a step"): the copy check is out of `reserve_for`; the diff3
boundary journey is strict (26 limits from 15,974,412 to 36,512,940, no refusal after admission);
every verification leg EXIT 0 again (faults journeys 148, boundary 4, core faults 181 / 1 ignored,
power_loss 4, replay unit 18, memory_replay 8 / 1 ignored, brit-cli 110 / 5 ignored, cli-journey
501 / 0 / 73 ignored, gitoxide-core 202 / 5 ignored, clippy, docs, fmt). Reported by the implementer.

**Fifth review (fresh reviewer, 22:37Z to 23:08Z, on `b122f0515d` and then on `99227182ad`): two
must-fix, would not push.** Scripts and outputs in session scratch `review-recovery6/` (not durable).
- Confirmed by running: a retained copy larger than the limit does NOT break recovery. At a limit of
  9,832,107 the conflict stop's abort copy is 13.3 MB and the cleanup copy 10.76 MB; `--abort`
  restores HEAD, index, every worktree file, the dirty tracked edit, the untracked file and the `.git`
  listing exactly; an abort killed at six retained-copy and cleanup checkpoints restores exactly on
  retry with `--status` naming a cure each time; a completed rebase's over-limit cleanup copy
  completes. Read: no other size cap in the copy, manifest, receipt or cleanup code (members are
  unmetered; the hard 1 GiB `Budget` in gix-utils is met only by per-record spools and per-object
  streams).
- Confirmed: the certain-copy tests are true lower bounds and refuse before anything changes; both
  `--skip` paths reserve with the factor and no metered copy write failed; the reserve-on-the-saved-
  record fix never under-reserved in the flows tried; no regression in the earlier reviewer's
  fixtures; `memory_replay.rs` byte-identical.
- **F1 (must-fix).** `rebase --interactive`, a pick turned into `edit`, a same-size change to a 4 MiB
  file, `--stage`, `--amend --no-edit`, `--continue`: admitted from 27,081,047 bytes, completes only
  from 32,666,659. In between it is refused at the child-acceptance step or at the final publication,
  each before writing, `--abort` restoring exactly — but the user's amend then survives only in the
  retained copy. Scaled to 1 GiB: an edited file of roughly 138 to 166 MB.
- **F2 (must-fix).** A conflict under `merge.conflictStyle=diff3` or `zdiff3` where both sides shrink
  a 1.6 MB base to 200 lines: admitted from 1,710,547, completes only from 4,356,033; refused at the
  conflict stop itself, or (from 2.3M) at the step after it. The module doc's claim that the 5/4
  factor covers the diff3 base is false. Under the default style the same shape completes.
- Not established by it: the 64 MiB figures; the delivery-only 1 GiB `Budget` cap; a 60-commit replay.

**Push 10, 23:10Z.** `99227182ad` was pushed to `origin run-ci/sprint3-integrate` WITH those two
findings open, for the lane's first cross-platform CI run (it had never been compiled on Windows or
macOS). It is not a landing candidate. Reach gate: `epr ready` READY; rust-style PASS (15 files, 1
crate, baseline `080f305797`).

**Round 6 design (fresh implementer, design note 23:25Z; reported, measured with the probe build).**
- Admission cannot see any stop: `admit` runs on an all-`pick` todo and the sequence editor runs
  afterwards, so nothing can be reserved at admission for an `edit`.
- F1's cause, corrected: at `--amend` the authoring child writes a receipt for EVERY object reachable
  from the stop's whole tree and its indexes, not only the edited file, and that child journal is
  archived into the parent's metered journal. An edit changing a 14-byte file beside an unrelated
  4 MiB file archived the unrelated blob: admitted from 4.22M, completes only from 6.35M. So the cost
  of one edit-and-amend is proportional to the tree. The real cure — leave out of the child's walk
  the objects the journal's closure already holds (`authoring/conflict.rs` near :2101) — is a custody
  change and a follow-up, not part of round 6. Until then a repository whose tree approaches the
  limit cannot edit-and-amend under brit at all.
- F2's cause, measured: the diff3 conflict file was 1,657,432 bytes (all three versions plus markers)
  against a planned step of 17,400; it is stored as the stop's payload and staged again at
  publication.

**Decision for round 6 (coordinator, 23:27Z).** One rule for every stop: a stop is entered only if its
own completion at unchanged size is reserved — the stop's records and payloads, a result no larger
than what the stop presents, and the publication. If that cannot be reserved, the rebase refuses when
the stop is found, before asking anything of the user. After a stop is entered only bytes the user
adds beyond that can be refused, and that refusal says where the work is kept.
- Edit stops: a todo gate at todo acceptance reserves the sum for every planned edit (refusing a todo
  before anything changes, or before it replaces the old one); the reservation is made again, exactly,
  when an edit stop is entered; every other step reserve carries the remaining edits.
- Conflict stops: reserved from the ACTUAL conflict files when the conflict is found — the stop, a
  resolution of the same size, the child's records and the publication. This covers diff3 bases,
  rename destinations and marker-heavy files alike.
- Declined: pricing the merge base at admission under diff3/zdiff3. It is sound, but it would refuse
  CLEAN merges of large both-changed files for every diff3 user (the 64 MiB neighbour fixture would be
  refused at 1 GiB under zdiff3), and no tighter sound bound exists without merging at admission.
- Known and to be stated in the docs: admission plans for one ordinary conflict; a later or larger
  conflict can still be refused when it is found, and resolutions already accepted are then kept only
  in the retained copy.
Phase 2 (implementation) starts when the final-verification agent releases the integration worktree.


**Queued follow-up round, not started.** Publish `metadata-lock-N.json`, `retention-N.json` and the
other single-pass records whole (spool, rename, sync), or treat an unparseable record as not written;
make `--status` say a record is unreadable instead of naming a cure that refuses; give the
interrupted-abort-plus-foreign-index-write state a named cure; decide what a torn cleanup receipt from
an older brit may be allowed to do. It comes after round 5 and needs its own independent review
before it merges.

Rebase stays a declared git fallback for the monorepo and for brit: the preflight refuses repositories
over roughly 182,000 objects, and the redesign above is not started.
