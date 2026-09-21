---
id: "backlog-epr-head-envelope-differs-per-peer"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The EPR head envelope carries per-peer operational facts, so its CID and its validator differ between sibling doorways serving the same declared head"
slug: "epr-head-envelope-differs-per-peer"
written: "2026-09-21"
author: "serving-edge failover-balance-stream campaign, 2026-09-21 household reading of story 6.1"
status: "open"
priority: "medium"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "habit:doorway-failover"
  - "habit:dataplane-convergence"
tags: [epr-head, content-addressing, validators, cdn, doorway, swap-test]
---

**The fact.** `GET /epr-head/<id>` returns a JSON envelope whose `cid` is content-addressed over the envelope
itself, and the envelope includes two fields that are each peer's LOCAL view rather than facts about the content:
`distribution.replicaCount` and `updated`. Two peers that agree on the declared head, the content hash and the blob
therefore serve different envelope bytes, a different `cid`, and — since story 6.1 (`1103207e4`) derives the
validator from the exact body — a different `ETag`.

**Evidence.** Household, 2026-09-21 ~02:15Z, fresh cast on storage `210cb2c1…` / doorway `691e13e5…`, both doorways
converged on one head and one blob for `elohim-host-landing` (`sprint-report-household-20260921T021222Z-6b85560c`).
`GET localhost:8888/epr-head/elohim-host-landing` and the same on `:8889`, both 989 bytes, identical except:

| field | doorway A (matthew) | doorway B (jessica) |
|---|---|---|
| `cid` | `bafyreic4xiptapnrev…` | `bafyreicj5xgvx366wn…` |
| `distribution.replicaCount` | 3 | 2 |
| `updated` | `2026-09-21 02:00:30` | `2026-09-21 02:00:31` |

`ETag` A `"bafkreic7nnfhhx…"`, B `"bafkreicvnacfdn…"`. `If-None-Match` with A's validator: A answers `304`, B answers
`200`. The validator mechanism itself works as designed (`no-cache`, `Vary: Authorization, Cookie`, 304 on a match).

**Why it matters.** The doorway swap test says a client pointed at a different doorway gets the same content. For
the EPR head it does not get the same bytes, so a CDN or a client holding one doorway's validator re-downloads from
its sibling on every failover, and "one declared head serves the same bytes on every doorway" (the campaign's first
readiness line) is false at the envelope even when head, content and blob all agree. It also means the envelope's
`cid` names a peer's moment, not the content's head — two honest peers publish two CIDs for one head.

**Smallest next step.** Decide what the head envelope IS. If it is the content's head: move per-peer operational
facts (`replicaCount`, the local row's `updated`) out of the content-addressed body — into response headers or a
sibling operational view — so the body, the `cid` and the validator are functions of the declared head alone. If
the envelope is deliberately a per-peer observation, then its `cid` must not be presented as the head's address and
6.1's validator should be derived from the peer-invariant subset. Find the builder in elohim-storage (the
`/epr-head/{id}` handler and the view in `elohim-views`) and the `epr-head` view schema before choosing; this is a
p2p-design-gate question (what is content-derived identity here), not a header tweak.

**Links.** Story 6.1 and the readiness checklist in
`genesis/docs/superpowers/plans/2026-09-19-serving-edge-failover-balance-stream-campaign-plan.md`. Addressing canon:
`elohim/elohim-storage/CLAUDE.md` §Design Vocabulary (CID-first). Habit:
`doorway/doorway-service/.epr-meta/doorway-failover.habit.md` DELTA 2026-09-21a.
