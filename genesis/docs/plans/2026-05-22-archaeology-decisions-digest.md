---
project: elohim-protocol
type: decision digest
status: operator review
created: 2026-05-22
governs: Sprint 0.5 sign-off; Sprint 1 + Sprint 5 entry conditions
companion-to:
  - genesis/docs/plans/2026-05-22-scenario-archaeology-and-archetype-map.md
  - genesis/docs/superpowers/specs/2026-05-21-qahal-architecture-vision.md
  - genesis/docs/plans/2026-05-21-qahal-mvp-roadmap.md
---

# Archaeology Decisions Digest — Sprint 0.5

> **What this is.** A scannable decision pack distilled from the 10.3K-word archaeology pass. The archaeology document inventoried all 76 .feature files, mapped each to the gospel-tier archetype catalog, and flagged 10 operator decisions at its close. This digest summarizes each decision, lays out options, and recommends one with rationale anchored to the vision spec. Read the table in Section 4 in 30 seconds and confirm-all or send back the items you want to discuss.

## 1. Overview

The archaeology pass — `2026-05-22-scenario-archaeology-and-archetype-map.md`, ~10.3K words, 770 lines — inventoried 76 .feature files at `genesis/a2o/features/`, mapped each to one or more entries in the gospel-tier collective-archetype catalog, and proposed a new directory taxonomy that puts the archetype axis primary. The corpus predates the qahal architecture vision; it was authored under a mixed-axis taxonomy (pillar + implementation-shape + content-surface).

**Headline findings.** Of 76 files, only 11 carry a Tier-0 archetype as primary tag; the rest are infrastructure (49) or cross-cutting (16). The household has 7 primary scenarios; **faith-community and life-group have zero**. Wisdom-commons has 3 with significant gaps. Sprint 5 has roughly 21 net-new Tier-0 scenarios to author (5 household witness-pattern gap-fills, 7 faith-community, 7 life-group, 7 wisdom-commons-federation), plus ~5 cross-cutting (3 friction-gradient, 2 imago-dei). The `qahal/` pillar bucket should disappear in the new taxonomy because neither file living there is archetype-specific. The dissolution principle (Section 2.11 of the spec) changes how several existing scenarios should be framed — `auth/user-management.feature` is the clearest case.

**Purpose of this digest.** The operator does not want to re-read 10.3K words to act on the 10 decisions. Each one below is summarized to ~150 words with a recommendation grounded in the vision spec — particularly the household-as-living-core, lived-contrast, dissolution-principle, and Imago Dei discriminator framings. Section 3 surfaces the few decisions whose resolution would prompt revisions to the spec itself. Section 4 is a single-table summary for fast confirm-all sign-off.

## 2. Per-decision summaries

### Decision 1: Tier-subdir vs flat archetype directories

**What's the decision needed?** Should the new directory tree be `archetypes/{household,faith-community,life-group,wisdom-commons}/` (flat) or `archetypes/tier-0/{...}/` (pre-bucketed for future Tier 1+2 / Tier 3)?

**Options:**
- (a) Flat for MVP — short paths now, restructure when Tier 1+2 authoring lands.
- (b) Pre-emptive tier subdirs — `archetypes/tier-0/household/...` from day one.
- (c) Hybrid — flat at MVP plus an explicit migration ADR for Tier 1+2 cutover.

**Recommendation:** **(a) Flat for MVP.**

**Rationale:** The household is "the living core, not one of four parallel Tier-0 worked examples" (spec framing). Tier subdirs at MVP would suggest premature parity across the four canonical narratives and obscure the fact that household carries the seed status. Paths stay short. When Tier 1+2 enters in Sprint 6+, a one-time rename to `archetypes/tier-0/...` is cheap and the moment to introduce tier vocabulary is when it has real referents.

**MVP-blocking?** No — but operator should confirm before Sprint 5 paths get locked.

### Decision 2: lamad/know-thyself-discovery disposition

**What's the decision needed?** Where does `lamad/know-thyself-discovery.feature` (Values Hierarchy + Attachment Style assessments) live in the new taxonomy?

**Options:**
- (a) `archetypes/household/` — self-knowledge built inside the household, the imagodei lens primarily exercised in intimate-care context.
- (b) `cross-cutting/attestation/` — discovery completion produces a milestone attestation, an attestation-substrate primitive.
- (c) New `cross-cutting/imagodei-self-knowledge/` bucket — recognizes the second of the "Imagodei three surfaces" (social profile / self-knowledge / account mgmt).

**Recommendation:** **(c) New cross-cutting bucket — `cross-cutting/imago-dei/self-knowledge/`** as a nested subdirectory under the imago-dei cross-cutting directory already planned in Section 5.

