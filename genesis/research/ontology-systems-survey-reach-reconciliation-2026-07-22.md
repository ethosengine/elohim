---
title: Ontology Systems Survey — inputs for the reach-vocabulary reconciliation
status: Capture
date: 2026-07-22
---

# Ontology Systems Survey — inputs for the reach-vocabulary reconciliation brainstorm

**Method:** deep-research workflow (6 search angles → 25 sources fetched → 118 claims extracted → 25 adversarially verified by 3-vote panels: 15 confirmed, 10 refuted) + journal mining for extracted-but-unpaneled claims from the authorization/semantic-web/domain angles.
**Purpose:** "Think outside the REA box" — what REA might be missing, before brainstorming the 5-way reach-vocabulary reconciliation (`genesis/data/timeline/backlog/reach-vocabulary-frontend-strand.md`; resilience README roadmap item 13).

**Verification key:** ✅ = survived 3-vote adversarial panel · ◐ = extracted from a primary source but not paneled (verification budget concentrated on Palantir/REA angles) · ✗ = refuted by panel.

---

## 1. What survived verification (the confirmed core)

1. ✅ **Palantir Foundry: ontology as the interaction surface, not an annotation over storage.** The ontology is the "digital twin" all stakeholders (human and AI) act *through*; nobody reads raw source tables. (palantir.com docs, 3-0 / 2-1)
2. ✅ **"Semantics must be paired with kinetics."** A noun/type model is insufficient without a first-class *action* model — Foundry models the mutation surface (transactions through multi-step updates) inside the ontology, framed as representing "the decisions of an enterprise, not simply the data." → **Lesson: model how reach CHANGES (grant/revoke/serve/heal) in the ontology, not only what reach IS.** (3-0)
3. ✅ **REA→ValueFlows→hREA is an unbroken, discipline-grounded, ISO-standardized lineage** (ISO/IEC 15944-4) already proven on a P2P agent-centric substrate (hREA on Holochain, GraphQL adapter, active through Dec 2025, beta). (3-0 ×3)
4. ✅ **The perspectival-label pattern:** REA/ValueFlows stores ONE neutral "independent view" event and *derives* each agent's perspectival label ("purchase" vs "sale") at read time. → The direct template for the five drifted reach vocabularies: neutral stored facts, vocabularies as derived perspectival projections. (3-0)
5. ✅ **Operational vs knowledge infrastructure:** REA separates instances (events/resources/agents) from a type-image/policy layer whose relationships *restrict legal configurations* of the operational layer; ValueFlows makes it three strata — Knowledge (rules/policies) / Plan (offers, promises) / Observation (what really happened) — never collapsed into one record type. → **Reach policy belongs in a knowledge layer validating instances, not as a status enum on rows.** (3-0 ×2)
6. ✅ **Axioms as executable knowledge; derived status classification is proven-old:** the CREASY/Prolog work (~1999) derived accounts-receivable/-payable/prepayments *at query time from formal definitions over REA primitives* — "a claim exists where there is a flow without the corresponding dual flow." The definition IS the explanation. (2-1, 3-0; prototype-scale caveat)
7. ✅ **One authoritative artifact, generated projections:** ValueFlows designates a single Turtle file as system-of-record; JSON-LD/HTML/OWL are derived; vocabulary kept technology-agnostic to outlive serializations. → **The five reach vocabularies drifted precisely because no single generative source-of-record existed.** (2-1, 3-0)

**Notable refuted claims (do not rely on):** Foundry's "exactly three schema kinds" framing (0-3); Foundry authorization-as-derived-at-evaluation-time as a co-equal pillar (1-2 — get this pattern from Zanzibar instead); hREA cross-backend interop with Bonfire "unchanged UIs" (0-3); ValueFlows Knowledge-level as clean federation extension point (1-2 — federation remains genuinely unsolved); the schema.org 700k-domain adoption figure and "universal ontological agreement is impossible" as sourced (1-2 / 0-3 — quote-support failures, not necessarily false).

---

## 2. The authorization lineage (◐ extracted, primary sources, unpaneled — flagged by the synthesis itself as the known gap requiring this mining)

This is the crux — reach is authorization-with-explanation — and the material is strong:

