---
title: "Human Participant in the Actor Plane — Acts, Attribution and Identity Reserve for the Repository Node"
id: human-participant-actor-plane-design
tier: spec
status: Draft
created: 2026-09-23
maintainers: Matthew Dowell + Claude Fable 5.1
class: protocol-canonical
context-tier: disclosed
steward: rust-architect
graduation-trigger: acts-attributed-to-participants habit green OR the actor plane graduates to imagodei Human/Agent entries on the household mesh
topic:
  - actor-plane
  - attribution
  - collective-memory
  - imagodei
  - eprfs
  - rea
  - identity-reserve
informed-by:
  - elohim/epr-rea/src/actor.rs
  - elohim/eprfs/epr-cli/src/flow/memory/import.rs
  - elohim/eprfs/eprfs-agent/src/memory.rs
  - elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs
  - .epr-meta/collective.json
cites:
  - "stewardship-over-sovereignty | the §3 lexicon (stewardship, agency, authority) this spec frames the human claim in; no sovereignty apex | sha256:995eb2079924ea2e | path: genesis/docs/architecture/stewardship-over-sovereignty.md"
  - "private-thought-governed-fruit | the reserve: claims and acts are fruit, recall receipts and transcripts never cross; §4 boundaries 5 and 6 bound the acts store | sha256:5b6f5cdb858277e4 | path: genesis/docs/architecture/private-thought-governed-fruit.md"
  - "cradle-to-grave-capability-gradient | mediated agency named out of scope; no ward entity added here | sha256:1a5b2f7e6433230f | path: genesis/docs/architecture/cradle-to-grave-capability-gradient.md"
  - "elohim-seam-map-concern-routing | places the actor plane: a repo-node rehearsal of the imagodei identity seam, not a new seam | sha256:fd5ced9f996ff5af | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md"
---

# Human Participant in the Actor Plane

## §0 — The defect this answers (measured 2026-09-23)

The repository is the first node: the protocol applied to its own source. Its actor plane today admits
one participant kind. `parse_agent_ref` (`elohim/epr-rea/src/actor.rs`) accepts only
`agent:<role>@<model>`; the human who directs, sources, edits, reviews and commits enters the shared
record only as a provenance string (`imported.gitAuthor`) and as a steward slot that names the
repository, not a person. Four consequences, each verified on disk:

1. **Authorship is rewritten, not appended.** `epr flow memory import` regenerates each
   `.eprfs/status/memory/contributions/<id>.json` from the entry bytes plus the *current* session's
   actor claim and rewrites the file when the text differs. For one record the `author` field has
   been overwritten four times in git history (implementer@opus-5 → orchestrator@fable-5-1 →
   librarian@opus-5.5, with blind-reader in between). The original contributor is recoverable only
   by walking git.
2. **The rich provenance is private; the shared surface is thin.** The append-only plane,
   `.eprfs/status/flows.jsonl`, holds eight cite events from six distinct actors for that same
   concern. It is gitignored. Only the single-author snapshot is tracked.
3. **Reviewer independence is computed against a mutable field.** `graduate` refuses when
   `review.provider == assertion.author`. If A authors, B re-imports, then A reviews, the gate
   passes as independent. And because the human can never be an author, an agent reviewing the
   human's entry passes trivially.
4. **The substrate copies the human's email into fruit.** 250 of 251 tracked contribution files
   carry `Name <email>` in `imported.gitAuthor`; every agent-attributed note carries
   `steward:<git-author-email>` as its last classified slot. Git history already carries the
   email, but the substrate building a second, joinable copy is the aggregator failure the SDO/RWA
   test names (private-thought-governed-fruit §4, boundaries 5 and 6).

The import writer's own comment states the objection correctly and chooses the wrong remedy:
*"minting `agent:<person>@human` would be the substrate asserting a persona nobody claimed."* The
substrate must never forge a persona. The remedy is not exclusion; it is that the human **claims**,
exactly as an agent claims. The target substrate already says so: on the DHT every participant is an
`AgentPubKey` with a source chain, and imagodei carries `Human` and `Agent` as sibling entry types
under the same key discipline. The repository node's grammar is narrower than the network it
rehearses, and a discipline rehearsed on a substrate that misreports provenance graduates with the
misreport in it.

## §1 — Framing (imago-dei floor, not crypto self-sovereignty)

The human is the real persona here, and the question *how is my identity protected* is load-bearing.
The answer is not a stronger key. It is the three words stewardship-over-sovereignty §3 reserves:

- **Stewardship.** The repository collective (`collective:ethosengine/elohim`) is the holon that
  holds the acts ledger. The human stewards it on behalf of the people who will depend on the node,
  and answers to them. The steward slot names the collective, never a person's email.
