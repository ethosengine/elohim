---
epr-habit-version: 1
id: pain-is-answered
invariant: >
  A peer's declared pain is answered, never dropped: an Approach written on the peer's own
  pledge raises care in proportion to the limit and pre-positions relief the peer itself
  consents to; a Breach executes what was agreed with a recorded gate-decision that reaches
  every party; a Breach with no prior Approach becomes a Refer with its evidence; and no open
  Approach, Breach or Refer ages past its declared window without a witnessed decision or a
  witnessed `pending`. The seat that answers is born narrow — it may act alone only to add
  holders and remove nothing — and widens only on evidence.
status: red
active: false
checks:
  - "a2o @concern:pain-is-answered (genesis/a2o/features/dataplane/operator-answers-pain.feature — Act I, household mesh: `just test mesh features/dataplane/operator-answers-pain.feature`, or from genesis/a2o `npx cucumber-js --config '' features/dataplane/operator-answers-pain.feature` (exit 1 today, undefined steps on purpose). Eight scenarios on existing types only — a `CommitmentByState` link on the pledge commitment for Approach/Breach/Recovered, the neighbours' own `custody-blob` commitments, `attestation:gate-decision` allow|block|pending, `attestation:stewardship-grant` for carry-forward at STAGING tier beneath the owner's earned head. The stock is the whole compute profile a device enrolled (disk, memory, cpu, bandwidth per `ark` `ResourceQuota`), sensed by the always-on gauges against declared bounds, fail-closed to Unmeasured — never the disk alone; the runtime-performance capture is the seat's RCA instrument after dispatch, not the peer's sensor.)"
first_move: >
  write the red: the household feature above, through the blind-reader loop, parsing clean and
  exiting 1 with undefined steps. The scenario is the specification; slice 2 of the spec
  (conductor-dataset free-space probe, fail-closed Unmeasured, capacity decline through the
  policy evaluator, `unverifiable` when the conductor is unreachable) is the first code, storage
  only and coordinator-only, gated by `just gate elohim-storage`.
retire-when: >
  when silence about pain is UNREPRESENTABLE rather than merely watched: the feedback projector
  refuses to let an open Approach, Breach or Refer age past its declared window without a
  witnessed decision or a witnessed `pending`, and the operator seat's commitment is the only
  path by which a remedy is executed — at which point the loop (sense → orchestrate → judge →
  resolve → inform → settle) is a property of the substrate and this session's hand-run of it
  has no successor to replace.
refs:
  - "genesis/docs/superpowers/specs/2026-10-09-elohim-operator-loop-wedged-peer-design.md (the design; §0 is this week's hand-run trace as specification, §5 the mintable nodes)"
  - "genesis/docs/superpowers/specs/2026-08-10-algedonic-feedback-signal-design.md (the algedonic ontology; its §3 hash-neutral claim corrected 2026-10-09)"
  - "genesis/docs/superpowers/specs/2026-10-06-device-footprint-residency-carrying-capacity-design.md (§5 the bands the stock is measured against)"
  - "elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md (the half-alive seam: a storage whose conductor is unreachable answers `unverifiable`)"
  - "elohim/elohim-storage/.epr-meta/liveness-has-one-source.habit.md (absence is a judgment with evidence, never a chain fact — this habit conforms)"
  - "elohim/elohim-storage/.epr-meta/blob-durability.habit.md (salvage after loss; this habit adds voluntary relief of a live-but-wedged peer)"
  - "elohim/holochain/dna/imagodei/.epr-meta/custodial-authority-answerable.habit.md (the collective-held grant the carry-forward rides; no on-behalf flag)"
  - "genesis/data/timeline/backlog/alpha-adam-peer-meta-store-disk-io-error.md (the incident: adam and eve at zero free, gertrude's slot dead, 2026-10-08)"
