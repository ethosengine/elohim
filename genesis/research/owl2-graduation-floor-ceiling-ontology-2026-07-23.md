---
title: "Graduating from OWL 2 — an ontology that knows where its own judgment ends"
status: Capture
date: 2026-07-23
---

# Graduating from OWL 2

*Third in the ontology arc. Companion to the [ontology-systems survey](epr:ontology-systems-survey-reach-reconciliation-2026-07-22) (what REA is missing) and the [letter to REA practitioners](epr:letter-to-rea-practitioners-observed-presence-2026-07-22) (the three organs). Those two asked what we need. This one evaluates the field's most serious attempt to provide it — W3C OWL 2 — and says what we take, what we refuse, and what we must hold before we are entitled to leave it behind.*

**Method:** the primary source read directly (`https://www.w3.org/TR/owl2-overview/` plus the Structural Specification, Profiles, both Semantics documents, Primer, and New Features), a 15-agent parallel corpus over the adjacent and successor lineages (SHACL/ShEx, PROV-O, ODRL, DID/VC, JSON-LD, RDF 1.2, Zanzibar/SpiceDB, Cedar, OPA/Rego, Datalog, AT Protocol Lexicon, Confluent schema registries) and our own substrate, 7 load-bearing claims each refuted by a 3-lens adversarial panel, 4 cross-analyses, a 3-angle proposal panel with judge synthesis, and a completeness critic whose three sharpest hits were then run to ground rather than filed.

**Verification key:** ✅ survived adversarial panel · ◐ sourced but unpaneled · ✗ refuted · ⚠ **corrects a claim made in a companion document.**

---

## 0. The thesis, in the operator's words

> *"This is why in some respects we need an ontology — so we can describe the floor and where the ceiling is, while leaving some things open to interpretation at the edge."*

That sentence is the finding, and it resolves the manifesto's standing objection to this entire line of work.

`manifesto.md:179` says *"Systems should be guided by principles of human flourishing rather than rigid rule sets."* Read as a ban on formalism, it forbids what we are doing. Read correctly, it forbids something narrower — its own gloss, three lines later, is *"trained on patterns of flourishing… not just policy violations… restoration and growth over punishment"* (`:183-187`). The manifesto is refusing **codification of the value domain**, not structure as such. And the corpus already contains a decision procedure: `constitution.md:662-673` returns `Refusal(layer, reasoning) | FlagForHuman(layer, ambiguity) | Permitted(reasoning)`.

So the ontology's job is **demarcation, not decision**. It says where the mechanical floor is, where the ceiling of delegated judgment sits, and — critically — it marks the region in between as *not the ontology's to decide*.

**And that is the capability no formalism evaluated here has.** ✅

OWL, SHACL, Cedar, Rego, Datalog, and Zanzibar are all **total within their scope**. Point them at a question and they answer it. Their silence means *unknown* (OWL, under the open-world assumption) or *deny* (Cedar, Zanzibar, default-deny engines) — never *"a person must decide this."* None has a first-class term for the boundary of its own competence. An ontology built for this protocol needs that term more than it needs subsumption, and it will not be found off the shelf.

We have already authored it three times, independently, without noticing:

| Home | Term | Shape |
|---|---|---|
| `.epr-meta` rule algebra | `class: ask` | authoring-time escalation |
| `constitution.md:662-673` | `FlagForHuman(layer, ambiguity)` | governance-time escalation |
| `reach_earning.rs:56-70` | `ReachVerdict::Pending` | serving-time escalation |

Three vocabularies for one concept — the exact drift shape the reach reconciliation exists to cure, landing on the single most important concept in the system. Naming this once is the highest-value ontological act available to us, and it costs a table, not an engine.

---

## 1. What OWL 2 actually is (primary source)

Read directly, OWL 2 is more disciplined and less mystical than its reputation. Two structural facts matter to us:

