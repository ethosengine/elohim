---
title: "Reach Ontology/Vocabulary Split — guiding principles for the 5-way reconciliation"
id: reach-ontology-vocabulary-split-spec
status: Draft
class: protocol-canonical
context-tier: disclosed
steward: rust-architect
graduation-trigger: superseded-by-implementation — graduate once the reconciliation sprint lands the canonical vocabulary, verdict surface, and fixture harness
created: 2026-07-22
domain: D2
topic: [reach, ontology, vocabulary-drift, verdict, authorization, announcement, epr-rea, mishpat, trust]
cites:
  - genesis/research/ontology-systems-survey-reach-reconciliation-2026-07-22.md
  - genesis/research/letter-to-rea-practitioners-observed-presence-2026-07-22.md
  - genesis/data/timeline/backlog/reach-vocabulary-frontend-strand.md
  - imagodei-profile-page-viewer-lens-design | Imagodei Profile-vs-Page | sha256:05caf5687b42f4ba | path: genesis/docs/superpowers/specs/2026-06-22-imagodei-profile-page-viewer-lens-design.md
  - elohim/sdk/schemas/v1/enums/reach.schema.json
  - elohim/elohim-storage/src/services/reach_earning.rs
  - elohim/elohim-storage/src/p2p/reach_authorization.rs
---

# Reach Ontology/Vocabulary Split — canonical guiding principles

> **Purpose.** The next sprint that picks up the reach-vocabulary reconciliation (resilience README roadmap item 13 + `reach-vocabulary-frontend-strand.md`) reads THIS document to know the guiding principles. It does not re-litigate them; it plans against them. Method/provenance: 2026-07-22 session — ontology-systems deep-research survey (adversarially verified) + operator-adjudicated thesis. Research grounding: the companion survey; audience-facing rationale: the REA letter (both in `genesis/research/`).

## 0. Thesis (one line)

**Reach is not a property of content; it is the standing verdict of a governed conversation between edges.** Store declarations and witnessed events; derive everything else at the evidence-holding edge; every verdict carries its explanation and its freshness; every observer may announce and be measured by said-vs-did variance; the intimate edge — not the datacenter — is where intelligence governs.

**And the ontology's job is demarcation, not decision** (added 2026-07-23, after the OWL 2 graduation evaluation). It describes where the deterministic floor is, where the ceiling of delegated judgment sits, and marks the band between as *not the ontology's to decide*. This is what reconciles the reach work with the manifesto's "values alignment over rules enforcement" (`manifesto.md:179`): that principle refuses **codification of the value domain**, not structure as such — its own gloss three lines later is about restoration over punishment, and `constitution.md:662-673` already ships a decision procedure. Rules govern the floor; judgment governs the middle. A rule layer large enough to decide the middle would have to encode the lens choice, and the lens choice would become law — which is precisely the failure written law exhibits and this design exists to avoid.

## 1. The split: one ontology, many projected vocabularies

The five drifted "reach" vocabularies are ontological compression of orthogonal axes. The reconciliation does NOT pick a winner among five — it demotes each to its honest role:

| Vocabulary (today) | True axis | Disposition |
|---|---|---|
| Schema 8 (`private…commons`, DNA-notarized) | **Declared reach floor** (relational sensitivity, content-scoped, no viewer term — that is a feature) | **CANONICAL declared vocabulary.** Stays the schema enum; source-of-record `elohim/sdk/schemas/v1/enums/reach.schema.json`; DNA `CORE_REACH_LEVELS` remains its notarized twin. |
| Rust services 8 (`personal…district/public`, `reach_earning.rs`) | none (stale brainstorm descendant) | **DIES.** Migrate to schema 8. |
| TS geographic 8 (`private/invited/local…bioregional/regional/commons`, 4 sites) | **Locality/placement** (dataplane: replication, eviction, caching) | **RENAMED out of "reach"** → a `locality`/`placement` vocabulary. Single TS edit point: `elohim/sdk/storage-client-ts/src/protocol-core.model.ts`; the other 3 sites re-export. Not part of the reach enum. |
| Resilience Part-V 5 (`household…organization/commons`) | **Custody** (who holds/replicates for whom) | **RENAMED** → custody vocabulary anchored on `CustodianCommitment` / `Mishpat::Commitment` lineage. |
| `VALID_REACH_LEVELS` 6 (`…federated…`, holochain.model.ts) | none (false "matches Rust" claim) | **DIES.** |

