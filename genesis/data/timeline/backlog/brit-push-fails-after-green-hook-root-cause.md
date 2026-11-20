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
  - "cycle-time-harvest-handoff | Cycle-time harvest from re-genesis 1 | sha256:38837fef544ae219 | path: genesis/docs/superpowers/sprints/2026-11-05-regenesis-1/briefs/cycle-time-harvest-handoff.md"
  - genesis/docs/superpowers/sprints/2026-11-05-regenesis-1/STATE.md
  - "task-brit-feedback-producer-report-20261111 | task-brit-feedback-producer-report | sha256:a226cd79248a91e0 | path: genesis/docs/superpowers/sprints/task-brit-feedback-producer-report.md"
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

## Fix on lane `sprint3/push-transport` (tip `4a60a61d`)

Merged on the integration branch `sprint3/integrate` (pushed head `bd28ecd8cf`). Not on brit `main`,
not pinned. Three implementation rounds, two independent reviews, a doc-link fix.

- The real cause is reported for a failure anywhere in the body. The push body is sent whole with
  Content-Length up to a bound (`http.postBuffer` when configured, else 64 MiB) and streamed above it.
  A cut upload above the bound reports "the transport closed while the pack was being streamed; the
  cause could not be read". Measured cost of buffering: a 200 MB push peaked at 702 MB RSS, against
  439 MB streamed and 396 MB for git.
- A 60 s deadline and stall guard on handshakes and fetches; no deadline on the push upload.
- A fresh connection after the pre-push hook, with re-validation that every ref's advertised id equals
  what the hook judged ("remote moved while the before-send callback ran; nothing was sent"). A ref
  whose requested id equals the id the hook judged is not re-validated: if it moved remotely, the other
  refs still go and that ref is reported not pushed.
- A ref already at the requested id is up to date.
- A push verified as delivered whose report was lost exits 0, with the note: "warning: the server's
  report was lost (<cause>). A fresh advertisement shows every ref this push named at its requested id;
  it cannot show whether this push applied them, whether server-side hooks ran, or what the server
  printed."
- Journeys on both backends with an HTTP fixture. The journeys file is Linux-only, and CI's
  test-journey job builds the max-pure binary for it.
- A CI step building the max-pure binary.
- Test and diagnostic knobs: `BRIT_HTTP_REQUEST_DEADLINE_SECONDS`, `BRIT_REQWEST_BIN`,
  `BRIT_REQWEST_REQUIRED`.

## Still open

- Every non-dry-run push handshakes twice and asks the credential helper twice, even with no hook or
  with `--no-verify`: the CLI callback returns `()`, which maps to re-validate. It stays so until
  `gitoxide-core/src/repository/push.rs` returns "send on this connection" when no hook ran.
- A fast-forward UPDATE of any non-`refs/heads` ref is refused before the hook ("refusing unverified
  fast-forward update of …"), so a combined branch+notes push fails whole. Creating notes works. The
  scratch example measured was a `refs/notes/brit/build/*` ref.
- A configured `core.hooksPath` whose directory is missing, or a non-executable pre-push, pushes
  ungated silently, as git does.
- Packs are undeltified.
- `--push-option` values are capability-checked but never transmitted.
- The curl backend hangs on an early error response with a large body (upstream code).
- The 60 s deadline also applies to fetch, clone and pull on max-pure builds, so a server silent for
  more than 60 s without keepalives now fails where git waits.
- `--atomic` is parsed but never requested from the server (`PushOptions.atomic` unused, no `atomic`
  capability sent): an "atomic" push is not atomic today.
- The CLI shows library wording ("before-send callback") where it used to say pre-push.
- Human-readable output labels refs already at their new id ` * [new/updated]` (porcelain is right).
- The lost-report warning is printed by the library itself, as a transitional measure.
- When signed push is implemented, its nonce must come from the second handshake.
- libcurl's retry of a POST on a dead reused connection is stopped only by the missing rewind
  callback; adding one would silently re-send the pack.

The two-handshake item and the items from the 60 s deadline onward belong to the follow-up lane in
`gitoxide-core/src/repository/push.rs`.

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