◐ **The authoritative artifact is an abstract UML structure, not a file.** *"The conceptual structure of OWL 2 ontologies is defined in the OWL 2 Structural Specification document"* — in UML, *"in abstract terms and without reference to any particular syntax."* Every concrete syntax is a projection. RDF/XML is *"the only syntax that must be supported by all OWL 2 tools"*; Functional, Manchester, OWL/XML and Turtle are optional. The round trip is declared **asymmetric**: structure→RDF is total, *"most OWL 2 ontologies represented as RDF graphs can be converted into the conceptual structure."*

◐ **The Overview never states the assumptions OWL is famous for.** No OWA, no UNA, no monotonicity sentence appears in it. Those come from the Primer (*"it may simply be missing (but possibly true), following the open-world assumption"*) and the Direct Semantics. The Overview's own commitments are performance contracts: EL *"enables polynomial time algorithms"*; QL *"enables conjunctive queries to be answered in LogSpace… using standard relational database technology"*; RL *"enables the implementation of polynomial time reasoning algorithms using rule-extended database technologies operating directly on RDF triples."*

---

## 2. The graduation ledger

**TAKE** adopt the mechanism · **TAKE-DISC** adopt the idea, not the machinery · **REPLACE** we need this, from another lineage · **LEAVE** actively refuse.

| OWL 2 capability | Disposition | Native home / replacement |
|---|---|---|
| **One-abstract-spec-many-syntaxes** | **TAKE** | The single most valuable thing in OWL 2 — and we already own the organ (§4) |
| Profile discipline (declared capability tiers) | TAKE-DISC | But in Cedar's *generative* form: reject the unanalyzable at seal, not at runtime |
| Annotation-inertness law | TAKE-DISC | Anything that gates a decision is a first-class term, never an annotation |
| Punning (shared name, no inheritance) | TAKE-DISC | `GovernancePolicyBinding` is already exactly this |
| Declared relation shapes (asymmetric, acyclic) | TAKE-DISC | Check at seal time (brit `has_cycle`), never by entailment |
| Datatype facets on `dateTime` | TAKE-DISC ⚠ | Validity intervals *are* expressible in OWL — corrects our companion's implication otherwise |
| Monotonicity | TAKE-DISC (facts) / **LEAVE** (verdicts) | Monotone-append attestations; non-monotone evaluator picks the declared head |
| Two-semantics split + correspondence theorem | TAKE-DISC | We have the shape (Python `combine()` vs Rust `evaluate_path_with`) and **no theorem** — name the obligation |
| EquivalentClasses, Union/Intersection, Complement | REPLACE | Stratified-negation Datalog / Zanzibar userset rewrites |
| Property chains | REPLACE | Zanzibar `tuple_to_userset` — exactly one bounded closure, acyclicity checked at seal |
| NegativeObjectPropertyAssertion | REPLACE | OWL's yields *inconsistency*, not a deny verdict. Zanzibar `exclusion` / Cedar `forbid` |
| Qualified cardinality | REPLACE | Under OWA it **merges identities** rather than rejecting — the classic trap |
| HasKey | REPLACE | Content-addressed identity: generative, not post-hoc |
| `owl:imports` / `versionIRI` | REPLACE | Content-addressed cascade + mechanical compat gate |
| Domain/Range | REPLACE | They *infer types*, they do not constrain. JSON Schema + `schema_contract.rs` |
| Functional / InverseFunctional, SameIndividual | **LEAVE** | Silent `sameAs` merge is identity forgery; CID supplies our UNA |
| Nominals + closure axioms | **LEAVE** | Closure must be re-materialized on every membership change |
| No-UNA | **LEAVE** | Re-opens identity forgery |
| OWA *at the verdict layer* | **LEAVE** | Default-deny is the requirement |
| OWA *at the fact plane* | TAKE-DISC | A peer's non-observation is not a negative fact — `BytePresence`/`VerificationStatus` already model unknown ≠ absent |
| Reasoner / entailment-as-query | **LEAVE** | See §3 |
| IRI dereference | **LEAVE** | But keep the identity discipline: identity is the full IRI, prefixes are sugar — same as slug-vs-CID |

