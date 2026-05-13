---
name: Elohim as specialist subagents — context-bound and ephemeral
description: Elohims are not monolithic agents; they spawn as specialist subagents with focused context (e.g., a defender subagent with deep imagodei profile access) and are governed by constitutional disclosure rules
type: project
originSessionId: a00556ea-66be-405a-985e-1a7a309d43a8
---
Elohims are LLMs, so they're context-bound and ephemeral by nature. An elohim isn't a single persistent agent that "knows everything about Matthew" — it's a pattern of specialist subagents spawned with focused context for specific responsibilities, each with its own manifest declaring inputs/outputs/disclosure rules.

**Concrete shape (2026-04-25 clarification):** A specialist is a **snapshot/fork of the human's imagodei context memory + a system-prompt wrapper** declaring the specialist's role and relationship to that context. Same shape as a chat agent on claude.ai today — base model, all the conversational context that has accumulated, and a role-shaping system prompt. "Spawning a defender" = take the imagodei context, fork it, apply the defender system prompt, give it standing to author specific DHT entry types.

**Examples of specialist roles:**

- **Defender specialist** — spawned when an attack is detected on its human. Reads the imagodei profile deeply, understands the human's baseline behavior, relationships, and current context. Authors defensive entries (anomaly, freeze, counter-challenge) on the human's behalf. Ephemeral to the defense incident.
- **Gate discernment specialist** — the elohim-agent rule-4 handler already is this: evaluates a specific request (e.g., recovery authorization) with relationship context and produces an assessment. Ephemeral to that decision.
- **Advocate specialist** — for governance disputes, appeals, attestation challenges. Represents the human in contested proceedings.
- **Steward specialist** — for content/resource stewardship decisions (what to flag, what to re-replicate, what allocations to make).

Each specialist has:
- A **manifest** declaring: what inputs it consumes (imagodei profile fields, DHT entries, anomaly signals, etc.), what outputs it produces (which DHT entry types it's authorized to author), what disclosure rules apply.
- **Scope-limited context** — only reads what it needs. A defender specialist doesn't need to see content mastery records; a steward specialist doesn't need to see intimate relationships.
- **Transparent action surface** — all authored DHT entries are public to the network; disclosure of the specialist's *internal reasoning* is governed constitutionally.

**Constitutional governance of disclosure:**

Collectives (households, churches, qahals) can define constitutional rules for what specialist outputs are public vs. private:

- "Defender specialist outputs are always public" (transparency school)
- "Defender specialist discloses to intimate circle first, waits N hours before public freeze" (proportional-response school)
- "Anomaly detections are public but specialist reasoning traces are intimate-only" (split-tier school)

These rules live in qahal/mishpat DNA as governance policy. The protocol primitives carry the enforcement hooks (e.g., "this `IdentityAnomaly` has disclosure tier = intimate"), but the rules themselves are determined by the collective the human belongs to.

**How to apply:**

- When designing any new elohim-mediated function, think "specialist manifest," not "add to the big elohim." What's the focused role? What context does it need? What entries can it author?
- When considering whether elohim action should be public or private, defer to constitutional governance rather than hardcoding either. Provide the tier-marker hooks; let qahal define the rules.
- Specialists are ephemeral — don't design state that assumes persistence between invocations. Re-read the imagodei profile fresh each time. Trust the protocol's DHT entries as the durable state layer.
- A human's imagodei profile (currently `Human` + `HumanRelationship` + `HumanityWitness` + `Attestation` + others) is the canonical context for any specialist spawned on their behalf. Design the profile to be comprehensive and well-structured enough to give specialists what they need without ad-hoc extras.
