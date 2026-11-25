---
title: "Register of friction met using brit as daily driver, 2026-11-25"
id: brit-friction-register
status: draft
class: process-meta
date: 2026-11-25
context-tier: disclosed
steward: human:matthew
author: "Claude (Sonnet 5.5), read-only evidence for integrator shem; counts by script over the raw reasons file, states from installed brit 0.3.4 help"
serves: []
tags: [brit, friction, evidence, daily-driver]
graduation-trigger: >-
  The brit team ranks its next sprint from this register, or a later register built from fresher
  transcripts replaces it.
cites: []
---

# Register of friction met using brit as daily driver, 2026-11-25

## What this is, how it was gathered, its limits

This is a register of the reasons agents on one device (shem) gave for running Git where the
repository's rule asks for brit, so the team that owns brit can plan from measured facts. It
proposes nothing as decided. Section 6 lists candidates for the brit team to rank.

How it was gathered:

1. The raw file `genesis/local-dev/f4-ask-20261123/brit-fallbacks-raw.txt` holds 918 distinct lines
   of the form `count  # Brit fallback: <reason>`, the stated reasons extracted from session
   transcripts, 3111 occurrences in all. Three lines begin `# Git fallback:` (6 occurrences) and
   are counted with the rest.
2. 90 lines (708 occurrences) were dropped as the instruction's own template text or dispatch
   briefs that quote it: the placeholder `<specific reason>` / `<reason>` (371 occurrences on one
   line alone), hook messages ("or a hook blocks it", "hook-enforced"), and the "Known gaps"
   boilerplate in agent briefs. 828 lines (2403 occurrences) remain and are analysed here.
3. The remaining lines were sorted into classes by a keyword script, first match wins (order:
   cherry-pick, rebase, notes, ls-remote, submodule, landing push, merge, commit, fetch, log,
   patch, reset, config, worktree, read forms, other). It is a rough sort: a line naming two verbs
   lands in the first class that matches, and 22 occurrences matched no class ("other").
4. For each class the installed brit was asked: `env -u EPR_BIN brit --version` printed
   `brit 0.3.4` (source HEAD e92dd41e6, "source state: unverified"), and `brit --help` and
   `brit <command> --help` were read for each command a reason names. No mutating command was run.
5. brit's own planning pool was read at `origin/main` of the submodule (de187d60f) with
   `brit show origin/main:docs/plans/2026-11-15-bounded-memory-backlog.md`: 82 items. The section
   "Peer feedback round, 2026-11-24" named by the brit team is not in that file at `origin/main`
   [GAP: the ref that holds it; the last sections read are "Usage feedback across commands" and
   "Trust decision"], so reports from this device filed after de187d60f are not matched below.
   Column "In brit's pool?" quotes the closest pool line or says NEW.

Limits, stated plainly:

- One device's transcripts. Other devices (ethosengine, the laptop) are not in the counts.
- A reason is an agent's statement, not a verified failure. The repository's own memory records
  that many were wrong: `check-brit-help-before-git-fallback.md` says dozens of "brit lacks
  rev-parse / ls-remote / -C" reasons were written without reading help, and the backlog row 24
  delta (2026-11-19) corrects them.
- A count is occurrences of a comment, not of distinct incidents. One long session repeating one
  push inflates a line; the file has no session ids and no timestamps [GAP: per-day or
  per-session counts, so recurrence cannot be separated from one long session].
- The file does not carry the brit version for most lines. Where a version is named in the text,
  0.2.6 appears in 147 occurrences, 0.2.0 in 84, 0.1.12 in 47, 0.1.13 in 20, 0.3.0 in 12, 0.3.1 in 8.
  Most stated reasons therefore predate the installed 0.3.4.
- States below come from `--help` text only. "CLOSED" means the verb or form now exists, not that
  it does what the agent needed. A hook stated in the repository (the leading `# Brit fallback:`
  comment) is not a brit defect and is shown separately.
