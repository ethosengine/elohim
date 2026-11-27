---
title: "The carrier acts as its device under a steward's expiring grant, and a live agent answers for it"
id: carrier-acts-under-a-steward-grant-design
status: Draft
class: process-meta
date: 2026-11-27
steward: human:matthew
context-tier: disclosed
topic:
  - carry
  - service-without-agent
  - steward-grant
  - accountability
  - feedback
  - algedonic
serves: [sprint-lands-each-closure-once, acts-attributed-to-participants]
graduation-trigger: >-
  the scenario genesis/a2o/features/devflow/carry-under-steward-grant.feature passes: a miss posted by one
  device's hand-over is taken, gated and answered by a second device's supervised carry-watch with no note
  sent and no session cued; the carry claim, the verdict and any feedback it captured each name the device as
  the one acting and human:matthew as the steward answering, and cite the grant; and from the first device
  `epr flow carry show` answers who carried it, under whose grant and until when
cites: []
---

# The carrier acts as its device under a steward's expiring grant, and a live agent answers for it

Plan station F4-ask-unattended (`2026-10-29-native-dev-tooling-over-peers-plan.md`) wants a device to take a
posted miss with no session told. The script that does it today (`held-tool-watch carry`) has three defects
(no build role read, yields only to `dev-integration`, nothing sets `GATE_LANDING_INTENT`) and could not hold a
cargo seat because no session stands behind it. This page settles who acts, under what, how it is asked, and
what keeps the candidate's code from running before it is vouched for.

## The steward's words

All said by human:matthew on 2026-11-27 in ethosengine session `d6cfb7e1-ee3c-4e45-bfb2-ae04ec4b50a3`, quoted
as typed. Everything else on this page is derivation, marked as such where it might be mistaken for his.

- ~17:35Z, the actor: "a service agent sponsored by the device/working under the steward's authorization... is
  the right instict there, the point though is it has to traceback to the steward of record somewhere.. so if
  something is happening somewhere, it can be asked.. what are you doing here, and acconutability is always
  there.. and it's possible the question/feedback can get get back to the steward if warrented/carried back by
  the chain. I don't think we have a direct consumer for this yet beside ourselves.. but that's a deliverable
  of the elohim protocol so that wholistic limits can be understood in relationship to the human stewards"
- ~17:40Z: "service agents need epr-level acconutability.. they just don't have an "agent" of themselves.. they
  need to be in communication with a live agent that can review/answer for th stward."
- ~17:45Z: "oh just a note.. there's feedback and algedonic signals.."
- ~18:00Z, on Jenkins: "i'm thinking jenkins sholud have it's grants.. as long as it's results can carry the
  accountability, and it's runtime can be asked.. and can reach the elohim network to consider and answer the
  question if necessary then I think it's ok."
- ~18:04Z: "because jenkins has access to pods and such.. whatever compute it has available.. as long as it's
  own limits are anserable to the elohim network.. then it can be honestly subject to governance and trust if it
  does something that an agent with an acconutable steward can undrstand, answer and respond to."
- ~18:08Z, why the boundary exists: "it's a safe assumption that the cluster IS my stewarded compute.. elohim
  protocol actually shares the same compute limit as all my other infra we're trying to subsume.. the elohim
  protocol network should be able to model this.. BUT that model has to assume that i could instead be using a
  cloud provider, and have a billionare patron that's affording me virtually unlimited compute.. elohim
  protocol needs to understand the limit to KNOW how it needs to relate to what it's getting... because an
  steward with that kind of backing CAN create a conflict of interest or a nexus of power that if unknowable..
  launders it's value into the network.. that's why this bonudary exists." Then: "that's what informs the
  ruiremnets here."
- ~18:15Z: "whatever touches the network is auditable by agents.. so that a service worker caught in a lie.. is
  held acconutable"
- ~18:20Z, four calls asked of him in this session:
  1. Grant issue: "quoted words suffice, because as long as the words are traceable.. there's a chain of
     acconutability.. back to my own act."
  2. Attribution: "visibily the devices.. but note this auth is the device, but could be the system.. because
     jenkins has a RBAC service grant of it's own.... to operate on k8s pods.. but that grant is for jenkins..
     so it CAN be more than just a device.., so we might actually need to think about how containers that are
     created are epr-service compliant, so the accountability is automatically extendable in that kind of
     system."
  3. The residual risk (below): "Acceptable, bounded".
  4. The gate-key change in the same lane (a separate commit): "Yes, with shem's guard".

