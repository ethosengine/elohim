---
title: The participation read — how can I help, where, and for whom
id: participation-read-design
status: Draft
class: process-meta
process_subdomain: agents
serves: [build-earns-reach-by-attestation, sprint-lands-each-closure-once, devices-share-one-berth]
date: 2026-11-22
context-tier: disclosed
steward: human:matthew
author: "gpt-6-astra (Codex), drafted at the direction of the session model (Claude Fable 5.2), which rules it"
refines: native-dev-plane-one-derivation-earned-reach-design
tags: [participation, standing, presence, intent, imagodei, agents, valueflows, berth, reach]
graduation-trigger: >-
  The participation read answers the three questions from locally held evidence without writing,
  preserves each source's meaning of open, distinguishes people from envelopes and agents,
  and passes visibility, absence, plural-role and capacity fixtures against a native reader and
  a fake. The three serving habits carry the corresponding evidence before graduation.
cites:
  - "native-dev-plane-one-derivation-earned-reach-design | One derivation, earned reach | sha256:024a89c3dba2699f | path: genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md"
  - "standing-read-for-counting-principals-design | The standing read for counting | sha256:449b53bb4c7f8850 | path: genesis/docs/superpowers/specs/2026-11-22-standing-read-for-counting-principals-design.md"
  - "native-dev-tooling-over-peers-plan | Native developer CI/CD over peers | sha256:aca3fb5f6781bc76 | path: genesis/docs/superpowers/plans/2026-10-29-native-dev-tooling-over-peers-plan.md"
---

# The participation read — how can I help, where, and for whom

Any agent, wherever it finds itself, can ask how to participate in the work already declared. The answer composes intent, standing, presence and recorded value. It creates no work.
The native dev tooling plan §9 supplies the carrier contracts; §10 keeps speed distinct from standing and earned reach. This read is the second consumer of the standing read's §3 contract.

## 1. Rulings from the steward

Each quotation below is his words, unedited, given on 2026-11-22.

**R1 — his words, unedited**
> what agent is working on what is a presence+intent signal keyed to each artifact, it's possible impacts calculable against the dependency graph

Working intent stays attached to the artifact. Impact is a local graph read over declarations.

**R2 — his words, unedited**
> those are live/async system signals that's should flow over the peer network if your downstream that shouldn't be a lock... but your worktree should be keyed to your local, but you should get live updates from upstream so it's frictionless to look over.. see the incoming impacts and adjust/adopt in realtime if it's useful.. as an optional way of collaboration

Signals inform optional collaboration. Incoming evidence changes neither local contents nor the local closure; inspection and adoption remain explicit acts.

**R3 — his words, unedited**
> an agent working in the same Dev space, the limit is the available compute to the container you're in as opposed to getting a signal. And the context is your ethos engine on a different machine. The contextual understanding of your constraints are integration coordination constraints... Either one could be bottlenecks I mean the last thing is if you have Jenkins that acts as a marketplace peer that's eager to do jobs and can accept jobs instantly from the marketplace, and it's also aware of its own limits like that's another kind of pure participant

The device senses its capacity. Coordination supplies context; a Jenkins carrier would answer under its own admission contract, without becoming an authority over another device's work.

**R4 — his words, unedited**
> We're trying to make it where any agent, wherever it finds itself in the network can easily answer the question. How can I help? Where can I participate? What can I do? Become easy questions to answer in a rich context

One on-demand read answers those three questions from evidence within the caller's reach.

**R5 — his words, unedited**
> Those questions also could draw, in those same moments, from the valueflows... So the what's this worth for what, and for who, and what else could be said.. that helps the reconciliation

Each item carries its existing valueflow evidence, including testimony and unknown attribution.

**R6 — his words, unedited**
> For whom.. remember that stewards/gifters and receivers.. we pluralism is the usual default, who needs what is a composition of standing and presence

Stewards, gifters and receivers remain separate sets. Who needs what is composed when asked.

**R7 — his words, unedited**
> It might be an enrichment of the 'A' and imagiodei(s) in  the REA..

The first enrichment is a read-side A view of identity and standing, without a new wire shape.

**R8 — his words, unedited**
> workspaces/envelopes and to some extent the elohim agents (since their identity are layered artifacts persistent and ephemeral)  are different /gradient-reading from the 'A' rea/identity/imagiodei primitives we're touching on

The A, the envelope and the layered elohim agent stay separate in every output.

**R9 — his words, unedited**
> presence has two planes too... ai presence .... Can be called, invoked, vs human presence.. my human is present and I can reach them readily... Or likely the next morning when they would be wanting to receive a message..

Agent availability concerns compute and grants. Human reachability concerns declared attention and carries narrower reach; a mooring never supplies that declaration.