- Brit's own output on this device carried `brit: feedback capture gap: recorder timeout; command
  result unchanged` on every call made while gathering this (see Table 2).

## Table 1. Friction classes, ranked by occurrences

Total kept: 2403 occurrences, 828 distinct reasons. "Distinct" counts lines in the class.
State key: STILL OPEN (verb or form absent from 0.3.4 help), CLOSED (the verb exists, form given),
UNVERIFIED (exists, the stated failure was not re-tested).

| # | Class | Occ. | Distinct | Two verbatim reasons | State against brit 0.3.4 | In brit's pool? | What the agent did instead |
|---|---|---|---|---|---|---|---|
| 1 | landing push | 624 | 125 | "the dev landing push — standing landing exception in CLAUDE.md; worktree hooksPath so the pushed tree's own hook runs" (52); "dev landing uses git push per CLAUDE.md temporary landing exception (brit push closes send-pack after a green hook)" (23) | STILL OPEN per CLAUDE.md "Brit is the daily driver" (the standing exception). `brit push [REPOSITORY] [REFSPEC]...` exists with `--dry-run`, `--atomic`, `--force-with-lease`; the send-pack closure was not re-tested (a push is a mutation). | NEW for the send-pack closure. Related: "A hook refusal on push prints a `Stack backtrace`" and "0.3.0 still peels tags on push" (fixed 0.3.3). | `GATE_EVIDENCE=reuse git push origin HEAD:dev` with the hook on, sometimes with `-c core.hooksPath=<worktree>/.husky/_`. |
| 2 | show / diff / rev-parse / cat read forms | 368 | 151 | "git grep at a specific revision; brit has no grep" (9); "brit cannot show a file at a non-checked-out revision; read-only" (8) | Mostly CLOSED: `brit show [REV]... [-- PATH]` takes `REV:PATH`, `--stat`, `--name-only`, `--name-status`, `--numstat`; `brit diff A..B` with `--stat`, `--name-only`, `diff tree OLD NEW`; `brit rev-parse` (`--verify`, `--abbrev-ref`, `--show-toplevel`); `brit cat <REVSPEC>`. STILL OPEN: grep, `log -S`, ls-files, check-ignore. | Partly: "No `brit grep <rev> -- <pattern>` and no `brit log -S <string>` pickaxe"; "`brit diff -- <path>` refuses any directory"; "`brit diff` in a worktree ... refuses with 'diff attributes and text conversion are not supported'" (seen 0.1.13). Others NEW. | `git show`, `git diff --stat`, `git rev-parse`, `git grep`, `git cat-file -e`. |
| 3 | ls-remote / remote-ref reads | 207 | 72 | "brit has no ls-remote" (14); "read-only remote ref listing, brit has no ls-remote" (8) | CLOSED: `brit ls-remote [REMOTE] [PATTERNS]...` with `--heads`, `--tags`, `--refs`, `--exit-code`; also `brit remote refs`. Hazard in Table 2 (wrong answer in a submodule). | Pool item "No remote-ref listing; reading a remote branch head needed `git ls-remote`" is still an unticked box though the verb exists. | `git ls-remote`. |
| 4 | submodule init / update / gitlink | 195 | 79 | "brit submodule has only `list`; no init/update capability" (12); "submodule update — brit has no submodule verb" (9) | `submodule update --init [PATH]...` exists in 0.3.4: CLOSED for the verb, UNVERIFIED in a fresh worktree. STILL OPEN: stage a gitlink at a named commit (reason: "brit add refuses submodule paths"). | "No submodule init or update (`brit submodule` only lists)" (unticked, now stale); "No command stages a submodule gitlink at a named commit without a checked-out submodule". | `git submodule update --init`, `git update-index --cacheinfo`. |
| 5 | cherry-pick | 150 | 61 | "cherry-pick — brit has no cherry-pick verb" (13); "cherry-pick one commit; brit has no cherry-pick" (6) | STILL OPEN: no `cherry-pick` in `brit --help`; `brit rebase --help` mentions none. | "No patch-apply or cherry-pick command; porting one reviewed commit needed `git apply --3way`." | `git cherry-pick`, `git apply --3way`. |
| 6 | ff-only / merge | 149 | 58 | "fast-forward merge — brit merge has no ff-only working-tree form (only file/tree/commit object merges)" (12); "brit merge writes trees/objects only; it cannot merge into the worktree branch" (8) | Local branch fast-forward STILL OPEN: `brit merge` still has only `file`, `tree`, `commit`. Remote fast-forward CLOSED in form: `brit pull [REMOTE] [BRANCH] --ff-only` (one explicit remote branch). Integrator's record: it cannot fast-forward when a submodule pin moved. | "`brit pull` refuses with 'shallow repositories require Git'"; "brit 0.2.6 has no merge verb" is in backlog row 24, not the pool. The pin-moved case is NEW. | `git merge --ff-only`, `git pull --ff-only`. |
| 7 | rebase | 146 | 53 | "brit rebase refused" (14); "continuing the git rebase already in progress (brit rebase failed with" (10) | `brit rebase <UPSTREAM>` exists with `--status --continue --skip --abort --retire --edit-todo --stage --amend --begin-split`. UNVERIFIED: not run (hazard in Table 2). | 0.3.3 ticked: "`brit rebase --status` in `ready-final` offers `--abort`", the kill-checkpoint staging directory, the author-env digest. Cost scaling on a full history: NEW [GAP: pool has no line on it]. | `git rebase --autostash`. |
| 8 | fetch / refspecs | 98 | 36 | "fetch with objects unpacked loose (shem is at the 32-index ceiling) and a detached checkout of the follow worktree at the landed commit" (10); "fetch — brit fetch aborted with a stack backtrace earlier this session" (5) | `brit fetch [REF_SPEC]...` exists; `brit fetch origin dev` is documented. UNVERIFIED: the abort, and 0.2.6's "no destination leaves the tracking ref" (backlog row 24). The loose-object reason is a pack-index ceiling, see class 19. | Pack-index growth: NEW (backlog row 24 only). | `git fetch`, objects unpacked loose [GAP: exact flags not in the reasons]. |
| 9 | commit creation | 81 | 29 | "brit commit has only verify/sign/describe — no commit creation" (12); "brit has no commit creation (staging uses brit add)" (8) | CLOSED: `brit commit -m`, `-F`, `--amend`, `--allow-empty`. Merge commits STILL OPEN: reasons say "brit commit documents no MERGE_HEAD/two-parent support" and refuses a pending SQUASH_MSG; the help names no merge-commit form. | "A stale `.git/worktrees/<name>/AUTO_MERGE` file makes `brit commit` refuse"; "`brit add -A` refuses the whole selection when the tree holds a nested repository". | `git commit`, `git commit-tree` for merges. |
| 10 | log forms | 77 | 34 | "brit log has no revision range" (6); "one-line log with subjects for a fixed count; brit log's output shape is multi-line and I need subject plus hash per line" (3) | CLOSED: `brit log [REVISION]... [-- PATH]` takes `A..B`, `^A`, `-n`, `--oneline`, `--first-parent`, `--sort`. STILL OPEN: `-S` pickaxe. | "`brit log -- <path>` answers that file-based lookup is not implemented" (0.2.0, unticked); pickaxe as above. | `git log A..B`, `git log -S`. |
| 11 | patch export / apply | 67 | 31 | "brit diff tree prints only changed tree entries, no patch hunks, for a commit-to-commit diff" (4); "format-patch size estimate; brit has no format-patch" (3) | `brit diff A..B` and `brit show` print patches now: CLOSED for patch text. STILL OPEN: format-patch, apply. | "No patch-apply or cherry-pick command". Format-patch NEW. | `git format-patch`, `git apply`. |
| 12 | notes refs | 62 | 21 | "notes fetch/merge/push have no brit equivalent" (13); "read-only notes inspection — brit has no notes-show for a custom notes ref" (4) | STILL OPEN: no `notes` verb in `brit --help`. | NEW. | `git fetch/push refs/notes/...`, `git notes`. |
| 13 | reset / restore / checkout | 49 | 21 | "realign local dev to the landed origin/dev — brit has no reset or branch-move verb; `git reset --keep` refuses instead of overwriting local changes" (6); "checkout -- one hook-written file in a worktree`)." (3) | `brit restore --staged/--worktree --source HEAD -- PATHS` and `brit switch BRANCH` exist: CLOSED for path restore and switching. STILL OPEN: reset, moving a branch ref. | "No path restore, no stash..." (path restore now exists); branch move NEW. | `git reset --keep`, `git checkout -- <file>`. |
| 14 | branch ops | 41 | 19 | "branch -D — brit has no branch deletion verb" (2); "branch --contains — brit branch list has no contains filter" (2) | `brit branch create/list/delete/move` exist: CLOSED for delete; `--contains` STILL OPEN (not in `brit branch list` help read). | NEW. | `git branch -D`, `git branch --contains`. |
| 15 | config read / write | 33 | 14 | "brit config has no write form; the reach hook prescribes git config for britReachBase" (4); "read the configured hooksPath value; brit has no config read" (3) | `brit config set` / `unset` exist: CLOSED for write. A value-only read is STILL OPEN (`brit config <key>` prints the section). | "No value-only config read ... add `brit config get <key>`"; write: "`brit config` has no write form" (0.2.0, unticked). | `git config --get`, `git config`. |
| 16 | worktree operations | 25 | 11 | "removal of worktrees already classified merged, clean and evidence-free ... brit 0.2.6 has no worktree-remove verb" (3); "brit has no branch-contains / worktree list query; read-only" (3) | CLOSED: `brit worktree list/add/remove`. No `prune` verb. | NEW. | `git worktree remove/prune`. |
| 17 | other | 22 | 9 | "read files at HEAD in a worktree to rebuild two files without the node-binding hunks" (6); "reproducing a test fixture byte for byte, and the fixture itself drives git" (2) | UNVERIFIED (mixed; the second is not friction with brit). | NEW. | various. |
| 18 | stash | 5 | 2 | "git stash unavailable in brit for a one-file revert check" (3); "blob-level comparison against dev history and an object-only snapshot; brit has no stash or rev diff" (2) | STILL OPEN: no `stash`. `brit snapshot` seals source trees (different purpose). | "No path restore, no stash or object-only snapshot". | `git stash create`. |
| 19 | repack / pack-index ceiling | 4 | 2 | "repack — brit has no gc/repack verb, and its gix reader refuses this store at 45 packs (" (2); "git repack -a -d in the main repository (the steward's named cure for the pack-index ceiling); brit 0.2.6 has no repack verb" (2) | STILL OPEN: no gc or repack verb in `brit --help` (`odb` and `fsck` exist). | The message was fixed ("At reduced trust above 32 pack indexes, 0.2.0 reports 'rev-spec is malformed'"); no maintenance verb. | `git repack -a -d`. |

