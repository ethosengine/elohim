---
id: records-lifecycle-part-a-primitives-plan
status: Draft
---

# Records Lifecycle — Part A Primitives Walkthrough Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to dispatch the 7 parallel agents. Each agent receives THIS plan plus their assigned primitive. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the seven stubbed primitive walkthrough sections (A.2 Event, A.3 Resource, A.4 Observation, A.5 Commitment, A.6 Attestation, A.7 FeedbackSignal, A.8 Links) in records-lifecycle-design.md with full walkthroughs matching the depth and structure of A.1 (EPR exemplar) — AND surface bottlenecks, chokepoints, and anti-patterns that the agent discovers during the deep architectural composition.

**Architecture:** Seven parallel Opus agents (rust-architect specialty). Each agent owns ONE primitive; reads grounding files first; follows the 11-sub-section template established by A.1 EPR; matches A.1 exemplar quality; returns the spec content alongside a structured concerns report. Spec content lands directly in records-lifecycle-design.md via Edit tool. Concerns reports return as the agent's response message.

**Tech Stack:** Markdown spec editing of `genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md` (Part A); grounding reads from `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` + `content_store/src/lib.rs` + `elohim/elohim-storage/src/views.rs` + `elohim/sdk/schemas/v1/views/*.schema.json` + pillar manifests.

---

## What every agent does first — context absorption

**Required reads before drafting:**

1. **Read the EPR walkthrough exemplar** — `genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md` Part A.1 (sections 1 through 11). This is the quality bar. Match its depth, concreteness, and hyperscale-analog discipline.

2. **Read the foundational frame** — same spec, §1 (Motivation), §2 (the eight primitives table), §2.1 (the section template), §2.2 (the Event/Resource naming note).

3. **Read the relevant epic** — whichever epic the spec's `realizes:` field cites that's most relevant to this primitive.

4. **Read the assigned-primitive's existing code surface** in:
   - `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` — search for the primitive's struct definition + LinkTypes if applicable
   - `elohim/elohim-storage/src/views.rs` — find the matching view type
   - `elohim/sdk/schemas/v1/views/` — find the matching JSON schema
   - `elohim/sdk/domains/<pillar>/manifest.json` — find how this primitive is extended via manifest

5. **Read any directly-relevant architecture spec** in `genesis/docs/content/elohim-protocol/architecture/` — for Observation, that's `2026-05-11-observation-event-layer-design.md`; for Attestation, that's `2026-05-11-attestation-consolidation-design.md`; etc. Cite them as upstream context.

## The 11-sub-section template (paste-from-EPR-A.1)

Every primitive walkthrough has these 11 sub-sections, numbered exactly:

```
### 1. What it is

One paragraph defining the primitive. What it holds, its identity model, where it lives, what it composes with.

### 2. Hyperscale analog

"Think X + Y, with Z." A familiar database/queue/cache/storage technology the reader knows. Be specific: don't say "like a database"; say "like a Postgres row with content-addressed identity, no master, gossip-validated writes." A skeptical systems architect should recognize the shape immediately.

### 3. Data flow

ASCII or text-diagram showing the lifecycle of one instance: author → validate → gossip → sync → project → query. Identify which actors handle which steps. End at "application queries (dashboard, app, doorway)."

### 4. Physical storage

Table with rows for: source of truth | operational copy | web2 projection (if any) | large attachments (if any). Each row names where the bits live (DHT / iroh-blob / SQL / Redis-shape) and the storage shape.

### 5. Gossip / sync layer

What goes over DHT (entry payload, size estimate, latency), what goes over libp2p sync plane (delta projections, cursor model), what goes over iroh-blob (large pull-fetched bytes). Concrete sizes and rates where possible.

### 6. Provenance — maintained vs intentionally degraded

Two lists:
- "Maintained cryptographically forever": what stays verifiable no matter what (signature chain, content-address, validator quorum, etc.)
- "Intentionally degraded (access cost, not truth)": what costs more to retrieve as lifecycle progresses (cold-archive, subordination, dissolution, right-to-be-forgotten)

End with: the substrate's commitment is to truth-verifiability, not free-access-forever. The CID is forever; cost of retrieval scales with lifecycle stage.

### 7. Agentic intelligence at scale

Where elohim cognition is **load-bearing** — what humans alone can't do at care-economy scale that elohim narration unlocks. Be specific about which elohim-agent specialization (inventory-elohim, vehicle-elohim, vision-elohim, care-stewardship-elohim, etc.) and what work it does. End with: "This is the value-prop unlock" or equivalent — connect to "scale love and care."

### 8. Scale: household → hub → global

Three nested bullets:
- Local DHT (household elohim-node): per-household footprint estimate, entry counts
- Hub (collective elohim-node): aggregation pattern, what hub holds vs federates
- Global: ~3000 entries-per-peer DHT capacity; what *earns* commons-reach vs what stays local

### 9. Limit-awareness / capture prevention

Substrate mechanics that prevent concentration / capture / Sybil: DHT validator quorum, friction-gradient limitarianism, reach-as-earned, elohim arbitration, anti-concentration recursion.

### 10. Network resilience

DHT shard-N redundancy; partition recovery via cursor-tracked sync; cold-archive K-of-N erasure recovery; doorway projection for the unconnected.

### 11. Dashboard worked example (preview)

How this primitive shows up in the Monarch/Mint dashboard worked example (or another archetype where appropriate). Specific SQL/query shape if relevant. Threads into `applications/<name>-application-design.md`.
```