**Drift-prevention law (ValueFlows .ttl lesson, verified):** exactly ONE generative source-of-record per vocabulary; every other appearance is a generated projection (schema.json → codegen → ts-rs/Rust constants) or an explicit re-export. A vocabulary value hand-typed in a second place is a defect. Add a schema-contract test that fails when Rust/TS/DNA disagree with the schema.

## 2. Declared floor vs derived verdict (the two-layer law)

- **Declared reach** = the author's Knowledge-level policy on a content unit. Small, closed, ordinal, DNA-validated at the integrity zome, cheap, no viewer term. It is a *commitment*, not a computation.
- **Effective reach** = a **derived verdict**, computed where the evidence lives:
  `verdict(content, viewer?, announcement?, freshness) → { Allowed | Blocked | Pending, evidence, explain? }`
  — generalizing the existing `ReachVerdict`/`StandingEvidence` shape (`reach_earning.rs`) and the reach_authorization pre-auth stage. The verdict is never stored as truth; caches of it are Category-C operational projections, reconstructable.
- **Composition law (hard):** derived layers may only **narrow, never widen**. The declared floor and the key envelope are sovereign; no standing score, lens, negotiation, or inference may override them. Deny-overrides, always. A hard denial is still explained (evidence: "declared private; no consented edge; no key").
- **Measured 2026-07-23 — the viewer term is not built.** `reach_earning.rs:81` is `evaluate(local_agent, author, requested_reach, conn, registry)`: no viewer parameter, no timestamp on `ReachVerdict`/`StandingEvidence`, sole caller `epr_compose.rs` — this is **author-side compose, not per-viewer serve**. The verdict *signature* reserves `viewer?`/`announcement?`/`freshness`; the *implementation* is deferred against the trigger in §4. Do not plan as though per-viewer reads exist.

## 2a. The ceiling marker (the most important term in the vocabulary)

Added 2026-07-23. **The ontology must be able to say "this is not mine to decide."** No external formalism has this term — OWL, SHACL, Cedar, Rego, Datalog and Zanzibar are all *total within their scope*: silence means `unknown` (open-world) or `deny` (default-deny), never *"a person must decide."* Demonstrated, not argued: an OWL 2 DL encoding of `Pending ≡ ¬Visible ⊓ ¬NotVisible` returns `Pending == owl:Nothing` under HermiT — **provably uninhabitable**, because `ObjectComplementOf` is total and partitions the domain, leaving no residue for a third value.

We have authored this term **four times independently**. One becomes canonical; the rest become projections or aliases.

| Home | Term today | Layer |
|---|---|---|
| `.epr-meta` rule algebra | `class: ask` | authoring |
| `constitution.md:662-673` | `FlagForHuman(layer, ambiguity)` | governance |
| `reach_earning.rs:56-70` | `ReachVerdict::Pending` | serving |
| `eprfs-meta/src/evaluation.rs:88` | `ValidatorOutcome::Flag{reason}` | validation |