Sum check: the 19 rows total 2403.

## Table 2. Failures and hazards that are not missing verbs

Sources: CLAUDE.md "Brit is the daily driver"; backlog `steward-device-arrival-open-items.md` rows 5,
24, 37; memory files named in the sources; the integrator's record (shem, 2026-11-23..25), which is
supplied and not re-verified here. No counts exist for these in the raw file except as noted.

| # | Failure or hazard | Source | State | In brit's pool? |
|---|---|---|---|---|
| 1 | brit push closes send-pack after a green hook, so dev landings use Git. Versions named in reasons: 0.1.12, 0.1.13. About 228 occurrences in class 1 cite "send-pack" or "closes the channel"; about 77 cite "not yet verified". | CLAUDE.md; raw file | STILL OPEN as a standing exception; not re-tested. Counter-evidence from the brit team's relay: 0.3.3's release tag was pushed with `brit push` and no Git fallback (to the forge, no dev hook). [GAP: the version in which the closure was last observed] | NEW |
| 2 | `brit rebase` on 0.2.6 is unsafe on a non-shallow repository: on a full clone of brit (17,972 commits) replaying one commit published after about 26 minutes and 110 GB of spooling, then cleanup ran 4.5 hours; on shem's monorepo it ran 6 minutes, wrote 29 GB, then refused "Directory staging is not supported". After it, `--continue` could not end and `brit switch` refused "pending replay". | memory `never-brit-rebase-through-0-2-6.md`; backlog row 24 delta 2026-11-19 | UNVERIFIED on 0.3.4 (0.3.3 fixed recovery defects; the memory says to recheck only when release notes name the replay redesign) | Partly: recovery items ticked; cost scaling NEW |
| 3 | `brit ls-remote` run inside a worktree's submodule read the local origin and answered "release absent" wrongly. | integrator's record, shem, 2026-11-23..25 | STILL OPEN [GAP: not reproduced here; no mutation, but needs a worktree with a submodule] | NEW |
| 4 | brit 0.3.4's verified release closure was refused by the device store: source-mismatch until the checkout's pin matched, then dirty-build because ingested from a dirty checkout. The working path rebuilt from source for about 30 minutes. | integrator's record; memory `brit-release-path.md` (step 6: `epr bin follow` builds from source, 30+ min, refuses `non-descendant` from a stale main checkout) | STILL OPEN | Partly: "`epr bin follow --tool brit` builds Brit from source although the published `brit-build-<sha>` release exists"; "`epr bin follow` cannot carry a lineage ruling" |
| 5 | A session stays pinned to the brit it started with while the device head moves. Pool: "The installed brit lags the released one: the 0.3.3 tag needed the freshly built binary, because 0.3.0 still peels tags on push." | integrator's record; pool; memory (`epr bin pin --tool brit` switches a running session) | STILL OPEN | Partly (the lag) |
| 6 | Every brit command prints a feedback-capture line that scripts filter. Seen here: `brit: feedback capture gap: recorder timeout; command result unchanged`. Earlier cause on shem: a stale exported `EPR_BIN` pointing at an epr with no `feedback` verb. | integrator's record; this run; pool; memory `brit-release-path.md` | STILL OPEN (a different wording today) | Yes: "Feedback capture on the shem device was failing because the launching terminal exported a stale `EPR_BIN`"; "Say why the recorder refused" |
| 7 | The command hook asks a first-line `# Brit fallback:` comment and agents trip on it for read-only forms that brit has (show of a commit, diff --stat of a range, rev-parse of a ref) or lacks (cherry-pick --continue). The raw file shows the cost: it carries "the hook blocks git without it, including read-only commands" in briefs, and about 1,700 of 3,767 git invocations since 2026-11-06 were `git -C <dir>` (backlog row 24). | integrator's record; raw file; backlog | STILL OPEN; the hook is repository machinery, not brit | NEW (not a brit item) |
| 8 | `brit pull --ff-only` cannot fast-forward when a submodule pin moved; `git merge --ff-only` was used. | integrator's record | STILL OPEN [GAP: not reproduced] | NEW |
| 9 | `brit fetch -r origin dev` with no destination reports up to date and leaves `refs/remotes/origin/dev` unmoved (0.2.6); and each fetch adds a pack index, nothing consolidates them, shem 28 and ethosengine 33 against a ceiling of 32 at reduced trust. | backlog row 24 delta 2026-11-19; row 5 (`brit fetch origin dev` had not moved the tracking ref; `brit fetch -r origin` did) | UNVERIFIED on 0.3.4. `brit fetch --help` now documents `brit fetch origin dev`. | NEW |
| 10 | `brit commit` refuses after a Git rebase, cherry-pick or squash left metadata ("merge metadata exists without ...", "merge commit cannot complete pending SQUASH_MSG"); a backtrace printed; `brit add -A` panicked (reasons). | backlog rows 5, 24; raw file | UNVERIFIED on 0.3.4 | Partly (AUTO_MERGE item; `add -A` nested repository refusal) |
| 11 | A hook refusal on push prints a `Stack backtrace` of `<unknown>` frames after the refusal text. | pool, 0.2.0 dogfood | UNVERIFIED | Yes |
| 12 | The brit pin's attestation once rested on a check that was green over failed legs (row 37); fixed on brit's integration branch by an allow-list step. | backlog row 37 | CLOSED on brit's side [GAP: whether the pinned commit carries it] | n/a |

