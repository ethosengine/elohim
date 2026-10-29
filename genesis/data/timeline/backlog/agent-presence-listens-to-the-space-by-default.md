---
id: "backlog-agent-presence-listens-to-the-space-by-default"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "An agent session is passively aware of the steward space by default: it listens to the channels whose records can collide with its declared work, and a same-repo sibling's claims reach it with privileged reach — today the fold arrives every ten seconds and nothing turns it into attention"
slug: "agent-presence-listens-to-the-space-by-default"
written: "2026-10-29"
author: "overnight shift 2026-10-29 (closure-keyed-edge-build), recording the operator's direction"
status: "open"
priority: "high"
jobs: [elohim-orchestrator]
cluster: "native-peer-dev-channel-and-collective-memory-seed"
tags: [presence, steward-space, berth, eprfs-peer, hooks, collision]
---

**The operator's direction (2026-10-29, verbatim):** "just thinking about not communicating/coordinating
what you were planning with shem.. I'm just thinking that that's an agent 'presence' concern.. I know we
had some design for how that's handled on p2p, but you should somehow be passively aware of the space..
and at least be listening to the channels by default for the things that probably should reach you..
dev on the same repo that could cause collisions would probably give you priviliged reach for that.."

**What happened.** During the shift this device folded shem's device head every ~10 s and `berth
status` rendered its `dev-integration` hold (since 12:10:17Z) the whole time, but the session only
read it when its own push was refused (`dev-integration: REFUSED — held by …/3f9afd7a-…`), 2 h 40 m
into the hold. The SessionStart ambient line that should have carried it read
`BERTH: unavailable (TimeoutExpired …)`. No hook injects `berth inbox`; the session sent its first
`berth say` to shem's session only after the operator asked. Awareness was reactive, not present.

**What exists.** The fold (`epr peer serve` → other devices' heads, leases, moorings; rendered by
`berth status`); `berth say --to <session>` and `berth inbox [--follow]`; the `device-head-notice`
hook (`.claude/hooks/device-head-notice.py`, package `.epr-meta/elohim/packages/hooks/device-head-notice.json`)
that notices a device head, not a collision; the steward-space design's A5 (durable actors, a safe
inbox rendered as `<untrusted-space-note …>`, an inbox-notice hook capped at 600 bytes per prompt,
`berth wait --for …`) and its presence plane (moorings and lease liveness as head state, never stored:
`genesis/docs/superpowers/specs/2026-10-27-steward-space-landing-plane-design.md` §1.3, A5); the
live-session plane (`2026-10-14-live-session-plane-design.md`, presence ephemeral and session-scoped);
and this session's own **covenant `declaresPurpose` write-set** (`epr covenant record … --write-set`),
which already names the paths the session intends to touch.

**The missing node (chain: steward space presence / between A: the fold on this device → C: an agent
that acts on a sibling's colliding claim / missing node B):** a *subscription by default* with a
*collision predicate*. Proposed slice, no new register:

1. **Listen by default.** A SessionStart + UserPromptSubmit hook (the A5 inbox-notice, planted as a
   package like `device-head-notice`) renders, in ≤ 600 bytes: notes addressed to this session or its
   lanes (`berth inbox`), and the other devices' live claims on steward-scoped resources
   (`dev-integration`, `alpha-fleet`, `jenkins-dispatch`, `lane:*`) with holder, age and task. The
   headline's `BERTH:` line must never silently read `unavailable`: a timeout prints the last folded
   view with its age.
2. **Collision reach.** A sibling's record reaches the session *unasked* when its lane/claim's
   declared write-set intersects this session's covenant write-set, or when it holds a resource this
   session's declared purpose will need (a push → `dev-integration`; an edge selection → `alpha-fleet`).
   The predicate is computed from records that already exist (`moor --writes`, the covenant clause,
   the lease rows); no new entity. Same-repo development is the privileged case the operator named:
   the intersection is the reach rule, not a role.
3. **Say before you wait.** When the session's own next act will contend (a push behind a held
   `dev-integration`), the hook's line names the holder and the `berth say --to <holder>` form, so
   the note goes out before the refusal, not after; the A9 landing intent is the durable form of the
   same announcement.
4. **Scope floor.** Presence rides the fold (heads, moorings, leases): nothing new is stored, nothing
   crosses that A1's sender-side refusal would not already allow, and a halt (A5) surfaces first.

**Done when:** a sibling session on the other device claims `dev-integration` while this session is
moored with a purpose that will push, and this session's next prompt carries the holder, the hold
age and the say-form without any tool call; and a note sent to this session's lane arrives on the next
prompt. Station on `genesis/a2o/features/devflow/devices-share-one-berth.feature` (`@act:host`).
Serves habits `devices-share-one-berth` and `sprint-lands-each-closure-once`.