**Laws.**
1. It is a **first-class verdict value** — never a fallthrough, a timeout, an error, or the absence of a rule.
2. It is **requisite-variety routing**, not a queue. `elohim-as-viable-system-2026-06-04.md:54` reads the existing `NovelSituation`/`InsufficientAuthority` escalation reasons as "variety the local level cannot absorb gets passed up to a level with more — requisite variety routing, written as an enum." The referral therefore carries **which layer and why**, per the constitution's own signature; subsidiarity is the routing function.
3. **`Unavailable` is split, not deleted.** A provider that cannot answer must not be forced to lie. It stays a provider report and is forbidden from ever yielding permit. The law is polarity: **authoring may fail open; serving and validation may not.**
4. **A referral that can only be heard by the thing being complained about is not a referral.** The un-mediated algedonic channel is a *dependency* of this term, not a separate epic — `beer-designing-freedom-elohim-critique-2026-06-04.md:27` records that "algedonic" appears nowhere in source, and Gap 2 names doorway as its natural home.
5. **The vocabulary routes; it never names the resolver's species** (clarified 2026-07-23, refined same day). `Refer` carries layer + reason — whether it lands with elohim counsel (machine-speed, reversible moves: pause/freeze/surface/verify), a human, or a quorum is **governed routing**, itself Knowledge-level policy and lens-plural per §2b. **Neither species is terminal; the method is** — simulate before adoption, observe empirically after, revert on divergence (`where-it-ends:59-61`; the eternity clause protects the dignity floor and the method itself, not a species). The floors are anti-capture devices pointed in *both* directions: against human capture, the global value definitions require elohim consensus (`manifesto.md:280` — human institutions are the proven capture vector for value definitions; best-self-not-present-self guards against duress; counsel is non-dismissable mid-attack); against elohim capture, the algedonic **signal** floor bypasses every elohim, consent/exit rights stand (consequences fall on participation, never the person), sortition guarantees participation, and genuine value conflicts reach layer-consensus with humans in it. Humans are not the deciders above the elohim — they are the **evidence**: flourishing has no instrument except the people living it, so elohim stewardship (test, simulate, experiment, follow the evidence) is answerable to witnessed lived experience, held lens-plural per §2b so the metric itself cannot be captured.

## 2b. Plural middle — lens selection

Added 2026-07-23. The band between floor and ceiling is not "undecided"; it is **multiply-decided**. There are several valid readings, each with a warrant, and the judgment is *which lens applies to this kairos*. "GDP is one way to look at it, and not the best lens for this moment" is the general case; the same move applies to which constitutional reading governs a community now.

This is the anti-flattening operation the protocol already performs on value, applied to a second axis. Currency flattens value into one scalar and REA/ValueFlows un-flattens it into "many currencies-as-remembering… rather than one coin that flattens them all into price" (`values-forward.md:107`). Written law flattens norm the same way, and the flattening is what legalism and bad faith attack. `EprHeadCoupling` already carries `value` and `governance` as co-equal legs; only the value leg has been un-flattened so far.

**Where the plurality may live.** Not in the floor. The floor's worth is that two peers materialize it byte-identically — the reason stratified Datalog's single perfect model was selected and the reason implementation-defined verdicts *fork the DHT*. So:

- **Floor** — deterministic, single-model, bit-identical across peers, and **lens-invariant**.
- **Middle** — plural, enumerable, lens-dependent; peers may legitimately differ. Forcing consensus here is the "one canonical ledger" failure Stance I.2 already rejects.
- **Ceiling** — the judgment selecting a lens for this kairos: attributed, witnessed, revisable.

**Selection discipline.** Constraining *which* lens may be chosen is an Ashby violation — a fixed rule set has fixed variety and will be outflanked by novel bad faith, which is why lens-minting is **free but witnessed**: an attributed artifact carrying its warrant, visible in the same cascade as any authored thing. Not gated; not invisible. What actually defeats forum-shopping is structural and already written: `manifesto.md:300` — an elohim is *"accountable to the network's wisdom, not loyal to its user the way a tool is loyal to its owner."* **A council with no client cannot be shopped.** Hold the defensible form of the variety claim (`elohim-as-viable-system-2026-06-04.md:68`): the elohim does not achieve requisite variety *for a person*; it is a high-variety attenuator **that asks the human how the attenuation should be done**, making variety-attenuation accountable to the attenuated.

**REA supplies the middle's native measure.** `epr-rea/src/fold.rs::fulfillment` returns `FulfillmentStatus{fulfilled_quantity, expected_quantity, ratio()}`: a commitment at 0.6 is neither true nor false, it is *60% discharged*. OWL says unknown, Cedar says deny, Datalog says not-derivable — REA says "partially fulfilled, here is the number." Our four judgment terms are **discrete re-inventions of a continuous measure we already own.** It enters as **witness evidence, never as a verdict variant** — the decision stays discrete (serve or don't); the evidence feeding the judgment is continuous.

## 3. The serving-cost bathtub (reach is also a performance contract)