## Table 3. Reported by the ThinkPad session, 2026-11-25, brit 0.3.3, not verified on shem

Relayed by the coordinator; none of these was run here. "In brit's pool?" is checked against the
pool file read above.

| # | Report | Check made here | In brit's pool? |
|---|---|---|---|
| 1 | The command hook refused `git merge-base --is-ancestor` though `brit merge-base --is-ancestor` works. | 0.3.4 help lists `brit merge-base --is-ancestor`. The refusal is the hook (Table 2 row 7). | NEW (hook) |
| 2 | No `brit ls-tree`. | `brit --help` has no `ls-tree`; `brit tree entries` exists ("Print entries in a given tree"). Whether it is the same output is UNVERIFIED. | NEW |
| 3 | The line "feedback capture gap ... requires explicit migrate --legacy-writers-quiesced" on most commands, including `brit status --no-write`. | On shem 0.3.4 the line reads "recorder timeout". The laptop wording was not seen here. | Partly (Table 2 row 6) |
| 4 | `follow brit` refused because the main checkout sits on a feature branch (pin does not descend from landed head). | Same family as memory `brit-release-path.md` step 6 (`non-descendant`). | Partly: "`epr bin follow` cannot carry a lineage ruling ... refuses `non-descendant`" |
| 5 | 46 pack indexes, over the reduced-trust cap of 32, make brit reads fail; no repack or maintenance verb, so the cure is Git. | Same as class 19 and Table 2 row 9. | Partly (the message only) |
| 6 | On a 95%-full disk, brit's journey tests need `TMPDIR` and the cargo target on disk because /tmp is RAM-backed; its QA scheduler needs Rust stable >= 1.95 while that host has 1.93.1. | Not checkable here. | NEW |
| 7 | Thin landing, dev e44b8f31b, installed `brit 0.3.0` (behind the 0.3.4 pin), no cargo pool: `brit worktree add` took 6 s and 313 MB, and `just dev prep` took 13 s and 354 MB with nothing downloaded. Prep printed `[prep] selected projects: (none)` and `[preflight] ok submodules`, so it asked for no submodule objects. Its `hooks-path.mjs` set `core.hooksPath` to the main checkout's `.husky/_`, so a worktree push runs the main checkout's hook unless `-c core.hooksPath=<worktree>/.husky/_` is given. Row 3's line still prints on every brit command. | Reported from the laptop itself. | NEW (hook path) |

