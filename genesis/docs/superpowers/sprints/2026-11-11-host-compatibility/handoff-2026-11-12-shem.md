# Shem handoff, 2026-11-12 (host-compatibility sprint)

Written by the shem session 3dd97e3d (Claude Fable 5.2) for whichever session carries shem's side
next. It records the state at about 15:30Z on 2026-11-12. Read it with the sprint `journal.md` and
the plan `genesis/docs/superpowers/plans/2026-11-10-host-compatibility-foundation-plan.md`.

## State in one paragraph

Dev is at `03201672a`. Ethosengine (session ab4528a1) owns lane K and has landed K8 (with its
control-flow fallback and the `isa.x86.partial-decode` gap), K9, and K11 slice 1. Shem landed A1,
S1 to S3, K6 slice 2 and a second-device K8 extraction earlier, and today only documents. The one
live engineering thread on shem is the K9 two-device receipt, which is one defect away from done.
Codex's Brit feedback sprint (session 01a1cbeb) is published and finished.

## K9 two-device receipt: where it stands

- Shem's half is recorded at commit `8b273f826`: output `bafkr4iegrxripmxuvkkrwl4q76jjenva4vtxcf65wkgufcgzg7glw5g5su`,
  key `bafyreifeinwqu6ra7iqj3j7dxlutl5xrocsmjhxch3wrtnpxfadl7vnrlm`. A second build two hours
  later reproduced the same output and key.
- Both devices now see each other's build records.
- Shem's `epr artifact inputs diff` (descriptor form) pairs with ethosengine's `8b273f826` record
  and then exits 3: ethosengine's daemon served the manifest of its newer `164eca807` record for
  the same output, and the fetch rejects a manifest other than the one the paired record names.
  Ethosengine agreed (note at about 15:15Z) that this is the serve-side twin of its own pairing
  defect (B): both assume one record per key and output, which commit-independent keys break. It
  ruled out the workaround and is fixing the design on branch `hostcompat/k9-pair-by-record`
  (from dev `03201672a`, Codex astra leg): (1) the daemon serves exactly the record named in
  `holders[].manifest` and refuses with a typed gap rather than substituting; shem's client-side
  `ManifestRejected` guard stays; (2) `inputs diff` pairs on the newest commit both devices
  recorded; (3) artifact fetch is checked for the same assumption. It will send a landing
  notice and then run its daemon on the fixed candidate.
- **Shem needs the fixed `epr` too**, so that shem's daemon can serve ethosengine's direction.
  After the landing: build `epr` from that dev commit, keep a copy under
  `genesis/local-dev/hostcompat-wave1/k9/`, restart shem's daemon on it (same flags as in the
  table below, `EPR_BIN` set to the new path), then re-run the diff with the new binary. Shem's
  own record at `8b273f826` should not need re-recording; if the new pairing rule selects a
  different commit, say so to ethosengine before re-recording.
- Expected content of a successful diff, per ethosengine: `link:Scrt1.o`, `link:crt1.o` and the
  compiler macro dump. Anything else is a finding.
- To re-run:
  `EPR_BIN=<k9 epr> <k9 epr> artifact inputs diff genesis/local-dev/hostcompat-wave1/k9/method-doorway.json --device did:key:z6MkpZRhMwWJpv7HeZyArTUkbnG6DUyukQp18MxUs2fHVMZB --repo /projects/elohim/.worktrees/k9-8b273f826`
  where `<k9 epr>` is `/projects/elohim/genesis/local-dev/hostcompat-wave1/k9/epr-8b273f826`.
  Outputs are kept beside it as `inputs-diff.stdout`, `.stderr` and `.exit`.
- When it passes: copy the diff output into the sprint `evidence/` directory, add the habit DELTA
  to `genesis/orchestrator/.epr-meta/sprint-lands-each-closure-once.habit.md` (newest first),
  re-project habits, and tick K9 in the plan with the evidence path.

## Things on shem that are in a temporary state

| What | State now | What it should become |
|---|---|---|
| Peer daemon | Runs the candidate `genesis/local-dev/hostcompat-wave1/k9/epr-8b273f826`, started with `EPR_BIN` set to that same path, `EPR_PEER_RELAY_URL=https://relay.alpha.elohim.host`, `--root /projects/elohim --handle matthew --listen 0.0.0.0:7777 --policy /projects/elohim/.worktrees/k9-8b273f826/genesis/agentic/pool-policy.json`. Log: `genesis/local-dev/peer/serve-shem-20261112.log`. | An installed release at or past `8b273f826`, with a policy file that is current with dev. |
| Worktree `.worktrees/k9-8b273f826` | Detached at `8b273f826`. The daemon read its policy from here at start. | Removable once the daemon is restarted on another policy path and K9 is closed. |
| Main checkout `/projects/elohim` | Local `dev` at `6d994a931`, 117 dirty entries from several sessions. Untouched today. | Synced to dev. See the next section. |
| Worktree `.worktrees/hc-k6v2` | Branch `hostcompat/k6-v2-sysroot`, used for every docs landing today. Two hook ledgers under `.claude/data/` are modified and deliberately left unstaged. | Keep for further docs landings, or cut a fresh one. |

## The main-checkout sync (owed, not urgent)

