---
id: "backlog-brit-push-fails-after-green-hook-root-cause"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "brit push fails after a green pre-push hook: the HTTP upload outlives reqwest's 30 s request timeout and the cause is discarded"
slug: "brit-push-fails-after-green-hook-root-cause"
written: "2026-11-19"
author: "claude-opus-5-5 (brit sprint 3, session 0e0924d5, steward of record human:matthew)"
status: "wip"
priority: "high"
relatedNodeIds: []
tags: [brit, daily-driver, push, transport, reqwest, landing-exception]
cites:
  - CLAUDE.md
  - genesis/orchestrator/device-notes-publish.mjs
  - genesis/docs/superpowers/sprints/2026-11-05-regenesis-1/briefs/cycle-time-harvest-handoff.md
  - genesis/docs/superpowers/sprints/2026-11-05-regenesis-1/STATE.md
  - genesis/docs/superpowers/sprints/task-brit-feedback-producer-report.md
  - genesis/data/timeline/backlog/ci-app-full-clone-under-degraded-github-egress.md
  - genesis/data/timeline/backlog/brit-sprint-3-handoff-2026-11-19.md
shift_objective: |
  Retire the CLAUDE.md landing exception for brit push. Install as the pin a brit build that
  carries the push-transport fixes (lane sprint3/push-transport) and the push-policy fixes, then
  make one supervised real landing to dev (full hook, `[pre-push]` lines in the log, a visible
  second handshake), followed by the device's own notes publish that CLAUDE.md prescribes, fast
  forward to refs/notes/brit/devices/<did>/build/*; then a second landing, from a linked worktree,
  whose notes publish updates existing device notes refs. Re-read the landing procedure in
  CLAUDE.md on origin/dev at landing time first.
---

# brit push fails after a green pre-push hook

The standing landing exception in `CLAUDE.md` ("brit 0.1.12 closing send-pack after a green hook")
was diagnosed on 2026-11-19.

## Recorded failures

- `genesis/docs/superpowers/sprints/2026-11-05-regenesis-1/briefs/cycle-time-harvest-handoff.md:82-86`:
  on both devices every landing ran the full hook, died with `send-pack: sending on a closed channel`
  after the green hook, and re-ran under `GATE_EVIDENCE=reuse git push origin HEAD:dev`.
- The same sprint's `STATE.md:169`: a SHORT hook failed the same way.
- `genesis/docs/superpowers/sprints/task-brit-feedback-producer-report.md:236-239`.

## What the text means

The receive-pack POST failed while the body was still being written, and brit discarded the cause.
`send-pack: … sending on a closed channel` is the std mpsc `SendError`. The second symptom is
`unexpected end of stream while reading the push report`.

## Reproduced

With the installed reqwest build against a local stub, four ways:

1. an upload longer than reqwest's default 30 s request timeout (brit never overrode it);
2. the server silently dropping the keep-alive connection during a hook shorter than reqwest's 90 s
   pool idle limit;
3. an early 401/413 while the body is large;
4. the POST connection refused.

## Likely production cause: the 30 s limit

brit sends undeltified packs. A one-line append to a 4.6 MB file POSTs 1,755,193 bytes against git's
594; a typical landing is about 351 KB against git's 24 KB thin pack. GitHub egress has been recorded
at 5–15 KiB/s (`genesis/data/timeline/backlog/ci-app-full-clone-under-degraded-github-egress.md`).

Hook duration alone does not cause it: a hook longer than 90 s gets a fresh connection.

Every shipped brit is built `max-pure` (reqwest backend), while default cargo builds and brit's CI
journeys use curl, so CI had never exercised the shipping transport.

## Fix on lane `sprint3/push-transport` (tip `0ca127c2`, second review, not merged)

- The real cause is reported for a failure anywhere in the body; the HTTP push body is now sent whole
  with Content-Length.
- A 60 s deadline and stall guard on handshakes and fetches; no deadline on the push upload.
- A fresh connection after the pre-push hook, with re-validation that every ref's advertised id equals
  what the hook judged ("remote moved while the before-send callback ran; nothing was sent").
- A ref already at the requested id is up to date.
- A push verified as delivered whose report was lost exits 0.
- Journeys on both backends with an HTTP fixture.
- A CI step building the max-pure binary.

## Still open

- Every push makes two handshakes and asks the credential helper twice, until
  `gitoxide-core/src/repository/push.rs` returns "send on this connection" when no hook ran.
- A fast-forward UPDATE of any non-`refs/heads` ref is refused before the hook ("refusing unverified
  fast-forward update of …"), so a combined branch+notes push fails whole. Creating notes works. The
  scratch example measured was a `refs/notes/brit/build/*` ref.
- A configured `core.hooksPath` whose directory is missing, or a non-executable pre-push, pushes
  ungated silently, as git does.
- Packs are undeltified.
- `--push-option` values are capability-checked but never transmitted.
- The curl backend hangs on an early error response with a large body (upstream code).

## What retiring the CLAUDE.md exception needs

The landing procedure in `CLAUDE.md` changed during this sprint, so re-read it from `CLAUDE.md` on
origin/dev at landing time. As of this branch it says: each device publishes its validation receipts
to its OWN forge destination, `refs/notes/brit/devices/<did>/build/*`, fast-forward only, with
`just dev notes publish` (`genesis/orchestrator/device-notes-publish.mjs`), and never pushes the
shared `refs/notes/brit/build/*` refs on origin.

Retiring the exception then needs:

1. A brit build carrying the transport and push-policy fixes, installed as the pin.
2. One supervised real landing to dev: full hook, `[pre-push]` lines in the log, a visible second
   handshake.
3. After the landing, the device's own notes published fast-forward to
   `refs/notes/brit/devices/<did>/build/*`. brit must be able to perform that fast-forward update of
   an existing ref under `refs/notes/brit/devices/<did>/build/*` (the defect above refuses it today),
   or the publish script keeps using git and that fallback is stated.
4. A second landing from a linked worktree, whose notes publish updates existing device notes refs.

The decided push-policy lane (one handshake when no hook ran, the notes/tag fast-forward rule,
refusing an ungated push when hooks are configured but missing or not executable, rendering the
delivery note in the CLI) has not started; it is listed in `brit-sprint-3-handoff-2026-11-19.md`.