**Quality bar:** A.1 (EPR) is ~1100 words. Each subsequent walkthrough should be in the 600–900 word range — concrete but not bloated. Tables, ASCII diagrams, and bulleted lists are encouraged.

## The seven primitives — assignment table

| Section | Primitive | Hyperscale analog | Key reads | Specific tensions to surface |
|---|---|---|---|---|
| A.2 | **Event** (`EconomicEvent` + action verb) | Kafka event with built-in double-ledger conservation; REA mass-balance | EconomicEvent struct in integrity zome; observation→event graduation in 2026-05-11-observation-event-layer-design.md | Action-verb polymorphism (`transfer`, `transform`, `dispose`, `mint`, `grant-reach`, ...); `stake_class: high` vs `operational` gating; how 1000:1 observation graduation works without bloating DHT |
| A.3 | **Resource** (`EconomicResource` + classification) | S3 object whose state is event-sourced; balance derived from events | EconomicResource + StewardedResource structs; tiered-quilt for cold-archive | Balance as derived-view (not stored); `resource_classified_as` for stewardship variants (Gap 5 consolidation); CID continuity through `surface` re-elevation; subordination via `parent_epr_cid`; dissolution to `closed` |
| A.4 | **Observation** | Splunk / structured-log stream with retention classes; ephemeral peer-witnessed evidence | 2026-05-11-observation-event-layer-design.md (this section is largely a CITATION of that spec, not a re-derivation) | Ephemerality (libp2p+iroh, never DHT); five retention classes; graduation paths (Path 1 Attestation, Path 2 summary Event); witness-not-surveillance posture; agent-private vs household vs community vs commons reach |
| A.5 | **Commitment** | Spring-Batch scheduled job (planned future Event) AND custody primitive for cold-archive | Commitment struct in integrity zome; custody-blob/custody-quilt/custody-shelved ladder | Dual role (planning AND custody); how Commitments fulfill into Events; cancellation; subscription/recurring patterns (used by Patreon archetype); cold-archive authoring (`Commitment(custody-quilt, tier_floor=shelved)` — the canonical submerge per Gap 4) |
| A.6 | **Attestation** (`Content` + `content_type: "attestation:*"`) | PKI certificate with auditable evidence chain back to observations | 2026-05-11-attestation-consolidation-design.md; Content struct + content_type discriminator | Four `proof_evidence.class` tiers (witness/audit/proof/confirmation per 2026-05-01 spec); evidence-chain integrity model with `observation_refs` pointing to iroh-blob log positions; issuer-vs-subject distinction; revocation via root-rewrite for right-to-be-forgotten |
| A.7 | **FeedbackSignal** | Webhook / event-notification gated by reach; the one social-move surface that earns DHT cost | feedback_signal.rs in integrity zome; signal_kind extensibility | Documented edge case (the ONLY social-move surface on DHT because reach-coupling requires authoring-time notarization); `signal_kind` extensibility; how FeedbackSignals contribute to reach earning/decay; manifest-declared validators; how Meta/Patreon/R&O archetypes compose these |
| A.8 | **Links** (`EprToEvent`, `EprToResource`, `AttestationToSubject`, etc.) | GraphQL edges; cheap, unbudgeted; where graph traversal happens | LinkTypes enum in integrity zome | Existing link types vs new ones added by this spec (Gap 1: `EprToEvent`, `EprToResource`); how links carry the graph that EPRs project as nodes; `parent_epr_cid` is a field BUT the link is what enables traversal; performance: how queries traverse efficiently without melting peers |

## Per-primitive task structure (the same for every agent)

### Task: Write Part A.<N> <primitive> walkthrough