Each declared level maps to a serving class — this mapping is part of the canonical vocabulary's meaning:
- **commons/public:** viewer-independent verdict → compute once, serve statically (cacheable, crawlable; the existing `commons`/non-`commons` seam at `validate_project_epr_commitment` is the cliff).
- **middle bands (community/familiar/trusted):** group-scoped verdicts → materialized relationship tuples + indexed check (Zanzibar shape), background reconciliation absorbs graph-walk cost. This is where trust-compute concentrates.
- **intimate/private/self:** gate is **cryptographic possession** (key envelope), not evaluation; per-viewer disclosure-fold cost is affordable because N is small. Doorway cannot leak what it cannot read.

## 4. Freshness is a first-class verdict term (the clock)

Serving from replicas ⇒ the new-enemy problem: **a revocation must order before what it protects.** Every verdict carries a freshness requirement/attestation. Anchor on the existing amber/green DHT-witnessing signal — derived from `dht_anchor_hash` presence, never stored — and extend it to govern revocation ordering. Zookie-style per-request freshness (caller trades staleness vs latency) is the transferable interface pattern for gossip-propagation delay.

## 5. The observer/announcement slot (design in from v1)

The verdict interface accepts an optional **announcement**: who/what the visitor is, on whose behalf (delegation cited as a `Mishpat::Commitment` — standing + revocation on-chain, composing with the REA compute-commitment primitive), and declared intent.
- **Identity discipline (the 'A'):** announced identity must be **resolvable** and its claims **inspectable**. Internally this already exists in DID-document shape: `agent_cid` is the canonical identifier; `AgentPeerBinding`/`peer_transport_manifest` resolve transport identities to it (never string-compare across namespaces); `ContributorPresence` claim verification (email/dns-txt) is credential-issuance-shaped; Mishpat delegation is a capability credential. **Interop:** the announcement slot should accept a W3C **Verifiable Presentation** as one announcement format (external dispatched agents will arrive carrying DIDs/VCs), mapping issuer-signed claims into verdict evidence. Adopt the DID/VC *mechanics* by reference; the framing stays community-grounded per the identity-ontology guard (claims community/institution-issued and backstopped; recovery survives key loss; a key is custody, not personhood).
- **v1 behavior:** anonymous → commons band (safety-metadata logging only); doorway-session-authenticated → viewer-lens engages (consent-gated `relationship_class × intimacy` per the imagodei viewer-lens spec); announced → negotiable depth.
- **Announcement is REA-native:** announcement = Intent; traffic = witnessed Events; the diff = **fulfillment variance (said-vs-did)** emitted as events feeding standing. Variance never *widens* access (composition law); fresh identities start at the floor so identity-reset cannot launder variance history.
- Anti-surveillance invariant: announcement is a *privilege offered to the visitor* that buys depth — never a toll; anonymity always yields the honest commons surface.

## 6. Where inference lives (the Cyc inversion)

No central/general reasoner, ever. The verdict function stays small enough to run on any node (hub-optional floor): plain code over locally-materialized facts in the hot path — today's engineering rail. But this is a *sequencing* choice, not a ceiling: the verdict/explanation interface must admit richer **local** inference (household elohim-counsel negotiating disclosure) as edge compute grows, without re-centralizing. Facts live at the edges that witnessed them (verify-locally-then-serve); explanations are computed where the evidence and keys live.

## 7. Deliverables the reconciliation sprint owes (Definition of Done)

1. Schema 8 confirmed canonical; Rust `reach_earning.rs` enum + tests migrated; `VALID_REACH_LEVELS` removed; schema-contract drift test added (fails on Rust/TS/DNA vs schema disagreement).
2. Geographic 8 renamed to locality/placement (single SDK edit point + 3 re-exports); Part-V custody vocabulary renamed and re-anchored; resilience README gap-matrix row updated to name all five strands and their dispositions.
3. Verdict surface speced (route + Rust shape) as the generalization of `ReachVerdict`, with `explain` opt-in (metered — tracing has cost) and freshness parameter; announcement slot present even if v1 only distinguishes anonymous/session.
4. **Fixture harness:** offline, deterministic "given these declarations/tuples/commitments, agent A sees exactly {…}" invariant tests for the verdict function (SpiceDB `zed validate` pattern). Ships with the reconciliation, not after.
5. Downstream audit: consumers hardcoding reach strings (`content_store_integrity` validator, doorway `reach_aware_serving`/`access_control`, steward `storage/reach.rs`, `p2p/reach_authorization.rs`) reconciled to the canonical enum or the renamed axis vocabularies. Data-aware migration (SpiceDB lesson): no vocabulary value removed while live rows still carry it — migrate rows first.
6. a2o scenario(s) capturing the composition law (narrow-never-widen; anonymous→commons; revocation-orders-before-serve) as regression stories.

