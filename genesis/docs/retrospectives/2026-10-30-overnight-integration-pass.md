# Retrospective: the overnight integration pass, 2026-10-27 to 2026-10-30

**Shift:** `2026-10-27T03-00-overnight-integration-pass` (shem, session 3f9afd7a; the journal is local to shem).
**Compiled by:** Claude at the steward's request, 2026-10-30. **Steward:** human:matthew.
**Verdict:** the pass landed a great deal, but for its last 18 hours dev did not move. ethosengine queued behind shem's big train. The steward had to step in repeatedly to get work moving. This sort of pass should not run this way again until the mechanisms in §4 exist.

## 1. What landed

Shem landed about 11 times on dev: gate-live, the fixes series, compute-worker supervision, the eprfs publish precheck, ports A and D+E, the worker source pin, the lanes push and the grant renewal. ethosengine landed 7 batches over berth coordination, with no human relaying messages.

Shem's pushes were refused or failed about 25 times. The longest attempts were 24,963 s (6 h 56 m), 15,930 s, 8,151 s and 7,856 s. The big train never landed; it is at 422bbf61b plus fixes at the time of writing.

## 2. Inventory of the steward's interventions

The steward intervened 36 times. Of those, 20 were needed only because shem was not doing its own job.

| Kind | Count | The steward's words (abridged) |
|---|---|---|
| **Charter and direction:** expected, not a failure | 11 | integration pass · use Codex to delegate · wrap up the gate work · peer-runtime contact · coordinate with ethosengine, don't wait on me · `/shift` overnight, traffic cop · integrate or clean out stranded branches · don't drop intended functionality · "b) pay down the crates" · ultracode on · Haiku can relay Codex |
| **Rulings only the steward can make:** expected | 5 | renew the runner grant · explicit renewal sequence · full story loop to READY · resized four shem volumes and raised ZFS refquota by hand · reported matthew's real disk limit |
| **"Is it stuck?" and status pings:** shem was silent | 6 | "what's taking so long? are you stuck?" · "stuck?" · "ok so we're doing ok?" · "how are the tasks going, are we landing stuff?" · "how long has the storage gate been running?" · "for at least 6 hrs the jenkins pipeline has been quiet" |
| **Peer messages shem had not read:** shem was not listening | 3 | "did you get the message from ethosengine?" · "ethosengine posted a message to you" · "ethosengine is waiting on us to land something" |
| **Corrections of shem's judgement:** shem should have seen it | 7 | don't let a stale manifest overwrite adam's config · "they should be accurate to our real limits" · set matthew at 80G for consistency · "why are we hitting formatting failures in the gate?" · never commit unlinted code; put it in `.epr-meta` · "that's gradient-reading applied to the coding process itself" · get a push landed and tell ethosengine |
| **A subagent's claim relayed unchecked:** shem should have verified | 1 | the steward's own reading found the "class can't expand" claim wrong |
| **Out-of-band operator action** | 3 | the volume resize and refquota raise, the matthew reading, the earlier grant renewal |
| **Approval and retry prompts:** shem's own commands paused | not counted, frequent | "stopping literally anytime to ask to retry a command, or to approve a complex bash, or anything that pauses you, and has me having to punch something random to keep going" (2026-10-30). Each pause stopped an unattended shift until the steward pressed a key. |

The 6 status pings, 3 unread-message prompts, 7 corrections and 1 unverified claim make 17. The 3 out-of-band actions are a consequence of the volumes filling up, and better sensing would have caught that earlier, so the total traced to shem falls short at about 20.

## 3. What went wrong

1. **The train was too big for one gate.** It carried storage, doorway, app, a2o, bridges, seams, governance and ports in one push. That selected about 41 gate projects, and every red re-paid all of them: elohim-storage alone ran 1,800 to 7,259 s, and memory-ceremony 1,545 to 8,382 s. No train that size can land on this host in a night. Smaller lanes that could have landed on their own waited behind it.
2. **Cheap checks were paid last.** The following each refused a push 1 to 90 minutes in, while a long compile was running:
   - rustfmt and Prettier drift from merge resolutions;
   - a seam registry left unrouted;
   - a manifest's Cargo closure;
   - two a11y colors;
   - a test fixture that copied `berth` without `_lib`.

   None of these ever passed through an edit hook. `.husky/pre-commit` checked governance only, and `lint-check.py` ran rustfmt only for `doorway/`.
