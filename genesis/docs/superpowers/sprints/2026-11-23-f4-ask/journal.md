---
id: f4-ask-sprint-journal
title: F4-ask — sprint journal
date: 2026-11-23
status: OPEN
steward: human:matthew
author: shem session cacccb55 (Claude Sonnet 5.5), integrating; ethosengine session d6cfb7e1 (Opus 5.5) building
serves: [sprint-lands-each-closure-once, measure-runs-on-a-peer, workspace-device-arrival, push-gate-pays-once]
cites: []
---

# F4-ask — sprint journal

## What the sprint is for

Station F4-ask of the native dev tooling plan (`../../plans/2026-10-29-native-dev-tooling-over-peers-plan.md`
§8) asks that the request to carry a gate be a record both devices read, raised before the gates,
and not a note that reaches a session only while the exchange under it moves. The steward ruled on
2026-11-23 that the first slice is built as written and that the carry claim is built with it.
One deliberate `[edge:roll]` landing carries the slice and the roll queue of arrival backlog row 75.

## Rulings

Both rulings were given at the start of the integrator session on 2026-11-23 and are recorded as
covenant clauses `bafyreibdcqucam…i3ky` and `bafyreibj2anuf4…ee2q`. The text of each clause was
not re-read for this record; what follows is the integrator's account of them.

- The sprint is station F4-ask. The first slice is built as written and the carry claim is built
  with it. The steward gave wide write latitude.
- One deliberate `[edge:roll]` landing carries the slice and the roll queue of row 75.

## Lanes

| Lane | Who | What | Head named in the report |
|---|---|---|---|
| Slice 1 | ethosengine | the `misses` crossing kind, the reader `misses.rs`, the hand-over caller, a berth follow line, five `@wip` scenarios | `self/ethosengine-f4-ask-1` `f257df0ab` |
| Terminal-env | ethosengine | `device-arrive` unsets an `EPR_BIN` older than the head | `self/ethosengine-terminal-env` `6aac6b0dd`, after one Codex review round |
| F3-probe | ethosengine, by hand | one landing with the forge unreachable | notes only |
| Row-20 | ethosengine | every non-attested gate is enrolled for reuse or names why not | `self/ethosengine-row-20` `b274eb426` |
| F6-probe | ethosengine, by hand | one green verdict offered for graduation | its journal at `genesis/local-dev/f6-probe/journal.md` on ethosengine, not read here |
| Test fix | ethosengine | two `sysroot_put` tests open the payload store by its format marker | `self/ethosengine-sysroot-test` `7f2d191bc` |
| Roll queue | shem | eight commits assembled as `f4/roll` | on `landing/f4-ask` |
| Carry claim | shem, delegated | `flow/carry.rs` and `genesis/orchestrator/gate-carry-claim.mjs` | in work when this record was written |

The 17 commits on `landing/f4-ask` above dev `179814d6b`, read from `brit log` by subject:
`7a543ee9`, `dd14d94b`, `d749369c`, `0ba7b32a`, `aac55aa1`, `3f8cf20c`, `b65a7ab3` and
`416e19b9` are the roll queue; `b36e4deb`, `680ffca8`, `2bd529ba` and `2251df15` are slice 1;
`7d37d74f` and `834ed61d` are Terminal-env; `dd421e0c` and `aa45fd2b` are Row-20; `411df920` is
the test fix. The mapping of each to the ethosengine head it came from is by subject and was not
checked by diff.

## Log

### 04:50Z to 13:30Z: the build, the roll queue, and a seven-hour stall

