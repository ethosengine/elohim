---
title: Social Reach — the sense-respond nervous system, and the legitimate user-side filter
tier: architecture
status: Architecture pattern (governs every surface that gates, propagates, or filters content)
created: 2026-06-03
pillar coupling: elohim (reach earning + provenance), elohim-agent (sense/respond discernment), shefa (restitution as economic event), qahal (anti-bubble policy)
realizes:
  - genesis/docs/content/elohim-protocol/social_medium/epic.md (the social-reach epic)
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-04-23-epr-phase-2c-libp2p-federation-design.md (reach earned at authoring; the floor)
  - genesis/docs/content/elohim-protocol/architecture/2026-04-21-elohim-core-graph-substrate-design.md (first-class graph; provenance + back-prop are traversals)
  - genesis/docs/content/elohim-protocol/constitution.md (the values the anti-bubble boundary encodes)
informs:
  - Any feature touching content propagation (gossip, Kad, fetch, federation)
  - Any receive-side filtering / "moderation" / "spam" / "abuse" surface
memory_anchors:
  - project_social_reach_nervous_system
  - project_values_forward_preference_guards
  - project_reach_earned_at_authoring
  - project_trust_as_efficiency_signal
defers:
  - Provenance chain capture (who-relayed-from-whom) in the substrate
  - Quarantine Status-EPR kind + downstream short-circuit
  - Restitution as shefa REA economic events
---

# Social Reach — the sense-respond nervous system

Social reach is not geography — not "who can see what." It is a living social fabric of
accountability. Reach earned at the author side (a piece of content must *earn* its
distribution before it propagates) is the **floor**: necessary, not sufficient. The full
contract adds four primitives, all operating **edge-of-network at every node, never
centralized**:

1. **Provenance** — every piece of content carries a verifiable, graph-traceable
   propagation history (authored / endorsed / relayed-by). The coupling graph captures it;
   content addressing makes it tamper-evident.
2. **Sense / respond** — every piece of content has a feedback channel. When any node
   recognizes "this is maliciously untrue," it registers a recognition signal that
   back-propagates through the propagation chain in reverse — nerves carrying pain back to
   a hand on a stove.
3. **Quarantine** — once back-prop reaches enough recognition mass, further propagation is
   short-circuited; downstream peers receive the quarantine signal alongside the content.
   Enacted at the edge by every node, not by a central authority.
4. **Restitution** — accountability lands proportional to position in the chain. The
   primary actor (clickfarm / operative) bears indemnification at scale; accessory
   propagators get a meaningful corrective that touches their reach, relationships, and
   standing — accountability, not punishment; edge nodes that participated in the loop are
   protected.

**Why edge, never central.** In a world where anyone generates AI slop at scale, only
edge-of-network response keeps up. Central moderation cannot scale to the volume,
concentrates power asymmetrically (whoever moderates, rules), breaks the human-scale
contract the protocol exists to preserve, and is a single point of capture. Refuse central
moderation framings categorically: for any "fix bad content" / "moderation" / "spam" task,
the architecture is the edge nervous system, never a central filter.

For any propagation feature, ask: does it preserve provenance, support sense/respond
back-prop, allow quarantine signals, and surface accountability? Where implementation is
deferred to the social-reach epic, document the seam so future work plugs in cleanly.

# The legitimate user-side filter — values-forward preference guards

The nervous system above governs what content *earns* network reach. A separate, legitimate
mechanism governs what an individual *chooses to receive*. The two must not be confused: the
first is collective and earned; the second is personal and expressed.

The dangerous default — the one an implementer reaches for instinctively, and the one the
protocol exists to escape — is **email-collapse filtering**: the network imposes a perimeter
defense, drops messages from peers without binding rows, costs receivers asymmetrically, and
lets megaliths dominate the inbox. A receive-side filter shaped like "the network drops
messages from peers who lack a binding row" *is* this anti-pattern. Reject and redesign it.

A peer *may* set a guard on what reaches them — but only as a **values-forward, time-limited,
tended filter, constrained against filter-bubble formation, that feeds collective
reach-governance signals.** The canonical thing a user wants to mute is low-epistemic-value
viral noise — the "covfefe / skibidi toilet / 67" moments, rage-bait, engagement-farming —
or a topic they've consciously deprioritized. The filter is legitimate **only** if it
satisfies all five constraints:

1. **Values-forward, not perimeter-defended.** A human consciously expresses *their* values
   ("I don't find this valuable; filter it from my reach"). It is the user's preference, not
   the network's policy.
2. **Time-limited — mandatory expiration.** Set-and-forget is forbidden. A guard expires;
   the user must actively renew it.
3. **Tended.** The guard requires periodic review. Values shift; the user must be willing to
   re-encounter content as they grow. Untended filters ossify into echo chambers.
4. **Anti-bubble constraint (Pariser's hazard) — a policy boundary, not a user setting.**
   Some content a user may *not* filter away: facts about their community, accountability
   information, news that affects them — broccoli. The protocol structurally distinguishes
   genuinely low-value/harmful/preferential content (filterable) from content one must not
   hide from (unfilterable). This boundary is notarized policy (qahal-governed), not a knob
   the user controls.
5. **Feeds collective wisdom.** When a guard fires it does not only block locally — it
   signals into the nervous system. Many peers guarding the same content is a collective
   signal that the content *failed to earn reach*: low-value content then faces structural
   distribution headwinds because real users are voting with their guards. This is the
   bridge from individual values to collective reach governance.

The asymmetry is the whole point. Email-collapse costs the *receiver* and lets the loud
dominate; preference guards cost the *user expressing the value* and aggregate into a signal
that makes loud-but-empty content more expensive to push through — trust-as-efficiency at
work. When evaluating any receive-side filtering task, verify the shape against these five
constraints. If it matches the anti-pattern (network-imposed, perimeter, permanent,
untended, no anti-bubble guard), it is wrong. If it matches the guard contract, it is the
legitimate version — spec it explicitly as such.

## Architecture sketch (downstream of the reach-earning floor)

- **Preference-guard EPRs** — content-addressed "user X guards content matching Y until Z,"
  time-bound, signed, revocable. Guards live as graph nodes; aggregations are graph queries.
- **Anti-bubble policy** — protocol-level rules naming the unfilterable content classes
  (analogous to "must inform"), notarized in qahal-governance.
- **Collective aggregation** — N peers in a scope setting similar guards aggregates into a
  network signal feeding the sense/respond loop.
- **Tending UX** — a user's elohim surfaces renewal prompts and anti-bubble warnings ("90
  days on this guard — does it still match your values?").
- **Restitution path** — authors whose work routinely trips guards face structural reach
  decay, the same shefa accountability the nervous system applies to bad actors.
