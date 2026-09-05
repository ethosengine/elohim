---
title: Concurrent agent attribution through the existing EPR and REA fabric
id: concurrent-agent-attribution-design
status: Draft
class: process-meta
context-tier: disclosed
steward: agent:architect@gpt-6
graduation-trigger: All six plan stations have executable evidence, including real Claude, Codex, and Gemini runs, an independently readable correction against a prior claim, and contextual valueflow discovery from a cold start.
date: 2026-09-05
serves: dev-system-equilibrium
cites:
  - "imagodei-surfaces | The stated and revealed self-knowledge model reused for shared agent understanding and contextual model-task fit. | sha256:e0abac6f6a6a0906 | path: genesis/docs/content/elohim-protocol/architecture/imagodei-surfaces-design.md"
  - "requisite-variety-guidestar-epr-family-composition | The composition law and primitive admission rule that exclude a parallel agent ontology from this design. | sha256:e1cf9e52fbe95c11 | path: genesis/docs/superpowers/specs/2026-08-12-requisite-variety-guidestar-epr-family-composition.md"
  - "epr-rea-valueflow-fabric | The existing agents, promises, resources and event edges through which concurrent attribution must flow. | sha256:1cec32527dbff6d7 | path: genesis/docs/superpowers/specs/2026-07-18-epr-rea-valueflow-fabric-design.md"
  - "actor-plane-inflight-identity-claims-design | The current session-scoped actor claim and explicit concurrency limitation this design extends. | sha256:6a6dee8249ae76ef | path: genesis/docs/superpowers/specs/2026-08-15-actor-plane-inflight-identity-claims-design.md"
  - "valueflow-authoring-surface-design | The existing claim, fulfill, note and context verbs that gain attribution automatically. | sha256:3036ad9306270f5a | path: genesis/docs/superpowers/specs/2026-09-05-valueflow-authoring-surface-design.md"
---

# Concurrent agent attribution

## 1. Purpose and decision

An agent takes a commitment, uses resources under governance, produces evidence, and can be
questioned by another agent or a human. This is REA expressed through EPR. Claude, Codex, and
Gemini are execution surfaces for those relationships. eprfs makes their governed resources
and observations available as ordinary local files.

The next increment repairs information lost at these crossings. It adds no Agent registry,
execution ontology, effort enum, scheduling service, authority verdict, or transport. It serves
the existing dev-system-equilibrium habit by making the agents taking and discharging promises
distinguishable. Attribution alone does not prove equilibrium; that habit stays red until its
own rate check greens. No new active habit is needed.

The first station preserves the exact existing ActorClaim CID in decisions and flow records.
Subsequent stations make concurrent persistence safe, carry worker scopes through real harnesses,
and bind capability use and measurements to the same fabric. FUSE, ark volumes, and storage
replication remain separate consumers of these primitives and are outside this plan.

## 2. Primitive reuse, before adapter design

### In-flight experience: friction follows meaning

The operator's steering is part of the acceptance contract: infrastructure should disappear
from ordinary agent work. Agents progressively discover the context, research, contractual
wisdom, Mishpat obligations, tools, telos and values relevant to the commitment in front of them.
The protocol's trust and social meaning must survive that discovery; this is not just a faster
tool launcher or an attribution logger.

Mechanical setup belongs to the harness projection: bind worker scope, register the applicable
actor claim, preserve its reference, and capture available evidence without repeated agent
bookkeeping. Explicit CLI flags remain diagnostic/compatibility surfaces, not the normal ritual.
Do not require agents to hand-compose session keys, copy claim CIDs, edit generated configuration,
or repeat a claim before each action. The first station adds provenance inside existing verbs.

The design target is the reliability of an HTTP interceptor: ordinary tool use carries the
right identity, scope and evidence without its caller remembering the plumbing. Explicit hooks
and commands are the implementation/debugging surfaces. Prove their composition, failure
behavior and observability so that routine callers can rely on them without attending to them.