- **Slice 1** (`f257df0ab` on ethosengine's branch). Ethosengine reported: a hand-over whose
  selected gates have no green verdict posts a `misses` crossing on its intent, one closure key per
  project, and a sibling reads it with `epr peer intents --misses`. A miss settles only on a
  verdict for that closure, or on the announcer's landed or withdrawn intent, never by clock. Its
  blind-reader loop on the new `landing-queue.feature` scenarios closed READY in three rounds,
  and its `just gate eprfs` on `f257df0ab` was green. These are its reports; shem has not re-run
  them.
- **Terminal-env** (`6aac6b0dd`). One Codex review round found two blocking defects: a newer or
  candidate build was unset too, and any exit 1 read as a refusal. Both were fixed. Ethosengine
  reported preflight 47 of 47, the hook's tests 4 of 4, and its orchestrator gates green.
- **Row-20** (`b274eb426`). Ethosengine reported 25 enrolled and 36 withheld gates, each withheld
  gate with a one-line reason, and a coverage test holding exactly one of the two. Its first
  orchestrator gate caught the test: with submodules present the registry held the attested brit,
  rakia and sophia gates, which the test had not seen. The fix put attested pins outside
  `gate_reuse`. Gate-evidence 96 of 96.
- **The `sysroot_put` test fix** (`7f2d191bc`). Two tests failed under `--all-features` at bare dev
  `179814d6b` on ethosengine (`BlobNotFound` at `tests/support/artifact_sysroot_put.rs:80`).
  Ethosengine diagnosed it as not host-specific: with the blob-store feature the payload root is
  the new layout and the test opened the legacy flat one. The fix opens it through the crate's own
  function. Reported 5 of 5 with and without the flag.
- **The roll queue, `f4/roll`, eight commits.** The fold's private flock guard was replaced by the
  shared one, the PARKED allowance was deleted, and the Jenkinsfile guards were picked. The eprfs
  gate ran green on shem in 1,811 s, over its 1,200 s ceiling.
- **The carry claim and the lander's wait** were still in work at the time of writing. The claim
  is `flow/carry.rs`; the wait is `genesis/orchestrator/gate-carry-claim.mjs` with
  `GATE_CARRY_WAIT=off|shadow|on`, default `shadow`. Neither is on the landing head.
- **Probe results as ethosengine reported them.** F3-probe: rung 1, the first forge step is git
  ref advertisement, before pre-push (`Could not resolve host`, no `[pre-push]` line). Rung 1b: a
  fresh worktree needs the forge earlier, because `submodule update --init` clones each submodule
  from GitHub. Rung 2: against a local bare stand-in forge with GitHub and HTTP unreachable for
  the hook, an empty commit landed (Phase L claim, fast-forward, settle released); no gate was
  selected, so pin attestation was not exercised. Its note then continues with a "Trap" about git
  moving local `refs/remotes/origin/d…`, and the note is cut off there in every copy shem holds.
  F6-probe: signer to `agent_cid` refused first (`epr identity standing` needs a running storage
  node, and `brit-epr` names no `agent_cid`); standing refused (roster row self-claimed); second
  principal refused at k=1 (the only other record is shem's green, same steward); the kind check is
  also reported refused and its text is cut off. A side finding it did not trace: every
  `gate-verdict@1` ethosengine wrote that night carries `occurredAt 2026-11-20T18:19:34Z`.
- **The integrator stalled from about 05:45Z to 13:14Z on one held tool call.** Seven notes from
  ethosengine arrived together at 13:14Z. Ethosengine had waited about seven hours with every
  hand-over green and ready. No note and no status line on either device said the session's own
  tool call had been pending for hours. Arrival backlog row 82 records it.
- **A wait loop that never ended.** A delegated agent's loops of the form
  `until ! pgrep -f "<string>"` matched their own command line. Arrival backlog row 83.
- **An `EPR_BIN` older than the device's head.** The integrator session inherited one. It refused
  the first hand-over push at preflight and left commit trailers `UNMEASURED` on eight commits.
  They were repaired locally before any push, by `epr session open --model` and a re-run of the
  trailer hook. It is the probable cause of `brit: feedback capture gap: epr … unknown command
  feedback`; that was not re-checked.
- **Jenkins.** PR #219 (`self/shem-f4-gateonly`, tip tagged `[edge:gate-only]`) was opened at
  about 13:22Z for the storage and doorway gates of the landing head, not for merge. The result
  was not known at the time of writing.
- **Row 74 is not written.** The remaining lock sites take the shared guard only after a design
  choice: `epr-lsp` would need a new dependency and a lock move, and `peer.rs`,
  `arrival_sysroot.rs` and `supervise.rs` need a patience parameter on the shared guard.

## Not done at the time of writing

- The carry claim and the lander-side wait are not on the landing head.
- The assembled head has not been gated on the second device. Ethosengine said it would gate a
  fresh worktree at `411df9207` (eprfs, then orchestrator and orchestrator-tree) and then act as
  device B; no result had arrived.
- The two-device Verify of F4-ask: a hand-over posts the misses, the second device takes one from
  `epr peer intents` with no note sent, and the lander's log shows the reuse line.
- The roll: the `[edge:roll]` landing itself, and the Jenkins read of it.
- Row 74, the PR #219 result, and the cause of the frozen `occurredAt`.

## Fallbacks recorded

Git was used for these operations in the sprint, on these reasons. What each was applied to is
not recorded here.

- **Git cherry-pick.** Brit has no cherry-pick verb.
- **Git commit-tree.** Brit has no commit-tree verb.
- **Git rebase `--exec`.** Brit rebase has stranded repositories on this device, so rebases here
  go through Git.

## 13:30Z to 15:30Z: the candidate assembled, five reds found by gates, and what Jenkins paid

- **The landing head** grew from `411df9207` to `c44633243`: the carry claim (three commits
  picked from `carry/f4-ask`, one conflict in the flag list of `flow/mod.rs`, both lanes' flags
  kept), the lander-side wait, the records commit, and four fixes named below. The publish
  precheck passes on it with no version bump: no published eprfs crate changed. The Edge
  selection comes from the lock guard in `device-runtime`.
- **Five reds, each found by a gate and none by the lane that wrote it:**
  1. `suite-list.test.mjs` refused `gate-carry-claim.test.mjs`, a new test file that was not in
     `suite-reads.json` (orchestrator gate, shem). One line.
  2. A carry unit test posted misses on two intents under one Change-Id and expected both open.
     Slice 1's rule settles the older, so the test was wrong and the rule right (eprfs gate, both
     devices, the same minute). The second intent takes its own change.
  3. The two-hour ceiling on `--valid-for`, added in review, broke the replay integration test,
     which passed 90,000 s. The integrator had run the library tests and not the integration
     tests (eprfs gate, both devices). The replay uses 7,200 s and the ceiling is asserted.
  4. `prepush-carry.test.mjs:208`, a flake already on dev: wall time equal to the sum when two
     fake gates end in one millisecond, 7 of 30 alone. Ethosengine's fix proves the overlap from
     the order of the marks file and uses no sleep; 100 of 100 alone (`67efbfafe`, picked).
  5. `arrival_sysroot` `sysroot_unobservable_arrival_degrades_and_never_writes`, 1 of 30 at
     eight threads, on a raw `flock` site that row 74 lists as owed. Not fixed on this head;
     ethosengine is building `lock_sysroot` on the shared guard as the follow-up landing.
     A second rare one, `discovery_uses_the_one_node_that_answers_and_never_guesses`, 1 of 100
     at eight threads, is not yet read.
- **Gates on `c44633243`, shem:** eprfs green (153 test binaries), orchestrator and
  orchestrator-tree green; rust-style green at `1bdee52dc`, and the two commits after it touch
  tests only. **Ethosengine**, from fresh worktrees: eprfs green on `411df9207` in 800 s, with
  thirty of thirty at four threads on the three binaries nearest the lock sites (the
  recall-session guard in `epr-cli` was not in that set); its lines for `c44633243` are in the
  landing's own record below.
- **Jenkins** paid the storage and doorway gates for the tree at `411df9207` through PR #219
  (`elohim-edge/PR-219` #2, SUCCESS, 3,211 s, its log line 219: `GATE_ONLY — strict cargo
  quality gates and local builds; publish, deploy and dataplane validation disabled`). It was
  sent that work by a tagged pull request. It read no miss and wrote no verdict a lander can
  reuse. On the same push the root pipeline's new guard ended before any agent
  (`elohim/PR-219` #1, NOT_BUILT, 11 log lines); the sophia branch job took a pod before it
  ended NOT_BUILT.
- **The Verify moved to the next landing.** This push changes `gate-*.mjs` and
  `pool-policy.json`, so the governance fence reuses no sibling verdict on it, and the hook that
  posts the misses is the main checkout's old one until this lands. The follow-up landing
  (ethosengine's `lock_sysroot` change) is handed over from shem, its misses are posted by the
  new hook, ethosengine takes the eprfs miss with a carry, and the lander's log is read for the
  reuse line. Station F4-ask stays unticked until then.
- **Fleet read before the roll** (15:00Z, public reads): matthew's storage `caughtUp: true`,
  pull `caughtUp`, projection reconcile `converged: true`, six peers connected;
  `doorway-alpha` 200 on the landing content. The per-pod Prometheus legs were not read from
  this device.
- **A correction to this journal's first section:** the `brit: feedback capture gap` line was
  attributed to the inherited `EPR_BIN`. It is not: brit 0.3.1 prints it with `EPR_BIN` unset
  too, on the device's head epr (`bafyreibmupfq…h6ea`), which has no `feedback` verb. It is a
  gap between brit 0.3.1 and that epr, for the brit lane.
- **Ethosengine on `c44633243`, from a fresh worktree** (its note of 15:2xZ): eprfs green in
  589 s, 153 test binaries; orchestrator green in 159 s; orchestrator-tree green in 158 s; each
  with evidence recorded on that device.