**R10 — his words, unedited**
> humans have embodied natural constraints that are necessary to be intimately understood.. to respect those vulnerabilities.. elohim agents have a different nature of constraints... that seam needs to be clear

A person's constraints and an agent's are different in kind, and no output or model treats one as the other with different numbers (P11).

## 2. What exists on 2026-11-22

The branch baseline is `reliability/coord-writes-and-follow`. Built this day means the following source surfaces, not a claim here about execution, timing or acceptance.

| Surface | State and source |
|---|---|
| Declared write sets in `berth status` | Built: live local moorings print writes (`genesis/agentic/bin/berth:2631`) |
| `berth impact` with graph impact | Built: local and sibling declarations compose impact (`genesis/agentic/bin/berth:2553`); shared/upstream/downstream derive from direct ownership (`genesis/orchestrator/gate-impact.mjs:78`) |
| Sibling-device moorings | Built: reads the channel flow store, write sets and expiry (`genesis/agentic/bin/berth:2486`) |
| Device capacity | Built: seats, RAM and I/O guards and running builds determine the device's answer (`genesis/agentic/bin/berth:2380`) |
| Participation read | Not built: the existing context result composes intents, commitments, notes, habits, gates and reconciliation (`elohim/eprfs/epr-cli/src/flow/context.rs:127`); `--participate` is this proposal |
| Declared human presence | Not built; no declared-human-presence record exists today |
| Live upstream lines in the follow | Not built: reader follow emits notes and silence notices (`genesis/agentic/bin/berth:3099`); ordinary follow does the same (`genesis/agentic/bin/berth:3147`) |
| Carrier beyond this device's second seat | Not built: the carrier list contains only `secondSeat` (`genesis/orchestrator/gate-carry.mjs:395`) |

## 3. Rulings

**P1 — Capacity and coordination are different constraints.** Capacity belongs to the device
that senses it and may refuse through `berth room`, guard lines or host admission. Coordination only informs. No read introduced here refuses, waits or says wait. An underlying capacity
refusal is displayed with its owner as evidence about an act, never as refusal of the read.

**P2 — Three objects remain separate.** The A is a person or collective: identity from imagodei, standing from the space, slow and ceremonial, paid once per relationship. The envelope is a
workspace, device, ark envelope or runner: where work runs, its limits, `self` reach, fresh in
seconds. The elohim agent has a persistent layer of role, model lineage and grant resolving to an A, and an ephemeral layer of session, mooring and task behaving like the envelope.

| Plane | Relationship | Freshness | Custody | Reach | What the cost multiplies by |
|---|---|---|---|---|---|
| A | Person or collective within a space | Ceremony; slow | Person and admitting space | Space-governed standing | Relationships, paid once per relationship |
| Envelope | Work placed on a device | Seconds | Owning device | `self` | Concurrent work and resource demand |
| Agent, persistent | Role and model lineage acting under a grant for an A | Grant and lineage changes | Grant issuer and artifact steward | Grant's scope | Granted relationships and lineage changes |
| Agent, ephemeral | Session, mooring, task in an envelope | Seconds | Acting device | `self`; permitted sibling context | Active sessions and their compute |

These are distinct costs. Adding a session adds neither a person nor a fresh standing ceremony.

**P3 — Presence has two planes.** Agent presence is `running now`, `invocable` when there is
room and a grant, or `not available here`. Capacity and grant decide it; it costs compute. Human presence is `reachable now`, `reachable at a time they would welcome`, or
`not to be disturbed`. The person or their device under their policy declares it. Activity never implies it. It costs attention and has narrower reach because it reveals desk presence.
An absent but invocable agent is called, not waited for. A needed human decision is asked once now, or independent work continues with the ask ready for the time the person would welcome it.

Missing node: standing → **person-owned reachability declaration** → decision routing. Its home would be imagodei's person/device policy surface, projected locally for the permitted
reader. No schema is designed here. Until that node exists, human reachability is `unknown`.

**P4 — For whom is plural and role-bearing.** Stewards, gifters and receivers are each sets.
Who needs what is standing joined with presence at read time. No beneficiary field is stored, and no singular beneficiary is shown. Use `resolve` and `count` from the standing spec §3,
retaining uncounted reasons. This participation read is their second consumer. Today native standing reaches only one handle, not authenticated distinct membership in a space:
`AgentRef` receives `human:<handle>` (`elohim/epr-rea/src/participants.rs:1117`). Say so explicitly. A one-member role set stays a set; unresolved roles remain unknown, not manufactured identities.

