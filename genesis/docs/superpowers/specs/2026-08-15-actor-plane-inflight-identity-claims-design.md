---
title: Actor Plane — In-Flight Identity Claims, Acceptance at Ratification, Adverse Attestation
id: actor-plane-inflight-identity-claims-design
status: Draft
class: design
domain: D9
sprint: actor-plane
requires_env: [household-nodes]
context-tier: disclosed
steward: orchestrator
graduation-trigger: decompose-complete OR superseded-by-implementation
cites:
  - "actor-plane-implementation-plan | the sealed task-by-task plan this spec is the design record of — Tasks 1-4 landed, Task 5 produced this document | sha256:3044daacc8d5b48f | path: genesis/docs/superpowers/plans/2026-08-15-actor-plane-implementation-plan.md"
  - "contributor-presence-commons-stewardship | the witness-vs-steward separation and the ContributorPresence graduation target every claim resolves to | sha256:9551a506a01ab1b7 | path: genesis/docs/superpowers/specs/2026-07-21-contributor-presence-commons-stewardship-design.md"
  - "sense-respond-governance-classifier | the honors-contract signatory (10.4A) the claim lifts to session scope, and the 11.2 correction vocabulary adverse attestation rides | sha256:c716a519ee6cc953 | path: genesis/docs/superpowers/specs/2026-07-15-sense-respond-governance-classifier-design.md"
  - "computation-attestation-graduated-rigor-design | the Witness/Audit tier ladder acceptance-at-ratification records against, unchanged | sha256:d767f5c1eb04c841 | path: genesis/docs/superpowers/specs/2026-05-01-computation-attestation-graduated-rigor-design.md"
  - "elohim-ceiling-design | Principle 8 — why no plausibility predicate is invented at acceptance: a mechanism nobody can audit is refused | sha256:24925a4c8e1d9420 | path: genesis/docs/superpowers/specs/2026-06-23-elohim-ceiling-design.md"
---

# Actor Plane — In-Flight Identity Claims, Acceptance at Ratification, Adverse Attestation

> **What this is.** The identity plane for agents and humans working inside this repository.
> An agent claims its own identity in-flight (`epr actor claim`); that claim shows up
> on the two surfaces that carry attribution today (value-flow notes and governance
> ledger rows); co-author trailers lift into the same `classified_as` vocabulary at
> projection time; and an acceptance ceremony at dev-merge ratification turns an
> honor-system claim into a witnessed record. Everything here composes from existing
> canon (ActorClaim is a dag-cbor atom; co-author entries ride `classified_as` inside
> canonical event bytes; acceptance is a graduated-rigor Witness station). No new
> primitive family was introduced.

## 1. The problem: trailer rosters as the degenerate form

Attribution in the dev harness was dispatcher-written trailer rosters: assertions *about* others, composed *after* the fact, only as good as the dispatcher's memory. Commit `e4c4accf3` (the three-trailer roster) is the named degenerate-form fixture. That shape fails on three axes. The named party cannot dispute the attribution. The attribution cannot change mid-run (a persona switch during a session leaves no trace). And the roster can only name what the dispatcher remembers, which degrades as session complexity grows. Backlog rows 16/16a (capability-attestation canon) described the target: identity that is self-claimed, in-flight, disputable, and durable across the surfaces where attribution matters. The operator directive of 2026-08-15 instantiated those rows.

## 2. Three legs

The design has three legs, composed from existing canon without introducing a new primitive family. Legs 1 and 2 are implemented and evidenced (commits in Section 8); Leg 3 and the Section 5 graduation path are design. They form an evidentiary ladder: trailer roster (asserted, post-hoc) is weaker than ActorClaim (self-claimed, in-flight), which is weaker than accepted-at-ratification (witnessed), which is weaker than adverse-corrected (evidenced). Each rung adds trust weight; none gates participation.

### 2.1 Leg 1 — Claim (honor-system floor, never blocks)

The claim command is `epr actor claim --as agent:<role>@<model> --session <id>`, with `epr actor current --session <id>` reading back the active identity. `epr` is the repository's native governance-and-valueflow CLI — one Rust binary (crate `elohim/eprfs/epr-cli`) installed on PATH, the same binary that serves as the governance plane's decision authority. The record shape is `ActorClaim { claimed, session, claimed_at, definition_cid }` in `elohim/epr-rea/src/actor.rs`. A fabric atom is any payload struct encoded as canonical dag-cbor; `atom_cid` mints its identity as a CIDv1 (dag-cbor, sha2-256) over exactly those bytes — identity is the content address, so restating any field re-addresses the record. ActorClaim is minted through that codec with a sole validating constructor.