Intentional friction belongs at meaningful boundaries: discovering a changed dependency,
reading the research a decision relies on, negotiating a promise, obtaining needed consent,
substantiating a claim, reviewing fulfillment, or routing a contested question. Reuse
`epr flow context`, pinned package/citation relationships, `.epr-meta` and the native evaluator
to surface the next relevant context and tools. Disclosure follows the existing scope/reach
contract; helpful discovery must not become indiscriminate context loading or disclosure.

The end-to-end test records avoidable infrastructure interventions. In a supported, configured
harness, switching workers or resuming a run must require zero manual attribution repairs;
an unsupported capability is surfaced once with an actionable explanation. The system must
still present the actual obligation and decision to the responsible agent/human. Automation
cannot silently satisfy contractual wisdom, supply consent, or turn refer into permit.

### Quiet operation, verifiable depth when care is needed

The insurance-policy analogy names a second acceptance requirement. Normal work should not
repeatedly demand attention to its safeguards. When harm, error, doubt or disagreement arises,
an authorized reader must be able to deepen the inquiry: outcome/artifact, event, promise,
actor claim, package and policy pins, observations, and any actually witnessed delegation and
identity lineage. This is the existing EPR/REA graph and its governance, not a parallel audit
database. The purpose is understanding, care, repair, responsibility and accountability.

Each answer distinguishes self-report, local content integrity, signed evidence and peer
witnessing. A missing source or unavailable witness is named, never filled with inferred trust.
Possessing a CID does not prove byte availability, permission to disclose, or truth of a claim.
Local persistence is not peer custody. Evidence needed for remote trust must follow the existing
reach, consent, custody and notary paths; this plan does not silently make the sidecar replicated.

Do not collect every interaction as insurance. Recipe-governed meaningful observations and
content-addressed dependencies preserve enough evidence for the stated claim, with retention
and disclosure governed by the relevant scope. A failure review must also reveal where the
available evidence stops. A deterministic judgment can refer to a responsible human/elohim;
accountability must not collapse into automatic blame or reputational penalties.

| Meaning | Existing home | Binding needed here |
|---|---|---|
| Agent | `epr-rea::AgentRef`; existing protocol Agent identities | Keep `provider`, `receiver`, and `raised_by` as AgentRef. An actor claim describes who is acting; it is not a second Agent. |
| Assertion and evidence | `epr::Envelope` (claims, coupling, reach, proof, supersedes), canonical codec | Preserve references to the actual assertions. Local CID integrity is not a signature or a network witness. |
| In-flight attribution | `ActorClaim`, `ActorRecord`, `ActorStore::current_for` | Reuse the existing opaque `session` lookup scope and returned claim CID. |
| Work promised | `Intent`, `Commitment`, `satisfies`, `in_scope_of` | The work item remains a commitment satisfying an intent in an accountable container. |
| Work observed | `FlowEvent`, `fulfills`, `resource`, `process` | Evidence remains an event against the artifact and promise, with its actor-claim reference retained. |
| Conversion | `Process`, pinned `ProcessSpec`, inputs and outputs | Reuse where a run actually groups resource conversion. Do not invent a Process solely as a second session identifier. |
| Resource | Any content-addressed atom/blob; resource state is an event fold | Packages, briefs, reports, and evidence already have addresses. No Resource registry. |
| Capability | elohim-agent manifest `USES`, `GOVERNS`, `BINDS`, `PROJECTS`, `MEASURES`; package `capabilityRefs` | Resolve the pinned package and the applicable governance; record actual use through existing compute/use vocabulary. |
| Model configuration | Package `modelHints` and runtime projection bindings | Preserve provider-native requested settings as configuration, including effort; do not equate providers' labels. |
| Observed effort | `Magnitude`, EPR measurement/quantity/confidence types; use/consume events | Record measured tokens/time/compute with units and evidence. Missing measurements remain absent, never zero. Requested effort is not measured use. |
| Delegated authority | Existing Mishpat `delegates-compute` Commitment, bounds and validity | Reuse at witnessed graduation. The local REA Commitment and the Mishpat action-string payload are distinct representations; the CLI does not already enforce their mapping. |
| Decision and contest | Native evaluator, EPR Verdict/Witness and permit/refuse/refer; flow correction/ruling/verdict notes | Attribution annotates the decision. A claim never authorizes itself or replaces the evaluator; a correction remains recordable when identity lookup fails. |
| DID and lineage | Existing DID bridge and protocol identity/controller/lineage records | Resolve an established agent identity. Never mint a DID from a model label, persona, or session. |

