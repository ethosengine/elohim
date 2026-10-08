---
title: "Sensed, Never Gathered — How the Network Observes Itself Without an Aggregator"
id: observability-epic
status: Vision / proof epic (drafted 2026-10-08)
class: protocol-canonical
artifact_kind: companion-epic
sovereignty-frame: adversary
companion_to:
  - genesis/docs/content/elohim-protocol/observer-protocol.md
  - genesis/docs/content/elohim-protocol/hardware-providence-commons.md
cites:
  - "observer-protocol | the witness-not-surveillance epic whose Part VIII cannot / must / humans-control this epic turns on the network's observation of itself | path: genesis/docs/content/elohim-protocol/observer-protocol.md"
  - "observation-event-layer-design | the three-layer cut (Observation / Event / Attestation), the observer's log address, the diversity threshold and the eight-stage hard cutover this epic's witness plane is built on | path: genesis/docs/content/elohim-protocol/architecture/2026-05-11-observation-event-layer-design.md"
  - "attestation-consolidation-design | the consolidated, DNA-hash-neutral attestation kind that a graduated observation lands in; no new entry type | path: genesis/docs/content/elohim-protocol/architecture/2026-05-11-attestation-consolidation-design.md"
  - "hardware-providence-commons | the sibling proof epic: the one cybernetic loop, the operator's authority bands, and the refusal '.epr-meta is not the runtime telemetry store' that this epic answers | path: genesis/docs/content/elohim-protocol/hardware-providence-commons.md"
  - "private-thought-governed-fruit | the six hard boundaries, 'the dragnet is never the sensors; it is the aggregator behind them', and participation as a sensitive, design-quantity surface | path: genesis/docs/architecture/private-thought-governed-fruit.md"
  - "stewardship-over-sovereignty | the authority canon: every operator act bounded_by a Commitment; soft-warn before reach escalation; the operator is a steward, never an owner | path: genesis/docs/architecture/stewardship-over-sovereignty.md"
  - "trust-as-efficiency-signal | validation on demand rather than always-on; why an always-scraping aggregator is the expensive and wrong attractor | path: genesis/docs/content/elohim-protocol/architecture/trust-as-efficiency-signal.md"
  - "plane-separation-design | the ruled observation plane: no DHT write while unchanged, one record closes a period, nothing ships without a reader, a witness is diversity not a signature, posture split from liveness | path: genesis/docs/superpowers/specs/2026-10-03-plane-separation-design.md"
  - "mutual-storage-replication-dwelling-hub-design | intent-first, observed-state-second; capacity sensing reusing infrastructure:system-sample; the replication-shortfall observation | path: genesis/docs/superpowers/specs/2026-05-28-mutual-storage-replication-dwelling-hub-design.md"
  - "records-lifecycle-design | §D.5 the observation-tier prerequisite and the heartbeat retirement cutover; §D.8 bridges deliver vendor events as Observations | path: genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md"
  - "elohim-seam-map-concern-routing | the placement atlas this epic adds the observation / self-sense seam to, and whose crosswalk row it corrects | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md"
  - "elohim-protocol-specification | the wire specification whose Scope hands self-observation to this epic and whose Attestation record is the only crossing from evidence into content | path: genesis/docs/content/elohim-protocol/protocol-specification.md"
---

# Sensed, Never Gathered — How the Network Observes Itself Without an Aggregator

_A companion to the Observer Protocol and to Hardware Providence._

_New to the protocol's vocabulary? The [glossary](epr:glossary) defines the
load-bearing terms (elohim, reach, standing, attestation, conductor) so any
document can be read on its own. §1.2 below defines the few this epic adds._

The [Observer Protocol](epr:observer-protocol) says how a household may be
seen: as a witness, never as a security camera, by eyes that serve the people
they watch rather than anyone watching from elsewhere. The [Hardware Providence
epic](epr:hardware-providence-commons) says how a rack cares for itself and
refuses to let `.epr-meta` or the DHT become its telemetry store. This epic
asks the question those two leave open:

> **What must the network observably do before the Elohim Protocol may claim
> that it sees its own health — every peer, every household, every holon and
> the commons — without any place where everyone can be seen?**

Every decentralized system re-centralizes at the same three points: the scrape
target, the log sink and the dashboard. The nodes are many; the place that
knows what all of them are doing is one, and whoever holds it holds the
network. The sentence that organizes everything below comes from
[Private Thought, Governed Fruit](epr:private-thought-governed-fruit): **the
dragnet is never the sensors; it is the aggregator behind them.** Sensing is
not the danger. The danger is where the aggregate is allowed to exist. This
epic is about that place: where it may exist, who holds it, what crosses out of
it, and what is refused.

---

## 1. The Claim, Stated Carefully

The network observes itself the way a body does: locally, continuously, and
without a central organ that watches all the others. A peer senses itself. A
sibling that stands in a declared relationship with it signs what it saw. When
enough distinct witnesses agree that something changed, that agreement, and
only that, becomes a record the community can hold. Every aggregate above a
single peer is held by the holon it describes and never leaves it as rows. The
elohim-operator, a software steward tending a household's machines, carries the
sensing, the folding, the signing and the first escalation. The household
depends on the result without reading a dashboard.

