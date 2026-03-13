# Future Distribution Models — Research Topics

> These topics were identified during the recognition pipeline design (2026-03-13) as areas needing deeper research before implementation. Each represents a significant evolution beyond the v0 linear proportional model.

---

## A. Multi-Dimensional Contribution Weighting

**Question**: Should contribution type affect weight differently depending on context?

Currently all contribution types (author, curator, reviewer, translator) are weighted equally — only `allocationRatio` and `affinityScore` matter. But curators matter more for discovery, authors for creation, reviewers for trust.

**Research directions**:
- Context-dependent weight matrices (content type x contribution type)
- How academic citation indices handle multi-author attribution
- Drips Network nested split patterns as analogue
- ValueFlows "input-process-output" chains for tracing which contributions produced which value

**Relevant existing code**: `contributionType` field on `StewardshipAllocation`, currently unused in distribution math.

---

## B. Temporal Decay / Accrual Models

**Question**: Should stewardship recognition change over time?

An original creator's share might diminish as maintainers contribute more. Or a curator who discovers content early might deserve ongoing recognition that accrues as content proves valuable.

**Research directions**:
- Half-life models (creator share decays, maintainer share grows)
- Vesting schedules (recognition accumulates over stewardship tenure)
- "Discovery premium" — early curation weighted higher than late
- How open source projects handle contributor attribution over time (bus factor models)
- Demurrage as applied to recognition, not just currency

**Relevant existing code**: `effectiveFrom` / `effectiveUntil` on allocations already model temporal bounds but aren't used in distribution.

---

## C. Layered Distribution Through EPR Tiers

**Question**: Should recognition flow through the delivery chain, not just to content creators?

Content flows: creation -> curation -> delivery -> verification. Each layer adds value. Stewards of infrastructure (delivery nodes) could receive a cut of recognition from content they serve.

**Research directions**:
- Drips-style cascading attribution through dependency graphs
- Content prerequisite graphs — if learning content B requires mastering content A, does A's steward get recognition when B is consumed?
- Infrastructure recognition — node stewards who deliver content receiving a delivery fee
- Verification premium — attestation of content quality as a recognized economic act
- How CDN economics (edge caching, bandwidth costs) map to P2P content delivery

**Relevant existing code**: EPR three-tier model (Head/Document/Bytes), `node_stewardship` with `affinity_score`.

---

## D. Multi-Swimlane Distribution

**Question**: How should recognition map across the five Unyt currency swimlanes?

Currently recognition is a single numeric value. The protocol defines five swimlanes (time, care, infrastructure, learning, creator). Different event types might produce recognition in different swimlanes.

**Research directions**:
- Mastery completion -> learning swimlane recognition
- Content delivery -> infrastructure swimlane recognition
- Peer tutoring/mentoring -> care swimlane recognition
- Original authorship -> creator swimlane recognition
- Cross-swimlane exchange rates and their governance
- How values-scanner results (personal values hierarchy) might influence swimlane weighting
- Unyt mutual credit creation patterns vs central token minting

**Relevant existing code**: `ExchangeRate` model, `ValueSwimLane` concept in research README, swimlane definitions in `shefa-dashboard.model.ts`.

---

## E. Cybernetic Feedback and Self-Regulation

**Question**: How should the distribution system self-correct?

Beyond static weights, the system needs feedback loops: if recognition concentrates, redistribute. If gaming is detected, dampen. If a domain is under-stewarded, incentivize.

**Research directions**:
- Cybernetic governors (Ashby's Law of Requisite Variety applied to economic flows)
- Anomaly detection for gaming patterns (rapid content creation for recognition farming)
- Network-level health metrics that trigger automatic weight adjustments
- Constitutional AI as the "regulator" in the cybernetic sense — not controlling, but constraining
- How biological systems handle resource distribution (nutrient transport, oxygen allocation)
- Elohim agents as the intelligence layer that interprets signals and negotiates with humans

**Relevant existing code**: `ConstitutionalLimitsStatus` in shefa models, `ElohimSignature` concept in research README.

---

## F. Explainability and Interrogation

**Question**: How does a human interrogate their elohim about distribution decisions?

When someone asks "why am I not getting more recognition?", the system needs to produce a human-understandable explanation that traces through the full pipeline.

**Research directions**:
- Pipeline trace as structured data vs natural language explanation
- Counterfactual reasoning ("you would have received X more if your affinity to this domain were higher")
- Comparative explanations ("steward B receives more because they have 3x your allocation ratio")
- How tax systems explain withholding calculations (W-4 worksheets as UX pattern)
- Elohim agent conversation patterns for economic interrogation
- Building trust through transparency vs information overload

**Relevant existing code**: `StageTrace` in recognition pipeline design carries all reasoning data. Elohim presence service has nudge/play/resolve patterns.