### ⚠ The correction our own companions owe

The letter (`:23`, `:39`) asserts OWL/REA cannot express exclusion. **The panel refuted this, and the refutation stands.** OWL 2 DL *can* derive `members ∪ directors − suspended`: `ObjectComplementOf` is literally `ΔI \ CE` in the Direct Semantics, and `ObjectOneOf` + `DifferentIndividuals` supply the closure the OWA withholds. Our stated reason for rejecting OWL was wrong, and the letter should be corrected before it is sent.

The *right* reasons are three, and they are stronger:

1. ✅ **Ex falso is a fail-open authorization catastrophe.** A late `suspendedIn` assertion against a closed enumeration yields inconsistency; from inconsistency, everything is entailed — including universal permit. The adversarial move against a naive OWL policy decision point is *poison the graph and receive access to everything*. That is the exact inversion of fail-safe.
2. ✅ **No profile can even state it.** EL has no complement; QL/RL admit it in superclass position only; RL's `cls-com` head is `false`. Nominals-plus-complement puts us in SROIQ's N2ExpTime corner — off every PTIME path, on household hardware.
3. ✅ **There is no subsumption to infer.** `reach.schema.json` is eight flat strings with `_ordinal`. All axes together are ~40 terms. The entire relation is `openness() -> u8`. Our real problem is one-generative-source-of-record-enforced-by-codegen — the schema-registry problem — and OWL contributes nothing to it.

✅ **Second concession, and it is the honest one:** SNOMED CT, the paradigm OWL success, clears our "when does OWL earn its cost" predicate only because its extensions, `effectiveTime`, inactivation refsets and per-realm subsets all live **outside** the OWL core in tabular refsets. A monotone kernel with a non-monotone evaluator over it. We are not rejecting the field's answer — we are re-deriving its layering with a stronger identity primitive and no triple store.

---

## 3. Why the decision shape, not the expressivity, is disqualifying

Three constraints from canon, verified against source, each of which independently rules out the RDF/OWL *deployment* shape while leaving its vocabulary borrowable:

✅ **No whole-person query.** `values-forward.md:207` — *"No council and no model can query the record to render a final account of who you are — the mechanism authorizes only bounded, purpose-limited reconstruction under witness."* Held as reserved to God alone, *"which is precisely why we set the bar where no institution here can reach it."* This forbids a global ABox and a universal join. Every triple-store adoption trends toward exactly the capability this refuses.

✅ **Provenance is intrinsic, not adjacent.** `constitution.md:845-846` — *"provenance travels as part of every claim, never as metadata about it."* OWL's annotation properties are semantically inert by design; the conventional PROV-O pattern is a side graph. Both are the forbidden shape.

✅ **Description without repair is a museum.** `constitution.md:847-849` — *"a graph that describes a harm but cannot be walked back to make material amends is a museum, not a substrate; reach can be returned, attestations revoked, stewardship redistributed, couplings honestly closed."* A monotonic entailment regime *is* the museum.

And one that cuts **for** structure: `constitution.md:837` — *"The only durable defense is pointable structure that breaks visibly when the word drifts from the thing."* That is a mandate for failing constraints — SHACL's shape, not OWL's.

◐ **SHACL is nonetheless refused, on a ground the survey underweighted:** *"validation with recursive shapes is not defined in SHACL and is left to SHACL processor implementations."* Under Holochain integrity validation an implementation-defined verdict does not degrade gracefully — **it forks the DHT.**

---

## 4. What we already have (and three things we were wrong about)

The strongest finding of the substrate map is that the doctrine we credit OWL 2 with is **already shipped in Rust**, pointed at packages instead of vocabularies:

```rust
// elohim/eprfs/epr-cli/src/authority.rs:16-44
pub enum AuthorityKind { Package, RuntimeSource, Unmanaged }

pub struct PackageAuthority {
    pub kind: AuthorityKind,
    pub package_id: String,
    pub master: Option<String>,           // which artifact is authoritative
    pub source_path: Option<String>,
    pub projection_paths: Vec<String>,    // the allowlist of generated surfaces
    pub governance_ref: Option<String>,
    pub gates: Vec<String>,
}
pub fn is_package_master_projection(&self, path: &str) -> bool { … }
```