That is the claim. It is stated against the standing condition the protocol
assumes: hostile agent swarms will surveil, penetrate and battle-test the
network, identities are free to mint, and the following many can be synthetic.
No boundary below rests on counting observers; it rests on their declared
relationships and on what each one is allowed to hold.

### 1.1 Self-sense, witness, fruit

Three words name the three records this epic is built from. They must stay
apart, because the whole discipline is the refusal to let one pass for another.

- **Self-sense** is what a peer knows about itself and keeps: its process
  state, its memory, its queue depths, the gauges it serves on its own port.
  It is honest and it is cheap, and it is evidence to nobody else.
- **A witness** is what a sibling signs about what it saw of a peer: that it
  answered, that it did not, how long it took. A witness carries the
  observer's signature and lives on the observer's own log. It is the only
  record that crosses from one peer to another, and it crosses only as far as
  the kind that produced it declares.
- **Fruit** is a crossed threshold: enough distinct witnesses, in one window,
  agreeing that a status changed. Fruit is the only thing that becomes an
  attestation, the only thing the commons may roll up, and the only thing the
  notary ever holds.

A **self-report** is self-sense dressed as a witness: a peer announcing its own
health as if a sibling had seen it. The [plane-separation ruling](epr:plane-separation-design)
already names it ("a lone daily digest is a self-report"). This epic refuses it
everywhere: a self-report graduates to nothing.

### 1.2 Words this epic uses

- **elohim**: the protocol's local AI agents, one per person or household,
  acting under the constitution.
- **elohim-operator**: the elohim that tends a household's hardware. A
  software steward, bounded by commitments; never the owner of the rack, and
  not the human "node operator" who runs it.
- **holon**: a space that holds its own activity ledger: a person, a
  household, a collective, a region. Each holon sees inside itself; nothing
  outside it may join its ledger to another's.
- **reach**: how far a thing travels, in eight rings from `private` to
  `commons`. Earned, never given.
- **observation**: one signed row on an observer's log: who saw, what, of
  whom, when, under which declared kind.
- **attestation**: a signed claim by an issuer about a subject, kept on the
  notary. The protocol's one crossing from evidence into content.
- **commitment**: a promise with a declared bound. Every act by an operator
  names the commitment that bounds it.
- **the notary**: the Holochain DHT. It holds proofs (who, what, when), never
  telemetry.
- **algedonic signal**: a pain or pleasure signal that bypasses the normal
  reporting hierarchy and reaches the responsible person directly. The word is
  Stafford Beer's, from his model of viable systems; here it is the typed
  signal a holon raises when a stock approaches or breaches a declared bound.

Implementation terms the body leans on, defined once:

- **conductor**: the Holochain runtime process on a device. It holds the
  person's keys and source chains and talks to the network. It sits beside
  the **storage peer**, the protocol's own process that holds and serves
  data; a peer, in this epic, is the pair.
- **DNA, zome, DNA hash**: a DNA is a Holochain application's rule set,
  packaged as modules called zomes; the *integrity* zomes define entry types
  and validation and the *coordinator* zomes define functions. The DNA hash
  covers the integrity rules; changing them makes a new, separate network.
- **source chain**: each agent's own tamper-evident, append-only log of its
  actions on a conductor.
- **EPR, the EPR crate**: an Elohim Protocol Record, the protocol's record
  type, in which knowledge, value and governance travel together as one
  reference. The crate (`elohim-epr`) is the Rust library that defines the
  record, its kinds, its reach, its witness and measure types.
- **doorway**: the gateway service that lets a browser reach the network: it
  proxies to a conductor, caches, and hosts the keys of people who have no
  device of their own yet. It sits beside the peers it serves and is a
  projection of them, never an authority over them.
- **CID**: a content identifier, the hash-derived address of a piece of
  content. Two peers holding the same bytes compute the same CID.
- **iroh**: one of the two peer-to-peer transports the storage peer speaks.
  The observer's log is addressed through it: `iroh://<observer>@<log>#<offset>`
  names one row on one observer's log.
- **view federation**: the carrier that lets a region read summaries across
  separate networks without any of them exporting rows.
- **re-quilt**: redistribute replicated shards across the surviving peers so
  that the declared number of copies holds.
- **measure folds, brit git notes, EPR envelope**: three ways the development
  system already records what it observed about itself: a fold is one
  observation written through the measure tooling; a brit note is a signed
  attestation kept in git; an envelope is the EPR record's header. The body
  names them only to say that nothing joins them yet.