**Rationale:** Know-thyself precedes Qahal participation; it builds the imagodei profile that Qahals then render through their lenses. Section 1.5 of the spec (Imago Dei discriminator) treats inherent dignity as substrate floor; self-knowledge is the human-side surface of the same primitive. Nesting under `cross-cutting/imago-dei/` keeps the discriminator's surfaces visibly co-located. Avoid forcing it into household (it pre-dates Qahal) or attestation (the attestation is the artifact, not the experience).

**MVP-blocking?** No — defer to Sprint 1 brainstorm if operator wants more design time.

### Decision 3: delivery/landing-page disposition

**What's the decision needed?** Merge `delivery/landing-page.feature` with `archetypes/wisdom-commons/landing-page-as-content.feature` (from `protocol/landing-page-dogfood`), or keep as INF:delivery smoke?

**Options:**
- (a) Merge into `archetypes/wisdom-commons/landing-page-as-content.feature`.
- (b) Keep as `infrastructure/delivery/landing-page.feature` smoke.
- (c) Graduate the basic-rendering scenarios to a Cypress smoke (per Section 3.2 graduate-candidate pattern).

**Recommendation:** **(b) Keep as infrastructure/delivery/ smoke**, with operator noting in scenario header that wisdom-commons archetype framing lives in the dogfood file.

**Rationale:** The two files cover different audiences. landing-page.feature is "does the page render with its declared sections?" — a delivery-pipeline assertion. landing-page-dogfood.feature is "the landing page is itself a wisdom-commons artifact carrying an in-kind REA Commitment" — the dissolution-principle-aligned human-experience scenario. Merging would erase the substrate/experience distinction. Graduating to Cypress is overkill for a single file that exercises a real archetype boundary.

**MVP-blocking?** No.

### Decision 4: content/relationship-idempotency split

**What's the decision needed?** Split `content/relationship-idempotency.feature` — graduate the import-idempotency scenarios to unit tests and keep "spouse relationship authored by both parties is created once" as a T0:household scenario? Or keep the file whole?

**Options:**
- (a) Split — extract household scenario; graduate the seed-hygiene scenarios.
- (b) Keep whole at `archetypes/household/spouse-bidirectional-authorship.feature` (with a tag for the hygiene scenarios).
- (c) Graduate the whole file (lose the household-archetype framing — not recommended).

**Recommendation:** **(a) Split.**

**Rationale:** Per memory `feedback_a2o_is_human_experience_not_dev_bugs`, seed-hygiene scenarios should never be feature files; they are anti-regression unit tests masquerading as learner experience. The Adam-Eve bidirectional-authorship scenario is canonical T0:household — it expresses the household's structural fact that marriage is authored by both parties. Splitting preserves the canonical scenario and removes corpus noise.

**MVP-blocking?** No — but the split sharpens the household corpus that Sprint 1 UX will draw from.

### Decision 5: content/ssr_capability disposition

**What's the decision needed?** Graduate the SSR capability-negotiation scenarios to substrate-level integration tests, keeping only the human-facing SSR-as-experience scenarios as feature files? Or keep `content/ssr_capability.feature` whole?

**Options:**
- (a) Graduate capability-negotiation scenarios; keep only human-experience scenarios (those already exist in `ssr/browser-hydrates-without-flash`, `ssr/external-webfetch-renders-content`).
- (b) Keep whole at `infrastructure/ssr/ssr-capability.feature`.

**Recommendation:** **(a) Graduate the capability-negotiation scenarios.**