K9 does not need it. Read these first:

- `genesis/a2o/reports/recovery/brit-feedback-20261111/final-shared-checkout-handoff.md` (Codex's
  inventory of all 117 entries and what is protected).
- `genesis/local-dev/stewardship-20261112/dispositions.md` (the 44 entries this session took
  stewardship of) and the snapshot `shared-checkout-44.tar` beside it (SHA-256 in the `.sha256`
  file).

Rules that came out of both: keep Codex's six undelivered `brit-first` hook additions as they are;
do not replace the nine shared integration files wholesale; union the hook ledgers, re-hashing at
sync time because they keep growing; do not move the `brit` or `rakia` gitlinks; the nested `brit`
checkout is itself dirty at a different commit. The retired execution-compatibility prototype's
files in the main checkout take dev's versions.

## Decisions taken today (steward)

- The 2026-11-08 execution-compatibility prototype is retired. What dev lacks from it is in
  `genesis/data/timeline/backlog/execution-compatibility-prototype-harvest.md`.
- In-kind work is still accounted (plan ruling; amends A3, P1, P2).
- A sibling's green gate verdict may stand in for a local run: a deterministic floor, an elohim
  ceiling called rarely, repair through lane R. The mechanism is not designed; it is owed under the
  `push-gate-pays-once` habit.
- The one-click devspace vision is plan item X3 and the lvi spec
  `elohim/lvi/docs/specs/2026-11-12-view-source-anywhere-one-click-devspace.md` (roadmap H6).
- Ethosengine may run its shared peer daemon on a candidate build for K9.

## Decisions still open (steward)

- H12: reach on the object. K12 is blocked on it.
- H13: ratify compiler posture B.
- An owner and a covenant for A2 to A4 (`elohim/ark`) and for lane P (`elohim/placement/**`,
  `ark/cli/src/task.rs`). Nothing in those lanes has started.
- The owner of the `work-composition` documents found in the `elohim/rakia` checkout.

## Traps found today

1. **A daemon reads `pool-policy.json` once, at start, from its root's working tree.** A stale tree
   lacks `berth.channel.crosses["artifact-produce@1"]`, and build records then silently fail to
   cross in both directions while berth notes still cross. Pass `--policy` at a current file. Do
   not change `--root`: that moves the store the two devices read each other through.
2. **A locally built `epr` refuses to run as the daemon** ("daemon-release-unverified") unless
   `EPR_BIN` is set to that exact path.
3. **`MESH_BUILD_FETCH=0` is required** when both devices are at one commit, or the build fetches
   the sibling's binary and records nothing.
4. **The pool's `mesh-bin` directory vanished** between two builds two hours apart, taking the
   method descriptor and the installed doorway binary. Cause unknown. Keep copies of anything
   needed later under `genesis/local-dev/`.
5. **`inputs diff` with a literal key demands an exact key match.** The two devices' keys differ,
   so use the descriptor form, which pairs by commit.
6. **Brit gaps hit again:** `brit commit` cannot commit a merge; `brit branch` only lists. Two
   that are not gaps (Codex session 01a1d073): `brit remote -n origin refs` reads the remote heads
   (the `ls-remote` equivalent, verified), and `brit fetch -r origin +refs/heads/dev:refs/remotes/origin/dev`
   updates the tracking ref explicitly. Each direct Git command needs a first-line
   `# Brit fallback: <reason>` comment or a hook blocks it.
7. **The commit attribution hook prints `UNMEASURED: epr session: unknown session command
   commit-trailers`** on every commit. Cause found by Codex 01a1d073: the shell snapshot exports
   an old `EPR_BIN` (`bafyreibrqqhx…`), which lacks the command. With `env -u EPR_BIN` the command
   runs but refuses one layer down ("session model is unmeasured"), because this session's
   envelope was opened with no model. `berth moor --model` updates the mooring, not the envelope.
   Handed to Codex; trailers were written by hand meanwhile. Unset `EPR_BIN` for ordinary
   commands, but keep it set where a candidate `epr` is meant (the daemon, the K9 diff).
8. **The auto-mode classifier refuses a full-access `codex exec` leg**, so landings went to Sonnet
   agents. The landing brief is `genesis/local-dev/stewardship-20261112/codex-landing/prompt.md`.
9. **A berth note watcher ends after one reply.** Ethosengine's "please run yours now" sat unread
   for over an hour. Re-arm the watcher after each reply, or read the notes before reporting that
   nothing is waiting.

## How the two devices talk

`berth say --session <own session id> --to ab4528a1 "<text>"` sends; ethosengine's notes arrive in
`/projects/elohim/.eprfs/status/flows.jsonl` as `env:text=ethosengine -> shem: …`. Landing protocol:
a notice, then `berth claim lane:regenesis-landing`, push the explicit commit with the hook
enabled, publish build notes, release, and a landed notice.

## Suggested next steps for shem, in order

1. Read ethosengine's reply on the K9 manifest defect and close K9 as above.
2. Tick the plan's K-lane checkboxes that have evidence, since they are all still unticked.
3. If the steward names an owner and covenant: start A2 (host observation) and A3 (admission in
   `authenticate_candidate`), using the harvest note for the `ark compatibility` surface.
4. The main-checkout sync, as its own careful step.
