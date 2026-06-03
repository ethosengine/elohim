---
id: records-lifecycle-applications-plan
status: Draft
---

# Records Lifecycle — Application Archetypes Full-Draft Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to dispatch the 7 parallel agents. Each agent receives THIS plan plus their assigned application. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Upgrade the seven application archetype composition-drafts (Khan Academy, Google Drive, Google Photos, Meta/Facebook, Patreon, Requests & Offers, AWS Compute) to full-drafts matching the Mint/Monarch exemplar in `applications/mint-monarch-application-design.md` — AND surface bottlenecks, chokepoints, anti-patterns, and substrate gaps that the deep architectural composition discovers.

**Architecture:** Seven parallel Opus agents (rust-architect or general-purpose specialty per app). Each agent owns ONE archetype; reads grounding files first; expands the existing composition-draft into the full-draft template established by Mint/Monarch; matches exemplar quality; returns the spec content alongside a structured concerns report. Spec content lands directly in `applications/<app>-application-design.md` via Edit tool. Concerns reports return as the agent's response message.

**Tech Stack:** Markdown spec editing of `genesis/docs/content/elohim-protocol/architecture/applications/<app>-application-design.md`; grounding reads from records-lifecycle Part A primitives + Mint/Monarch exemplar + the application's existing composition-draft + relevant code surfaces.

---

## What every agent does first — context absorption

**Required reads before drafting:**

1. **Read the Mint/Monarch full-draft exemplar** — `genesis/docs/content/elohim-protocol/architecture/applications/mint-monarch-application-design.md` end-to-end. This is the quality bar — depth, table structure, the "how one transaction flows" walkthrough, the storage-footprint table, the network-bandwidth profile, the DHT-impact analysis, the render-speed walkthrough, the cross-household aggregation explanation, the agentic-intelligence section, the bridges section, the code-anchors table.

2. **Read the application's existing composition-draft** — `genesis/docs/content/elohim-protocol/architecture/applications/<app>-application-design.md`. This already has the frontmatter, the primitive-composition table, the stress points, and the scale-answer sketch. You're EXPANDING it, not rewriting from scratch.