- **Agency.** The human acts within a web of dependency (agents, tools, prior contributors) and the
  act is theirs. A claim is the human saying *I am acting now*, in flight, disputable by them later,
  superseded without rewrite. Same shape, same honor-system ceiling, as an agent's claim.
- **Authority.** Always qualified. A human claim confers no network membership, authentication or
  publication authority (collective.json charter). It confers exactly what an agent claim confers:
  an addressable statement a later attestation can agree or disagree with.

No tier is named apex. There is no "self-sovereign" seat; there is a participant kind.

### §1.1 — Witnessed, not narrated (operator, 2026-09-23)

An agent acting in the workspace carries no mental load for its own provenance. When it changes a
file the **harness witnesses** that and captures the act into eprfs (the PostToolUse observer mints
it, the same way it already mints claims and fulfilments from brief/report writes). No agent runs
`epr flow memory import`, a witness verb, or any attribution step in order to be recorded; a
discipline that depends on participants remembering to record themselves is the one a swarm forgets
first. Friction is spent only where validation needs it (a gate, a review, a graduate): the points
where trust is built and accountability exercised. Every writer named in §2 is therefore a harness
duty, and a station whose writer is missing from the harness reports that as its gap rather than
asking an agent to do the hook's job by hand.

## §2 — P2P Design Gate

### Entity: ParticipantClaim (the `human:` kind of `ActorClaim`)

- **Classification**: Attested-Private (B2). The claim record itself lives on the participant's
  own append-only ledger (`.eprfs/status/actors.jsonl`, sidecar, never tracked). Its *effect*, the
  acts it signs, is what crosses into shared space.
- **Justification**: Nobody needs the claim to validate anything except the acts that cite it, and
  those acts are fruit. On the DHT the analog already exists: imagodei `Human` (Notarized, keyed by
  `AgentPubKey`) with `HumanityWitness` as the attestation of humanity; the repo-node claim is the
  rehearsal of that entry, not a new type.
- **Head-Plane Cost Budget**: n-a (B2; the attestation is the act, priced under ContributionAct).
- **Network Stakes**: all four stages. **Floor-protected**: a claim is a `Constitutional` record;
  it never cheapens at Simulacra, and honesty about what it proves (nothing beyond the statement's
  existence) never cheapens either.
- **Content Address Strategy**: Agent-Scoped Composite, unchanged — the dag-cbor atom CID of
  `(claimed, session, tree, definition_cid)`. For a human, `definition_cid` is **absent** (a human
  has no build); the atom omits the field rather than carrying a sentinel, so "the same persona,
  rebuilt" remains a real change for agents and a category error for humans.
- **Address Justification**: identity is who-claimed-what-in-which-run; content-derived would make
  two identical statements one actor, slug would name nobody.
- **Grammar**: `human:<handle>`, `<handle>` is `[a-z0-9-]+`, claimed by the human. It is never
  derived from git, never an email, never minted by the substrate. `AgentRef` (`epr-rea::model`)
  stays the one slot: its doc already says the canonical identity is the `uhCAk…` key; both
  `agent:` and `human:` are repo-node rehearsals filling that slot until the household mesh mints
  the key.
- **Binding to other namespaces is a consent act** (boundary 5). A `human:` claim may carry
  `bindings: [{namespace: "git", ref: "<display name as committed>"}]` written by the human's own
  claim. The substrate never infers the binding from `git log`, never writes the email into it, and
  never joins by string. Cross-signing is the graduation (habit `identity-cross-signed`); until
  then the binding is self-asserted and **inadmissible for economic attribution**.
- **Source of Truth**: the participant's own ledger (repo-node: `actors.jsonl`; network: private
  source chain).
- **Integrity Zome + DNA-hash class**: `imagodei_integrity` — **DNA-hash-NEUTRAL** for this spec.
  `Human` and `HumanityWitness` already exist; nothing here touches the integrity zome.
- **Coordinator Zome**: none in this slice; the repo-node writer is `epr actor claim`.
- **Projections**: none. A claim is not indexed, listed or counted (boundary 6: identities are
  free to mint; no boundary rests on counting them).
- **HTTP Route**: none.
- **SDO/RWA Test**: the worst holder of `actors.jsonl` sees which handles ran which sessions on
  this host. It is a private sidecar on the holon's own machine; it never crosses. Bounded by
  boundaries 2 and 6.
- **Anti-Pattern Check**: caught and corrected — the earlier design placed the human as a
  provenance string derived from git (a substrate-inferred cross-namespace join). Corrected to a
  claim the human makes. No sovereignty apex named.

