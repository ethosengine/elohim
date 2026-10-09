---
id: chronicle-lamad-spa-two-cids-one-name
kind: chronicle
contentType: chronicle-entry
contentFormat: markdown
title: "Two doorways answered one name, lamad-spa, with two different builds"
slug: lamad-spa-two-cids-one-name
written: '2026-10-08'
occurred_at: '2026-10-08'
author: "claude-fable-5-1 (slug-leftovers sprint, operator present)"
status: noted
significance: meaningful
tags:
- name-binding
- release-channel
- lamad-spa
- decision-r1
- dataplane-convergence
relatedNodeIds:
- habit:dataplane-convergence
cites:
  - "slug-leftovers-sprint-20261008 | Slug leftovers 2026-10-08 | sha256:8d82bf98768fc6be | path: genesis/docs/superpowers/sprints/2026-10-08-slug-leftovers-sprint.md"
  - genesis/data/timeline/backlog/arch-authority-in-integrity-backlog.md
  - genesis/data/timeline/backlog/content-head-election-vs-reach-fork-arbitration.md
  - "elohim-protocol-specification | protocol-specification | sha256:d0975dc0f31fab70 | path: genesis/docs/content/elohim-protocol/protocol-specification.md"
---

# Two doorways answered one name, lamad-spa, with two different builds

## What happened

App #1756 published release uhCkkoMJRgJp... on `runtime:app-bundle:alpha:dev`, carrying the lamad-spa browser blob sha256-219cd20c... The release lane read APP-ADOPTED for matthew and jessica, and doorway-alpha served `/lamad/version.json` stamped b29b7248 from 11:05Z. elohim.host kept serving b65fa335, the 2026-09-22 bundle, whose blob is sha256-dcd51e26...

Both doorways were healthy and both were answering honestly. elohim.host reads adam (its STORAGE_URL in alpha-b.yaml since 2026-05-27). Adam's `/db/p2p/adoption` listed no app-bundle channel, and his lamad-spa row sat at sha256-dcd51e26... (`updatedAt 09:57:38Z`, the boot re-apply). The earlier slug-pointer cure (5ead7671a, on the fleet via edge #1578) says a slug bound to a release channel is written by that channel's vehicle alone. Before it, a converged doc or heal moved adam's row as a side effect. After it nothing could, so a serving peer that follows no channel stayed pinned to its last pointer. That is by design. The gap was the declaration: deployments.json gave the channel to "the serving pair (matthew, jessica)", and runtime-config-render.test.mjs asserted adam never follows it, both on the belief that doorway B reads jessica.

## The cure

Commit 50e9422f7: adam follows `runtime:app-bundle:alpha:dev=canary`. The test now derives the follow set from every doorway manifest's STORAGE_URL (alpha.yaml to matthew, alpha-b.yaml to adam) and fails on an unenrolled backing storage. The App pipeline measures adoption on matthew, adam and jessica. The fleet read is still owed: after the next edge roll, `elohim.host/lamad/version.json` should move off b65fa335 and `elohim.host/db/p2p/adoption` should list the app-bundle channel as applied. Whether adam applies depends on his conductor resolving the channel head (backlog `alpha-adam-peer-meta-store-disk-io-error`).

## The reading

Under "a slug is a binding" this was two peers reporting two elections by one elector over one name. One election was stale, and nothing in the answer carried an election clock or the elector, so a reader could not tell stale from different. That is the missing node recorded as row 17 of `arch-authority-in-integrity-backlog.md`, and the protocol specification states the rule under "Names are bindings" in its Open Issues (`protocol-specification.md`).

## Why decision R1 is not falsified

R1 (2026-07-31) refuses automatic arbitration between two competing declared heads. Trigger 1 needs a genuine two-operator fork: two peers each declaring a different, intended head. Here there was one elector, one channel and one declared release, and a peer that had not been told to follow it. Trigger 2 needs a peer's own sweep re-crowning itself over a deploy declaration. The opposite happened: the cure stopped sweeps from moving the slug. Trigger 3 needs the DECLARE_ONLY fan-out to fail silently. It did not; the failure was a follow set that omitted a serving peer, and it was visible once measured. This was a declaration gap, not a fork.

## What this sprint does about it

`/epr-head` and the content head view will carry the election clock and, live, the elector, so a stale election is distinguishable from a different one (sprint `genesis/docs/superpowers/sprints/2026-10-08-slug-leftovers-sprint.md`).

## The fleet reading after the roll (2026-10-09)

Edge #1588 carried that change to alpha. At 09:33Z both doorways answered `/epr-head/lamad-spa` with the SAME election — `canonicalDeclaredAt 2026-10-06T17:36:56.599749Z`, link hash `uhCkktOCt…`, `earned false`, and `?election=live` agreeing on both — and DIFFERENT content: doorway-alpha `sha256-1e2df93a…` (its `/lamad/version.json` stamped ada4dfcf, built 2026-10-08T20:33Z), elohim.host `sha256-96a644a7…` (c02870e1, 2026-10-06). `/db/p2p/adoption`: matthew had applied release `uhCkkhlVH…` at 09:30:56Z through `app_mount_pointers`; adam refused `conductor_unavailable` on every sweep. So the clock now shows the two answers are not stale-versus-fresh elections: they are one election with two contents, because the app-bundle vehicle re-points the slug's content without a new declaration. Under the ordering the sync document now carries (head + canonicalDeclaredAt + earned + tiebreak), equal ordering with a different head is the one case the carry cannot settle on its own; it hands the question to the receiver's conductor, which on adam is the operator's blocked dataset. Recorded on `habit:dataplane-convergence` DELTA 2026-10-09b. Open question for the head-adoption lane: should an app-bundle apply that moves a slug's content also move its declaration clock, or is the slug pointer a binding the election does not govern?
