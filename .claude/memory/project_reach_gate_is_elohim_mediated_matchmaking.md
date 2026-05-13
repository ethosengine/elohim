---
name: Reach-earning gate is elohim-mediated matchmaking, not a binary policy check
description: The full gate is an elohim agent reading newcomer's imagodei against a collective's BYO manifest, producing welcome + sponsor suggestions ("you'll fit, talk to Adam Rachel and Susan"). The substrate gate is the deterministic floor underneath.
type: project
originSessionId: 42abe5eb-4a48-4a2a-8142-604a4c7a1bd3
---
The reach-earning gate has two layers that must NOT be conflated:

**Substrate layer (this is what Phase 3.5 wires):**
- Pure deterministic evaluator: `(author, subject, requested_reach, conn) → ReachVerdict`
- Reads Standing + manifest floor classes + quarantine state
- Ephemeral compose-time evaluation, no DHT entity, no local table
- Returns structured verdict: `{ decision: Allowed | Blocked | Pending, reason, floor_class_match, evidence_summary }`
- `Pending` = "I cannot decide alone; requires discernment"
- For Phase 3.5 lifetime, `Pending` collapses to `Blocked` until the elohim layer exists

**Discernment layer (future sprint, NOT this one):**
- An elohim agent reads the substrate's `Pending` verdict + the newcomer's imagodei (psyche, traits, journaling) + the collective's BYO manifest + the collective's existing membership graph
- Produces relational matchmaking: "yes you fit here, talk to Adam, Rachel, and Susan — they'll resonate with what you bring"
- Suggests sponsors who can vouch on the newcomer's behalf
- May trigger Vouch entries to elevate Standing
- Composes with project_elohim_subagent_specialists (a specialist elohim role)
- Composes with project_collective_is_stewardship_unit (each collective has its own BYO manifest)

**Why this distinction matters:**
- The substrate gate must NEVER hard-block on "no information yet" — Unknown collapses to Pending, not Blocked
- The verdict shape must be designed for the future elohim layer to consume (structured, with evidence summary, not boolean)
- Without this distinction, a substrate-only gate becomes a sponsor-friendly first-mover-advantage — newcomers from disconnected social graphs are stuck
- This is the constructive answer to "how does Standing::Unknown not become a permanent purgatory"

**How to apply:**
- Design ReachVerdict as a structured type from day one, even if the consumer is just a boolean check
- Reach-earning gate returns Pending freely; the caller decides how to handle (compose-path treats Pending == Blocked at substrate alone; elohim-mediated caller treats Pending as "engage discernment")
- BYO collective manifests are first-class; reach gate consults the *collective's* manifest, not just the global protocol manifest
- Reach gate is pure (no persistence); persistence is on the discernment side (Vouch entries, elohim attestations)