### Entity: ContributionAct (author · edit · review · retract · graduate on a contribution)

- **Classification**: Linked (A2). An act is an event *on* a contribution, not a thing in its own
  right. Repo-node: an append-only line beside the contribution; network: a Holochain link on the
  contribution entry with the verb and actor in the tag.
- **Justification**: The protocol would be lying if an act were silently rewritten (the measured
  defect), and an act has no meaning without the contribution it is on.
- **Head-Plane Cost Budget**: A2-via-link, no new head. Repo-node: ~251 contributions × ~6 acts
  today ≈ 1.5k lines; at one year, order 10k lines across ~500 files. No sweep, no Kad record.
- **Network Stakes**: all four. **Floor-protected**: `CounterEvidence` (a `verdict:changes-requested`
  or a retraction always reaches the author) and `Constitutional` (the first author act is never
  cheapened or dropped).
- **Content Address Strategy**: Agent-Scoped Composite `(provider, verb, resource body CID,
  occurredAt, actor-claim CID)` — the existing `FlowRecord::Event` atom CID, unchanged. The line in
  the tracked store **is the same record, same CID**, as the one in `flows.jsonl`: a projection of
  the fruit subset of the private ledger into shared space, not a second system.
- **Source of Truth**: repo-node, the tracked acts file (fruit placed into shared space; the same
  reasoning `.gitignore` already records for the contributions store). Network: DHT link.
- **Integrity Zome + DNA-hash class**: not in this slice. When contributions graduate to the
  network the link type rides the consolidating home (elohim DNA, where attestations and votes
  already consolidated) — declared here so nobody mints it on imagodei.
- **Coordinator Zome**: none in this slice.
- **Projections**: `.eprfs/status/memory/contributions/<id>.acts.jsonl` (tracked, append-only,
  one `FlowRecord::Event` per line). `author` in the snapshot becomes **derived and frozen**: the
  provider of the first act, written once, never rewritten by a later import. A re-import whose
  source CID moved appends an `edit` act (`run:observation` with the new resource CID) by the
  current actor and leaves `author` alone. Automerge: n-a (A2).
- **HTTP Route**: none.
- **SDO/RWA Test**: the worst holder of the tracked store sees which handles authored, edited and
  reviewed which memory concerns, and when, at `reach: repository`. That is the declared reach of
  the collective (collective.json `sourceRules`), the repository is public, and inside a holon
  peers see each other by design. What it must **not** see: an email, a transcript, a recall
  receipt, or a join to any other namespace. Bounded by boundaries 1, 2, 5 and 6.
- **Anti-Pattern Check**: caught — the single rewritable `author` field is the "per-host authored
  write of a notarized field" shape from the catalog, one layer up. Corrected to derive-and-freeze
  plus append-only acts.

### Entity: IndependentReview (decision predicate, Step 4)

- **Kind**: `pure-decision-predicate`. `independent(review, acts) := review.provider ∉ {a.provider
  | a ∈ acts, a.verb ∈ {author, edit}}` **and** the review's actor claim is not the same
  `definition_cid` lineage as any author/edit claim (C9: a persona rebuilt is not a different
  reviewer). Replaces `review.provider == assertion.author`.