**Role and model charset.** Role is `[a-z0-9-]+`, model is `[a-z0-9.-]+` — lowercase-only, because case variants of a role are a free impersonation surface, and both strings become path segments and log keys where case-folding would silently merge distinct identities.

**`claimed_at` is git HEAD author date, never wall clock.** The constructor refuses when no HEAD exists. `epr-rea` reads no git and no clock: the caller supplies the timestamp. This constraint exists because two peers folding the same records must mint identical CIDs — a wall-clock dependency would make the atom's content address vary by machine.

**`definition_cid`** is `sha256:<64hex>` of the raw bytes of `.epr-meta/elohim/packages/agents/<role>.json`. This follows the `evaluator_identity()` pattern — `epr govern` names the exact evaluator build by the sha256 of its own binary so a decision can be disputed against a specific build, and `definition_cid` is that move applied to a persona package. It identifies the *build*, never the instance: two runs of one persona share the same `definition_cid`. `None` is the honest-absence value (no package on disk), and it is omitted from encoding when absent, following the `Commitment::bound` additive discipline (an optional field omitted from the encoded bytes when absent, so records authored before the field existed keep their exact bytes and CIDs).

**The sidecar store.** Claims persist in `SidecarActorStore` on `.eprfs/status/actors.jsonl` — a log *separate* from `flows.jsonl`. The separation is load-bearing: identity reads must be able to fail without costing the value-chain log, and a decision authority consulting identity must be able to fail to read it and still decide. Lines are append-only `{cid, record}` pairs, with CID re-verified on read (a tampered line is an Integrity error, not drift). The record uses a tagged `ActorRecord` envelope with a single `Claim` variant today; the acceptance leg (Section 2.3) will land as a second variant without a line-format break, and `ActorStore::claims()` is an exhaustive match so adding a variant is a compile error at every decision point.

**Latest-wins stacking.** Claims stack; the latest per session wins, resolved by append order via `rfind`. Claims made against the same tree share a `claimed_at` timestamp, so a timestamp sort could not order a same-tree persona switch — append position is the only ordering that always works. History is never rewritten: a superseded claim is evidence, not garbage. Idempotence is scoped to "already current," not "CID seen," because position carries meaning in a latest-wins log. Re-claiming a superseded identity is a real act (the agent returned to a previous persona), not a redundant one.

**Nothing blocks on a claim.** A claim that could block would be a credential, and nothing in this plane can verify one.

### 2.2 Leg 2 — Attribution surfaces (where a claim shows up)

Claims become visible on the three surfaces that carry attribution: value-flow notes, governance ledger rows, and co-author trailers lifted at projection time.

#### 2.2a Value-flow notes (`epr flow note --as / --session`)

The note command resolves identity through three arms, evaluated in order before the append:

