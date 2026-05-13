---
name: Values-forward preference guards — the legitimate user-side filter
description: Humans CAN set personal preference guards on what reaches them — but only as values-forward, time-limited, tended filters with anti-filter-bubble constraints that feed collective reach-governance signals. Distinct from email-collapse network-imposed filtering.
type: project
originSessionId: f534b7ae-d435-4ab8-ab3b-f7d23b6b0ed9
---
There IS a legitimate role for peer-side preference filtering — but it has a very specific shape. It is **values-forward, time-limited, tended, constrained against filter-bubble formation, and feeds collective wisdom signals**. It is NOT what receive-side network-imposed filtering looks like. Get the shape right or revert to the email-collapse anti-pattern.

**Canonical scenarios — what users want to filter:**

- "Covfefe", "Amanda Black's Friday", "Mr. Beast", "skibidi toilet", "67" — viral content moments with very little justifiable epistemic value
- Brain-rot / rage-bait / engagement-farming patterns the user has chosen not to spend attention on
- Specific topics the user has opted to deprioritize (lifestyle preference, not censorship)

**The contract — five constraints that distinguish this from the anti-pattern:**

1. **Values-forward, not perimeter-defended.** A human consciously expresses their values: "I don't find this kind of content valuable; please filter it from my reach." It is the user's preference, not the network's policy.

2. **Time-limited (mandatory expiration).** The filter must have a built-in expiration. Set-and-forget is not allowed. The user must actively renew if they still want it.

3. **Tended.** The filter requires active care from the peer — periodic review, recognition that values shift, willingness to re-encounter content as one grows. Without tending, filters stagnate and ossify into echo chambers.

4. **Anti-filter-bubble constraints (Eli Pariser's hazard).** The mechanism must be structurally constrained against letting users hide in their own epistemic reality. Some content cannot and probably shouldn't be hidden from — facts about your community, accountability information, news that affects you, broccoli ("you might not like it but it's good for you"). The system distinguishes:
   - Genuinely low-value / harmful / preferential content (OK to filter)
   - Content you shouldn't hide from (filter cannot block)
   This is a **policy boundary** built into the protocol, not a setting users control.

5. **Feeds collective wisdom signals.** When a filter fires, it doesn't just block locally — it signals into the network nervous system. Many peers filtering the same content = collective wisdom signal that the content failed to earn reach. This becomes input to **collective reach governance**: low-value content faces structural distribution headwinds because real users are voting with their preference guards. This is the bridge between individual values and collective network behavior.

**Why this is DIFFERENT from the email-collapse anti-pattern:**

| Email-collapse filtering | Values-forward preference guards |
|---|---|
| Network-imposed | Human-set |
| Perimeter defense | Values expression |
| Permanent / set-and-forget | Time-limited / mandatory expiration |
| Untended | Requires active care |
| No anti-bubble constraint | Built-in epistemic-bubble guard |
| Cost on receivers asymmetrically | Cost on user expressing values |
| Megaliths can dominate | Aggregates into collective wisdom |
| Hides everything indiscriminately | Constrained scope (low-value/harmful/preferential only) |
| Per-message validator drops messages | Personal scope guard, network gets the signal |

**Architecture sketch (FUTURE work — downstream of Phase 2B):**

This belongs to the social-reach epic (`project_social_reach_nervous_system`):

- **Preference guard EPRs** — content-addressed expressions of "user X has set a guard for content matching Y until Z." Time-bound, signed, revocable.
- **Anti-bubble policy** — protocol-level rules defining what content categories are off-limits to filter (analogous to "must inform" content classes). Possibly notarized in qahal-governance DNA.
- **Collective aggregation** — when N peers in a scope set similar guards, that aggregates into a network signal that the content failed to earn reach. Feeds the sense/respond nervous system.
- **Tending UX** — periodic prompts to review/renew filters; "you've had this filter for 90 days — does it still match your values?"
- **Restitution path** — content authors whose work routinely triggers preference guards face structural reach decay (trust-as-efficiency at work).

**How to apply:**

- Receive-side filtering tasks: ALWAYS verify the shape matches values-forward preference guards (the five constraints above), NOT the email-collapse anti-pattern. If the shape is "network drops messages from peers without binding rows" — that's the anti-pattern. If the shape is "user X set a values-forward guard on content matching Y until Z, with anti-bubble constraints + tending + collective signal" — that's the legitimate version.
- Phase 2B Batch D.4 implements the FLOOR (author-side earning + receiver pre-authorization classification). It does NOT implement values-forward preference guards — those are downstream social-reach epic work.
- If a receive-side filter is implemented without the five constraints, it is the wrong shape. Reject and redesign.
- If a receive-side filter is implemented WITH the five constraints, it is values-forward preference guards — legitimate, but check it's specced as such and fits the social-reach epic architecture.

**Connection to existing memory pins:**

- `project_reach_earned_at_authoring` — the floor; values-forward guards are a downstream nuance, not a contradiction
- `project_social_reach_nervous_system` — guards firing IS the sense/respond input that drives collective reach governance
- `project_trust_as_efficiency_signal` — guards aggregate into trust signals that make distribution more efficient (low-value content costs more to push through)
- `project_first_class_graph_pattern` — guards live as graph nodes (preference EPRs), aggregations are graph queries
- `project_elohim_as_counsel` — a user's elohim helps them tend their guards, surfacing tending prompts and anti-bubble warnings