Source inspection: `elohim/epr/src/{envelope,witness,verdict,measure}.rs`,
`elohim/epr-rea/src/{model,actor,store,fold,scope}.rs`,
`elohim/sdk/domains/elohim-agent/manifest.json`,
`elohim/sdk/schemas/v1/commitments/delegates-compute.schema.json`, and
`bridges/did/did-bridge/src/did_elohim.rs`.

### Shared understanding: human self-knowledge and situated model–task fit

Imagodei's self-knowledge design composes stated preferences, values, personality and aspirations
with revealed patterns in participation. The operator extends this same understanding subsystem
to agent infrastructure: understanding which model is suited to which commitment is a consumer
of it, not a separate profiler or ranking ontology. Imagodei's existing integrity `Agent` already
includes `human`, `organization`, `ai-agent` and `elohim`; `AgentProgress` is agent-addressed.
Shared primitives do not imply identical instruments or interpretations for people and models.
Human dignity, consent, changing capacities and self-understanding are not reducible to task yield.

Psephos here means psychometric self-knowledge. The current ballot renderer named `psephos` is
known naming drift, explicitly ruled on in
`genesis/data/timeline/backlog/2026-09-05-psephos-naming-drift-backlog.md`; its rename is separate
work, not evidence for separating psychometrics from agent understanding.

The implemented reuse floor is precise:

- `sophia/packages/sophia-core/src/types.ts` supplies `Recognition`, distinguishing demonstrated
  mastery, resonance, reflection and governance evidence; `scoring-registry.ts` supplies strategy
  registration. `psyche-survey/src/aggregate.ts` aggregates resonance. These are inputs to
  interpretation, not claims of verified identity or calibrated capability.
- Lamad's `InstrumentDefinition` and `instrument-registry.ts` already carry content-delivered
  instrument definitions and interpretation. The Sophia loader exposes `PsycheAPI` and
  `PsychometricInterpretation`. Reuse and make this seam portable for its actual consumers;
  do not introduce a second instrument registry or deepen a core-to-Lamad dependency.
- `ElohimCapabilityProfile` in `elohim/elohim-views/src/infrastructure.rs` already describes
  model/configuration, specialties, skills and strengths. It is currently operator-configured:
  a field named observed strengths is not proof that observations computed its value.
  Existing agent-subject `AttestationView` supplies issuer, evidence, proof class, supersession
  and revocation vocabulary for graduation, not a reason to publish private understanding.

Current limits are implementation work, not presumed infrastructure: no `psyche-core` package
exists in this checkout despite documentation references. The app-local interpreter supports
highest-subscale and thresholds; dimensional interpretation currently collapses to a primary
subscale, and profile-matching/custom fall through without interpretation. The loader assigns
confidence 1 when a primary type exists; that is not calibrated certainty. Discovery persistence
is currently humanId-bound, localStorage-backed and replaces results per assessment. It is not
already shared append-only native agent-understanding persistence or a working model selector.

The required composition is: historical actor claim → attributed REA observations and declared
inputs → addressed instrument/interpretation evidence → contextual fit against an existing
intent/commitment. Preserve these as distinguishable layers. An inferred or emergent preference
is a revisable hypothesis, not the subject's stated preference, a hidden truth, or consent.
Disagreement between stated and revealed patterns invites contextual inquiry and correction.
Self-assessment confidence, interpretation uncertainty and confidence in a witnessed outcome
are different meanings; never silently substitute one for another.

Fit must explain the task, relevant capacities and values, model/version/configuration and tool
context, evidence freshness and limitations. Unknown fit stays unknown; a declared capability
can support a provisional suggestion but must be labeled declared. Suitability neither grants
authority nor claims work. Private psychometric evidence stays in its governed private scope:
public Agent metadata visibility filters do not restrict DHT gossip. Existing reputation folds
explicitly exclude dev-context decisions and produce no scalar score; local development fit
must not silently confer public reputation, standing or reach.