What worked there, per the report: `brit fetch origin <branch>` and `brit worktree add --detach`.
`brit worktree add --help` here confirms `-d, --detach`.

## What the numbers say

Shares are of the 2403 kept occurrences (and of all 3111 in brackets).

- Top three classes: landing push 624, read forms 368, ls-remote 207 = 1199 occurrences, 49.9%
  (38.5%). Landing push alone is 26.0% (20.1%).
- The single closure that removes the most: a brit push that completes a gated dev landing. It
  would remove up to 624 occurrences. About 228 of them name the send-pack closure; the rest
  name the standing exception, the worktree hooks path, or side-ref and notes pushes, so the
  removable share depends on whether the exception text is also rewritten once push is verified.
- Classes where a verb or form already exists in 0.3.4, so the cure is instructions and hook
  text and not brit: ls-remote 207, commit creation 81 (non-merge), log forms 77 (ranges), fetch 98
  (documented form), branch ops 41 (delete), worktree 25, config write 33 (set), and most of
  class 2. That is at least 562 occurrences (23.4%) in whole-class terms (207+81+77+98+41+25+33),
  and up to 1125 (46.8%) if submodule update (195) and the closed part of read forms (368)
  are counted. The pool also still shows unticked boxes (ls-remote, submodule init/update) that
  0.3.4 help answers, so the pool needs a pass as well.
