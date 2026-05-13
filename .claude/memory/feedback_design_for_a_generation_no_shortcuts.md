---
name: Design for a generation — no shortcuts on substrate decisions
description: When the user explicitly invokes "design for a generation" or rejects pragmatic shortcuts, prefer the W3C-standards / fundamentals option over the path of least resistance, even when the cost is substantially higher
type: feedback
originSessionId: 75ea3d40-9dd2-4e0c-ba96-dcb49c5221b5
---
Rule: when a substrate-level decision comes up (component framework, protocol design, identity layer, persistence model), present the full spectrum honestly — don't bias toward the pragmatic short-term option. If the user invokes "design for a generation" / "no shortcuts" / "fundamentals," pivot to the standards-aligned option even when it costs substantially more upfront.

**Why:** observed 2026-05-06 during the elohim-styles → Lit/WC pivot. User's first instinct was to pay the full Lit tax upfront rather than start with Angular components and migrate later via `@angular/elements`. Their reasoning: "the protocol is meant to outlive any single frontend framework." Recurring pattern in this codebase — see also project_no_sovereignty_stewardship_over_ownership, project_subsume_g_f_a_via_it_just_works, project_intelligence_revolution_scales_to_humans. The user repeatedly rejects expedient framings in favour of fundamentals.

**How to apply:**
- When proposing options, always include the W3C-standards / open-protocol / first-principles option as a real choice, not a footnote.
- Don't pre-bias the recommendation toward "least cost now" when the conversation is about substrate.
- If proposing a "pragmatic middle" (e.g. `@angular/elements` as escape hatch from Angular-only commitment), name it as a pragmatic-middle and explicitly contrast with the fundamentals option.
- The user wants to pay engineering-time taxes upfront when the decision is load-bearing. Take them at their word — "design for a generation" is a real constraint, not aspiration.