Station 5 preserves observations usable by this shared subsystem. Station 6 must first prove
portable interpretation and honest uncertainty with a real fit consumer before claiming rich
model–task matching. Neither station needs a new Agent, profile store or public DHT head.
Any change to persistence/disclosure or native attestation publication must pass the P2P gate
at its owning seam before implementation; the current plan does not authorize public telemetry.

## 3. Measured gaps and scope

`ActorClaim.session` already scopes a run and `current_for` matches it exactly. However, the
actor-plane v1 harness discipline shares the top-level session across personas and therefore
serializes them. Independent worker scopes require a transport binding, not another identity.

`govern::claimed_for` discards the CID returned by `current_for`; the flow resolver discards
everything except the claimed label. Consequently two workers with the same role/model cannot
be distinguished in their flow evidence, and later claim changes cannot be audited from that
label alone. The loss affects note, claim, and task-report fulfill through their shared resolver.

`SidecarActorStore::open` checks existence before `File::create`; simultaneous first opens can
truncate a new log. Append/read have no explicit cross-process record coordination. The same
persistence family needs review in FlowStore. These are source findings, not reproduced failures
at spec birth. Station 2 must reproduce and close them before concurrent safety is claimed.

Current flow CLI precedence is explicit session, Claude environment, Elohim environment; actor
requires explicit session and govern accepts explicit session only. The Claude hook passes the
top-level payload session. These surfaces do not yet carry one consistent worker binding.
`epr doctor` reports missing stock Codex agent and hook projections. Antigravity skill projection
does not prove Gemini CLI integration. Station 4 requires real installed-runtime evidence.

## 4. P2P design gate: existing records, one retained relationship

### Actor claim and flow attribution

- Classification: existing local authored observation floor, intended Attested-Private (B2)
  at graduation. The sidecar is currently the durable local source, not a Holochain private
  source-chain projection. This implementation gap is explicit; nothing here stamps it witnessed.
- Identity: existing canonical dag-cbor atom CID. The reference identifies the ActorClaim used
  at authoring, not today's latest claim. AgentRef and protocol agent keys remain distinct from
  this claim address. The legacy package `definition_cid` fingerprint is not migrated here.
- Address scope: an existing opaque session string names an external run before a claim exists;
  its operational purpose justifies the non-CID lookup key. It is not the Agent identity.
- Cost: no additional DHT entries or heads in this plan. Local claim count is per worker/identity
  change, not per tool call. An illustrative 10 workers/day over 250 days gives 2,500 claims/year
  plus switches; event counts depend on recipe-defined meaningful edges. At graduation, bundle
  outcome evidence under existing container/incident roots rather than adding a head per call.
- Network stakes: local attribution is usable offline at every declared stage. Constitutional
  delegation, local relationships, and counter-evidence retain the floor; lower stakes cannot
  turn a self-report into authority. No reach vocabulary is newly canonized.
- Integrity/coordinator: no integrity or coordinator changes; DNA-hash-NEUTRAL. Existing
  `content_store_integrity::EntryTypes` includes Agent, Commitment and EconomicEvent; the existing
  `content_store::create_rea_economic_event` and post-commit projection are graduation candidates,
  not invoked by this slice. No invented create-actor zome or HTTP route.
- Projections: existing local JSONL, governance JSON, and flow context. No SQLite migration,
  Automerge projection, byte-transport choice, or remote publication is introduced.

### Decision-surface concern answers

These answers bound this design; existing registry rows for attribution and actor stamps are
updated with the station's tests. Later adapter/authority surfaces must register their own proofs.

