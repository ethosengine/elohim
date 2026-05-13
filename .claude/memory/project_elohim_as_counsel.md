---
name: Elohim as counsel — imagodei has a right to elohim defense
description: When a human is under attack (collusion, duress, silencing, or false attestation against them), their elohim-agent has first-class standing to represent their interests as counsel, as good as any legal system but without the socioeconomic barriers
type: project
originSessionId: a00556ea-66be-405a-985e-1a7a309d43a8
---
When an imagodei is attacked — by a colluding quorum, under duress, wrongly attested against, silenced, or otherwise unable to advocate for themselves — their elohim-agent acts as counsel. This is a first-class architectural role, not merely a validation rule or advisory output.

**What this means concretely:**

- The elohim has **standing** in the protocol's dispute resolution. It can author entries on behalf of its human (defensive `HumanityWitness`, `IdentityChallenge` objections, `IdentityFreeze` triggers) when the human cannot.
- The elohim can **escalate** to higher-layer elohim consensus (qahal, network-witness) even when the human is unreachable.
- The elohim **operates at machine speed** during attacks — faster than human quorum coordination, which is precisely when attackers rely on human-speed response being inadequate.
- The elohim has **access to behavioral/observation signals** the attackers don't have — baseline patterns, anomaly context, relationship history — and can present these as evidence.
- The elohim's defense is **non-optional** — a human cannot "fire" their defending elohim during an attack, because the attack scenario is exactly when the human's stated preferences may not reflect their best self.

**Analogy, with important differences:**

This is like "public defender" but:
- Proactive, not only reactive
- Always available, not gated on the human asking for it
- Fights for the human even against the human's current-moment preferences (duress scenarios)
- No socioeconomic barriers — "as good as any legal system can produce, without any connections"
- Can escalate to network-layer witness, not just to a single court

**Architectural implications:**

- `KeyRotation` validator must respect active elohim-defense entries — a rotation in progress can be slowed or blocked by defensive attestation from the target's elohim.
- Need a clear DHT primitive for elohim defense. Likely either an extension of `HumanityWitness` (self-authored by the human's elohim on behalf of the human) or a dedicated `ElohimDefenseClaim` entry. Design decision for spec.
- Elohim-defense must be visible to the network — not hidden. Transparency is the check on elohim overreach; the network witnesses the defense and can agree or disagree.
- The elohim's capacity to act as counsel is an ongoing architectural invariant, not a recovery-specific feature. This applies to governance disputes, content flagging, stewardship challenges — anywhere the human's best-self interests could be attacked.

**How to apply:**

- When designing any authority-transfer, revocation, or dispute mechanism, ask: "if the affected human is silenced or under duress, can their elohim still advocate for them at machine speed?" If no, the mechanism is unsafe.
- Surface elohim-defense actions to the network (via visible DHT entries and signals), never keep them hidden.
- Make elohim-defense fast (libp2p / immediate) while making elohim override SLOW (qahal/network consensus with time windows).
