---
id: "backlog-observation-signing-graduation"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Sign observations — device key at the repo node, agent key at the peer; resolve the as-asserted observer namespace"
slug: "observation-signing-graduation"
written: "2026-09-24"
author: "agent:implementer@claude-opus-5-5 (Lane A, task A9)"
status: "backlog"
priority: "high"
domain: "D2"
area: "elohim-storage observation layer + epr-cli device key"
tags: [observation, signing, attention, agent-private, observer-namespace, device-key, lane-a]
relatedNodeIds:
  - attention-witnessed-privately
  - acts-attributed-to-participants
cites:
  - genesis/docs/superpowers/plans/2026-09-25-post-station-4-participant-classifier-search-attention-sprint.md
  - genesis/docs/content/elohim-protocol/architecture/2026-05-11-observation-event-layer-design.md
  - elohim/elohim-storage/src/api/observations.rs
  - elohim/eprfs/epr-cli/src/device_key.rs
shift_objective: |
  Make every accepted observation carry a real signature, so the lifestream can drop its
  "signature absent" omission and the ack can say signed: "device" or "agent" instead of
  "absent". The repo node signs with the participant device key (Lane P); the storage peer
  signs with the conductor agent key. In the same pass, collapse the observer namespace split
  (browser JWT human id vs Tauri agent key) so one person has one observer_cid on their node.
  Done when POST /api/v1/observations stores a verifiable signature_b64, the stream view has no
  signature omission for signed rows, and a row posted from the browser and a row posted from
  the native shell by the same person appear under one observer.
---

# Sign observations and resolve the observer namespace

**The gap.** `POST /api/v1/observations` (ruling R-A2) accepts a person's own observation
with `signature_b64 = ""`. The ack names this plainly: `signed: "absent"` and
`observerCidNamespace: "as-asserted"`. The lifestream view (`GET
/api/v1/observations/stream`) carries the missing signature as a named `omissions[]` line, and
`/lamad/me/stream` prints it. The ack and the omission line are honest about the gap, but the gap is still there: a
row in the `observations` table is attested only by the storage peer that wrote it.

**The namespace split.** `observer_cid` is the `X-Agent-Cid` header, verbatim:

- on the browser path the doorway injects the JWT `human_id` (`uhCHk…` for a hosted
  registrant, or a `human-<name>` slug);
- the Tauri shell writes its conductor agent key (`uhCA…`).

The same person reading on two surfaces therefore has two observers, and their stream
splits in two. A2 named this instead of hiding it (`as-asserted`); resolving it belongs with
signing, because the key that signs decides which id is canonical.

**Direction.**

1. At the repo node, the participant device key (`elohim/eprfs/epr-cli/src/device_key.rs`,
   Lane P) signs the observation's canonical bytes. The roster says which human the key
   belongs to.
2. At the peer, the storage process signs with the conductor agent key that the hosted
   registration installed. The doorway already knows the binding from human to agent.
3. The ack's `signed` field gains `device | agent`. `observerCidNamespace` becomes
   `agent` once one canonical id is chosen, and the stream omission applies only to rows
   that are genuinely unsigned (legacy rows).

**Constraints.** No DHT entry type (R-A5). The row stays Private (B). The signature proves
authorship to the person's own devices, and it grants no reach. The retire-when of the habit
`attention-witnessed-privately` (a signed, persisted, agent-private-encrypted iroh log) is the
far end of this line. This atom is the first step toward it.