- Still absent: cherry-pick 150, local ff-only merge 149, notes 62, format-patch/apply, stash,
  reset, repack, pickaxe and grep.
- Where a version is named, 0.1.x lines cite missing fetch, rev-parse and commit; 0.2.x cite
  submodule, ls-remote and range log; 0.3.x lines cite cherry-pick and the push exception. The
  file has no dates, so this is read from text only and shows no trend.

Classes NEW to brit's pool (no matching line found):

1. The send-pack closure after a green hook (landing push).
2. Notes refs: fetch, merge, push, show for `refs/notes/*`.
3. Local branch move or reset; local fast-forward merge as a verb.
4. `pull --ff-only` when a submodule pin moved.
5. `ls-remote` inside a worktree's submodule reading the local origin.
6. Pack-index accumulation from fetch, and any maintenance verb (pool has only the message fix).
7. Release acceptance and session pin (the device-store side; pool has the "builds from source" line).
8. `brit ls-tree` equivalence, `format-patch`, branch `--contains`.
9. The laptop's disk, /tmp and Rust-version prerequisites (Table 3 row 6).
10. Hook friction on read-only forms (belongs to the repository, not brit).

## Reconciling brit with the epr runtime

Facts and gaps only, from the sources; nothing is proposed here.

- Bin store, ingest, follow, land. brit reaches a device as a tool through `epr bin follow --tool
  brit --repo <clean checkout at origin/dev>`, which reads the checkout's pin, builds from source
  (release, one job, 30+ minutes, claim `cargo` first), and `epr bin pin --tool brit --cid <cid>`
  switches a running session (memory `brit-release-path.md`). From a stale or dirty main checkout
  it refuses `non-descendant`. A published `brit-build-<sha>` release exists for the same commit
  and is not used (pool). brit 0.3.4's verified closure was refused: source-mismatch, then
  dirty-build (integrator). `epr bin land` takes `--rebase-lineage`; `follow` does not (pool).
  The laptop (95% full disk) was refused "checkout-dirty" / "non-descendant" (integrator).