3. **Known environment gaps recurred for the third time.** Cluster row 45 (2026-10-15) recorded that a fresh worktree fails pre-push on missing environment, found only after a long compile. It recurred on 10-24 and again here: no node_modules (fixes push 5), no submodules (push 18 and big train push 1), no element dists (push 19 attempt 4), no app dist or sophia assets for the prologue, and no `wasm-pack`. **A lesson captured as a backlog row did not stop it recurring.** Capture is not prevention.
4. **Review came after the merge.** The reclaim's data-loss findings arrived while the reclaim already sat in the train being pushed, and forced a dormant-by-default flip at the last minute.
5. **A standing flake on dev made the gate a coin toss.** The `tests/epr` harness waited on one side of a connection only, so `DialFailure` failed 4 of 5 runs on dev under load. Every storage-touching push from either device carried that risk. It was found only on the seventh hour of push 3.
6. **The landing was serialized and no one was watching for stalls.** By the steward's ruling, ethosengine held until shem reported lane green. Shem never reported green, and nothing measured how long dev had been still. When shem said "clear to push", that was not read as releasing the standing ruling.
7. **Shem was silent, then noisy, while waiting.** Long gates passed without progress notes, so the steward asked "stuck?" six times. A pass-and-fail monitor then flooded the chat with benign events. A grep for `FAILED` missed `test … FAILED` lines.
8. **Shem tripped on its own tools.** Its verify lease blocked its own `mesh start`. `mesh wait` then idled 20 minutes on a start that had never begun. A merge resolution done by script skipped the formatter.
9. **Declared and live state drifted.** The manifests said 20Gi; four volumes were full at 20Gi and grown by hand to 80Gi, and matthew held 29G on a "20Gi" hostpath. The resize script read an unreadable storage class as "cannot expand", and shem relayed that as fact.
10. **Unattended work paused on approvals.** Long compound Bash lines, `cd`-chained commands, ad-hoc `python3 - <<EOF` edits, a binary download and retry prompts each fell outside the allow rules or the auto-mode classifier, so the shift stopped until the steward pressed a key. Each pause went unrecorded, and during an overnight shift it cost hours. The sibling memory [overnight-asks-stall-unattended-work] already names this; nothing measures it.
11. **The host was overloaded.** Load ran from 25 to 53 on 24 CPUs. RAM-guard sheds killed clippy and memory-ceremony. Gates ran beside the IDE's 14 GB extension host, the household mesh and parallel agents' cargo builds.

## 4. Mechanisms: refuse early, detect, or hand off

A lesson recorded only as prose has failed twice before (row 45). Every item below therefore names a mechanism with a home, a probe and an owner shape. Rows 59 to 68 in [the workspace-discipline cluster](../../data/timeline/backlog/arch-workspace-discipline-backlog.md) carry them.