1. **`--as` named outright** — the provider field becomes the agent ref directly.
2. **Session** — the `--session` flag, falling back to `CLAUDE_SESSION_ID`, then `ELOHIM_SESSION_ID`. Blank values are treated as absence, not as empty-string identity. Env vars are resolved in the CLI shell so that `note()` itself stays environment-free. When a session is found, the sidecar's `current_for` resolution supplies the active claim.
3. **Neither** — byte-identical legacy behavior. The golden CID pin (a unit test asserting a literal fixture record's CID against a hardcoded string, so encoding drift fails loudly) is unchanged.

Agent arms (arms 1 and 2) append `steward:<git-author-email>` as the *last* `classified_as` slot. `classified_as` is a list-of-strings classification field carried inside the hashed bytes of REA records (events here); convention puts the tag first, the subject second, and later slots under prefixes (`reason:`, `switched-to:`, `steward:`, `co-author:`), and because the value lives inside the canonical bytes, whatever lands there rides the record's content address into every store. The human whose key signs the tree stays answerable when an agent authors inside it. Steward is a property of the commit, not of the claim — losing it is how attribution turns into deniability. The slot is last because leading slots are read positionally by other legs.

Identity-plane failures (no sidecar, no claim, unreadable store) fall through to author-attribution with a `stderr` notice. The note is the durable thing; an enrichment that can veto its subject is a dependency in the wrong direction. There is one refusal: malformed `--as`. Substituting the author identity for a named identity would mint a record asserting someone else spoke.

#### 2.2b Governance ledger rows (`epr govern --session`)

The governance command adds an `"actor"` key to its payload only when asked: `{claimed, session, definitionCid, source: "claim"|"unclaimed"}`. Two honest source states, no third. Every sidecar failure collapses to `unclaimed`: the identity plane never breaks the decision authority, and a tampered claim reads as `unclaimed`, never as the identity the tamperer wrote. Store existence is checked before open — a governance READ must not create `.eprfs/`.

The Python plumbing is additive: `epr_client.govern(session=None)` and `epr_meta.witness(actor=None)`. The ledger key is present only when the plane was consulted; absence means "not consulted," which is distinct from `unclaimed` (that value lives inside the dict, never inferred from the key's absence). The resolver hook threads the `stdin session_id` and captures the native actor (`_ACTOR` beside `_EVALUATOR`). Separately, the git gate (a CLI covering every author, not just Claude) resolves a session from `CLAUDE_SESSION_ID`/`ELOHIM_SESSION_ID` on a best-effort basis. Governance ledger rows now carry who-acted next to which-evaluator-decided.

#### 2.2c Co-author trailers (`epr flow project`)

The `producing_commit` function lifts `Co-Authored-By` trailers from git log output (`%(trailers:key=Co-Authored-By,valueonly,separator=%x1e)`; an old-git guard treats an unexpanded `%(trailers` specifier in the output as an empty roster). The normalizer `normalize_co_author` is a pure function: `"Name <email>"` maps through the email domain — `noreply@anthropic.com` yields `agent:<name-slug>`, `noreply@ethosengine.com` yields `collective:<name-slug>`, an unknown domain yields the lowercased email, and junk yields `None` (one bad trailer never fails a projection). The slug rule is: lowercase, non-alphanumeric runs collapse to one dash, trimmed; an empty slug yields `None`, because a term whose subject is the empty string names everyone.

Produce events gain sorted, deduplicated `co-author:<normalized>` entries in `classified_as`. The provider stays the signing author (the steward's signed envelope). Slots are sorted and deduplicated because the slot list is part of the event's address, and typing order is not collaboration structure.

**The dedupe guard.** Enrichment applies to newly-minted events only. Retro-attribution — applying trailers from a re-projection to events that already exist — is a deliberate, separate migration, never a side-effect of re-projection, because re-projection would otherwise double-count every trailer-bearing Produce in every stock fold (the stocks layer folds events into level/inflow/outflow measures, so a double-counted Produce inflates inflow). The guard uses `EventKey { action, provider, resource, process, occurred_at }` built from existing events; a new-CID-same-key match counts as present. `classified_as` is deliberately excluded from EventKey because it is the field that may grow — including it would restate the CID and guard nothing. The guard carries two separate proofs. The guard-bite check is a fixture proof: a message-only amend fixture (tree, body CID, process, author, and instant all unchanged; only the roster grows — the exact divergence a CID-only guard cannot see) where temporarily disarming the guard makes the test fail with a double-count. The full-real-history evidence is the live re-projection: 421 events counted already-present, 1 new. The single new record is the actor-plane plan's own Produce event, carrying `co-author:agent:claude-fable-5`, `co-author:agent:claude-opus-5`, and `co-author:collective:ethosengine`.

### 2.3 Leg 3 — Acceptance at ratification and adverse attestation

This leg is design, mostly not built in this change. The two halves:

**Acceptance.** Ratification at dev-merge is the acceptance act — a graduated-rigor Witness station. It is *recorded, not judged*: no plausibility predicate is invented, because a mechanism nobody can audit is what Principle 8 (elohim-ceiling) rejects. The acceptance ceremony fills commitment-dispatch-puller C9 (session holder maps to identity) and respects C1 (anti-self-election: a claim never elects itself).

**Adverse attestation.** Contest escalates the next acceptance to the Audit tier, riding graduated rigor unchanged. Adverse attestation uses classifier section 11.2 `correction` with an evidence CID against the claimed identity — never a bare accusation. Aggregation keys only on evidenced corrections. At v1 adverse attestation records but never blocks; teeth (propagation and standing effects) arrive at DHT graduation.

## 3. Two vocabularies share the `agent:` prefix

This section records a p2p-design-gate finding that must be understood before extending either vocabulary.

Two structurally different name-forms share the `agent:` string prefix, and they must never be confused:

**Actor-plane refs** take the form `agent:<role>@<model>`. They are validated by `parse_agent_ref`, self-claimed (or dispatcher-registered for shell-less personas), session-scoped, supersedable, and disputable. The `@` half is the structural marker.

**Trailer-derived names** take the form `agent:<name-slug>` and appear under the `co-author:` slot prefix. They are steward-asserted display-name vocabulary, post-hoc, built by heuristic domain mapping from email addresses in `Co-Authored-By` trailers. They are not identity resolution.

The two are deliberately structurally disjoint: the slug form carries no `@` half, so `parse_agent_ref` refuses it by construction. The two vocabularies can never be programmatically confused by anything that validates through the parser. Identity resolution to a durable identity — joining across these namespaces — happens at graduation via `ContributorPresence` and a DID bridge, never by string equality. Cross-namespace identity joins by string equality are the named anti-pattern; the structural disjointness is the compile-time guard against it.

## 4. The concurrency seam

The dispatcher is the top-level session agent doing the orchestrating (claimed here as the `orchestrator` role); a persona is a packaged subagent definition under `.epr-meta/elohim/packages/agents/` that the dispatcher runs. Subagents share the dispatcher's session id (hooks receive the top-level `session_id`), so a persona claim supersedes the orchestrator's claim for the whole session while the persona runs. Latest-wins is per-session, not per-actor — this is a consequence of the stacking rule (Section 2.1), not a bug in it.

Two personas in this system — scribe and blind-reader — are deliberately shell-less. Blind-reader's isolation is its instrument (it reads without the ability to write); scribe's constraint is narrower (no shell, only file tools). Neither can self-claim. Their agent packages (v1.1.0) carry an Attribution section naming the discipline: the dispatcher registers the persona claim on their behalf.

**The v1 discipline.** The dispatcher registers the persona claim before the persona's first round, serializes persona rounds (no concurrent persona claims on one session), avoids its own governed writes (writes that pass through the `.epr-meta` compose-gate, such as edits under governed trees like this specs directory) inside the persona window, and re-claims its own identity after the persona finishes. A persona switch is a one-line append to `actors.jsonl`.

**The graduation question is open and named.** Per-instance claims (distinguishing two concurrent runs of the same persona) or subsession ids would resolve the concurrency seam fully. Neither is resolved in v1; the serialization discipline is the mitigation. This is recorded as run note `bafyreicfb4...zpsm` on the implementation plan.

## 5. Dataplane scaling story

Everything today is projection-plane. The p2p-design-gate result:

**Git is the truth layer.** `.eprfs/` is gitignored operational state; the durable truth is git commit objects. Trailers live inside the steward-signed, content-addressed commit. brit is the covenantal version-control layer — an expansion of gitoxide living at `elohim/brit` — whose design makes commit trailers the protocol surface: its engine crate (brit-epr) parses and validates RFC-822 trailers against a pluggable app schema and carries CID utilities and signing hooks. The trailer is brit's own protocol surface. "Git with extra trailer discipline" is brit's own degraded-mode description. `actors.jsonl` is an authored local operational log (the honor-system floor). Its store mirrors the FlowStore polymorphic-persistence seam — `ActorStore` is the same trait shape the diesel projection and DHT rails implement against, so the sidecar is the offline floor of the same three-depth design, not a dead end.

**`classified_as` entries ride atom bytes.** Co-author and agent entries live inside the canonical dag-cbor atom bytes of each event. Because they are inside the atom, they ride the `FlowStore` trait into diesel/sqlite projections and DHT rails unchanged — no schema change, no new wire messages. A trailer never crosses the p2p wire as a trailer; it is lifted once at the git boundary and lives as structured data from that point forward.

**Graduation path.** Each persistent agent identity becomes a `ContributorPresence(unclaimed, steward_id=operator)`. Co-authorship becomes an `attestation:witnessed-ascription` manifest kind, requiring zero new entry types (it rides the consolidated attestation home). Counterfeit identity is the named impersonation-claim signal gap. The cryptographic ceiling is brit's `AgentKey` (ed25519 at `.git/brit/agent-key`, phase-2a attestation primitives) and a DID bridge. Head-plane cost is a graduation-policy knob (recipe-declared crystallization, bundled under a `ContributorPresence` root), not a per-commit tax.

**Migration trajectory.** `normalize_co_author` and the `producing_commit` family are kept pure and small because they migrate into brit's commit-lift when brit absorbs commit-object lifting. `Co-Authored-By` should register in brit's trailer key grammar at that point.

## 6. Explicit non-paths

These were evaluated and rejected during design:

**KeyRotation / CryptographicQuorum** — red-team-refuted. The honor-system floor does not need rotation machinery, and adding it would imply a credential regime that does not exist here.

**`governance` as a `SubstrateSignal` member** — adding an enum variant to `SubstrateSignal` is an integrity-zome change and moves the DNA hash. That price was refused.

**Escalation-provenance in canonical bytes** — how a record was later handled is carried by later records, never baked into the hashed bytes of the earlier one. An atom whose address depended on its own subsequent handling would re-address for a reason that is not a change in what it says.

**Self-issued credentials as apex** — the identity-sovereignty ontology guard applies: community governance backstops individuals; self-issued credentials are never the apex tier.

**No plausibility predicate at acceptance** — rationale in Section 2.3.

## 7. Seam registry

Registered in this change, per the Step 4 birth rule (the p2p-design-gate's Step 4: a decision point registers its concern answers at birth, enforced by the placement-audit census). The C-codes referenced below are the concern canon — sixteen recurring failure classes (C0 through C14) mined from repeated production incidents, cataloged in `.claude/epr-meta/policies.yaml` and `concerns.yaml`; every new decision point answers them at registration.

**epr-rea:** `ActorClaim::new` and `parse_agent_ref` (boundary-answer refusals — malformed input is refused at construction, never downstream). `ActorStore::current_for` (latest-wins resolution rule).

**epr-cli:** `note` three-arm `resolve_attribution` (reason and outcome per arm; C4 honest absence — missing data is surfaced, never silently papered over; C5 evidence-not-authority — the identity plane informs decisions but never overrides the decision authority). `govern` `actor_stamp` (boundary-answer-type `claim` or `unclaimed`; identity plane never breaks decision authority). `normalize_co_author` (pure normalizer). `project` `EventKey` dedupe guard (C6b idempotent effect — re-running the same projection never changes the outcome beyond the first application).

## 8. Evidence

**Commits.** `967158231` (Task 1 — ActorClaim, ActorStore, CLI claim/current). `e6ad72cf0` (Tasks 2+3 — note attribution arms, govern actor stamp). `c981229bf` (template plumbing). `c936435f7` (Task 4 — co-author trailers lifted onto Produce events).

**Golden CIDs.** `actor_claim_cid_is_stable`: `bafyreib3xfuy6hchjcbtrjktxfho52oqbs5pbu2e5depagycipxx4jnvpi`. `note_event_cid_is_stable`: unchanged (unflagged notes are byte-identical to pre-change behavior).

**First live records.** Orchestrator claim: `bafyreifa6atceormur2io4frex7q7m2gp2pjazo3mvdo2mpklvasuf7kza` (`agent:orchestrator@claude-fable-5`, `definitionCid` null — honest absence, no package on disk). First arm-2 attributed note: `bafyreicfb4h5akwymqkswzgwstnlczisgvokmg27tkyvav44trykvjzpsm`.

**This spec as claim-switch evidence.** This document was itself written under the claim-switch discipline it documents. The persona claim `bafyreigfjneabvlkyt7h2n6b6eiaf6ex2ptt7ol2h25dq35zdkg555h66y` (`agent:scribe@claude-opus-4-6`, `definitionCid` `sha256:e117a5a9266ae62bc62d075349cd8a92a84db2bb657aea16302ee9491d3bce30` — the raw bytes of the v1.1.0 scribe package) superseded the orchestrator's claim for the writing window and was named in the claim outcome.

**Stale-binary degradation.** A pre-`--session` `epr` binary on `PATH` exits 2 on `--session`. `epr_client` returns `None`. The Python evaluator decides with a `DEGRADED_NOTICE` (never silent). Degradation is designed, not an error.