- **Concern canon**: C1 anti-self-election **answered** (the reviewer cannot be any actor who
  shaped the body). C2 monotonic authority **answered** (existing: a later contrary verdict is not
  erased by selecting an older approval). C4 honest absence **answered** (no acts file ⇒ the
  predicate refuses with "no attributed acts", never passes). C9 identity lineage **partial**
  (lineage compares `definition_cid` for agents; a human has none, so two humans are distinct by
  handle only, honor-system). C12 consent **answered** (bindings only by the participant's claim).
  C14 witnessed residual **answered** (a refused review is itself a recorded act). C0, C3, C5,
  C6a/b, C7, C8, C10, C11, C13: **n-a** (local predicate over an in-memory set; no liveness,
  backpressure or serving surface).
- **Registration**: one row in `elohim/eprfs/epr-cli/seam-registry.yaml` at implementation, with
  `contractTests` naming the three tests in §5 (explicit `null` + `gapNote` until they exist).

### Entity: IdentityReserve (what the substrate may copy about a human)

- **Classification**: Ephemeral (C) — it is a rule over projections, not a store. Declared here
  because it retires four hundred existing copies.
- **Rule**: the substrate writes a human into fruit only as the handle the human claimed. Where no
  claim exists it writes `(unclaimed)`, an honest absence, exactly as `git_author` already writes
  `(untracked …)`. `imported.gitAuthor` is replaced by `imported.gitName` (the display name only,
  which the commit already publishes) and the `steward:` note slot names the collective id, never an
  email. A one-time migration act rewrites the 250 files and is itself attributed.
- **SDO/RWA Test**: after the rule, the tracked store carries no email and no cross-namespace key.
  Boundaries 5 and 6 hold by construction.

### Design Constraints Discovered

- **Bootstrap honesty.** The commitment that carries this spec is minted before the grammar exists,
  so it is attributed to an agent with the human in the steward slot: the defect, recorded once
  more on purpose, with an observation note naming it. The first act under the new grammar is the
  human claiming.
- **Reach never widens.** A human claim carries `reach: session`; acts inherit the contribution's
  reach; `contribution cannot widen source restriction` (validation.rs) stays the rule.
- **Ordering.** Grammar → import derive-and-freeze + acts projection → independence predicate →
  identity-reserve migration. The migration is last because it needs the handle to exist.

## §3 — Attribution relationship (the produce of labor, the commons, and the fruit)

Once every participant is a first-class actor, the acts ledger is already an REA event stream:
`provider`, `receiver`, `resource`, `quantity`, `inScopeOf`. Attribution then becomes a
**derivation over events**, never a declared share. Three parties, in the Georgist grain the
operator named:

| Party | What they bring | REA reading | Where the value flows |
|---|---|---|---|
| The human | attention, direction, sourcing, guidance, review, the standing that constitutes the collective | `human:<handle>` as provider on `direct`, `source`, `review`, `ruling` acts | to the human, as the produce of their labor |
| The AI agents | generated code, edits, verdicts, run-notes | `agent:<role>@<model>` as provider on `author`, `edit`, `review` acts | to the **commons pool that constituted the persona**, not the instance: an agent's hard layer is a kind shared by every run and identities are free to mint, so the pool (the package `definition_cid` lineage and the collective that dispatched it) is the attributable unit |
| The network | what the code accomplishes as a running whole | the collective as receiver; downstream `ContributorPresence` and economic events on the household mesh | to the commons, as rent on the shared substrate, from which the pools are fed |

Three rules keep this honest, and each is already canon:

1. **Attribution rides admissible bindings only.** A self-asserted claim is evidence of a
   statement, not a credential. Economic joins over the acts ledger stay in **observe** posture
   until claims are cross-signed (habit `identity-cross-signed`; the attribution cut already
   refuses a sentinel binding in elohim-storage). Nothing in this spec issues, tallies or pays.
2. **Thoughts never enter the ledger.** Direction and attention are attributed by their *acts*
   (a ruling note, a claim, a review, a source pinned), never by transcripts or recall receipts.
3. **Reach is earned by fruit under a steward's standing.** A pool's share of the commons is a
   function of witnessed acts over time, not of how many agent identities it minted.

### §3.1 — Declared bootstrap bounds (operator, 2026-09-23; development-stage, revisable)

A fixed ratio is the wrong primitive. The protocol's discipline is a bounded curve (dignity floor
+ limitarian cap) with the shares between derived from witnessed acts. The bootstrap consensus
(one human and the agents they consult, which is not an independent consensus in the protocol's
sense and is recorded as such) declares three structural rules and the numbers they need:

1. **Rent versus labor is computed from the cite graph, not negotiated.** A valueflow arriving at
   an EPR splits into a *retained* portion for that EPR's participants and an *upstream* portion
   that flows along its `cites:` / seals to the EPRs it inherits from, recursively, until it lands
   on the `.epr-meta` anchor with nothing further to cite. The anchor is the sink — the lowest
   accountable aggregate. One declared number: the **retention factor `r`**, bootstrap `r = 1/2`
   (half retained, half upstream, geometric across the cascade, sums to one).
2. **Within the retained portion, witnessed acts only, judgment weighted equal to production.**
   Raw acts are free to mint (as identities are), so a volume-weighted split is the swarm's capture
   vector. Only acts another party consumed count: reviewed, cited, graduated, or ruled on. At
   bootstrap the verb classes `direct · source · review · ruling` (judgment) and `author · edit`
   (production) carry **equal weight**; the human's attention must not round to zero against
   generated volume.
3. **An AI agent's produce IS rent.** (AI agent: an `agent:<role>@<model>` participant; not to be
   confused with the human stewards, who are also agents in the substrate's sense and whose produce
   is labor.) What an AI agent generates is the unearned increment of a commons it did not make
   (training corpora, prior contributors, the substrate it runs on), so it flows to the pool that
   constituted the persona, never to the instance; and at bootstrap that pool is the collective, not
   the vendor. Model vendors are paid at market rate out of the operator's pocket;
   in-protocol the model is **provenance, never a payee**. The ledger records which model
   did which act as information (currency is information) and no vendor accrues protocol standing
   from it. The agent-pool portion lands on the `.epr-meta` anchor holding the persona packages.

