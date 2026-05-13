---
name: Social reach is a sense-respond nervous system — provenance, feedback, quarantine, restitution
description: Reach earning is the floor; the full contract requires a network nervous system — provenance traceable, sense/respond feedback back-propagating through propagation chains, quarantine of bad actors, and restitution. Edge-of-network response at every node. This is the social-reach epic.
type: project
originSessionId: f534b7ae-d435-4ab8-ab3b-f7d23b6b0ed9
---
**The aunt-and-rage-bait scenario (canonical example):**

An aunt reshares rage-bait — a harrowing picture captioned "white slavery in America." If anyone bothered to look, the figures are wearing wooden shoes — a dead giveaway the picture is from Western European mining history. The aunt doesn't care; the feeling of grievance feels true to her, and what IS true does not matter. She redistributes the falsehood. Everyone in her network now has to process the slop.

**Three layers of responsibility:**

1. **Primary** — the clickfarm / political operative who created the content. Bears the deepest accountability.
2. **Accessory** — the aunt. By redistributing without verification, she becomes an accessory to the harm.
3. **Network** — every downstream peer that propagates becomes accessory in proportion to how they relayed.

**The architecture this demands — a nervous system at the edge:**

Reach earning at the author side (memory pin `project_reach_earned_at_authoring`) is the **floor**. It is necessary but not sufficient. The full social-reach contract requires four primitives, all operating edge-of-network at every node — never centralized:

1. **Provenance** — every piece of content carries a verifiable, graph-traceable propagation history. Who authored it. Who endorsed it. Who passed it along. The graph topology captures this. Content addressing makes it tamper-evident.

2. **Sense / respond — the network nervous system** — every piece of content has a feedback channel. When ANY node in the propagation graph recognizes "this is maliciously untrue," it registers a recognition signal. That signal back-propagates through the propagation chain in reverse — like nerves carrying pain back to a hand on a stove.

3. **Quarantine** — when back-prop reaches enough recognition mass, the bad-actor content is contained: further propagation is short-circuited; downstream peers receive the quarantine signal alongside the original content; the content is marked for removal/correction. Quarantine is enacted at the edge by every node, not by a central authority.

4. **Restitution / responsibilities** — accountability lands proportional to position in the chain:
   - Primary actor: indemnification, responsibility, restitution at scale to the harm caused
   - Accessory propagators: a meaningful corrective signal — like the aunt learning "don't touch the stove" — that touches their reach, their relationships, their standing. Not punishment, but real accountability that affects future propagation.
   - Edge nodes: protected because they participated in the sense/respond loop

**Why edge-of-network, not central moderation:**

In a world where anyone can generate AI slop at scale, only edge-of-network response can keep up. Central moderation:
- Cannot scale to the volume
- Concentrates power asymmetrically (whoever moderates, rules)
- Breaks the human-scale contract the protocol exists to preserve
- Is a single point of capture / failure

Edge response:
- Every node participates in sensing
- Every node participates in responding
- The cost is distributed among those who derive value
- No central choke point can be co-opted
- The protocol stays workable for households / individuals / small collectives

**Why this is THE core of the social-reach epic:**

"Social reach" is not geography (who can see what). It is a living social fabric of accountability. The four primitives — provenance, sense/respond, quarantine, restitution — turn reach into something that can self-heal. Without them, reach earning is just an admission gate; with them, reach is an ongoing relationship between author, content, propagators, and the network as a whole.

**How to apply:**

- For any feature touching content propagation (gossip, Kad, fetch, federation): ask whether it preserves provenance, supports sense/respond back-prop, allows quarantine signals, and surfaces accountability. If it doesn't, design extensions clearly even if implementation is deferred to the social-reach epic.
- For any "fix bad content" / "moderation" / "abuse" / "spam" task: refuse central moderation framings categorically. The architecture is edge-of-network nervous system, never central filter.
- Reach earning (Phase 2B Batch D.4) is the floor; the four primitives are downstream epics. Document the seam so future work plugs in cleanly.
- Provenance lives in: signed envelopes (who authored), coupling graph (knowledge ↔ value ↔ governance), propagation chain (TODO: capture who-relayed-from-whom in the substrate).
- Sense/respond likely lives in: elohim-agent specialist subagents (defenders, advocates, gate-discerners — see memory pin `project_elohim_subagent_specialists`), plus a feedback EPR kind that back-propagates through coupling refs.
- Quarantine likely lives in: a Status-EPR kind (or Attestation kind) that revokes downstream propagation; receiver storage projects this and short-circuits further fetch/serve for quarantined content.
- Restitution likely lives in: shefa (mutual credit / REA economic events) — accountability becomes a real economic signal, not just a moral note.

**Connection to existing memory pins:**

- `project_reach_earned_at_authoring` — earning is the floor of this architecture; the nervous system is the ongoing contract
- `project_first_class_graph_pattern` — provenance + back-prop are graph traversals; the protocol must first-class the graph
- `project_elohim_as_counsel` — when humans are attacked / under duress / accessory to harm, their elohim represents them in the sense/respond loop
- `project_ungrudging_service` — quarantine + restitution must operate without grudging; the protocol heals without requiring acknowledgment from those it corrects
- `project_three_layer_truth_model` — DHT notarizes provenance + quarantine attestations; libp2p propagates feedback signals; doorway projects accountability views