- ~20:00Z, told that "Digest at the turn" fails twice (an agent with a shell can append and sign a forged turn,
  since the device key is a plain file of the same user, and the chain catches only rewrites; and the log is a
  standing, easily reversed record of every turn he types), he chose the offered option "Approve in the portal":
  only grants are witnessed, by his approval in the consent portal on one of his devices, and no log of his
  turns exists. This replaces the ~18:40Z "Digest at the turn".
- ~19:15Z, asked whether grants are EPRs and offered the grant as a native REA Commitment instead of tags on
  other records: "yes that sounds resonable".
- ~18:40Z, two more calls asked of him in this session (his choices among offered options, quoted as the option
  labels he chose): on checking a quote, "Digest at the turn" (nothing reads conversation text later: when he
  speaks, the harness records only a digest of his turn on the device, and a grant's quote is checked against
  that digest); on covenant content, "Both, quote stays home" (a carry grant may outlive its session, at most
  24 hours, and its signed summary crosses to his other devices without his quoted words, which stay on the
  recording device).

## Who acts: the options and what each costs

| Option | What it is | Cost | Verdict |
|---|---|---|---|
| A. a new `ParticipantRef` kind (today there are three: agent, human, service) | `runner:<role>@<did>` | a new kind makes an older epr refuse the WHOLE flow log (`FlowRecord` has no catch-all, store.rs:22-32; an unknown prefix parses as a malformed agent, actor.rs:310); epr-rea selects an Edge roll (holochain build-manifest.json:131); and it names the service as an agent, against the ruling | rejected |
| B. `agent:carrier@<build>` | the agent form | no AI model stands behind it | rejected |
| C. `service:carrier` | the provenance form | `service:` is not an actor by definition (actor.rs:269-272; enforced: elohim/epr-rea/src/actor.rs `parse_acting_participant`, :321) | rejected |
| D. **the runner as itself, under a steward grant** | a device's did:key (or a system's identity, below) acts; the steward answers | no ontology change; needs one change to attribution (the next section) and a home for the grant | **recommended** |
| E. the Holochain `delegates-compute` grant | elohim-storage api/compute_grants.rs:85-161 | names a Holochain AgentPubKey, lives behind the storage daemon | later home |
| F. the affiliation record (eprfs-agent memory.rs `sponsor` / `acts_for`) | a sponsored line in the local Steward fold | it models membership in a collective, not authority to run work; weighed and not reused, but its sponsor-chain check is the precedent for the grant's | not reused |

## Attribution: visibly the runner's, answered for by the steward

Today, with no session, `resolve_attribution_on` (flow/note.rs:1419-1452) gives actor = steward =
`human:matthew`, so unattended machine work reads as his own labour, told apart only by a tag, while actor.rs:
256-262 says the substrate must tell them apart. His call 2 settles it: the act is **visibly the runner's**.

The runner cannot go in the actor slot as it is: a bare `did:key` is no `ParticipantRef` form, and every reader
that parses that slot would refuse or misread it as a malformed agent (actor.rs:310): `named_identity` and
`parse_acting_participant` (note.rs:1375-1382, used by carry, claim, fulfill, intend, note and habits), the
author-equality check at note.rs:1234, the recall index's acting filter (recall/index.rs:508) and memory import
(import.rs:1369). That would be option A's cost by the side door. So the record uses forms those readers
already accept: the **actor slot is left empty**, the existing authored arm (`Attribution::authored`,
note.rs:1335), which every reader handles as "no participant"; the **steward slot names `human:matthew`**; and
the **runner is named by its signature and `env:runner=<did>`**. The one resolver gains this case (empty
actor, steward set) rather than a second resolver beside it. A human's labour is then not inferred from a
machine's act, and no reader parses a new form. The cost, stated: a reader that looks only at the actor slot
sees nobody, not the device; it must read `env:runner` to see who ran. Moving the runner into the slot is
option A, with its roll, and a later ruling.

## The grant: his quoted words, traceable to the act that said them

His call 1: a grant needs no act only he can perform, provided the words are traceable to his own act. So what
the design must guarantee is that a **false quote is catchable**.