- **a standing compute grant**: the commitment a household steward (the
  person who runs the household's node) issues to the operator agent, naming
  what it may do and at what rate.

---

## 2. Why Self-Observation Is the Commons Proof

A hyperscaler watches its fleet from one place because it owns the fleet. The
protocol has no such place because nobody owns the network, and the moment one
is built, somebody does. Monitoring is therefore the quietest way a
decentralized system fails: the peers stay plural, the dashboard becomes
singular, and the operator of the dashboard becomes the party every peer
depends on to know whether it is well.

The forcing function here is the same one the rack provides for hardware: it
cannot be faked. Either a peer that dies is noticed by its siblings, or it is
not. Either a household can learn that its doorway is unreachable without
asking a company's server, or it cannot. Either the region's view of coverage
is computed from records the holons chose to release, or it is computed by
joining rows the holons never saw leave.

Holochain core's own observability shows the shape of the gap. Its metrics
crate was built to feed a load-test bench (Wind Tunnel: scenario binaries,
InfluxDB dashboards, a weekly run on a scheduler, torn down afterwards). That
is a developer observing a conductor under synthetic load. It is excellent at
what it is for, and it is not a network observing itself by consent. The
fleet this protocol runs on a Kubernetes test bench with a Prometheus scraper
is the same thing one notch over: an operator observing their own holon. Both
are legitimate as **self-sense of one holon**. Neither is the aggregate this
epic refuses to let exist, until the moment their series are read as the
network's truth, fed into placement or standing, or joined across holons.

---

## 3. P2P Design Gate: Five Planes, Three Records, Two Holdings

This epic mints no entry type, no table, no route and no protocol id. It
classifies what already exists and binds each plane to the primitive that
carries it. The protocol classifies every data entity by where its truth
lives: Notarized (A) on the DHT, Linked (A2) as a link on something notarized,
Private (B) on one agent's own chain, Attested-Private (B2) private with a
notarized proof of its effect, or Ephemeral (C) rebuilt on read. Five planes: three records (self-sense, witness, fruit) and two
holdings (the holon pool, the commons rollup).

| Plane | Class | What it is | Carried by |
|---|---|---|---|
| **Self-sense** | Ephemeral (C), agent-private | A peer's own instruments, rebuilt on every read | Process state; the `/metrics` text a peer serves on its own port; folded into the `infrastructure:system-sample` observation kind; `Quantity{Confidence}` for anything that becomes a number |
| **Witness** | Attested-Private (B2) on the observer's signed log | A sibling's signed row about what it saw | `EprKind::Observation` and `WitnessedInteraction` in the EPR crate; one append-only log per observer; addressed as `iroh://<observer>@<log>#<offset>`; kind declared in the pillar manifest with reach, retention, diversity threshold and `graduates_to` |
| **Fruit** | Notarized (A) on the consolidated attestation kind | One record that closes a period or marks a transition | The elohim DNA's `attestation:*` content kind (`attestation:doorway-health-summary` is already declared); diversity-threshold graduation; transition-only; no DNA hash moves |
| **Holon pool** | Ephemeral (C) projection, held inside the holon | The holon's own read over its witnesses | The hub role's projection over the rows its members' kinds allow it to hold; rebuilt from the logs; whether a pool is ever a party in its own right is held in the commons-pool design |
| **Commons rollup** | Ephemeral (C) over fruit only | Coverage and diversity at region scale | `CoverageRollup` (flat and recursive) across view federation, consuming attestation CIDs and counts of the unobserved, never rows |

**Design constraints the gate discovered**, each already ruled elsewhere and
restated here so the epic holds them together:

- The notary holds fruit, never telemetry. A peer's liveness is not a notary
  fact (plane separation §5).
- Posture is a declaration written on change. Liveness is substrate presence,
  armed by the transport, with a short TTL. They are split, and the order of
  the split is binding: arm presence, move the readers, then change the
  writer.
- A witness is diversity, not a signature. One signed row is one observer; a
  threshold needs distinct households and, for some kinds, distinct regions.
- One record closes each period. Silence after `period_end` reads
  "unobserved", which is the honest-absence answer. Absence is never health.
- Nothing ships without a reader. A record that nothing reads gives honest
  absence to nobody; a timer that writes it anyway breaks the promise that an
  idle network is free.

**A reading of the plane's costs, once.** The protocol reads any costly path
through a fixed set of questions (the *gradient reading*: what mechanism, who
stands at each end, what kind of cost, how widely held, which plane, what unit,
what phase, whose lane). Asked of the observation plane as it stands today, the
answers are:

1. **Mechanism.** First, telemetry that cannot be attributed: probes that lie,
   including a free-storage check that reads "100%" when it fails. Then idle
   writes to the notary. Then observability as its own load: a log sink
   saturated by the logs about it.
2. **Who stands at each end.** Self-sense is a peer and itself. A witness
   within a holon is a declared relationship. Graduation is earned by a
   diversity of attested observers. The commons reader is a stranger. Today
   only the two ends are priced: liveness is either a peer's own gauge or a
   notary write, and nothing in between is cheaper for being trusted.
3. **Kind of cost.** Verification paid where none was needed (a peer notarizing
   its own liveness). Waiting (a status read stale by two to fifteen minutes).
   Blindness (absence read as health). And cost owed unconditionally
   regardless of trust: the saturated log sink, histogram buckets that make
   every quantile unusable, and status calls holding the conductor's admission
   permits for most of a minute while a person's own write waited.
4. **How widely held.** Every peer's status, on the notary, held by everyone;
   where it should be held by the holon, with independence observed through
   the diversity tags at graduation.
5. **Which plane.** Attention's work is paying on the notary's paths. The
   planes are fused, and that fusion is the first thing to read.
6. **Unit.** Per peer per tick, where it should be once per transition.
7. **Phase.** Steady state writes constantly; only transitions should write.
8. **Lane.** Background work standing in the interactive lane: the admission
   permits above were held by status calls while a person waited.

Trust is paid twice wherever the same fact is derived several ways. The
ceremony that may never be compressed, whatever the trust: the witness
signature, the diversity threshold, the commitment that bounds every operator
act, the soft warning before a kind's reach widens, and honest absence.

---

## 4. One Cybernetic Loop

The loop is the one [Hardware Providence](epr:hardware-providence-commons)
states, read for observation:

```text
observe → interpret → compare with commitment → form intent
   → obtain authority → commit resources → act → re-observe
   → attest effect → settle value → learn
```

Observe is sense. Interpret is fold, with a confidence that says what kind of
claim this is and why an interval is unknown. Compare is against a declared
bound on a commitment. Attest effect is graduation. Learn is a widened
interval, a new kind, or a corrected threshold. A loop is not closed because a
gauge moved, a signal fired or a restart was sent. It closes when a new
observation proves the required condition holds, or proves it did not and the
next path has begun.

### The operator's authority bands, applied to observation

The operator's acts on this plane are six: observe, retain, aggregate,
disclose, escalate, and actuate on what it observed. The four bands of the
hardware epic apply to each.

| Band | Observation acts |
|---|---|
| **Autonomous and reversible** | Sense itself. Fold its own scrape. Sign a witness of its own holon's infrastructure under a declared kind, at that kind's reach. Compute the holon pool. Raise an *approach* signal. Widen an interval. Mark a period unobserved. Restart a bounded service, re-quilt, throttle, fail over, within declared limits, journaling and verifying. |
| **Pre-authorized commitment** | Observe a sibling peer under a replication or collective commitment. Keep rows past their class only because an issued attestation cites them. Issue the graduated attestation when the threshold is met. Roll fruit into the commons at the kind's reach. Escalate a *breach* to the steward of the commitment's other party. |
| **Explicit assent** | Expose a raw agent-private log. Widen a kind's reach (through the soft-warn ceremony). Retain past class for any other reason. Export a pool outside its holon. Bind a transport identity to an agent. Take a peer offline, erase, exceed a budget. |
| **Prohibited (substrate refusal)** | Observe a person's activity kinds at all. Observe without a declared kind. Fabricate a measurement. Read unmeasured as zero or as healthy. Leak raw intimate evidence. Join two holons' ledgers. Carry a participant label on any series that leaves the holon. Gate a decision on an instrument. Silence a contesting witness. Hide a signal, or route it to a dashboard before the person. Actuate on another holon's hardware. Let an alert stand in for an attestation. |

**What the operator may read**: its own self-sense; the witness rows of its
own holon's infrastructure kinds, filtered by its holon; rows that reached it
from other holons only through the diversity view, as counts, for the
graduation a kind declares; fruit from the commons; never a person-kind, not
even on its own hub.

**What the operator may write**: its own observations, signed with the
peer's observer identity, with a pre-observation and a post-observation
bracketing every actuation; an attestation of effect, as a subtype of the
consolidated attestation kind, whose evidence names the pre and post rows and
whose verdict is `held`, `failed` or `unobserved`; algedonic signals for
commitments its holon is party to. Never a notarized status for a peer or a
period it did not itself observe. The commitment it acts under is the standing
compute grant its household steward issued; the operator's verbs are
scoped by that grant, and disclosure is never a standing scope but an act of
assent each time.

### Capture is a stewardship relationship

Whoever holds a collector is a steward of what it captures. That sentence
turns observability from a plumbing choice into the protocol's own vocabulary,
and it names what is missing. Today the cluster's Prometheus holds per-call
series about every hosted account on the fleet with no authority basis, no
scope, no expiry, no review and nobody the subject could appeal to. Those
accounts are still test fixtures, so no person is yet exposed; the series
would not change shape on the day one is. Nobody lied about it; nobody
declared it. A hosting operator holding the same series
off a port is in the same position. The exporter made it worse only by making
it easy.

The protocol already has the shape for declaring it. A stewardship grant in
the identity pillar carries a steward and a subject, an authority basis with
evidence and a verifier, a set of capabilities one of which is literally
activity monitoring, a mandatory expiry and review date, a delegation depth,
and an appeal. What it lacks is **scope**: which observation kinds, which
labels, at which reach, retained for how long. That scope is the one addition
this plane asks of the identity pillar, and it rides the consolidated
stewardship-grant attestation or a link on the grant, never a new entry type.

With scope declared, "observability scales with relationship" becomes
mechanical. It is the trust gradient applied to capture:

- **A peer and itself.** It reads its own instruments. No grant, no cost,
  nothing leaves the host.
- **A declared steward of capture.** A hub for its members, a doorway for its
  hosted participants, an operator for the peers it runs, a host for the ports
  it hosts. The grant names the scope, the reach and the expiry; the subject
  sees it in their own activity log and may appeal it; the series carry only
  the labels the scope allows.
- **Earned standing.** Cross-holon aggregation over attested fruit, with the
  diversity threshold as the standing that was earned.
- **Strangers.** Nothing. An unscraped port. Absence is honest.

Read this way, the fleet's Prometheus stops being a violation and becomes a
declared steward: the operator of the neighbourhood holds a capture grant over
the peers they run, at community reach, scoped to machine kinds and never to a
participant label, reviewed on a date. That is one grant and one label filter
away from where the fleet stands today, and it is the same move a hosting
operator would make per host. What no load-test bench could ever surface is
exactly this, because a bench has no subjects; a fleet with hosted people
does.

Today the operator's reconcile verb records that it tried before it acts, and
nothing records whether the act held. That is exactly the "action handler
reporting that it tried" the hardware epic forbids. The loop closes when the
verb carries its pre-observation, its intended postcondition, the
post-observation appended on completion, and an effect row whose verdict is
`unobserved` whenever the loop ended without one.

---

## 5. The First Proving Story: A Peer Goes Quiet

A storage peer in a household of three loses power. It does not write that it
left; nothing that dies can. Within one presence window its two siblings
notice that its substrate presence lapsed. Each signs a witness row: *this
peer, unreachable, at this time*, under the household's infrastructure kind,
at household reach. Neither row goes to the notary.

The household's hub, which is one of the two surviving peers wearing a role,
folds its own pool: one period opened, one peer unobserved. The operator on
that hub compares against the household's replication commitment, finds the
margin below its bound, and raises an approach signal to the household's
person, on the device that felt it, before the margin is breached. The person
sees one line: a device needs attention, and the household's copies are still
safe for a stated time. They are not shown a graph.

The operator fails over the peer's shards within its autonomous band, journals
the act, and re-observes: the copies are where it said. It writes a pre row,
a post row, and an effect attestation whose evidence names both. When the
peer returns, its siblings sign that it answered, the period closes, and one
record of the transition is graduated, because the kind's threshold was met by
distinct observers. The doorway that serves this household's guests reads the
closed period through its reader and adjusts what it promises. The region,
much later, sees that one household in a hundred had a period with one
unobserved peer, as a count.

At no point was there a place that knew all of this at once. The aggregator
was never consulted because there is not one.

**Specializations** the same story must survive: a store growing at idle while
nothing happens (the writers are the problem, not the peers); an admission
gate stuck shut on a doorway (the breaker opens, counts a reason, and the
signal reaches the person before the watchdog restarts the pod); a label leak
(a scrape that names a participant is refused at the sensor, not caught at
the dashboard); a fixture graduating (a seeded observation becomes a fixture
attestation and nothing that settles).

---

## 6. The Ergonomics of Scale

Attention does not compose. The habit register that governs this repository
allows at most two habits to be active at once, and gives the reason in one
line: it bounds attention, one operator, one day job, and attention does not
compose. The same law bounds what an elohim may surface to a household, a
hub, a collective or a region. A person at any rung is shown at most two open
concerns. The rest is held by the operator and surfaces as a count.

Donella Meadows gives the second law. A system whose response time exceeds its
respite time is out of control, and the only cures are to quicken the response
or slow the disturbance; "try harder" is not a lever. So every rung is
designed so that nothing a person is asked to attend to has a respite shorter
than their natural return cadence. Everything faster is closed by the
operator or does not exist at that rung. *Approach* signals exist for this
reason: a breach alert is already late.

| Rung | What the person sees | What they are asked to do | Respite | Who closes the loop | Never shown |
|---|---|---|---|---|---|
| **A household, one device** (a phone with a hosted key, or a laptop with its own peer) | One line per device: well, needs you, or unobserved for N periods. For each thing they published: pending, elected or lost. Whether a wait is a wait, and on what. | Explicit-assent acts only: expose private diagnostics, take offline, exceed a budget, erase. | Until the person next opens the app. Hours to days. Anything shorter is closed without them. | The device's operator agent, or for a hosted key the doorway's operator, which can see what passes through it and must say so. Seconds to minutes; reversible acts only. | Raw series. Histograms. The doorway's metrics about them. Anyone else's anything. |
| **A dwelling hub** (a family node) | A household card: N devices well, needing attention, or unobserved. The hub's own commitments met or not. Coverage time of disk and copies. Open pain addressed to household commitments. | Pre-authorized acts to inspect or revoke (a replacement purchase). Assent as above. Who may join. | Capacity measured as coverage time: how long the stock lasts at the drain. A failing disk's respite is the coverage until the last copy is at risk. | The hub's elohim-operator within minutes: re-quilt, evacuate, throttle. The steward for assent. Peer hubs for mutual replication. A response slower than the respite is itself a breach. | Members' attention logs. The raw system samples. Zome-call series. Any per-agent anything. |
| **A collective hub** (several dwellings in a pool) | Households observed and unobserved this window, as counts. Collective commitments met or not. Open pain. The diversity summary: distinct households, distinct regions. | Membership and assent. The dispute procedure for a contested attestation. | Pool coverage time. Reciprocity grace. The margin of holders that can fail before loss. | Each member hub's operator, on its own hardware. The collective's audit sweep names imbalance; it never actuates a member's hardware. | Which household observed what. Any cross-member join. Inside the holon, members see each other's names by design, never each other's rows. |
| **A region, the commons** | Holons declared, observed, unobserved. Coverage ratios computed over attestations. Algedonic signals on commons-reach commitments. Release adoption health. | Governance acts, on cadence. | Weeks: soak windows, adoption discipline. | Stewards, through governance. The operator at this rung only rolls up. | Anything below the collective's own identity. Any household. Any person. |

**The rule for crossing a rung.** What crosses is fruit and a count, never
rows. Fruit is an attestation of a transition, graduated by the diversity
threshold the kind declares, or an algedonic signal addressed to a commitment
whose reach includes the receiving rung. The count is the number of periods
or holons that closed unobserved: the honest denominator, reported as
"unobserved", never as healthy and never omitted. Never raw series, never
per-member rows, never a join across two holons, never a histogram.

**The person is never the operator.** No screen at any rung asks a person to
read a series, acknowledge an alert or restart a thing. The person is asked
for assent or told an outcome with its evidence. The operator at each rung is
the only reader of rows, and only its own holon's.

In Stafford Beer's model of viable systems, self-observation is System 3*,
the sporadic direct-inspection channel that checks whether the reporting
hierarchy's own reports are true, and the algedonic signal is the pain channel
that bypasses the hierarchy altogether. The
reading this protocol has already given itself applies: the channel today is
mediated by an elohim at every hop, and every mediation attenuates. The
un-attenuated path is the person's own device, which is why the first
receiver of every signal is the person who felt it. And the claim to requisite
variety is not made: the operator is a high-variety attenuator on the
person's behalf, not a container for the person.

---

## 7. Subsuming Prometheus

The hardware epic calls the protocol's relation to the commanding heights,
the centralized infrastructure incumbents whose services it gradually takes
over, a negotiated subsumption, not conquest. The same stance applies to the fleet's
own monitoring stack. Prometheus is five jobs in one process: scrape, store,
rules, alert routing, and federation. Each has a protocol-native home. The
steps below take the jobs over in order, and each step ends with Prometheus
still running and doing less.

1. **Label hygiene at the sensor.** The conductor's instruments label every
   zome call with the calling agent's public key and every emitted signal with
   its cell. Any exposition, Influx or Prometheus, ships those labels. They are
   removed at the instrument, not hashed (a stable hash of a key is a
   pseudonym, linkable over time). What remains (`dna_hash`, `zome`, `fn`) is
   enumerable from the installed DNAs. A render test refuses any label key
   that names a participant.
2. **The peer folds its own scrape into an observation.** The storage peer
   samples itself at the cadence it already reports capacity, reads its own
   conductor on loopback, and appends an agent-private `system-sample` row to
   its own log. The gauges it serves become a render of that row. There is one
   sampler per peer. What a sibling may know of this peer's capacity comes
   from the peer's pledge, never from its sample.
3. **Siblings sign what they see.** The doorway's periodic probe of its peers
   stops writing an attestation per probe and writes a witness row per probe,
   at community reach, under the kind the manifest already declares with its
   threshold. A peer-observes-peer kind takes the same shape. Every row is
   signed, or it says it is not, and an unsigned row counts toward no
   threshold.
4. **Graduation replaces rules; the algedonic channel replaces the
   alertmanager.** The graduation evaluator ticks. A transition that meets its
   threshold becomes one attestation. The operator compares self-sense against
   the bounds its commitments declare and raises approach and breach signals,
   with a reset and a hold time so a signal can clear, and with a floor
   breach for absence: a period that closed with no observation. The alert
   rules the fleet runs today are kept as a shadow until every alert they
   would have fired was preceded by a signal that reached the holon first.
5. **Each holon holds its own pool.** The hub role owns the rows at household
   and community reach; placement and the resilience card read posture and
   substrate presence; the mutuality audit runs. The heartbeat that wrote a
   peer's status to the notary every minute is split, in the binding order,
   into a posture written on change and a presence the transport arms. Five
   liveness writers become two. An idle household makes zero notary writes.
6. **The commons rolls up fruit.** Coverage at region scale is computed over
   attestations and counts of the unobserved, across view federation, and
   nothing new enters the notary.
7. **The inversion.** The storage peer renders its holon's projection as
   series: open signals, coverage, unobserved counts. The fleet's Prometheus
   reads that, and only that. The conductor is no longer a scrape target. The
   alert rules are deleted. Cross-holon panels exist only over the region's
   rollup, served by the hub, because Prometheus joining per-peer series
   across holons *is* the aggregator behind the sensors. The fleet loses
   per-agent latency drill-down. That loss is the design.

Steps one through four need no new entry type and move no DNA hash. Step five
needs the signing gap closed: today a witness row is accepted without a
signature and the stream merely says so (§11), and a holon's pool can compute
its diversity threshold only over rows whose observer is verified. Only step
seven retires anything.

**On a bench the `agent` label is a fixture; on a hosting fleet it is a
participant.** That sentence is the whole answer to whether a Prometheus
exposition belongs in Holochain core: yes, as self-sense served to the host,
with the labels fixed at the sensor; and no, never as the thing a fleet
operator scrapes off a per-person conductor, because on such a conductor the
exposition itself is the participant.

---

## 8. The Holochain-Core Integrity Boundary

Observation adds nothing to what the conductor is, and the protocol promises
Holochain core that it never will. Seven promises, written to travel with the
exporter contribution:

1. **No instrument gates a zome call.** Recording is fire-and-forget; a
   metrics failure never errors or delays the call.
2. **No observation is a notary fact.** The metrics crate writes nothing to a
   source chain or the DHT. A conductor's self-sense is process state.
3. **No per-agent label leaves the process.** `agent` and `cell_id` are
   removed at the instrument. Every remaining label is enumerable from the
   installed DNAs; a label whose value set grows with participants or with
   history is refused in review. Cardinality is bounded by closed
   vocabularies, not by hope.
4. **The exposition binds only when asked, and only where asked.** No port
   opens unless the environment names one; the documented example is
   loopback; it is pull-only, one route, and a bind failure is logged while
   the conductor continues. The missing series is the signal.
5. **The admin API dumps remain the operator's.** They are read over the
   admin socket by the co-located peer, never rendered by the exporter, never
   gossiped.
6. **A conductor's observations ride the storage peer's observer identity.**
   When the co-located peer folds the conductor's self-sense into its log, the
   row is signed by the peer's observer key, never by a hosted agent's key.
   The conductor signs nothing.
7. **Evidence is never authority.** No integrity zome, validation rule or
   coordinator reads a metric, a health attestation or a rollup to permit or
   refuse anything. Health changes placement and temperature, never
   membership or standing. An observation kind may graduate to an attestation
   or a summary event, never to a commitment, a delegation or a reach grant.
   Every per-decision counter carries a typed reason drawn from a closed
   alphabet, which is by construction never an identity.

Two things the protocol would welcome from core and does not require:
histogram bucket bounds that make zome-call quantiles usable, and a changelog
entry for the metrics crate.

---

## 9. The Six Hard Boundaries, Restated for Observation

[Private Thought, Governed Fruit](epr:private-thought-governed-fruit) §4 sets
six boundaries no design may cross. Each has an observation reading.

1. **Thoughts never enter a notarized plane.** Attention and every agent-private
   self-sense kind never graduate. Only crossed thresholds land, and an
   observer's log root is not an entry on the notary.
2. **Activity ledgers are held by the holon they describe.** The pool is
   computed inside the holon. The rollup consumes attestations only. No
   doorway projection joins two pools; projections declare a retention floor
   and are keyed per space.
3. **Counsel context is the participant's.** Observations a counsel makes for
   a person are theirs to erase, through the forget request that becomes a
   forget decision, never by unilateral pruning.
4. **Counter-evidence is floor-protected.** A contested-evidence referral or a
   correction against an attestation always reaches its subject, and no
   rollup can suppress it.
5. **Correlation across identity namespaces is a consent act.** A transport
   peer id, an iroh node id and an agent key are never joined by an
   aggregator. The conductor's `agent` label on a per-call series is exactly
   that join, and it is removed at the sensor.
6. **Participation itself is sensitive.** The number of series that leave a
   holon, their label cardinality and the gossip metadata per kind are
   declared quantities with ceilings. Scrape endpoints bind to the holon's
   own network. At intimate and trusted reaches, kinds are agent-private or
   household, and an outside observer learns nothing from the observation
   plane about who participates, in which collectives, when, or with whom.

---

## 10. Proof Obligations

This epic becomes an acceptance contract when these obligations have
repeatable evidence.

| Obligation | Required proof |
|---|---|
| Sense honestly | Every quantity is three-valued: measured, unmeasured, or unknown with a reason on the wire. Unmeasured never reads as zero. |
| Label at the sensor | A test over the rendered scrape text of every exposition finds no label that names a participant. |
| Witness with diversity | A threshold is met across distinct households (and regions where declared), never by sample count from one observer. |
| Graduate, do not alert | One record closes a period; the per-tick notary write for a heartbeat is gone; a transition is recorded once. |
| Hold per holon | A pool is never exported; inspection of the federation payload shows attestations and counts, never rows. |
| Roll up fruit only | Every input to the region's rollup is an attestation CID or an unobserved count. |
| Bound the surveillable surface | Series count, label cardinality and gossip metadata per kind are measured against declared ceilings. |
| Serve the operator, not the dashboard | Every operator decision has a counted reason and a post-action re-observation; a verb with no post row is listed as unverified, never as success. |
| Fail safe under absence | An absent series or a failed probe fails closed; it is never read as free, healthy or present. |
| Forget on request | A redacted log root round-trips through the forget flow and the pool rebuilds without it. |
| Stay idle-free | Observation cannot become its own load: an idle household makes no notary writes and the log sink is not saturated by the logs about it. |
| Keep fixtures out of settlement | An evidence class rides every row and attestation and only descends; a fixture never settles. |

The standard of evidence is the household triad first, then the neighbourhood
fleet. The harness's `observability` capability is reserved for the one kind
of rung that queries the fleet's own aggregator, and it is used only to prove
that the aggregator holds nothing it should not.

---

## 11. Honest Build State

This epic is a target and a proof ladder, not a claim that the target ships.

### Live foundations

- A peer serves its own self-sense and the operator's runtime surface is
  commitment-gated (`operator-runtime-surface`, green). The storage peer's own
  exposition carries no participant label.
- Attention is an agent-private observation, never gossiped, never joined
  from outside (`attention-witnessed-privately`, green).
- Measures are three-valued and never show absence as zero
  (`measure-honesty-local`, green).
- The EPR crate carries `Observation`, `WitnessedInteraction` and
  `AttentionTending` kinds, and a quantity type whose confidence says what
  kind of claim it is and why an interval is unknown.
- The infrastructure manifest declares `system-sample` as agent-private and
  `doorway-heartbeat` with a threshold of three distinct households and five
  observations in an hour, graduating to a health-summary attestation that
  the integrity zome already admits.
- Coverage rollup exists, flat and recursive, and a cross-DHT carrier for it.
- Every per-decision reason already has a compile shape, and the trust
  pricer and freshness verdicts implement it.
- The build and release plane observes itself in signed git notes, sprint
  reports and measure folds.

### Partial or designed

- The observation log: declared with its own five-rung reach ladder (a
  separate vocabulary from the content's eight rings, joined only by
  graduation), only the agent-private rung enforced in code, rows accepted
  unsigned and the stream saying so.
- The graduation evaluator: written, referenced by nothing, with the
  attestation reader for closed periods still a stub.
- The algedonic signal: schema and latch exist, with no reset, no floor, no
  time dimension, no notary kind and no production emitter.
- Three observation channels that nothing joins: the measure folds, the
  observation log, and the brit git notes. None is an EPR envelope.
- The reach vocabularies: the observation ladder (five rungs) and the content
  reach (eight rings) are bridged only by graduation, and content reach is
  enforced on one of five planes.

### Open frontier

- The holon pool as a projection; the commons rollup over attestations only.
- The capture grant's scope: kinds, labels, reach and retention on a
  stewardship grant, so every collector has a declared steward, an expiry and
  an appeal.
- The evidence class on every row and attestation.
- Label hygiene in the conductor fork, and the fleet's Prometheus as a reader
  of the operator's projection.
- Witness-ledger rotation.
- The red habits this plane feeds: a peer's death witnessed by its siblings;
  an idle network that is free (today an idle household projects tens of
  thousands of actions a day against a declared ceiling of five thousand, the
  doorway's registration heartbeat the larger writer); zome-call cost bounded
  by a declared ceiling; runtime performance read without mistaking absence
  for health; a measure that runs on a peer.

### The sharp edges

- A lone doorway's probe becomes a notary attestation with no diversity: a
  self-report graduated as fruit.
- The free-storage probe defaults to "100%" when it fails: absence read as
  health.
- The quiesce preflight reads an absent, unmeasured series as a pass.
- The conductor's histogram buckets make every quantile unusable.
- A peer's liveness is derived five ways at four cadences, and notarized
  twice.

---

## 12. Demonstration Ladder

The epic graduates through observable stories.

**Household, one device**

1. An unmeasured quantity prints `not-yet-instrumented`, never zero, from one
   laptop with nothing else reachable.
2. The node at rest is quiet, and its scrape text carries no participant
   label.

**Household triad**

3. A peer's death is witnessed by its siblings, not announced by itself; three
   observers hold one witness.

**Neighbourhood**

4. Three households graduate one doorway-health attestation, and the
   heartbeat's notary write retires in the same cutover.
5. An approach signal arrives before a breach, and the respite is measured to
   exceed the response.

**Commons**

6. The region's rollup is computed from attestations only, and an auditor
   re-fetches every cited observation by its log address.
7. The fleet's Prometheus reads the operator's projection, and a scenario
   that queries it finds no identity label in it.

**Exit**

8. Delete Prometheus, Loki and Grafana. Nothing a household depends on
   changes.

Each rung has, or will have, an a2o acceptance scenario (the protocol's
behaviour-driven acceptance stories, written as Gherkin features), substrate tests and
real evidence on the household mesh or the fleet. A green unit test for a
signal enum is not proof that the network saw itself.

---

## 13. What This Epic Refuses to Claim

- `.epr-meta` is not the telemetry store. The measure registry declares what
  may be measured; the fold ledger records what was. Neither is a time-series
  database.
- A `/metrics` scrape is not a witness.
- A dashboard is not a holon.
- An alert is not an attestation.
- Unmeasured never reads as zero.
- A self-report is not diversity.
- A label is not consent.
- A heartbeat is not a health record.
- A green scrape is not a closed loop.
- A metric is not authority.
- A load-test bench is not the operator.
- A habit for this plane does not live under this directory; it lives where
  the behaviour it governs lives.
- This epic mints no entry type, route, table or protocol id.

---

## Conclusion

The Observer Protocol ends with eyes that serve love: a household seen by
those who know it, never by a camera that reports elsewhere, which is the
image this epic opened with. This epic
turns those eyes inward, onto the network that carries them. A peer knows
itself. A sibling signs what it saw. Enough of them, agreeing, make one record
the community can hold. Nothing above a holon ever holds the rows, and the
commons sees only fruit and the honest count of what went unobserved.

A network that can see its own health without a place where everyone can be
seen is not a smaller version of a monitored fleet. It is a different thing,
and it is the only version of observability a commons can afford to have.

---

## Technical Realization

- The witness plane and graduation path:
  [observation/event layer design](epr:observation-event-layer-design).
- The consolidated attestation kind fruit lands in:
  [attestation consolidation design](epr:attestation-consolidation-design).
- The rulings on the observation plane: [plane separation](epr:plane-separation-design) §5.
- The placement atlas's observation seam: [seam map](epr:elohim-seam-map-concern-routing) §3.16.
- The eight-stage prerequisite and the heartbeat cutover:
  [records lifecycle](epr:records-lifecycle-design) §D.5.
- The infrastructure manifest's observation kinds:
  `elohim/sdk/domains/infrastructure/manifest.json`.
- The survey that forced the question, and the two readings §6 leans on ([viable system](../../../research/elohim-as-viable-system-2026-06-04.md), [Meadows](../../../research/meadows-systems-dynamics-cross-pollination-2026-08-11.md)):
  [Permaculture DAO / Prometheus cross-pollination](../../../research/permaculture-dao-prometheus-cross-pollination-2026-10-08.md).