One master, an allowlist of authorized projections, a governance ref, and gates — *"is this path a legitimate projection of that master?"* as a shipped predicate. **The axis registry is not a new organ. It is this organ, aimed at one more subject.** ⚠ (This corrects the workflow's own recommendation, which proposed a parallel JSON registry without noticing the Rust type.)

The rest of the inventory, honestly stated:

| Obligation | Status |
|---|---|
| Name terms | **ALREADY** — 35 closed value-sets with generative source-of-record pointers; strictly more than `owl:versionIRI`, which names no generator |
| Axioms as validation | **ALREADY** — `schema_contract.rs`, `required_coupling()`, DNA `ValidateCallbackResult`. SHACL's job, done without RDF |
| Explanation | **PARTIAL, better than expected** — `bounds-validation-result-view.schema.json` is SHACL-`ValidationResult`-shaped *and stronger*, carrying positive per-check witnesses that `sh:conforms true` structurally cannot emit. `StageTrace` ships named intermediates. `epr-cli/src/explain.rs` is a live explain surface |
| Observer organ | **PARTIAL, our strongest** — `epr-rea/src/model.rs` has `Intent`/`Commitment`/`FlowEvent` with **CID-hashed Fulfillment/Satisfaction edges, tamper-evident by construction** — ahead of most hREA implementations. Missing: a `Variance` term |
| Classes / relations | **PARTIAL** — flat labels, no subsumption; `EprGraph` carries one undifferentiated edge kind |
| Provenance | **PARTIAL** — `CompositionGraph`/`DerivationEdge` give `source→derived, relation, attributedTo`; but `attributed_to` is an unresolved `String`, and there is **no timestamp anywhere** |
| Versioning | **⚠ OVERCLAIMED** — `policy@version` is a mutable YAML row with an author-asserted integer and immutability enforced by a *comment*. Only 7 of 36 rules are pinned at all. This is literally the `versionIRI` failure we claim to have solved |
| Derivation | **NOT** — `.epr-meta` performs zero derivation; no rule's output is any rule's input |

### Three claims we made that the evidence refuted

⚠ **`.epr-meta` is not a closed-world rule language.** It is **default-allow and fail-open** (`epr-meta-resolver.py:315` — `sys.exit(0)  # silent allow`). Its modality is 3 blocking classes, not 5 — `class: dispatch` has zero instances repo-wide. And 17 of 36 rules delegate their real semantics to opaque host Python, of which unregistered refs silently degrade to `inject` — a soundness inversion no comparator has.

⚠ **`BlobCid::verifies` is codec-broken with zero production callers.** It hardcodes `Self::compute` (dag-cbor `0x71`) while document bodies use `compute_raw` (`0x55`), so it returns `false` for every raw-codec CID against its own bytes. "Self-verifying" is a latent bug, not a property.

⚠ **The per-viewer premise is aspiration, not load — now measured, not deferred.** `reach_earning.rs:81` is `evaluate(local_agent, author, requested_reach, conn, registry)`. There is no viewer parameter; `local_agent` is the *evaluating peer*; the sole caller is `epr_compose.rs` — **author-side compose, not per-viewer serve**. No timestamp exists on `ReachVerdict` or `StandingEvidence`. Every per-viewer/freshness/revocation requirement in the arc traces to §2–§5 of the guiding-principles spec.

---

## 5. Retention is the temporal half of the same gradient

Reach and retention are one verdict on two axes:

- **Reach** — disclosure across the *social* axis: who may see this now.
- **Retention** — disclosure across the *temporal* axis: what may still be seen later, by anyone, including us.

Same structure both times: a **declared floor** (schema-8 reach / the memory class, both closed, declared at creation), a **derived judgment** above it, **narrowing-only** composition, an **explanation** owed. Memory class is to retention exactly what declared reach is to visibility. The verdict surface should therefore be *axis-generic* with reach as its first instance — cheap now, expensive to retrofit.

The Observer Protocol makes this concrete and inverts the usual fact/derivation relation: `observer-protocol.md:22-30` destroys the visual data after processing, so *"only structured story remains."* The derived artifact is the sole survivor. **Retention is the judgment** — keeping is the act that must be justified, not forgetting. The deterministic floor is already written: the class × primitive legality matrix (`architecture/2026-05-10-memory-lifecycle-design.md:216-220`) makes `forget` **forbidden** for attestation and identity-core, `merge` **structurally unavailable** for attestation, with ambiguity resolving to the *more-protective* class. The ceiling has its discipline too: *"Lifecycle decisions are principled, not arbitrary. Each operation has earning criteria."*

Four narrow gaps stand between that and expressibility, all in `eprfs-core`:

1. `BytePresence` (`awareness.rs:52-60`) cannot distinguish **released by judgment** from `Missing`/lost. That is the moral difference between witness and surveillance, and it currently has no term.
2. `DerivationKind` (`composition.rs:33-45`) has no compaction/distillation kind — *"this is what I judged worth keeping from that."*
3. `DerivationEdge` carries `attributed_to` (who judged) but no **rationale**. `GovernanceRule` already has `why:` — the pattern exists one file away.
4. Nothing carries a timestamp.

Grace is implementable: `values-forward.md:207` promises consequences fall on participation, *"never erasure — your record persists; what is refused is any totalizing verdict over it."* So a graceful forget leaves a residue — *something was here, it was let go, by whom, and why* — while the content is gone. That is a surviving `CompositionNode` (which holds a CID and identity, **not bytes**) plus an attributed edge, a typed rationale, and a `BytePresence::Released`.

---

## 6. The decision

**Do not adopt an ontology language.** Take one doctrine, aim an existing organ at one new subject, and mark the ceiling.

**Do now — no formalism, highest value.** Extend `check-reach-drift.mjs` from seed-JSON to `.ts`/`.rs` source scanning, failing on any canonical-set literal outside an authorized projection list, wired into `.husky/pre-push`. *This is the highest-value item in the entire corpus and it is a regex.* Then the renames: `steward/node/src/storage/reach.rs` retired (dormant, zero callers); brit's build-status `ReachLevel` renamed off the name collision; geographic-8 → `LocalityLevel` generated from one schema; `reach_earning.rs` literals replaced by generated constants. **Two non-negotiable ordering laws:** the gate precedes the rename (without it the pass re-drifts within a sprint — exactly how five strands diverged), and the rename is atomic (the ts-rs cross-crate trap makes partial strictly worse than none).

**Fix what we already claim.** `BlobCid::verifies` codec dispatch; `contentHash` on policy rows so `id@version` is a pin rather than a name. Every one of these is a defect *under our current design*, found only because we went looking for an ontology. That is the honest yield of the exercise.

**Name the ceiling.** One table reconciling `ask` / `FlagForHuman` / `Pending` into a single term, and the law that it is a first-class verdict value — never a fallthrough, never a timeout, never the absence of a rule.

**Bind these before pressure arrives** (Cedar ethic — cheap now, expensive to retrofit): no general negation, exclusion only as stratified `not exception_j` with a head-guard carrying the parent grant, so narrow-never-widen holds *syntactically*; an unresolvable reference is a seal-time error, never a softening; at most one bounded transitive closure over a relation proven acyclic; every operator emits a witness; **default-deny at runtime** (explicitly reversing `.epr-meta`'s fail-open — authoring may fail open, access may not); zero RDF crates, ever; deleting any borrowed term must change zero verdicts; borrowed vocabulary lives in `bridges/`, never in the canonical view spine.

**Take from each lineage the composition law; refuse the root.** DID's resolution shape and `Controller::Many`, not `did:key`-as-identity. VC's `issuer`/`validFrom`/`credentialStatus`, not holder-key-as-subject-proof — a credential presented by a recovery-issued key must verify. UCAN's attenuation law (a delegation may only narrow — our composition law, independently derived), not its keypair root, which makes recovery definitionally impossible and is the rejected apex formalized. Zanzibar entire, including the Expand witness tree; it is ontologically neutral, which is what makes it the safest borrow. **PROV-O's ten term names as labels. ODRL refused** — it defines the role "ODRL Evaluator" and supplies *no evaluation algorithm*, so conformant implementations disagree on every verdict; it fails the deleting-a-term-changes-no-verdict test by construction.

**Refused outright:** OWL, SHACL, triple stores, reasoners, `DerivationPolicy`, a rule engine, an exclusion operator, speculative `as_of`/`viewer` fields, JSON-LD `@context` on canonical views, and any unification of the three deciders — three independently-correct deciders beat one wrongly-shared abstraction built on a premise §4 just measured as absent.

**Deferred against named observables:** CLOCK/`Watermark` picks up if a real story needs a read whose verdict must reflect a revocation, or a delegated-compute grant survives revocation in the wild via `find_active_delegates_compute` (which filters `state='active' AND revoked_at IS NULL` over a local projection **with no freshness bound** — a live new-enemy window). A rule layer picks up only if the `validator: epr:*` ratio drops below 17/36 *and* ≥2 production verdicts come from a declared rule. **If in six months `reach_earning.rs` is still 100% hand-written `if`/`match` and the rule engine exists only in fixtures, the centralized-reasoner re-reading of Cyc was cope** — and we should say so.

---

## 7. What this analysis is most likely to have wrong

Held open honestly, because the critic earned it:

- **Nobody encoded the actual rule.** Zero lines of OWL, Datalog, Cedar, or SpiceDB schema appear in ~8,000 words of analysis. The ex-falso disqualifier — the single most decisive argument — was reasoned, not demonstrated against a live reasoner. Twenty minutes with HermiT or ELK would convert it to a demonstration or a retraction. **Do this before citing §2 externally.**
- **The conclusion is suspiciously convenient.** "Don't adopt a formalism; ship a card and a gate" is approximately the pre-existing sprint plan with a citation stapled on. The tell is that the expressivity argument was *conceded* and the verdict re-grounded on three reasons found after the concession — the same post-hoc move we diagnose in the Cyc re-reading.
- **The two-codegen-authorities "indictment" was never demonstrated with a single instance of actual divergence.** If they have never diverged, the corpus's top finding is aesthetic.
- ⚠ **ValueFlows got to the one-abstract-spec doctrine first, in our own adjacent lineage, and we never opened the artifact.** The survey already records that VF designates a single Turtle file as system-of-record with JSON-LD/HTML/OWL derived. Crediting OWL 2 with the doctrine while leaving `all_vf.TTL` unread is a citation failure we should fix before the REA letter goes out.
- **The letter itself is owed a correction and got no artifact here.** Its exclusion claim is refuted (§2). Its `vf:AgentRelationship`-as-tuple-store proposal remains the strongest live idea in the arc and is untested against `all_vf.TTL`.

---

## 8. One finding outside the ontology question

A live contradiction in gospel-tier canon, surfaced by the vision sweep and verified directly. `values-forward.md:103` — Stance I.2, *"Not a blockchain, and not a token. The substrate is agent-authored source chains, DHT-notarized."* Against `constitution.md`, fourteen references the other way: *"blockchain-anchored"* (`:59`), *"The blockchain constitution co-locates our treasure with our values"* (`:75`), `# Hash: [blockchain-anchored]` in five schema blocks, *"Verify each layer's hash against blockchain anchor"* (`:651`).

`governance-layers-architecture.md:140` has **already been corrected** — *"not amendable policy, and not a blockchain smart contract."* So the reconciliation happened in two documents and skipped the constitution.

Same shape as the five reach vocabularies: one concept, corrected in some homes, stale in the one carrying the most authority. The constitution is the stale strand.