**P5 — Enrich the A on the read side first.** Compose identity and standing when asked. Presence never becomes part of the A and is never written into a notarized record. This design
changes neither `AgentRef`, provider/receiver nor any wire shape. `Intent` names a desired flow and its raiser (`elohim/epr-rea/src/model.rs:79`); commitments and events retain provider/receiver
(`elohim/epr-rea/src/model.rs:352`, `elohim/epr-rea/src/model.rs:377`). These fields do not supply all three role sets by implication. Only recorded links justify membership in a role set.

`AgentRef` is documented as a Holochain hash (`elohim/epr-rea/src/model.rs:22`), while native standing returns a handle. A later, separately gated change must settle that mismatch after the
view is proven. epr-rea is published (`elohim/epr-rea/Cargo.toml:10`); that later step requires a version bump, dependency resolution file movement and an Edge roll. It is outside these cuts.

**P6 — One read composes existing reads and mints nothing.** The surface is `epr flow context . --participate [--json]`. Text and JSON carry the same distinctions and unknowns.

| Question | Reads composed | Meaning retained |
|---|---|---|
| Where can I participate? | Local `berth status`, folded sibling moorings, `berth impact`, artifact scope, honoured covenant write sets | Presence and graph relation are context; covenant scope remains authority |
| What can I do? | Device-owned `berth room cargo --json`, host admission for the artifact, grant references, reach and covenant scope | Capacity and grant are separate; unreadable capacity is unknown |
| How can I help? | Context's intents, commitments, habits and notes; station projection; landing intent state; recorded misses and carry evidence; ledger | Every source keeps its own open state and testimony |

Honoured covenant purpose exposes `writeSet` (`elohim/eprfs/epr-cli/src/covenant.rs:813`). Host admission observes the host and reads artifact requirements (`elohim/ark/cli/src/host.rs:43`).
Those reads inform fit; calling participation itself performs no admission or invocation act.

| Source | Open means |
|---|---|
| Station | Unticked intent; a tick is a claim, not fulfilment (`elohim/eprfs/epr-cli/src/flow/project.rs:1001`) |
| Commitment | Undischarged, possibly already owned (`elohim/eprfs/epr-cli/src/flow/context.rs:107`) |
| Habit | Declared red or unwired; retain any disagreement between declaration and evidence |
| Landing intent | Live until landed, withdrawn, dismissed or superseded (`elohim/eprfs/epr-cli/src/flow/intend.rs:162`) |
| Broadcast note | Testimony, never a claimable commitment |

A live landing does not imply recorded misses or available carry. Missing joins remain unknown.

**P7 — Every item carries its evidence envelope.** All fields come from recorded flows and explicit artifact links. The answer prints `unknown` when evidence does not establish a field.

| Field | Contract |
|---|---|
| Serves | Existing intent, artifact and habit references |
| For whom | Stewards, gifters, receivers as sets, with standing basis and unresolved reasons |
| Worth | Only recorded units: gate seconds and red mass, retaining source and time |
| What has been said | Claims, verdicts, prior attempts and testimony with their own authors |
| What taking it would record | Existing claim and fulfilment acts, habit references, or carry offer/outcome acts as applicable |

Gate seconds already have a local flow read (`genesis/orchestrator/gate-impact.mjs:92`); habit
rows carry red mass (`elohim/eprfs/epr-cli/src/flow/habits.rs:975`). A commitment's bound is a
ceiling (`elohim/epr-rea/src/model.rs:364`), never worth. Potential reuse seconds are not a saving.
Taking remains a separate authorized act; reading records nothing and promises nothing.

**P8 — Fit is filters and annotations with reasons.** Show reach, covenant write set, the
device's own capacity answer and shared/upstream/downstream graph relation. Compute no merit
number and no merit-based ordering. Do not reuse the actionable view's ordering. Preserve source
order within source groups. If nothing fits, say so and show visible exclusions and their reasons.
Apply visibility before composition; hidden records leak through neither counts nor explanations.

**P9 — Help needing an agent differs from help needing a person.** Agent help lists who is
running or invocable and where there is room. Human help lists stewards with standing and each
one's declared reachability. An item needing a person who is not reachable reads `ready for <when>`.
If no time is declared, it reads `ready for unknown — reachability undeclared`. It stays ready.

**P10 — Reconciliation is a read.** Join `epr flow ledger` with cost observations and explicit
grant references. Ledger already gathers claims, fulfilments and verdicts across their distinct
addresses (`elohim/eprfs/epr-cli/src/flow/ledger.rs:5`). Missing attribution stays missing.
No retrospective attribution is created, and no absent grant is supplied by a familiar handle.

**P11 — The seam between a person's constraints and an agent's is explicit.** It governs how
P2, P3 and P9 are read.