| Concern | State at design birth | Answer / named limit |
|---|---|---|
| C0 plane | answered | Local authored evidence and projections; no remote authority changes. |
| C1 anti-self-election | partial | Existing claims grant nothing; witnessed acceptance remains a later boundary. |
| C2 authority | partial | Preserve historical references and append-only corrections; no new delegation enforcement. |
| C3 liveness | partial | Identity lookup never suppresses governance/corrections; concurrent I/O requires station 2. |
| C4 absence | partial | Preserve existing missing/corrupt fallback and no fabricated claim pin; expose adapter absence in station 4. |
| C5 evidence | partial | Claim CID identifies a statement, not verified identity, authority, or model execution. |
| C6a bounded work | partial | Existing sidecar full scans remain; station 2 measures representative logs before optimization. |
| C6b idempotence | partial | Existing atom dedup reused; cross-process read/check/append needs station 2. |
| C7 advertise/serve | n-a | No network advertisement or serving path in station 1. |
| C8 observability | partial | Station 1 retains claim CID in each supported authored record. |
| C9 lineage | partial | Claim history retained; protocol DID/controller resolution stays in existing graduation work. |
| C10 evolution | partial | No atom fields added; old bytes remain readable; unbound writes keep their previous encoding. |
| C11 backpressure | partial | Station 2 must report lock/I/O failure honestly; no resource admission guarantee here. |
| C12 consent | partial | Existing permission/evaluator boundaries remain authoritative; harness proof pending. |
| C13 graduated authority | partial | No new verdict mapping; refer remains refer. Live adapter proof pending. |
| C14 witnessed residual | partial | Failed/missing attribution does not erase the decision; durable outage evidence needs adapter acceptance. |

## 5. Station 1 contract: retain the existing claim address

When a session resolves an actor, carry the `(claim CID, ActorClaim)` pair already returned by
ActorStore through the authoring operation. Do not perform a second lookup after writing.

Governance JSON gains `actor.claimCid` containing the resolved CID, or null for an unclaimed
session. Calls without a session retain no actor key. The governance decision itself is unchanged.

Flow note, flow claim, and task-report fulfill add `actor-claim:<CID>` to their existing
`classified_as` vector (Commitment.resource_spec for claims). The slot follows existing
descriptive slots and precedes `steward:`, which remains last. Leading positional slots remain
unchanged. No new EPR/REA atom, enum, field, hash implementation, or sidecar is created.

Explicit `--as` remains direct attribution and takes precedence; it does not infer or fabricate
a historical claim even if the label matches one. Missing or corrupt session lookup preserves
the established fallback and emits no actor-claim slot. Old stored atoms are not rewritten.
New session-resolved records intentionally have a different CID because they now state more;
unbound/direct records retain their old encoding. Canonical atom CID computation is reused.

Two workers using the same role/model but distinct existing session scopes must emit distinct
claim references. A later claim in one scope changes only future writes resolved in that scope.
Historical evidence continues to name the old claim. This enables the station 5 join for a
third party to address that claim in an existing correction note without discharging any
commitment or acquiring authority. Today's note target reader only searches FlowStore; the
ActorStore join is explicitly unfinished and is not claimed as station 1 behavior.

## 6. Remaining stations and acceptance story

Station 2 establishes process-safe sidecar creation, append/read coherence and idempotence for
actors and flows, including simultaneous initial opens, crash residue and concurrent claims on
one intent. Reuse filesystem primitives and existing store seams. No background registry daemon.

Station 3 carries an explicit worker scope through existing actor, govern and flow CLI surfaces.
Use the existing session argument as the transport target. A native worker override must outrank
a shared vendor session; an explicitly unclaimed worker must never borrow the parent's claim.
Preserve legacy session-only callers. If a harness cannot identify the originating worker, state
that limitation; never infer it from the last persona registered.

Station 4 projects the native packages and lifecycle bindings into supported installed Claude,
Codex and Gemini surfaces. Verify each runtime's actual version/API at implementation time.
Projected files are generated package-first. Record which controls are mechanically enforced,
advisory, or unavailable. Shell-less workers need dispatch-attributed claims in their own scope.
An explicit CLI simulation is a contract test, not evidence of native lifecycle integration.
Bind registration and scope automatically at the lifecycle boundary. Reuse flow context to
present the commitment's relevant research, governance and tools progressively; do not invent
a parallel onboarding checklist or dispatcher-owned memory of what an agent ought to read.
Test an actual refusal, a referral and a changed obligation: relevant context must arrive before
the governed action, and missing adapter support must not turn refusal/referral into permission.
Record unsupported/unreadable binding once per worker scope through existing observation events.
Station 1 preserves immediate stderr notices only; durable once-per-scope observability is this
station's obligation, not a property of the earlier implementation.