- Session pin. A session keeps the brit it started with while the device head moves (integrator).
  The pool records a related lag: 0.3.0 still peeled tags on push when 0.3.3 was released.
  [GAP: how a session learns that its brit is older than the device head.]
- Notes refs under `refs/notes/brit/*`. Gate verdicts and builds are notes; each device publishes
  to its own `refs/notes/brit/devices/<did>/build/*` with `just dev notes publish`, fast-forward
  only; the shared `refs/notes/brit/build/*` on origin is frozen (CLAUDE.md; backlog rows 3 and
  the notes rows at lines 119 to 146). brit has no notes verb (class 12), so those operations are
  Git. `DEVICE_NOTES_PUSH=brit` exists for a file remote only (backlog line 136).
- The gate's attested pin check. `gate-attest.mjs` reads the pinned brit commit's named CI check
  upstream (CLAUDE.md). A brit pin move selects direct consumers but not Edge (memory). Row 37:
  the attestation once rested on an aggregate green over failed legs; brit added an allow-list
  step. [GAP: which brit commits carry it beyond `1bf96648cb`.]
- Commit trailers via epr session. Git's `prepare-commit-msg` asks `epr session commit-trailers
  --json`; brit's own commits print "attribution UNMEASURED" because "the prepare-commit-msg path
  finds no session adapter in this submodule" (pool). Whether `brit commit` runs the same hook on
  shem [GAP: not tested here].
- Feedback capture. brit calls `epr feedback`; a stale exported `EPR_BIN` (an epr without the
  verb) makes every brit call print the gap line; the recorder discards the epr's stderr, so a
  missing verb reads like any refusal (pool). Today's wording on shem is "recorder timeout".
  The laptop's wording names `migrate --legacy-writers-quiesced` (Table 3).
- The hook. The brit-first PreToolUse hook is repository machinery (CLAUDE.md), not brit. It
  admits git only with a first-line comment and trips on read-only forms (Table 2 row 7).
- Trust environment. `epr artifact put` spawns `brit-build-ref` without the trust environment, so
  it fails after a git ownership refusal (backlog, around line 748). The pool's trust section
  records that brit does not read `GIT_CONFIG_PARAMETERS`, which is how `git -c` reaches
  children such as a hook that runs brit.

## Candidate sprint items

Candidates for the brit team to rank, not decisions. Occurrence numbers are class totals from
Table 1, an upper bound on what a closure removes.

1. Verified `brit push` for a gated dev landing (pre-push hook runs, push completes, no
   send-pack closure). Closes class 1, 624 occurrences. Verify: push an empty docs commit to a
   local bare remote with the hook enabled; fails if the exit is nonzero or the remote ref did
   not move or no `[pre-push]` lines appear.
2. `brit cherry-pick` (including a gitlink-only commit). Closes class 5, 150. Verify: pick a
   commit that moves a submodule pin onto another branch; fails if the tree differs from
   `git cherry-pick`'s.
3. Local fast-forward `merge --ff-only <ref>`, and `pull --ff-only` when a submodule pin moved.
   Closes class 6, 149. Verify: fast-forward across a pin move; fails if the worktree or
   gitlink differs from `git merge --ff-only`.
4. Notes verbs (show, fetch, push, merge fast-forward) for `refs/notes/*`. Closes class 12, 62.
   Verify: round-trip a `refs/notes/brit/devices/<did>/build/x` ref through a bare remote;
   fails on a non-fast-forward accepted or a ref left unmoved.
5. `ls-remote` that reads the remote of the repository named, including in a worktree's
   submodule. Closes the wrong "release absent" answer (Table 2 row 3); the class is already 207
   in a closed state. Verify: run inside a worktree's submodule whose local origin differs from
   the remote; fails if its answer differs from the remote's.
6. Stage a gitlink at a named commit without a checked-out submodule. Closes part of class 4,
   195. Verify: `brit add` the pin of a not-initialized submodule; fails if the index entry
   is not the named commit.
7. `brit grep <rev>`, `log -S`, `ls-files`, `check-ignore`, `ls-tree`-equivalent output. Closes the
   open part of classes 2 and 10 (of 445 together). Verify: compare output to Git's on the
   monorepo at one commit; fails on any byte difference.
8. `format-patch` and `apply`. Closes class 11, 67. Verify: export and re-apply a range;
   fails if the result tree differs.
9. A maintenance verb (repack or consolidate pack indexes) and a reader that does not fail at 46
   packs. Closes class 19 (4 here) and the laptop's pack failure, and the ethosengine/shem fetch
   ceiling. Verify: a store at 46 packs becomes readable by `brit log` after the verb.
10. `reset`/branch move, `stash`, `config get <key>`. Closes classes 13, 18, 15 (up to 87 together).
    Verify: `brit config get core.hooksPath` exits 1 when unset and prints the value alone when
    set; fails otherwise.

Not candidates for brit but needed for the numbers to move: rewrite the CLAUDE.md fallback
guidance and the hook so read-only forms that brit now has (classes 2, 3, 10, 14, 16) do not need
a comment, and tick the stale pool boxes. Owner: the repository's steward, not brit.