| | A person | An elohim agent |
|---|---|---|
| Nature of the constraint | Embodied: sleep, attention, health, family, a finite day | Compute, context, the budget and scope of its grant, the life of its session, no memory beyond what it writes down |
| Can it be parallelised, restarted or bought with capacity | No | Yes, within a grant and a device's room |
| What knowing it is | Intimate knowledge of a vulnerability | An operating fact |
| Who holds it | The person's own device, under their policy | The agent's session and its grant, readable where it works |
| What crosses to others | Only the effect: `reachable now`, `ready for <when>`. Never the reason | Its state and its limits, stated plainly |
| Why it is read | To protect: so work does not land on the person at the wrong time | To place work well, and so others know what it cannot be relied on for |
| Words used | reachability, rhythm | capacity, grant, budget, invocable |

- A person is never printed in capacity terms and is never offered as something to invoke.
- Nothing in this design raises throughput by spending a person's attention. Attention is spent
  last.
- The asymmetry runs one way. An agent absorbs what it can so the person is asked less: it
  waits, gathers its asks into one, does the independent work first and has the question ready.
  A person is not asked to work around an agent's limits.
- Silence from a person is the ordinary case and is never read as a fault. On 2026-11-22 a
  session's stall sensor prompted five notifications in about ten minutes for one silence while
  the steward was away; that is the failure this ruling names.
- How a person's rhythm comes to be known is theirs to declare and is held on their own device.
  This design builds none of it. It is the missing node of P3, and it stops there.

## 4. P2P design gate

The answer is class C: ephemeral composition reconstructed from permitted local source records.
Nothing is minted. Existing artifact CIDs name the evidence; the answer has no identity or head.
No entry type is added or reused for persistence, no coordinator returns a new address, and no
signal, route, SQLite table or Automerge projection is introduced. The DNA hash is unchanged.
Additional heads at birth and one year are zero; this answer adds no recurring quiesce work.

Alone, the reader answers from local evidence. Siblings enrich it without becoming prerequisites.
Transport is inapplicable to the answer. Existing peer signals remain within their admitted reach.
The human-presence declaration's class is open: private B versus reconstructable C depends on
its eventual authority and lifetime; any peer-verifiable effect needs a separate B2 assessment.
This design authorizes none of those declarations and keeps their schema outside its scope.

SDO/RWA boundary: a reader sees only evidence its space permits. No cross-space identity join,
activity inference or compelled attention is permitted. Unknown is safe to display without counts.

## 5. Cuts

1. **Local composition.** Write set: epr-cli flow dispatch, `flow/context.rs`, a participation
   reader and focused tests under `elohim/eprfs/epr-cli/`. Compose the read-side A view in Rust.
   Cargo is required; no published-crate change or Edge roll is required by this cut.
2. **Sibling context.** Write set: `genesis/agentic/bin/berth`, `genesis/agentic/berth_test.py`,
   `genesis/orchestrator/gate-impact.mjs` and its tests, and epr-cli participation tests.
   Carry freshness, graph provenance and optional upstream lines through the existing follow.
   Cargo is required for CLI changes; no published-crate change or Edge roll is required.
3. **Carry reconciliation.** Write set: epr-cli participation/ledger composition and tests,
   `genesis/orchestrator/gate-carry.mjs` and its contract tests for exposed admission evidence.
   Join recorded costs, grants and outcomes; keep absent carriers explicit. Cargo is required
   for CLI changes; no published-crate change or Edge roll is required. A new carrier is separate.

These cuts do not deploy anything. Any later reach boundary must evaluate its actual write set.

Validation protocol: exercise native readers and fakes with absent stores, owned commitments,
unticked stations, expired sibling declarations, missing grants, and plural unresolved roles.
Assert zero writes, equal text/JSON meaning, visible exclusion reasons and no hidden-record counts.
The store opener can create directories (`elohim/epr-rea/src/store.rs:210`); the composition must
avoid initializing absent stores. Feed preserved records to readers without invoking writing paths.
Test an invocable agent separately from a human ready tomorrow; neither case delays the answer.
Test follow replay for one upstream line per revision and no change to local contents or gate keys.

## 6. What it must not become

It must not assign or dispatch work automatically, compare participants on a leaderboard, nag at
session start, become a second backlog, or surveil a person's activity. The caller asks for context.
The answer never adopts incoming work, mints a claim, contacts a person or invokes an agent.

## 7. Open

Human-presence authority, classification, retention and permitted readers remain undetermined.
The standing contract's full native-to-DNA join remains its own work. Grant-to-role attribution
is unknown wherever the recorded links are absent. Sibling freshness and follow attachment need
explicit bounds; no latency is promised here. A Jenkins or sibling carrier needs its own contract
implementation. The later AgentRef correction remains separately gated after this view is proven.