Station 5 records actual capability use and available effort through existing resource/event
and measurement vocabulary, and demonstrates the full chain:

1. Three agents, plus two same-role workers, take distinct existing commitments locally.
2. Each governed action and produced report retains its exact claim and resource references.
3. One worker switches model and another restarts; peers' attribution and prior evidence stand.
4. A reviewer follows the artifact to its commitment and actor claim and records an evidenced
   disagreement. The note does not discharge work or elevate its author's authority.
5. No measurement is invented; declared settings remain distinct from observed usage. Offline
   evidence remains available. Remote witness/DID status is explicit rather than implied.
6. Workers receive the relevant context and tools through existing projections; model changes,
   restarts and handoffs require no manual session-key or claim-CID repair. The reviewer can
   identify the intended telos, values and obligations that governed the work, and the points
   where evidence, consent or judgment was deliberately required.
7. After a deliberately failed outcome and loss of the originating process, an authorized
   reviewer follows the retained graph through the exact historical actor claim and governance
   inputs. It can verify available evidence, name missing or unwitnessed links, record a
   correction, and route the responsibility/repair question through the existing governance
   contract. No broad transcript access, automatic blame, or invented witness is required.

Station 5 explicitly extends existing note-target/context readers to join ActorStore by claim
CID. Test `epr flow note --on <historical-claim-CID>` directly; a reference in a string slot alone
does not make this walk work. Preserve non-discharging correction semantics and disclosure limits.

## 7. Discovering where to begin: the same story viewed forward

An agent beginning without a file path or gap ID needs a useful entry into the valueflow. The
operator wants the graph or semantic neighborhood to reveal how values flow, what needs care,
and where effort would serve the declared telos. This discovery is the forward-facing use of
the same evidence that supports retrospective accountability; it is not a second task ranking
system or an opaque urgency score.

Reuse `FlowWalk::{walk_back,walk_forward}`, `Scopes`, fulfillment/resource/stock folds, sealed
dependencies, habit evidence and `epr flow context`. These already describe provenance,
dependencies, unfulfilled promises, observed conditions and applicable governance. Current
context still requires a path or CID; cold-start selection and a graph/semantic presentation
over this fabric are unfinished bindings, not functionality inferred from those types existing.

Discovery starts from the declared workspace/scope and any stated purpose, then progressively
reveals a bounded set of relevant flows. A selected candidate explains: which intention it
serves, its accountable agents, input/output resources, outstanding commitments, affected
dependents, applicable wisdom and governance, and the evidence behind signals calling for
attention. The next tool/context is offered through existing package bindings. A suggestion
does not claim the work or grant authority to do it.

Graph edges must distinguish declared promises, observed events, fulfillment and governed
dependencies using the existing relationships. A semantic/vector neighborhood, if useful,
is a derived discovery aid: similarity is not an authored relationship, a verified dependency,
trust, or priority. Selecting a nearby node must resolve to the actual addressed record and its
provenance. Missing/stale embeddings must not prevent exact graph or local textual discovery.
Do not add a vector store, graph database, ontology or universal priority metric just to draw
this view; first reuse available projection/search infrastructure and prove the reader's need.

Interpret focus signals with their units, window, freshness, confidence/absence and evidence.
An undischarged promise, a breached declared bound, a stale dependency and a contested assertion
are different reasons for attention. Reuse their native folds/verdicts; volume of activity and
centrality are not measures of social value. Where priorities conflict, surface the reasons and
the governing human/elohim judgment rather than fabricating an objective winner. Preserve the
WIP fence and existing commitments when suggesting a starting point.

Station 6 acceptance: a fresh agent receives only the workspace and a purpose, discovers a
relevant valueflow without being handed an internal ID, and can explain a bounded next action
through the underlying evidence and values. A changed signal changes the explanation only when
its evidence warrants it. The agent can walk backward to the verifiable story and forward to
the consequences of intervening. It can distinguish no relevant work from unavailable evidence,
and a rendered graph neighborhood from a suggestion based on semantic similarity. Discovery
must respect disclosure boundaries, including any search/embedding projection.

Completion requires the story to pass across the actual three harnesses. Station 1 is an
independently useful repair, not completion of that story.