| # | Mechanism | Prevents | Kind |
|---|---|---|---|
| M1 | **Landing budget.** Pre-push prints the summed p90 of the selected gates. Above a ceiling (start at 60 min) it refuses and names the split: the lanes whose closures need no serving receipt and no storage gate land first. | 1, 6 | refuse |
| M2 | **Cheap tier first.** Pre-push runs every cheap leg for every selected project before any compile: fmt, Prettier, lint, harnesses, manifest closure, seam matrix, a11y. Any red stops the run in under 2 minutes. | 2 | refuse early |
| M3 | **Commit-time format ratchet** in `.epr-meta` (steward ruling 2026-10-30), on `feat/commit-format-gate`; two review findings are still open. | 2 | refuse at commit |
| M4 | **Worktree preflight**, the build of row 45: `just worktree prepare` plus a pre-push preflight that refuses in seconds when submodules, node_modules, element or app dists, assets or declared tools are missing. | 3 | refuse early |
| M5 | **Train admission.** A branch joins a train only with a green Codex review recorded against its head. The merge refuses otherwise. | 4 | refuse |
| M6 | **Flake ledger plus a quarantine rule.** A gate red that passes on rerun at the same closure is fingerprinted. Two hits open a flake atom and name the test. Plus a harness lint for one-sided connection waits. | 5 | detect |
| M7 | **Dev-stall sensor.** When dev has not moved in N hours (start at 4) while a peer has a queued landing, the holder is told, the steward is told, and the lane passes to the peer by default. Hold-until-green rulings carry an expiry. | 6 | detect, then hand off |
| M8 | **Waiting discipline.** A failure-only monitor by default, and a status note every 30 minutes during any wait over 20 minutes, giving the gate, elapsed time against p90, and next step. The berth inbox is read at every wake. | 7 | behaviour plus tool |
| M9 | **Lease class upgrade within the same session**, and `mesh wait` exits when no start is in flight. | 8 | fix the tool |
| M10 | **Declared-versus-live volume check** in the edge deploy log, plus subagent claims verified before relay (memory rule). | 9 | detect |
| M11 | **Host budget for gates.** A push gate refuses to start heavy legs when load or RAM is above the ram-guard soft line, and names what is consuming it. | 11 | refuse early |
| M12 | **Pause ledger plus an allow-rule ratchet.** Every permission prompt, classifier denial or retry ask during a shift is recorded with the command shape. The shift prefers the forms that already match allow rules: one short command per call, `git -C` or `--justfile` instead of `cd`, Edit instead of shell heredocs. Shapes that recur (two or more) are proposed as narrow allow rules at shift close (the `generalize-permissions` skill). A shift with no steward present treats a pause as a red. | 10 | detect, then ratchet |

M1, M2, M4 and M12 together would have prevented most of this pass's lost wall-clock and most of the steward's interruptions. They come first.

## 5. Process rulings (until M1, M2 and M4 land)

- One serving-path lane per push. Docs, CI and governance lanes go separately and first.
- Before any push, run the cheap tier by hand over everything a merge touched.
- Report progress on every long wait, and read the berth inbox on every wake.
- Write commands in the shapes the allow rules already admit; record every pause.
- A hold on a peer expires at 4 h of dev standing still unless renewed.

## 6. Design sketch: a session stasis loop (steward request, 2026-10-30)

> **Re-scoped later the same day.** The steward narrowed this loop to the session dialogue itself: an open permission prompt, a turn that ends in a question, and the steward waiting on a quiet agent. It became a lab-neutral hierarchy of concern: a deterministic rung, then capability levels C1 to C4, then the human. An independent coordinator runs it, and presence decides between the collaborative and automated paths. The design now lives in [the dialogue stasis loop spec](../superpowers/specs/2026-10-30-dialogue-stasis-loop-design.md), which supersedes the sketch below. Peer lease demand (row 63), gate wait reporting (row 64) and host pressure (row 67) stay on their own rows. Only row 68, the pause ledger, folds into the loop, as its journal and its learning step.

The steward asked for a loop inside the session that notices when the session is stopped and keeps it moving. "Stopped" covers a permission prompt left unanswered for more than 3 minutes, someone waiting on us, or any other pain signal the lookout catches. The loop should adapt over time so the session stays in stasis.

**One constraint shapes the design.** A stopped session cannot watch itself: while a prompt is open, the agent takes no turns. The watcher therefore runs outside the agent's turn. It can wake or notify the session, but it never acts in its place. **It never approves the session's own prompts.** That would be permission laundering. Approval stays with the steward. The loop shortens the wait and removes the cause next time.

**Sensors.** Each is a measure with a bound, judged by `elohim_epr_rea::Bound` and written through `epr flow sense`. There is no private ledger.

