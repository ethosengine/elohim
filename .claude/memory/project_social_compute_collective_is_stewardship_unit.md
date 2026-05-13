---
name: Collective is the stewardship unit — social compute epic
description: Household is one kind of collective; stewardship/DePIN contracts also run between church memberships, patron/creator circles, DAOs. Design for collective-general, not household-specific.
type: project
originSessionId: 17546f03-3ee8-4704-bdf9-18d0d64baf9b
---
Households are the foundational case but not the only case. The protocol's "social compute" epic enables:

- A **church** can run its own website, directory, and CMS from its members' peers.
- A **vlogger/blogger/podcaster** can host blog, community, podcasts, and other ContentNode resources with their **patrons** providing storage, backup, and distribution.
- A **DAO-like group** can pool storage and compute across its members.

All of these run on DHT-notarized stewardship contracts (REA commitments) across a stewarded tranche of storage + compute + bandwidth on peer nodes. The household case is the degenerate/first-class case; the general case is **any collective** (kind=household | kind=church | kind=patron-circle | kind=dao | …).

**Why:** User framing (2026-04-19, self-healing dataplane Plan 1 pause): "beyond the household collectives what we're also trying to enable is our 'social compute' epic… per those DHT contracts on the stewarded tranche of storage on their peer nodes."

**How to apply:**
- Prefer **"collective"** or **"stewarding party"** vocabulary in wire types, query logic, and UI rendering over hardcoded "household."
- In code: group/rank by `collective_id` (pointing at the `collectives` table — households are already `collectives.kind='household'`), not strictly by `humans.household_id`.
- In UI: render the collective kind from data (e.g., "3 households" OR "2 patron circles" OR mixed), not hardcoded "households."
- When a human has multiple affiliations (household + church + patron roles), the relevant collective for a given stewardship is the one the commitment is made under (`rea_commitments.collective_context` or similar) — not the human's primary household.
- Existing `humans.household_id` is a pointer to the human's primary household collective; for Plan 1 scope (household-only contracts today) it IS the stewardship collective. Keep the column; generalize the queries.
- When a field is household-specific, name it so (`householdsStewarding` for the household-kind metric); when it's the general supertype metric, use `stewardingCollectives` or similar.
- **Red flag:** any new wire type or query that hardcodes "household" as the stewardship unit. This locks out social compute.