- ◐ **Zanzibar's declared/derived split is the field's cleanest:** stored relation tuples (`object#relation@user`) are declared facts; **userset rewrite rules** (computed_userset, tuple_to_userset, union/intersection/exclusion) derive permissions at query time. Policy changes without rewriting stored data.
- ◐ **Scale proof:** >2 trillion ACLs, 95% of checks <10ms, 99.999% uptime — derived relationship-computed authorization does NOT require precomputed enums, at planetary scale.
- ◐ **Explanation is API-native:** the Expand API returns the effective userset as a *tree showing the chain of relations granting access*; SpiceDB's `CheckPermission withTracing` returns a debug_trace of paths traversed (with real runtime overhead — explanation costs, meter it); `zed permission check --explain` exposes timing, cache hits, cycle detection.
- ◐ **Derived permissions are testable as invariants:** SpiceDB "Expected Relations" exhaustively enumerates every derivation path to a relation; `zed validate` unit-tests the authorization ontology offline against YAML fixtures. Caveated access has explicit "maybe" semantics.
- ◐ **The "new enemy problem":** derived authorization over distributed replicas requires *external consistency* — a revocation must be respected before newly-added sensitive content is served. **Directly load-bearing for a gossip-propagated P2P substrate serving from stale views.** Zanzibar's answer: per-request freshness tokens ("zookies") letting callers trade staleness for latency — a transferable pattern for reach checks under DHT propagation delay.
- ◐ **Schema evolution is constrained by live data:** SpiceDB refuses to drop a relation while instances exist — vocabulary migration must be data-aware, not just schema-aware.
- ◐ **Solid interop:** authorization scope *relationally derived* (scope `Inherited`: access to child data derives from shape-tree references to an authorized parent) rather than enumerated per-resource; parties agree on data *shape*, not storage layout.

## 3. Cautionary datapoints from the wider field (◐ unpaneled)

- ◐ **Cyc, the centralized-reasoner datapoint:** ~40 years, ~30M assertions, ~$200M, ~2000 person-years; never outperformed conventional systems; the general reasoner was too slow, fragmenting into **>1100 specialized inference engines**; closed anti-federation stance (OpenCyc killed to prevent "fragmenting") accelerated irrelevance. *Read carefully — see §4.7 for what this does and does not prove.*
- ◐ **Semantic-web survivorship:** practitioners ranked OWL and SPARQL most problematic for adoption; what went mainstream were *bridge technologies in developers' native shapes* (JSON-LD = JSON + sugar; schema.org). → **Ship the reach ontology in native shapes (ts-rs types, schema enums, Rust), not a new formalism.**
- ◐ **Foundry's two heresies worth noting:** it performs *no* OWL-style inference (all relationships modeled explicitly) and cannot export to open standards — the lock-in critique ("if the semantic layer is controlled by an external vendor, sovereignty is weakened") is precisely the commons argument for owning ours.
- ◐ **FIBO:** modular sub-ontologies partitioned by domain (a precedent for orthogonal-axis decomposition), but adoption chronically impeded by scarce ontology-engineering expertise and legacy-model resistance — heavyweight OWL lineage struggles even with institutional backing.

---

## 4. Synthesis: what REA is missing for the reach epic

REA gives us: neutral events + perspectival derivation, knowledge/plan/observation strata, executable axioms, said-vs-did (commitment/fulfillment) variance. It does NOT give us:

1. **An authorization algebra.** REA has no native "may X see/serve Y" concept. Zanzibar does: tuples + rewrite rules + set algebra (union/intersection/**exclusion** — REA has no negation), with per-request freshness. **Borrow the whole shape.**
2. **The consistency/staleness contract.** REA assumes a shared ledger context; a P2P reach system serves from replicas. The new-enemy problem and zookie pattern are the missing engineering: reach verdicts need an explicit freshness semantics tied to DHT propagation (amber/green is already our witnessing signal — extend the thinking to revocation ordering).
3. **Explanation as a metered API, not a report.** REA explains via the record's structure; Zanzibar/SpiceDB expose explanation as a *first-class query capability with acknowledged cost*. Design the reach verdict surface with `explain=false` cheap-path and `explain=true` traced-path from day one.
4. **Testability of the derivation itself.** Expected-relations enumeration + offline fixture validation = the authorization ontology as a unit-testable artifact. Our reconciliation should ship with a fixture harness: "given these tuples/commitments, agent A sees exactly {…}" — invariants, not vibes.
5. **Observer/negotiation term.** Neither REA nor Zanzibar models the *viewer's declared intent*. The imagodei viewer-lens spec (relationship_class × intimacy, consent-gated) plus the announcement thread (announce who/what/intent → infer-or-negotiate → said-vs-did variance feeding trust) is genuinely novel composition — REA supplies the variance algebra (Intent→Commitment→Fulfillment), Zanzibar supplies the check algebra, UMA-style claims-presentation supplies the negotiation shape.
6. **Vocabulary-drift prevention machinery.** The single-source-of-record + generated-projections pattern (ValueFlows' .ttl → everything) matched to our existing codegen spine (schema.json → ts-rs → generated TS). The reconciliation's durable fix is not picking a winner among five vocabularies — it's making four of them *generated* or *renamed to their true axis* (locality, custody) so drift is structurally impossible.
7. **Where inference lives — the centralization re-reading of the Cyc lesson.** The surface lesson is "never build a general reasoner." The honest lesson is narrower: **a general reasoner failed as a centralized artifact** — one organization hand-curating one giant closed KB, one inference bottleneck fragmenting into 1100 special-case engines, federation forbidden. That is a *bottleneck-of-centralized-intelligence* failure, and the reach epic's guiding principle exists precisely to not recreate it. We are building for the era where **inference is ubiquitous — in every home** — and the semantic web moves *out of the datacenter*: each node reasons locally over its own witnessed slice (verify-locally-then-serve is already the substrate's trust contract), explanations are computed at the edge where the evidence lives, and the network converges socially (attestation, gossip, consent) rather than through a central reasoner. Distributed inference load is the design assumption, not the failure mode. The near-term engineering rail stands — derive with plain code over materialized facts in the hot path (the content-graph-native-Rust and P1 storage-as-projection decisions) — but as a *sequencing* choice, not a ceiling: the verdict/explanation surface should be designed so richer local inference can grow into it as household compute grows, without re-centralizing. Cyc also warns against the *anti-federation* reflex specifically: the commons answer is vocabularies and evidence that federate by construction, so no one node ever needs to hold the whole ontology to reason well about its part.

## 5. Carry-forward into the brainstorm

- Declared reach stays the schema-8 enum, DNA-notarized — the *declared floor* (Knowledge-level policy + commons-band serving contract; the bathtub curve: crypto-gate at private end, tuple-check middle, static commons end).
- Effective reach = derived verdict: `verdict(content, viewer?, announcement?, freshness) → {Allowed|Blocked|Pending, evidence, explain?}` — generalizing the existing `ReachVerdict`/`StandingEvidence` shape; monotone-narrowing composition, hard floors sovereign.
- Geographic-8 → renamed locality/placement vocabulary (dataplane concern); Part-V-5 → custody vocabulary (CustodianCommitment / Mishpat::Commitment); `VALID_REACH_LEVELS`-6 and services-8 die in migration.
- Announcement slot in the verdict interface from v1 (anonymous→commons; authenticated→lens; announced→negotiable), variance events feeding standing.
- Fixture-based verdict test harness as part of the reconciliation deliverable.
- Open questions the research left standing: federation of independently-evolved vocabularies remains unsolved everywhere (AT Protocol lexicons worth a focused look); derived-status performance at P2P scale (cache-as-projection-of-truth); prior art for multi-axis composite derivation is thin — we'd be near the frontier there.

**Sources:** palantir.com docs (2), valueflo.ws (3), h-REA/hREA repo, Geerts & McCarthy 2000 (ResearchGate), authzed.com (3: annotated Zanzibar paper, SpiceDB validation docs, learn/zanzibar), Solid data-interoperability spec, Hitzler et al. "Semantic Web: Two Decades On" (SW-190387), ACM Queue 2857276, Cyc essay (yuxi-liu-wired), FIBO overview (globalfintechseries), Bluesky/AT Protocol paper (arXiv 2402.03239), + blogs (labeled).
