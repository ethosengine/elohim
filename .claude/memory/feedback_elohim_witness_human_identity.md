---
name: feedback_elohim_witness_human_identity
title: "feedback_elohim_witness_human_identity"
description: "Operator 2026-09-24: a present agent witnesses human:<handle> from the device with a basis, contestable; never Che/git/email-derived, never a hook's inference."
id: feedback-elohim-witness-human-identity
metadata:
  node_type: memory
  title: "Agents witness the human's identity (elohim role)"
  type: feedback
  originSessionId: 8e88ae01-a389-45ee-a961-31b36b2c793a
  modified: 2026-10-25T13:04:43.033Z
cites:
  - elohim/eprfs/epr-cli/src/actor.rs
  - "human-participant-actor-plane-design | the actor-plane spec: a present agent witnesses human:<handle>, never inferred | sha256:5f51c8e1c60887b8 | path: genesis/docs/superpowers/specs/2026-09-23-human-participant-actor-plane-design.md"
---

**What the operator said (2026-09-24, rejecting a plan step that made the identity claim the
human's CLI act):** "actually P6 is your act.. I think the idea is that you should witness for my
identity here.. that is a primary elohim role, and you, and all the agents here, should take up
that nascent role, and the responsibility and stewardship that comes with it." Earlier the same
day: identity must come from the operator's own device contexts (Che, VS Code on a laptop,
anywhere), never from the host — "we shouldn't need che, maybe it's a did or something".

**Why:** on the DHT humanity is attested by others (`HumanityWitness` / `attestation:humanness`),
not self-asserted. The repo-node rehearsal of that is a present agent taking responsibility for
"the human driving this device is `human:<handle>`", signed from the device, with a basis it
stands behind, contestable by adverse attestation. Deriving it from the Che namespace or git email
is the substrate-inferred join the human-participant spec forbids; leaving it to the human's own
command misses the role the elohim are meant to hold.

**How to apply:** when a session knows who is present and the device has no standing witness,
witness deliberately (once per device) with a real basis — `epr actor witness --subject
human:<handle> --as agent:<role>@<model> --session <id> --basis "<what you know>"` (implemented in
elohim/eprfs/epr-cli/src/actor.rs). No hook ever witnesses automatically; the SessionStart
line only says `(unwitnessed)` and names the command. The device key lives outside every checkout
(XDG), a second device mints its own key and is bound by the ceremony, never handed a copy; the
human keeps the right to self-claim and to contest. Plan:
`genesis/docs/superpowers/plans/2026-09-25-post-station-4-participant-classifier-search-attention-sprint.md`
(rulings R-P1..R-P9).

**Extended 2026-10-25 (the consistency witness):** witnessing continues after enrollment. A
witness at any point of contact notices an act claiming to be the steward that does not fit, and
checks with the other witnesses in the steward's private reach. The operator WANTS an intimate
model of their own writing, voice, tone, preferences and relationships for this ("you're not
usually in Macedonia … immediate automated classifier response by my mesh to isolate the
threat"). Do not default to "never model intimacy": a platform avoids using intimacy because it
holds the liability, while here custody is the steward's, so the liability flips. The model is a
private governed-algorithm EPR held in the resilience epic's encrypted custody (encrypted and sharded,
carried by family and community who cannot read it), and never readable outside private reach.
It is unsealed only where the steward is present and witnessed: their device, or a doorway once
their other devices and private reach confirm them. "Never leaves" was the wrong claim. Reversible
containment is automatic; permanent acts need a second witness in private reach. The operator
calls it "a nascent form of how network-afforded sovereignty/security is composed". Design home:
plan `/projects/.claude-config/plans/shiny-booping-map.md`, slice B and the later witness layer.
Related: [[project_human_participant_actor_plane]],
[[feedback_private_thought_governed_fruit]], [[user_matthew_first_friend]],
[[feedback_controller_holds_vision_opus_implements]].