| Signal | Source that already exists | Bound to start |
|---|---|---|
| Prompt open | Claude Code `Notification` hook (permission or idle prompt) writes `stall-open@1` with time and command shape; `Stop`/`PostToolUse` closes it | 3 min |
| Peer waiting | `berth inbox` unread notes addressed to the session; queued claims on a lease the session holds | 10 min unread, 30 min queued |
| Dev standing still | dev head age vs queued landings (row 63) | 4 h |
| Silent long wait | no user-visible output while a background gate runs | 30 min |
| Gate pain | gate-live reds; run time past p90 | first red; 1.5×p90 |
| Host pressure | ram-guard level, sheds | soft line |

**Watcher.** A small loop owned by the device, not by one session. Two hosts would work: a supervised task beside `epr peer serve`, or the `ark` envelope. It reads the sensors every minute. A breach goes through a fixed ladder; it never improvises. Each rung is recorded:

1. **Notify the steward.** A `PushNotification` names the stall: "shem waiting 3 min on approval for `<shape>`". The steward answers from their phone instead of finding it in the morning.
2. **Release what others wait on.** When a stall passes 15 min and the session holds `dev-integration`, `alpha-fleet` or `mesh` while a peer queues, the watcher sends `berth say`. It releases the lease only when the steward's standing rule allows that, which makes the lane handoff of row 63 automatic.
3. **Wake the session.** For peer notes or gate reds while the session is idle (not stuck on a prompt), it injects the event through the existing `gate-live-inject` path or a `ScheduleWakeup`, so the agent acts on its next turn.
4. **Ratchet.** It counts every stall by its command shape. At shift close, the shapes seen twice or more become narrow allow-rule proposals (`generalize-permissions`). The steward accepts or rejects each one. The agent also keeps a "do instead" list (short single commands, `git -C`, Edit over heredocs). The ledger feeds it, so the next shift avoids the shapes that stalled.

**Stasis measure.** `session-stall-minutes@1` per shift, with a declared ceiling. The `algedonic-designer` station adds sensors for pains this table misses. `delivery-stasis` reads the measure each round. Green means stall-minutes stay under the ceiling for two shifts running, one of them unattended.

**What it must not do:** approve prompts; edit permissions itself; restart or kill another session's work; act on a peer note as if it were the steward's word. Notes stay untrusted, so the ladder only notifies and releases under standing rules.

This is row 69 of the workspace-discipline cluster. It subsumes rows 63, 64 and 68 as its sensors and ratchet, so the four ship together.

### 6.1 The loop is backed by a dedicated agent (steward direction, 2026-10-30)

The watcher only notices and escalates. Judgement belongs to a dedicated agent, dispatched when a sensor fires, whose temperament is consideration, care and simplification. Working name: **`stasis-keeper`**. It ships as an agent definition plus a skill, so other sessions can invoke the same discipline.

- **What it reads.** The stall record: the command shape, the session's last intent, and who is waiting (the steward asleep, a peer queued on a lease, a Jenkins lane idle). It also reads the pain signal that fired.
- **What it asks, in order.**
  1. Is there a simpler step that reaches the same end within the rules that already allow it? Examples: one short command instead of a compound one, `git -C` instead of `cd`, Edit instead of a heredoc, a read-only probe instead of a write.
  2. Who is being kept waiting, and what is fair to them? Release the lane, hand the work to the peer, or keep it and say so.
  3. If only the steward can unblock it, what is the one-line ask with a recommendation, so a phone answer is enough?
- **What it returns.** Exactly one of four pivots:
  - **reshape:** the simpler command, handed back to the session;
  - **hand off:** release the lease and notify the peer, under a standing rule;
  - **ask:** a single push notification to the steward;
  - **park:** record the item in a home, such as a backlog atom, and move the session on.

  Every pivot is witnessed in the stall ledger with its reason.
- **What it never does.** Approve the open prompt, widen permissions, act on a peer's note as the steward's word, or undo someone's work.
- **Its pair.** `algedonic-designer` designs sensors from raw exhaust before any measure exists. `stasis-keeper` answers sensors that have fired, at run time. The ratchet links them: a pivot repeated three times becomes either an allow-rule proposal or a new sensor bound.
- **Measure.** `session-stall-minutes@1` falls, and pivots resolve without the steward. Track a "reshape or hand off" share against "ask"; it should rise while asks fall.