**Rationale:** Per `project_ssr_is_compute_capability_claim` memory, SSR capability is a compute-shape claim feature-gated locally and advertised via compute-report — substrate plumbing. The human experience is "I see content on a slow connection / on a non-JS client / in a social-card preview," which is already covered by three other ssr/*.feature files. The capability-advertised-honored-degraded plumbing is an integration assertion, not a learner-shaped scenario.

**MVP-blocking?** No.

### Decision 6: qahal/collective-governance rewrite scope

**What's the decision needed?** Full vote-as-decision → witness-as-consensus rewrite for `qahal/collective-governance.feature`, or partial preservation?

**Options:**
- (a) Full rewrite — replace "vote on a proposal" with "witness published; affected Qahals respond" per Section 7.5 of the spec.
- (b) Partial — keep proposal-authoring scenarios; rewrite vote scenarios; preserve "block with justification" as "steward refuses commons participation."
- (c) Defer rewrite to Sprint 5 storyteller pass; mark file with a known-drift comment for now.

**Recommendation:** **(a) Full rewrite during Sprint 5 storyteller pass.**

**Rationale:** Section 7.5 of the gospel-tier spec is explicit: "voting is replaced by witness." Keeping vote-shaped scenarios in the corpus while the spec teaches witness-as-coordination is exactly the kind of substrate/scenario drift the archaeology surfaced. A full rewrite is the cheapest way to land the witness primitive in executable form — and Sprint 5 storyteller authoring is the right venue. Splitting halves leaves landmines. The ranked-choice curriculum scenario is genuinely separable and may fit T0:life-group better — flag as a Sprint 5 split decision.

**MVP-blocking?** **Yes** — at the level of "the canonical governance scenario in the corpus must not contradict the spec." Sprint 5 cannot author wisdom-commons without resolving this.

### Decision 7: Anonymous voting in collective-governance

**What's the decision needed?** The anonymous-voting scenario in collective-governance currently posits anonymous attestations. The spec's interpretability requirement (Section 1.5: substrate floor of inherent dignity; transparent attribution as alignment substrate) treats anonymous witness as a substrate violation. Keep or graduate?

**Options:**
- (a) Graduate — anonymous attestation defeats the interpretability/transparent-attribution requirement; remove entirely.
- (b) Reframe as **pseudonymous-with-recoverable-provenance** — witness is published under a pseudonym; commons-elohim can surface authorship under defined conditions (consent, harm, council convening).
- (c) Keep as-is — accept the substrate violation for now.

**Recommendation:** **(b) Reframe as pseudonymous-with-recoverable-provenance.**

**Rationale:** Anonymous-as-such is a substrate violation per the memory `project_values_forward_disclosure_accountability` and the spec's transparent-attribution requirement. But complete identity-bound publication chills legitimate dissent (whistleblowing, minority objection inside a power-imbalanced Qahal). The substrate's answer should be pseudonymous-with-conditional-disclosure — the same shape recovery uses (graduated authority for de-anonymization). This is genuinely a Sprint 2 design question more than a Sprint 0.5 disposition — the digest's recommendation is to mark the scenario as `@design-question` and resolve in Sprint 2 brainstorm.

**MVP-blocking?** **No, but spec-bearing** — see Section 3 below.

### Decision 8: persona-testnet-validation memorialize vs maintain

**What's the decision needed?** Confirm or revise the proposal to memorialize `deployment/persona-testnet-validation.feature` — preserve as historical reference for "20 humans on one box" milestone but no longer actively run, because the alpha-cluster 6-peer topology supersedes the 5-conductor topology.

**Options:**
- (a) Memorialize per archaeology recommendation.
- (b) Maintain as active a2o coverage of the household + faith-community + local-economy seed-topology lesson.
- (c) Memorialize the specific 5-conductor framing; extract a present-tense archetype scenario from the lesson and keep that active.

**Recommendation:** **(c) Memorialize the 5-conductor framing; extract present-tense archetype scenario.**

**Rationale:** The lesson — that household + faith-community + local-economy can run on commodity hardware — is canonical and worth keeping live. The 5-conductor + 20-humans-on-one-box specifics are bound to a moment the alpha cluster has moved past. Memorializing the specifics while extracting the substrate-level lesson into a `archetypes/faith-community/` or `infrastructure/deployment/` scenario preserves both archaeological record and active coverage.

**MVP-blocking?** No.

### Decision 9: Graduate-candidates confirmation

**What's the decision needed?** Confirm graduate-to-unit-test for three files: `deployment/conductor-admin-reachability.feature`, `deployment/staging-validation.feature`, `browser/navigation-browser.feature`. All three are flagged as anti-regression smoke without learner-experience content.

**Options:**
- (a) Graduate all three — convert to integration/Cypress assertions outside the .feature corpus.
- (b) Keep all three as infrastructure scenarios with explicit `@smoke` tag.
- (c) Graduate the two deployment-shape ones; keep navigation-browser as the browser-route-mounts canary.

**Recommendation:** **(a) Graduate all three.**

**Rationale:** Per `feedback_a2o_is_human_experience_not_dev_bugs`, a2o is human experience, not dev-side regression nets. All three exercise "does the page/socket load?" which is the definition of smoke. They should live as Cypress smoke or integration assertions — still maintained, still gating CI — but not occupy archetype-scoped corpus weight. Keeping `@smoke`-tagged feature files dilutes the corpus's signal: archaeology already noted these as 3 of the 76 that don't earn their place.

**MVP-blocking?** No.

### Decision 10: Tag-rename pass scope

**What's the decision needed?** Adopt `@archetype:household`, `@cc:reach`, `@inf:recovery` tag conventions now, or defer to Sprint 5+?

**Options:**
- (a) Land tag rename in the same migration pass that moves the files.
- (b) Defer tag rename to Sprint 5 — file moves first, tags second.
- (c) Add new tags as additions in the migration pass; deprecate old pillar tags in Sprint 5.

**Recommendation:** **(c) Add new tags in migration pass; deprecate old pillar tags in Sprint 5.**

**Rationale:** Tag-rename is mechanical; doing it during the file-move pass is one operation, not two. Deprecating old pillar tags (`@lamad`, `@qahal`, `@shefa`) in a second pass lets CI / cucumber filters keep working through the transition. This is the lowest-risk path. Doing all at once (option a) risks breaking unrelated cucumber filters; deferring entirely (option b) leaves the corpus in a half-migrated tag state for a full sprint.

**MVP-blocking?** No.

## 3. Decisions that bear on the spec framing

Most decisions here are about disposition of existing files; they do not touch the spec. Three do — flagging here so the operator can decide whether to revise the spec, the digest's recommendation, or both.

### Decision 6 (governance rewrite) — touches Section 7.5 of the spec

The recommendation is a full rewrite of `qahal/collective-governance.feature` aligned to "voting is replaced by witness" (Section 7.5). The spec is already explicit about witness-as-coordination; **no spec revision needed**. The decision is about whether the scenario corpus catches up to the spec in Sprint 5 (recommendation: yes).

### Decision 7 (anonymous voting) — touches Section 1.5 of the spec

The recommended reframe (pseudonymous-with-recoverable-provenance) is consistent with Section 1.5's interpretability-and-transparent-attribution requirement, but introduces a new primitive — **graduated de-anonymization authority** — that mirrors the recovery substrate's intimate-quorum → council pattern. The spec does not currently articulate this. **If the operator accepts the recommendation, Section 1.5 of the spec would benefit from a paragraph naming pseudonymous-with-conditional-disclosure as the substrate's resolution of the dissent-vs-attribution tension.** This is the only decision that would prompt a spec edit.

### Decision 2 (know-thyself bucket) — touches Section 1.5 framing

The recommendation creates a `cross-cutting/imago-dei/self-knowledge/` subdirectory. Section 1.5 of the spec treats Imago Dei primarily as a discriminator — the substrate floor that refuses dignity-denying Qahal configurations. The "self-knowledge as imagodei surface" framing is more developed in the `project_imagodei_three_surfaces` memory than in the spec itself. **Light spec edit recommended:** Section 1.5 could note the three imagodei surfaces (social profile, self-knowledge, account management) and identify self-knowledge as the human-side of the discriminator. Optional, not blocking.

### What does NOT need spec revision

- The household-as-living-core / value-scanner / lived-contrast claims (Sections 1.2, 4 preamble, 7.6a) are untouched — the digest's recommendations preserve household-as-seed status.
- The dissolution principle (Section 2.11) is reinforced by the archaeology's flag on `auth/user-management.feature` (transient-bridge rewrite, Sprint 5).
- The commons-elohim co-steward role is named correctly in the archaeology (renamed from "shadow" per memory).
- The collective-archetype catalog (Sections 5-6) is untouched — Tier 1+2 / Tier 3 stays empty by design.

## 4. Quick-decide table

| # | Decision | Recommendation | Blocking? | Defer to? |
|---|---|---|---|---|
| 1 | Tier-subdir vs flat archetype dirs | **Flat for MVP** (`archetypes/household/`, etc.) | No | Sprint 5 path-lock |
| 2 | lamad/know-thyself disposition | **`cross-cutting/imago-dei/self-knowledge/`** | No | Sprint 1 brainstorm if more design wanted |
| 3 | delivery/landing-page disposition | **Keep as `infrastructure/delivery/landing-page.feature` smoke**; dogfood file holds the archetype framing | No | — |
| 4 | content/relationship-idempotency split | **Split** — extract Adam-Eve household scenario; graduate hygiene scenarios to unit tests | No | — |
| 5 | content/ssr_capability disposition | **Graduate capability-negotiation scenarios**; human-facing SSR scenarios already covered | No | — |
| 6 | qahal/collective-governance rewrite scope | **Full rewrite** in Sprint 5 — vote → witness per Section 7.5 | **Yes** (Sprint 5 entry) | Sprint 5 storyteller pass |
| 7 | Anonymous voting | **Reframe as pseudonymous-with-recoverable-provenance**; mark `@design-question` | No (but spec-bearing — see §3) | Sprint 2 brainstorm |
| 8 | persona-testnet-validation memorialize | **Memorialize 5-conductor framing**; extract present-tense archetype scenario | No | — |
| 9 | Graduate candidates (3 files) | **Graduate all three** to Cypress / integration tests | No | — |
| 10 | Tag-rename pass scope | **Add new tags in migration pass**; deprecate old pillar tags in Sprint 5 | No | — |

---

*Digest end. Status: ready for operator confirm-all or per-item revision. Once signed off, Decision 6 is the only one that gates Sprint 5 entry; the rest shape execution but do not block.*