---
2026-10-09: DECLARED `unwired` → RED the same day. Born from the week adam's and eve's conductor
datasets hit zero free and crash-looped while their storage processes kept answering sweeps with
cached heads, and gertrude's conductor slot refused hosted registrations with nothing to say about
why. The operator's direction: a peer should refuse before it wedges and say so; governance
should not stop because a peer did; peers should step in and relieve — and anything not working
pleasantly should be able to invoke an agent that does RCA → plan → implement → resolve through
the system's own interfaces, making mishpat as it goes. This session was that loop run by hand,
and the spec transcribes it (§0). Grounding by three sweeps and one adversarial review (recorded
in the spec): no algedonic signal has a DHT home; the FeedbackSignal path is DNA-hash-moving
(contradicting its own spec, corrected); admission is concurrency-only with no capacity input;
nothing measures the conductor dataset's free space and the probe fails open to 100%; salvage
never fires on a wedged-but-gossiping peer; the live election already keeps a carried head at
staging beneath the owner's earned head — the one bound that actually stops a collusion capture,
so the design keeps it rather than inventing lineage ordering; no Refer route exists and a human
inbox as the only exit is a C3 hole, closed by a witnessed `pending`. Carrier chosen by the
operator: Mishpat REA, refined to the A2 `CommitmentByState` link on the pledge. Two operator
framings bound the design: Approach is "keep an eye out for me" (care in proportion to a limit,
relief pre-consented by the peer while it can still sign) and Breach is "something broke" (the
agreed plan executes); and the stock is the whole compute profile the device enrolled, not disk.
Feature `genesis/a2o/features/dataplane/operator-answers-pain.feature` authored RED the same day
through the blind-reader loop (cost recorded on its commit). No code. NO status change beyond
unwired → red.
Blind-reader loop cost for the feature (3 rounds, a fresh reader each, cap reached without converging): round 1 correctness 1 / interpretability 7 / preference 3; round 2 1/4/3; round 3 3/6/1. Resolved: actor named in every key step; device steward, custody rows, holds gauge, sweep, Refer window, mesh, tick, `@requires:owned-substrate` and `@requires:multi-node` defined; neighbours' votes recorded on the proposal; intents authored on the neighbours' own initiative; the Breach state link asserted before anything follows; relief bytes fetched from the network and verified, never from the wedged peer. DEFERRED BY NAME for the operator: (correctness) no scenario for a REJECTED relief proposal — the consequence of a "no" vote is an open design question, not a ninth scenario to invent; (correctness) vocabulary claims no scenario exercises ("raising care", "an intent obliges nothing"); (interpretability) the concrete tick length is undeclared; (interpretability) motivation lives in scenario comments, a Gherkin limit; (preference) the tick paragraph is long. Round-2 preference findings not acted on: Scenario 4 framing, comment placement, five bands with one tested.

DELTA 2026-10-09b (RED preserved; the first real-world instance of this habit's trigger, root cause named by the operator, no cure in code): the operator cleared adam's and eve's conductors at ~11:40Z and named the root — shem's ZFS snapshot configuration (sanoid/syncoid) counts snapshots against the pods' PVC quota, so every snapshot run shrank the runtime band's available space until the conductors' sqlite could not open its database (`SqliteError code 14 "unable to open database file"` on every list_apps as seen by alpha-b's doorway; `conductor_unavailable` on adam's adoption channel). The operator's reading, close to verbatim: that sort of resiliency is what the protocol is meant to own itself; the tank/snapshot was an external imposition on the conductors provisioned on those PVCs, and it is exactly the kind of thing that looks like an outside problem to the network's integrity that should have triggered unaffected peers to step in, diagnose, and hold the load on those peers' behalf. Read against the spec: the stock was `runtime × disk`, its limit was never declared (no bound_ref), the one free-space probe fails open to 100%, so no Approach was ever written and the Breach could not be — the slow path with no evidence set, which is why a human had to be the sensor. Nothing on the network distinguished "the disk is full" from "something outside the protocol is consuming the disk"; the design's §3 band-×-primitive stock, sensed by always-on gauges fail-closed to Unmeasured, is the first station that would have named it. Evidence this time: operator action on shem, not a gauge. NO status change.