**Two records, one act (his ruling of ~19:15Z: the grant is an EPR).** The words are recorded as a covenant
clause: `epr covenant record --kind grantsCarry --party human:matthew --quote "<his words>" --valid-until
<RFC3339>` (covenant.rs:993-1060: quote, validity, revocation, the prior steward affirmation; the clause is a
brit-signed note, and it stays on the recording device). The **grant itself is a native REA `Commitment`** in
that device's flow store (epr-rea model.rs:352-371), minted by the same act:

| Field | Value |
|---|---|
| `action` | `use` (the steward lends the use of a runner's compute and standing) |
| `provider`, `receiver` | both `human:matthew`: he lends his own device's work to his own work, so no participant slot holds a form a reader cannot parse |
| `resourceSpec.classifiedAs` | `carry-grant@1`, `runner:<did>`, `project:<name>` per covered gate (or `project:*`), `ref:<glob>` per covered branch, `words:<clause cid>` |
| `validUntil` | the expiry, at most 24 hours ahead; `validFrom` the time it was said |
| `state` | `Active`; revocation is a later record that settles it, as a released carry does |
| `inScopeOf` | the repository scope, as every carry record |

Its CID is the grant's address. A Commitment stays on the device that wrote it (the fold appends only events),
so the grant **crosses as one signed `Affirm` event** on the Commitment, built by the landing module's one event
builder like the carry claim (flow/carry.rs, "Two records"), carrying the Commitment's fields in its env slots.
Every reader folds it as it folds any flow event; an older epr folds it unchanged and simply does not act on it.
The quote does not cross (his ruling "Both, quote stays home"): only `words:<clause cid>`, the address of the
words on the recording device. **This changes what a clause means.** A clause is honoured within its own session today
(covenant.rs:1-5: the steward answers once per agent session, and each answer is a clause for that session).
`grantsCarry` is honoured beyond its session: it authorises a sessionless process after the session that
recorded it ends, until it expires or is revoked. What the grant adds:

- **Provenance recorded with the quote:** the session and device that wrote it, when, and where he said it (the
  session id and turn time, or the surface, such as a terminal command he ran himself). Recorded only from the
  main conversation: the subagent-stamp guard refuses it from a subagent.
- **A required, short expiry:** `--valid-until` is required for `grantsCarry` and at most 24 hours ahead, on the
  same device as across devices (today the flag is optional; the grant kind makes it required).
- **Renewal needs new words.** An extension is a new clause whose quote differs from every earlier
  `grantsCarry` quote of the same party and whose turn is newer than the grant it extends; reusing the old quote
  is refused (today `covenant record` refuses only an empty quote, covenant.rs:1005-1009). Revocation is a new
  clause by the same route. Nothing writes a successor on its own.
- **What crosses.** The `Affirm` event above, signed by the recording device, and on every carry claim
  `env:grant=<commitment cid>` (an address). That is covenant-derived content leaving the device, which his
  ruling ("Both, quote stays home") allows without the quote. No separate `env:grant-doc` and no device mirror
  file: the carrier reads the grant from its own fold, so nothing outside the flow store can stand in for it.

**The portal witness** (slice 1; his ruling of ~20:00Z, "Approve in the portal").

His words are still recorded, as the covenant clause above, but what proves he granted it is an act of his own
on a surface no agent on the device drives: an approval in the consent portal, through the ceremony
`crates/consent-grant` already runs for device enrollment (`ceremony`: the asking side sends a request, a portal
where his controller is signed in shows a consent screen, his controller cell signs, and a single-use code bound
to the asker by PKCE comes back; `ConsentRecord` is content-addressed and can be shown to anyone).

- *The ask.* Recording a grant sends a request naming one new act, `carry.grant`, with what the screen must
  show: the runner device, the covered projects and branches, the expiry, and the quoted words. He may agree to
  less than was asked (the crate's rule), never more.
- *The witness.* The returned `ConsentRecord` (asked acts, agreed acts, his identity root, the device) is the
  grant's witness. Its CID is added to the Commitment as `consent:<cid>`, and the crossing `Affirm` event carries
  it, so a sibling or a lander verifies the controller cell's signature over the record, not the device key.
- *What this changes in the crate.* Its acts are recognition today ("Both acts here are recognition ... Neither
  says what the device may do to any content", act.rs:8-9; "It grants no authority over any content",
  lib.rs:33-34). `carry.grant` is the first act of authority, so the crate's stated boundary moves: listed for
  him below, and the module documentation changes with it.
- *Surfaces.* Any session on any runtime may ask, Codex and a terminal command included, because the witness is
  his portal approval, not the session. No log of his turns is kept, nothing reads conversation text, and his
  turn count and timing do not cross.
- *Residual.* An agent that can drive the browser in which his controller is signed in (a browser-automation tool
  in his own session) could approve for him. The portal therefore asks for a fresh confirmation on the consent
  screen (the crate's witness beat) and the grant records which portal approved it.

**Who audits the grant, and when.** The verifier is one read, `epr flow carry verify-grant <cid>`, run (1) by the
answering session before the first claim under a grant and (2) by the lander before it reuses a carried verdict,
on any device: it checks the `ConsentRecord`'s controller signature and its agreed acts against the
Commitment's coverage and expiry. A grant with no valid consent record carries nothing; a grant whose consent
record does not match it (wider coverage, longer expiry, another runner) is a false grant held against the
session that wrote it: the verifier emits `carry-grant-unverified@1` naming that session, captures feedback
naming the steward, and voids the grant with a revocation.

## The record a carry act leaves, and the trace

| Record | Names | Trace to the steward |
|---|---|---|
| carry claim (crosses) | actor empty; steward = human:matthew; signer = the device did; `env:runner=<did>`, `env:grant=<commitment cid>`, `env:carry=<id>`, the miss, commit, closure | the grant clause names the steward and the session he spoke in |
| gate verdict (`gate-verdict@1`, crosses) | the device signer, the closure, `env:grant` | same |
| feedback item (device-local) | `participant` = the steward; detail key `grant` = the clause CID (the feedback lifecycle spec adds it); `EPR_FEEDBACK_INVOCATION` = the carry id | its closing rule reads `participant` |
| algedonic observation | `carry-unanswered@1`, `carry-grant-expired@1`, `carry-grant-unverified@1` (a quote not found) | each needs a declaration in the measure registry and a bound, and a slot in the headline, which today renders a closed set (report.rs:100-140, 2367-2385): both are in the carry-watch slice |

The answering session is in none of these records (sessions are replaced; the grant resolves who answers now),
and the device did appears only on what leaves the device.

`epr flow carry show [--device <did>] [--carry <id>]` answers from the folded carry claims: the runner, the
steward, the grant (decoded, its CID recomputed, the clause's session and time), its expiry, the miss, commit,
closure, since when and the outcome. **Who answers now** is not in those records, so carry show reports it only
on the carrying device (from its berth state); from another device it says so and names the device to ask.

A **lander** checks the grant before reusing a carried verdict: a verdict whose grant is expired, void or does
not cover the project is not reused, and the lander runs the gate itself. Without that check the grant would be
accountability only.

## The live agent that answers for it

His "in communication with a live agent", made checkable (today proved twice that a moored session with a
follow can sit for hours without reading a note): the answering session is **on the carrying device** (a berth
acknowledgement is visible only on the reader's device), and a carry claims only after that session has
**acknowledged** the claim note (`berth inbox --reader … --ack`) within 120 seconds (a working default, open
for the steward). No acknowledgement in time declines the carry, recorded. If the answering session goes away
mid-carry, the job in flight finishes (it holds a seat and a worktree; stopping it would waste the work without
making it safer) and records `carry-unanswered@1`; no new carry starts until an answering session acknowledges
again. A note addressed to the carrier needs an address berth does not have today (berth addresses sessions):
`carrier@<device>` routed to the acknowledged answering session is new berth work, listed below.

## One attribution resolver

The read feedback capture needs and the carrier needs are one: for whom does this act, and under what
authority, from files only (the session envelope and claim, the roster, and the grant Commitment read from the flow sidecar without taking its lock), within capture's 2 s
and without the store lock. It is the existing resolver extended, not a second one beside it. A read that
cannot resolve refuses the ACT (the carrier does not carry), not the record of a problem: capture stores an
unresolved one as "no participant" within its 2 s, so a tool's report of a problem is not lost.

## The carry-watch, its isolation, and which commits it may run

- `epr flow carry-watch` is an ark `processes[]` child under `epr device supervise`. Correction to the first
  draft: the supervisor does not hold `device-ark`; the session that runs `epr device supervise` claims it
  with a TTL, and the sessionless path refuses (supervise.rs:1434-1462; berth:4030 "no P1 session"). A cargo
  seat held under a grant with no session is therefore a **new berth holder kind**, scoped to `cargo`, live
  while the grant is valid and the supervised process is alive.
- **The bounds he accepted, each with where it is enforced, in this order.** Every refusal records a declined
  carry claim (`env:declined=<reason>`), which crosses like any carry claim and shows in `carry show`. An older
  epr ignores the `env:` tag, so a declined claim also carries a `valid_until` at or before its own time: every
  reader, old or new, sees it lapsed and does not wait on it.
  1. *At grant admission* (when carry-watch loads the grant): the carrying device is on the roster standing for
     the grant's steward, the grant names this device, and it verifies as above. Else no carries; reason
     `grant`.
  2. *On reading a miss, before any fetch:* the miss's signer is a device on the roster standing for the same
     steward, and its gate project is one the grant covers. Else reason `signer` or `declined-by-grant`.
  3. *At claim:* the grant is unexpired and its answering session has acknowledged (above). Else reason
     `expired` or `unanswered`.
  4. *Fetch, with no checkout:* the commit the miss names is fetched by exact id into the runner's object store,
     and must be reachable from a ref the grant names (`merge-base --is-ancestor`). Else reason `ref`.
  5. *A detached worktree, read as data:* as held-tool-watch does today (:47-60, :1064-1090), a detached
     worktree is created with checkout hooks, LFS and attribute filters disabled and checked clean, and the
     Cargo home kept outside it. The closure is then recomputed with the RUNNER's trusted orchestrator code from
     a private execution root, reading the worktree as data (the closure includes cargo-derived and directory
     reads, so a key from the object store alone exists nowhere today), and must equal the miss's key. Else
     reason `key`, and the worktree is removed.
  6. *Only then* the vouched-for recipe runs in that worktree. The candidate's code first runs here.
- **What isolates, stated plainly:** `ark task run` gives no isolation today (`EffectiveTier::None` →
  `Isolation::None`, ark core task.rs:666). Only the separate worktree and the disabled hooks and filters
  separate the candidate; its recipe runs with the device user's rights. That is part of the residual below.
- **Residual risk, accepted by the steward ("Acceptable, bounded"):** anyone who can push a branch the grant
  names and get a miss posted by one of his standing devices gets that branch's recipes run unattended under
  the grant, on his own devices, with the device user's rights. Bounded by steps 1-6, the 24-hour expiry and the
  audit trail.
- It yields to any held cargo lease that is not its own, to `dev-integration`, and to an open interactive gate
  run (`gate-live`). The lander sets `GATE_LANDING_INTENT` in pre-push.
- **`GATE_CARRY_WAIT=on` waits for the lander's grant check.** Slice 2 (handed over separately as 982f1830d, read safe, not yet landed) gives every dev
  gate its landing intent, but `GATE_CARRY_WAIT` stays `shadow` until the lander checks the grant: turned on
  before that, a matched intent could hold a landing up to the wait's hard cap (1800 s) on a carry no grant
  vouches for.
- **The lander's check has a pin lag:** a lander running an epr older than this change ignores `env:grant` and
  reuses a carried verdict without checking it, until every lander is upgraded. Until then the grant is
  accountability, not a gate, on those landers.

## Runners other than a device, and the work they spawn

His call 2: the acting authority "CAN be more than just a device": Jenkins holds an RBAC service grant of its
own to operate on k8s pods, and the containers such a system creates should be "epr-service compliant, so the
accountability is automatically extendable". Derivation, for his ruling: a **runner** is whatever identity acts
under a grant (a device did today; a system identity such as Jenkins's later). Work a runner spawns (a pod, a
container, an ark task) carries its grant forward: the spawned workload receives the grant's CID and its
parent runner in its environment or labels, its results cite both, and a child grant is a narrowing of its
parent's and not wider. Then a result from any depth traces to the runner that started it and to the steward,
without each container being separately enrolled. Not built in any slice here; the first carrier is a device.

## The four conditions every grant must meet, and what is wired today

From his words on Jenkins (~18:00Z, ~18:04Z): (1) results carry the accountability; (2) the runtime can be
asked; (3) it can reach the network to consider and answer. And (4) "its own limits are answerable to the elohim
network". The rest of this paragraph is derivation. For (4) a runner declares its compute and its reach and
whose compute it is and who affords it (his ~18:08Z words: own hardware, a provider, a patron). A declaration
home exists: ark `RuntimeManifest.envelope` (manifest.rs:20-48, 133-150), whose own documentation says a
declaration is not enforcement. **Condition 4 is UNWIRED for both runners today**: nothing observes a carry
exceeding its envelope (`EffectiveTier::None`, task.rs:666; plan row 16: no dispatch reads capacity; measured
today are per-step seconds, the RAM guard and the cargo-seat count), and for Jenkins nothing can detect a result
from beyond its declared reach. Saying a granted Jenkins result "does not count" adds nothing, because
gate-grade.mjs:14-20 already counts sponsored results zero. Wiring it is its own station, F4-limits-answerable in the plan (not started).

## Working postures (defaults until the steward rules otherwise)

- **Narrowing:** a grant may name fewer gate projects than every miss (`"*"` by default), checked against the
  gate registry at issue and at claim; capacity admission still applies on top; a miss the grant does not cover
  is recorded `declined-by-grant`, visible in carry show.
- **Cross-device:** a grant is recorded in a session on whichever of his devices he spoke in, and names exactly
  one recipient device; the recipient admits it only when its own roster fold shows both devices bound to the
  same steward; carry show verifies the clause against the device it names as issuer. At most 24 hours; a
  revocation crosses as a new clause; a recipient that has folded no head of the issuer within berth's
  `freshness_s` takes no new carries.
- **Jenkins:** acts under a grant as a runner, not an actor; its results cite the grant; counting them toward
  sufficiency is a separate ruling, and they count zero until then.

## Open for the steward

- **`carry.grant` moves the consent crate's boundary** from recognition to authority (above).
- **The browser residual:** an agent able to drive the browser where his controller is signed in could approve.

Ruled at ~18:40Z and no longer open: the quote check (digest at the turn, replaced at ~20:00Z by the portal witness), the signed digest crossing without
the quote, and a grant outliving its session. Still to verify in slice 1: that `brit covenant list` reads
clauses across sessions, which the new-words rule needs.

- **The first carrier ships with condition 4 unmet:** nothing observes a carry exceeding its declared limits
  (above). Whether to ship it before station F4-limits-answerable lands is his call.
- Whether cross-device recording (a grant said on one device for another) stands. With the portal witness any
  device can verify the grant, so the earlier reason against it is gone.
- The liveness standard: the 120-second acknowledgement bound, and whether a live follow without an
  acknowledgement is enough.
- The runner-spawned accountability above: whether a child workload's grant-forwarding is the shape he means
  by "epr-service compliant". A named follow-on, not in these slices.
- Whether a granted Jenkins result should ever count toward sufficiency.

## Known gaps in slice 2, from its review

The integrator's reader passed slice 2 (`self/ethosengine-lander-intent`, 982f1830d) as safe to land and named
these, to cure in the carry-watch slice (3) or later: a `just gate` killed by SIGKILL or power loss leaves an open
run, and the carrier declines for up to its 6-hour staleness window; any other cargo claim, a rust-analyzer check
among them, stops a carry each time, and a stopped carry is not recorded once, so the watcher may retry and stop
again until the miss's wait ends; one failed `berth who cargo` read mid-gate stops the gate; the mid-gate check
spawns two berth reads a second for the gate's life; untested are main and `self/*` pushes skipping the lookup,
`LANDING_INTENT=off`, a preset `GATE_LANDING_INTENT` winning, two live intents sharing a Change-Id, and a malformed
Change-Id.

## Story first, then slices, and what rolls

Slice 0 is the scenario, before any code: `genesis/a2o/features/devflow/carry-under-steward-grant.feature` (it
does not exist yet), through the blind-reader loop. Its scenarios: the graduation trigger above; a carry
declined with no acknowledged answering session; a miss declined at each of steps 2, 4 and 5; and a grant whose
quote is not found being voided. Then, each its own commit: (1) the attribution case, the grant kind with its
required expiry, new-words renewal, the grant Commitment and its signed `Affirm` event, the `carry.grant` consent act and portal witness, and `carry verify-grant`; (2)
the lander's intent, the yields and the lander's grant check; (3) carry-watch under the supervisor with steps
1-6, the grant-held cargo lease, the acknowledged answering agent and the three measures with their registry
declarations and headline slot, together, so no carrier ships without an answering agent; (4) carry show; (5)
the `carrier@<device>` berth address. device-runtime/** is an Edge watch glob (holochain build-manifest.json:126),
so slices that touch it ride an authorised roll bundle; epr-cli is the eprfs gate; berth is agentic tools;
epr-rea is untouched.