| Portion at any EPR | Flows to | Set by |
|---|---|---|
| upstream `(1 − r)` | cited EPRs recursively, then the anchor | derived from the cite graph |
| retained `r`, human acts | the human who claimed them | derived from witnessed acts |
| retained `r`, AI-agent acts | the anchor's commons pool | derived from witnessed acts |

Bounds on top: every witnessed act earns a nonzero **floor**; no participant exceeds a declared
**cap** of any one anchor's flow, the excess rolling up the anchor cascade (a pressure against the
cap is a design signal, not a capacity request). Today one human witnesses, so their derived share
is large and honest; it dilutes by others' acts, never by fiat, and every number here becomes
revisable the moment a second human witnesses.

**Where the friction is spent (operator, 2026-09-23).** The acts ledger is the frictionless floor
the harness keeps (§1.1). Above it sits the REA Values Scanner epic: agents *deliberate* to author
the REA stories that say what a flow was for, and the attribution scenario in this section is one of
those stories. The protocol takes on the coordination frictions that kept people from surfacing
deeper values, care among them, so that the richer story can be told at all; the ledger's job is to
make that story derivable from witnessed acts rather than asserted over them. (No spec-tree document
carries the epic by that name yet; the name is the operator's and this line is its first anchor.)

The mechanics of the pools (how a share is realised, how the cascade is folded, how a pool
constitutes a persona) remain a **downstream arc with its own gate**: it joins shefa's REA plane and
the contributor-presence work, and it must pass the p2p-design-gate on its own entities. This
spec's deliverable is the ledger those economics will be derived *from*, with every party on it,
and the declared bounds above as the first measures that ledger is read against.

## §4 — Habit

Declared beside this spec as `.epr-meta/acts-attributed-to-participants.habit.md`, status
`unwired` at birth: committed to, with the checks named and not yet runnable. It flips red when the
first test in §5 exists and fails, green when all three pass and the migration act is on the ledger.

## §5 — Checks (the tests that flip the habit)

1. `elohim-epr-rea`: `parse_participant_ref` accepts `human:<handle>` and `agent:<role>@<model>`,
   refuses `agent:<person>@human`, an email, and any substrate-minted form.
2. `elohim-epr-cli` memory import: re-importing a contribution under a different session leaves
   `author` byte-identical and appends exactly one act to `<id>.acts.jsonl`; a second identical run
   appends nothing.
3. `elohim-epr-cli` graduate: a review by any actor who authored or edited the body is refused
   naming C1; a review with no acts file is refused naming honest absence.

## §5.1 — Stations (the projector reads these; each flips one check in the habit atom)

- [ ] Station 1 — grammar: `parse_participant_ref` accepts `human:<handle>` beside `agent:<role>@<model>`; `definition_cid` optional and absent for a human; test `participant_ref` born with it; `epr actor claim --as human:<handle>` works
- [ ] Station 2 — the human claims once: a `human:<handle>` claim is **standing per workspace** (agents claim per session; a human is not a run), registered by the operator one time and read by the harness thereafter; the first act under the new grammar is theirs (recorded as an observation note on this spec's commitment)
- [ ] Station 3 — derive-and-freeze author + `<id>.acts.jsonl` projection, written by the **harness** (the PostToolUse observer on a memory-entry write, and `memory import` when the harness runs it); no agent runs an import to be attributed; test `memory_import_freezes_author`
- [ ] Station 4 — `IndependentReview` predicate replaces the single-field check in `graduate`; row in `elohim/eprfs/epr-cli/seam-registry.yaml`; test `graduate_refuses_shaping_actor`
- [ ] Station 5 — identity-reserve migration: `imported.gitAuthor` → `imported.gitName`, `steward:` slot names the collective id, the 250 tracked files rewritten by one attributed act; the habit's email check goes green
- [ ] Station 6 — §3.1 bounds declared as measures in `.claude/epr-meta/measures.yaml` (`attribution-retention-factor@1`, verb-class weights, floor, cap) so the ledger can be read against them; no share is realised

## §6 — Out of scope, named

- Cross-signing a human claim (the `identity-cross-signed` habit owns it).
- Guardian/ward and mediated agency in the repo node (cradle-to-grave gradient; no ward entity
  exists yet and this spec adds none).
- Any share, tally, currency or payout derived from the ledger (§3 downstream arc).