**Files:**
- Modify: `genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md` (the A.<N> stub section, locate via `grep -n "^## A\.<N>" <file>`)
- Read-only: A.1 EPR section (same file, lines ~200–298)
- Read-only: the four required spec/code reads from "What every agent does first"

- [ ] **Step 1: Absorb context**

Read the EPR exemplar A.1 in full. Read the assigned primitive's struct in the integrity zome. Read the matching view type and JSON schema. Read the relevant architecture spec if one exists for this primitive (e.g., Observation has its own canonical spec; Event/Resource do not).

- [ ] **Step 2: Locate the stub**

```bash
grep -n "^## A\.<N>" genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md
```

Read the stub's "Stubbed — full draft pending" line for the hint of what to cover.

- [ ] **Step 3: Write the 11 sub-sections**

Use the template from this plan. Match A.1 depth. Be concrete. Use the hyperscale analog assigned in the assignment table. Pull from the existing code surfaces (cite real struct names and field names from the integrity zome). Where the primitive has an existing canonical spec (Observation, Attestation), CITE it rather than re-derive (the section becomes a digested pointer with the 11 sub-sections still answered briefly).

- [ ] **Step 4: Replace the stub via Edit**

The stub is a single-line italicized blockquote starting with `> *Stubbed — full draft pending...*`. Replace this entire block with the 11-section walkthrough.

- [ ] **Step 5: Verify section structure**

```bash
sed -n '/^## A\.<N>/,/^## A\.<N+1>/p' genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md | grep "^### " | wc -l
```

Expected: 11 (sub-sections 1 through 11 all present).

- [ ] **Step 6: Surface concerns**

Return as part of the agent's response message (NOT in the spec content itself) a structured concerns report:

```markdown
## Concerns surfaced while writing Part A.<N> <primitive>

### Bottlenecks
- [where this primitive risks overwhelming peer storage/gossip/compute]

### Chokepoints
- [centralized actors, single-hub load-bearing, single-points-of-failure]

### Anti-patterns
- [conflicts with substrate principles; specific principle named]

### Substrate gaps beyond the original 10
- [things needed that aren't in the current gap list]

### Cross-spec drift risk
- [where this primitive's choices conflict with another primitive's or application's]

### What I couldn't fully resolve
- [questions the operator needs to decide]
```

- [ ] **Step 7: Commit**

```bash
git add genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md
git commit -m "spec(architecture): records-lifecycle Part A.<N> — <primitive> walkthrough"
```

## Self-Review (per agent, before returning)

**1. Did I match A.1 EPR exemplar depth and structure?** — 11 sub-sections present; word count 600–900; tables/diagrams where they add concreteness.

**2. Did I avoid re-deriving existing canonical content?** — For Observation and Attestation, I CITED their canonical specs rather than re-writing the same material; for primitives without canonical specs (Event, Resource, Commitment, FeedbackSignal, Links) I derived from code surfaces.

**3. Did I keep §11 (dashboard worked example) threaded to a real `applications/<name>-application-design.md`?** — Not generic; names a specific archetype where this primitive shows up.

**4. Did I surface concerns honestly?** — At least one bottleneck, one chokepoint, one anti-pattern temptation, one cross-spec drift risk. If I had nothing to surface, did I look hard enough?

## Quality bar checklist (for the orchestrator's Phase 2 review)

- [ ] §1 — What it is: one clear paragraph, not multiple
- [ ] §2 — Hyperscale analog: concrete technology name, not "like a database"
- [ ] §3 — Data flow: actor-by-actor, not vague
- [ ] §4 — Physical storage: table with rows; storage shape named per row
- [ ] §5 — Gossip / sync: concrete sizes and latencies where possible
- [ ] §6 — Provenance: two lists (maintained vs degraded); ends with cost-shedding rationale
- [ ] §7 — Agentic IQ: specific elohim-agent specialization named; value-prop unlock connection
- [ ] §8 — Scale: three levels (household/hub/global); concrete numbers
- [ ] §9 — Limit-awareness: substrate mechanics, not aspirations
- [ ] §10 — Resilience: redundancy + partition + cold-archive + doorway
- [ ] §11 — Dashboard: specific archetype + SQL shape if relevant
- [ ] Concerns report: structured, substantive, honest

## Execution handoff

This plan is dispatched via `superpowers:subagent-driven-development` from the master orchestration plan. Each agent receives this plan plus its assigned primitive. The orchestrator (operator) reviews returns after all 7 agents complete.

If any walkthrough is below quality bar, re-dispatch with corrective context citing the specific quality-bar items missed.
