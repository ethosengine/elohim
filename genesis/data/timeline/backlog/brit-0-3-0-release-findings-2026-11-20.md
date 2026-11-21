---
id: "backlog-brit-0-3-0-release-findings-2026-11-20"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Brit 0.3.0 release: what landed, and the gaps the release run found"
slug: "brit-0-3-0-release-findings-2026-11-20"
written: "2026-11-20"
author: "claude-opus-5-5 (brit 0.3.0 release, session 46c39d0c, steward of record human:matthew)"
status: "open"
priority: "medium"
relatedNodeIds: []
tags: [brit, release, daily-driver, ci, dialogue-stasis, shem]
---

# Brit 0.3.0 release findings (2026-11-20)

Session `46c39d0c` (shem), steward of record human:matthew. The steward's purpose: pull in all the
Brit work, run on a current epr, integrate, run a Brit backlog sprint and release (rulings: Brit
release, version 0.3.0, the four sprint items below; no other monorepo lanes this pass).

## What landed

Brit `main` fast-forwarded from `e65a9158e0` to `4eb2e8cc0b` (`chore(release): brit 0.3.0`):

- `325faff2a1`: merge of sprint 3 (`run-ci/sprint3-integrate` `6507caaaa1`, twelve lanes) onto
  main (0.2.6 + the 468-commit gitoxide upstream integration). Merge head `91942dfd7`, run
  44798244116, Tests pass green.
- Four sprint lanes from the sprint-3 handoff's queue, each green on its own `run-ci/lane-*` run:
  push policy (`319d8d0a1`, run 44828926787), closed stdout (`ae5931b52`, run 44820023096), log
  encoding and `-C` wording (`641ec752b`, run 44818923773), CI aggregate and journeys (`c27ea7e94`,
  run 44811229071).
- Release head `4eb2e8cc0b`: run 44839003249 Tests pass green; nonpublishing release
  qualification run 44839064593 green. Release notes: `docs/releases/brit-0.3.0.md` in the brit
  repository.

The sprint-3 handoff (`brit-sprint-3-handoff-2026-11-19.md`) decisions 1 (land 6507caaaa1), 2 (move
the pin) and 6 (CI aggregate and journeys) are taken; queued items 5, 6 and 7 are delivered.
Decision 6's "add `test` to `Tests pass`'s needs" did not apply: no such job exists on this head;
the unit-test legs are matrix legs of `run`, already needed.

## Open, found by this run

| # | Finding | Evidence | Next |
|---|---|---|---|
| 1 | The monorepo's "Git for landing pushes" exception (root CLAUDE.md) is not yet retired. The transport fault behind it was fixed upstream of this release; this release adds the delivered-push, one-handshake and ungated-push refusals. | lane push-policy evidence `genesis/local-dev/release-20261120/lane-push-policy/evidence.txt`; test `delivered_push_succeeds_when_its_reader_closed_the_report_stream` | After the 0.3.0 pin, run the supervised real landings the push root-cause entry lists with `brit push`, then retire the exception. |
| 2 | `cargo-deny` advisories red (pre-existing on main): heapless unsound reports (fix: >= 0.9.4), chrono unmaintained, one with no safe upgrade. | run 44839003249 `advisories@linux-x86_64` | A dependency lane: `cargo update -p heapless` and its publish bumps; decide chrono. |
| 3 | `size@linux-x86_64` red: 409.5 KB against a 225 KB budget (320.5 KB before the upstream merge; main run 44549281637). | runs 44798244116, 44839003249 | Re-baseline the budget or cut size; observed, not gating. |
| 4 | The deferred `tests@windows-x86_64` leg (readiness.json "observed, not maintained", `blocking: false`) is a leg of `run`, so `Tests pass` cannot conclude until it finishes: 60-90+ min after every blocking leg was green, on every run this session. | runs 44788638161, 44811229071, 44818923773 | Move deferred legs to a job `Tests pass` does not need, or let the aggregate conclude when only non-blocking legs remain. |
| 5 | `brit config` has no setter: every lane needed `git config --local branch.<b>.britReachBase` before its first `brit push`. | four lane notes under `genesis/local-dev/release-20261120/` | A `brit config set` (or a `brit push --reach-base`). |
| 6 | The rust-style checker the reach hook expects was stale in the brit worktrees; lanes pushed with `BRIT_RUST_STYLE_PREFIX` pointing at a freshly installed prefix. | `genesis/local-dev/release-20261120/rust-style-prefix` | Let the hook install or locate a current checker itself. |
| 7 | Dialogue-stasis false positive: an answered AskUserQuestion opened an incident with correlation `low/none`, a keeper consultation ran while the steward was answering, the prompt closed by inference, and further consultations ran after it closed, relaying "parked, request PushNotification" repeatedly. Same on the second question (incident :25). | `.eprfs/status/stasis/46c39d0c-…/events.jsonl` seq 1-13; `.claude/hooks/dialogue-stasis-sense.py` PermissionRequest/PostToolUse correlation | Treat an AskUserQuestion dialog as a human answering, not an authorization stall; never consult on a closed incident. |
| 8 | The Claude process inherited `EPR_BIN` pinned to epr 0.3.2 from its launching shell, so hooks ran an epr without `feedback` and `session commit-trailers` (attribution UNMEASURED) while the store's head was 0.3.3. The legacy Python `bin_store.py` left in `/projects/.claude-config/tools/bin-store/bin/` still answers `bootstrap` and refuses two-bin registries (`debug/epr/epr`). | `genesis/local-dev/release-20261120/session-evidence.md` | Arrival should name a stale inherited `EPR_BIN`; retire the legacy Python copy from the store's `bin/`. |