3. **Read records-lifecycle Part A primitive walkthroughs** for the primitives this application heavily uses — these provide the substrate vocabulary. (Note: Part A.2-A.8 are being written in parallel; if an agent is dispatched before they land, fall back to citing the primitive's stub plus the relevant canonical spec.)

4. **Read the application's grounding code surfaces**:
   - For applications heavy on Angular: `app/elohim-app/src/app/<pillar>/` services
   - For applications heavy on substrate: `elohim/elohim-storage/src/views.rs`, `elohim/sdk/schemas/v1/views/`, `elohim/sdk/domains/<pillar>/manifest.json`
   - For applications with bridge requirements: `bridges/valueflows/` (reference pattern); planned `bridges/<vendor>/` directories

5. **Read the application's `realizes:` epic** — the epic narrative that this archetype gives technical form to. Quote it minimally; don't re-narrate; cite by section if helpful.

## The Mint/Monarch full-draft template (paste-from-exemplar)

Every application archetype full-draft has these sections, matching the exemplar:

```
## The grandma test
One paragraph: what does the user actually see and feel? What's the experience that feels like the
legacy app but is substrate-native? The user is "grandma" — non-technical, on a phone or laptop.

## Primitive composition
Table with rows for each user-visible concept and its primitive mapping. Columns:
"What you see" | "Primitive" | "Notes"
Use the exact primitive vocabulary from records-lifecycle Part A.

End with: "<N> primitives, ~<M> discriminator values, no special-casing for this application."

## How one <X> flows
End-to-end trace of a representative interaction in numbered steps. Show:
- The external trigger (user action or bridge webhook)
- The Observation (if any)
- The graduation
- The DHT write (or sync, or libp2p direct-message)
- The local SQL projection update
- The dashboard / surface refresh
Include timing where possible ("<100 ms" etc).

End with one paragraph on bidirectionality / cash-out.

## Storage footprint per household
Table: Item | Count | Size | Total
End with one sentence: "Fits on a phone / laptop / etc."

## Network bandwidth profile
Concrete monthly bandwidth per household; concrete per-transaction sizes.

## DHT entry impact
Concrete entry counts globally + per-peer; explanation of why reach-scoping keeps per-peer footprint
bounded.

## Why <X> renders fast / why <Y> doesn't melt the network / etc.
One sub-section per major performance concern, each with specific SQL/query/operation shape.

## Dissolution in practice
How `Event(action="dispose")` shows up in this archetype; how `closed` state affects queries; the
cradle-to-cradle hook (deferred to a later spec but mentioned here).

## Where agentic intelligence carries the load
Specific elohim-agent specializations; what they narrate that humans wouldn't bear to; the
value-prop unlock for this archetype.

## What the multi-party / collective / cross-household view shows
How federation works for this archetype — federated query vs data replication;
hub aggregation; cash-out preserving privacy.

## Bridges (legacy interop / cash-out)
Specific `bridges/<vendor>/` crates this archetype uses or plans; bidirectionality;
backfill from legacy export; cash-out behavior.

## Code anchors
Table: Surface | Path
Names the specific files this archetype will land in / extend.

## What this proves about the substrate
3-5 bullet claims a skeptical systems architect should be able to make after reading this archetype.
```

**Quality bar:** Mint/Monarch is ~1300 words. Each application full-draft should be in the 1000–1500 word range — concrete, specific, with real numbers and real file paths.

## The seven applications — assignment table

| App | File | Existing composition-draft anchors | Specific tensions to surface |
|---|---|---|---|
| **Khan Academy** | `khan-academy-application-design.md` | Course/Lesson/Quiz mapping; learner attempt as Event; mastery as Resource | Long branching mastery trajectories at scale; sophia-element integration; cohort-hub federation without per-learner data leak; credentialing flow gaps |
| **Google Drive** | `google-drive-application-design.md` | Document/Folder/Edit; Automerge CRDT for collab; FTS over local SQL | Working-set caching strategy; search-everything at scale; real-time collab without full corpus replication; sharing reach mechanics |
| **Google Photos** | `google-photos-application-design.md` | Photo as EPR with media_cid; auto-tag Attestations; face-cluster | Massive blob storage per household; vision-elohim privacy posture; shared albums without replicating bytes; cold-photo recovery |
| **Meta / Facebook** | `meta-facebook-application-design.md` | Profile/Post/Comment/Friend; reach-gated feed without engagement-optimization | Graph traversal at social scale; feed ranking that doesn't weaponize attention; community moderation; consolidation events for misinformation per living_memory epic |
| **Patreon** | `patreon-application-design.md` | Tier as Commitment; recurring patronage as Event; exclusive content via reach-gated EPR | Subscription billing reliability; tier-gated content access; payout flows; recurring-failure recovery; cash-out from both sides |
| **Requests & Offers** | `requests-offers-application-design.md` | Offer/Request EPRs; match as Event; cooperative procurement; VF-GraphQL bridge | Marketplace matching at scale; trust without central rating; cooperative pooling; reach-scoping to prevent global product catalogs |
| **AWS Compute** | `aws-compute-application-design.md` | Capacity declaration as Commitment; job as Event; verification per compute-attestation spec | Real-time capacity matching; provider trust; verification of paid-for compute; the substrate's own compute use as bootstrapping demand |

## Per-application task structure (the same for every agent)

### Task: Upgrade <app> archetype to full-draft

**Files:**
- Modify: `genesis/docs/content/elohim-protocol/architecture/applications/<app>-application-design.md`
- Read-only: `applications/mint-monarch-application-design.md` (exemplar)
- Read-only: records-lifecycle Part A primitive walkthroughs for cited primitives
- Read-only: code surfaces per the existing composition-draft's "Code anchors" section

- [ ] **Step 1: Absorb context**

Read the Mint/Monarch exemplar end-to-end. Read your composition-draft end-to-end. Read records-lifecycle Part A.1 (EPR exemplar) and the primitives most heavily used by your archetype.

- [ ] **Step 2: Preserve frontmatter**

Your composition-draft already has good frontmatter (`tier: architecture`, `realizes:`, `informed-by:`, `informs:`, `defers:`). DO NOT modify the frontmatter — only update the body. If the body's expansion reveals new things to add to `informs:` or `informed-by:`, those go in via a separate explicit Edit, not as a side-effect.

- [ ] **Step 3: Preserve the primitive-composition table**

Your composition-draft has a primitive-composition table. KEEP IT. Expand it if you discover additional primitive mappings while writing other sections; never remove rows.

- [ ] **Step 4: Expand the body to match Mint/Monarch full-draft**

Add all the sections from the template above. Where your composition-draft has a "Stress points the substrate handles" or "Scale answer" section, those expand into the storage-footprint table + network-bandwidth profile + DHT-entry impact + why-this-renders-fast + why-this-doesn't-melt sections in the full-draft.

Be concrete. Real numbers (back-of-envelope is fine; cite the math). Real file paths. Real SQL where useful.

- [ ] **Step 5: Verify structure**

```bash
grep "^## " genesis/docs/content/elohim-protocol/architecture/applications/<app>-application-design.md | wc -l
```

Expected: At least 11 H2-section headers (matching Mint/Monarch's section count, allowing for some variation per archetype).

- [ ] **Step 6: Surface concerns**

Return as part of the agent's response message (NOT in the spec content) a structured concerns report:

```markdown
## Concerns surfaced while writing <app> archetype

### Bottlenecks (application-specific)
- [where this archetype risks overwhelming peer storage / bandwidth / compute / RAM]

### Chokepoints
- [centralized actors / single-hub load-bearing surfaces that emerge for this archetype]

### Anti-pattern temptations
- [places where it's tempting to violate substrate principles (e.g., wanting a centralized index, a platform-side ACL, a new DHT entry type, an engagement-optimization signal)]

### Substrate gaps surfaced by this archetype
- [things this archetype needs that the substrate doesn't have; concrete gap proposals]

### Cross-archetype drift risk
- [where this archetype's choices might conflict with another archetype's choices; especially around shared primitives like Reach, FeedbackSignal, Resource classifications]

### Bridge complexity / legacy migration risks
- [where the legacy-incumbent migration is genuinely hard; what bridges need that's nontrivial]

### What I couldn't fully resolve
- [decisions the operator needs to make]
```

- [ ] **Step 7: Commit**

```bash
git add genesis/docs/content/elohim-protocol/architecture/applications/<app>-application-design.md
git commit -m "spec(architecture): <app> archetype full-draft"
```

## Self-Review (per agent, before returning)

**1. Did I match Mint/Monarch full-draft depth?** — 11+ sections present; word count 1000–1500; tables with real numbers; SQL where relevant; specific file paths.

**2. Did I preserve the composition-draft's primitive map?** — The primitive-composition table is intact; rows preserved; only added if I discovered more.

**3. Did I keep frontmatter unchanged unless explicitly justified?** — `realizes:` / `informed-by:` / `informs:` only changed via separate explicit Edit with rationale.

**4. Did I name specific elohim-agent specializations?** — The "agentic intelligence" section is specific (e.g., inventory-elohim for stuff narration, vision-elohim for photo tags) not generic ("elohim do AI stuff").

**5. Did I cite real code paths?** — The code-anchors table points to files that actually exist (or are explicitly marked "planned").

**6. Did I surface concerns honestly?** — At least one bottleneck, one chokepoint, one anti-pattern temptation, one substrate gap, one cross-archetype drift risk. If nothing surfaced, did I look hard enough?

## Quality bar checklist (for the orchestrator's Phase 2 review)

- [ ] Grandma test: concrete, named features the user sees
- [ ] Primitive composition table: preserved + possibly extended; only the eight primitives used
- [ ] Flow walkthrough: end-to-end numbered, with timing
- [ ] Storage footprint: table with real number estimates
- [ ] Network bandwidth: per-month per-household number
- [ ] DHT impact: concrete entry-count + per-peer math
- [ ] Render-speed walkthroughs: SQL shape or query mechanics named
- [ ] Cross-household / multi-party view: federated query, not data replication
- [ ] Agentic intelligence: specific specializations + value-prop connection
- [ ] Bridges: specific `bridges/<vendor>/` crates named (real or planned)
- [ ] Code anchors: real paths
- [ ] What this proves about the substrate: 3-5 architect-facing claims
- [ ] Concerns report: structured, substantive, honest

## Anti-patterns to forbid

These are violations of substrate principles; agents must NOT introduce them when expanding archetypes:

- **Inventing a new DHT entry type** — all compositions use only the eight primitives + manifest discriminators
- **Centralized index / search service** — search is local-SQL-FTS over the user's own corpus; cross-corpus is federated query
- **Platform-owned ACL** — reach gating is substrate-native via reach scope + Attestations
- **Engagement-optimization signal** — feed ranking uses earned-reach + standing, not predicted-attention
- **Sovereignty / ownership language** — use "steward" / "contributor" / "authored"; never "own" / "ownership"
- **Hand-waved scale claims** — "scales because it's P2P" is not an answer; concrete numbers + mechanism

## Execution handoff

This plan is dispatched via `superpowers:subagent-driven-development` from the master orchestration plan. Each agent receives this plan plus its assigned application. The orchestrator (operator) reviews returns after all 7 agents complete.

If any full-draft is below quality bar, re-dispatch with corrective context citing the specific quality-bar items missed.