**Out of scope for the reconciliation sprint** (sequenced behind it, do not bundle): T4-4 reach-governed serving enforcement; full negotiation/UMA-style claims flow; variance→standing feedback loop; locality-driven placement engine. The reconciliation makes their interfaces possible; it does not build them.

## 8. Open questions (carry into planning, not blockers)

- Federation of independently-evolved vocabularies is unsolved field-wide (ValueFlows Knowledge-level-extension claim was refuted in verification); AT Protocol lexicon namespacing is the strongest candidate pattern if/when external parties extend our vocabularies.
- ~~Middle-band tuple materialization… `vf:AgentRelationship` is the ontology-native tuple~~ — **ANSWERED 2026-07-23, and the citation was wrong.** `AgentRelationship`, `AgentRelationshipRole`, `Fulfillment` and `Satisfaction` are **not in `all_vf.TTL`** (verified by three independent fetches of the real artifact at `codeberg.org/valueflows/pages` — the `valueflo.ws/specification/all_vf.TTL` URL 404s). The bridge codes against hREA/VF-GraphQL, not the ontology; the REA letter must be corrected to cite accordingly. The *proposal* survives, relocated and stronger: `human_relationships` (`db/models.rs:252`) is a Zanzibar tuple **plus** bilateral dual consent (`consent_given_by_a`/`_by_b` — no Zanzibar analog, since tuples are written unilaterally by the object's owner) **plus** an expiry clock (`expires_at`/`verified_at` — the clock organ, already live). It remains projection-only (`dht_anchor_hash` nullable), so it is not yet authoritative.
- Whether `Pending` verdicts (evidence not yet propagated) render as amber UX uniformly.
- DID-method mapping: whether `agent_cid` + `AgentPeerBinding` formalize as a `did:` method (DID document = the peer-transport manifest) and whether `ContributorPresence` claim-verification graduates to issuing Verifiable Credentials — interop dividend vs added surface; sequenced with, not inside, the reconciliation sprint.

## 9. The adoption ledger — what we take from each lineage

Added 2026-07-23. Evidence: `epr:owl2-graduation-floor-ceiling-ontology-2026-07-23`. **The pattern across every row: take the composition algebra, refuse the root.** In each formalism the rejected framing lives in what *grounds* authority, never in how authority *attenuates*.

| Lineage | ADOPT | REFUSE |
|---|---|---|
| **RDFS** | The whole usable layer: `label`, `comment`, `domain`, `range`, `subClassOf`, `subPropertyOf`, `inverseOf` as a **typed dictionary**. This is the level our own closest lineage actually operates at — `all_vf.TTL` uses `owl:Class`/`ObjectProperty`/`DatatypeProperty`/`inverseOf`/`unionOf` and **no** `Restriction`, `disjointWith`, `TransitiveProperty`, `FunctionalProperty`, or `someValuesFrom`, so "nothing a reasoner could derive a contradiction from." Also borrow `vs:term_status` (stable/testing) — vocabulary maturity as an annotation. | Nothing to refuse; RDFS is inert by construction, which is the point. |
| **OWL 2** | Exactly one doctrine — **one abstract structure is the conformance target, one mandatory interchange form, everything else a projection, round-trip declared asymmetric.** Plus: the profile ethic in Cedar's *generative* form; the annotation-inertness law (anything gating a decision is a first-class term, never an annotation); punning-without-inheritance; declared relation shapes checked at seal. **Correction:** the one-abstract-spec discipline as *we* encountered it is VF's pyLODE/rdflib publishing pipeline, not OWL the formalism — OWL contributes term vocabulary, VF contributes the doctrine. | Entailment-as-query; open-world at the verdict layer; no-UNA; `HasKey`; cardinality-as-constraint; nominals + closure axioms; `owl:imports` axiom-merge; IRI dereference; reasoners; RDF crates. **Demonstrated failure:** closure and monotone growth are directly opposed — the three closure axioms required to defeat the OWA are the same axioms a late `suspendedIn` detonates. |
| **Datalog** | **Stratified negation as the honest floor.** Perfect-model determinism is the DHT-validation property (two peers materialize byte-identically). The head-guard pattern makes narrow-never-widen **syntactic and grep-able**. Breaks on: non-stratifiable negation, recursive aggregation, floats, serialization order, EDB divergence. | General negation; unbounded recursion; anything that makes the model non-unique. |
| **Cedar** | The **ethic**, not the crate: restrict the language so analysis stays decidable, and enforce it with a validator — *unanalyzable is ill-typed, not merely slow*. Deny-overrides as an **engine invariant** (monotone under policy addition) rather than `if`-arm ordering. `determining_policies` as a witness shape. | The crate — it is a fourth decider with a 2-valued response, and the term we most need is the one its type cannot express. |
| **Zanzibar** | All of it — tuples, userset rewrite (union / intersection / **exclusion**), and the **Expand witness tree**. Ontologically neutral, which is what makes it the safest borrow. | Only the implicit binding: `user` resolves to `agent_cid`, never a pubkey. |
| **REA / ValueFlows** | The **three strata** (Knowledge / Plan / Observation) as a modeling axiom — every other formalism is single-stratum. **Duality**, which makes provenance structural (`walk_back`/`walk_forward`) rather than an add-on. And `FulfillmentStatus::ratio()` as the middle band's continuous measure (§2b). Keep `TranslationPoint{semantic_cost, ontological_commitment}` — a genuinely novel self-audit of each mapping's ontological cost. | Any adjudicating role. VF has **no** decision procedure — no entailment, no fixpoint, no authorization algorithm, no conflict resolution; a revoked and an active commitment for the same scope simply coexist. It is the substrate's domain vocabulary and must never be asked to decide. |
| **PROV-O** | ~10 term names as labels, explain-only, `bridges/`-homed, zero verdict impact. | The side-graph pattern — `constitution.md:845` requires provenance to travel *as part of every claim, never as metadata about it*. |
| **ODRL** | Nothing. | Refused outright: it defines the role "ODRL Evaluator" and supplies **no evaluation algorithm**, so conformant implementations disagree on every verdict. It fails the deleting-a-term-changes-no-verdict test by construction. |
| **DID / VC** | Resolution shape; `Controller::Many`; `issuer` / `validFrom` / `validUntil` / `credentialStatus`; JOSE/COSE enveloping (which also dodges RDF canonicalization). | `did:key`-as-identity (a key is custody, not personhood) and holder-key-as-subject-proof — **a credential presented by a recovery-issued key must verify.** |
| **UCAN / ZCAP** | The **attenuation law** — a delegation may only narrow. Our composition law, independently derived. | The keypair root. Every chain terminating at "the key that owns the resource" makes recovery definitionally impossible; root in a Mishpat `Commitment` whose authority is collectively ratified. |
| **SHACL** | The `ValidationResult` *shape* only — and ours is already stronger (`bounds-validation-result-view.schema.json` carries positive per-check witnesses; `sh:conforms true` emits none). | The engine. *"Validation with recursive shapes is not defined in SHACL and is left to SHACL processor implementations"* — under Holochain integrity validation an implementation-defined verdict does not degrade, **it forks the DHT.** |

**Bound now** (Cedar ethic — cheap now, expensive to retrofit): no general negation, exclusion only as stratified `not exception_j` with a head-guard carrying the parent grant · unresolvable reference is a seal-time error, never a softening · at most one bounded transitive closure over a relation proven acyclic · every operator emits a witness · default-deny at runtime · zero RDF crates · deleting any borrowed term must change zero verdicts · borrowed vocabulary lives in `bridges/`, never the canonical view spine · no enum member hand-typed outside its card's projections list.
