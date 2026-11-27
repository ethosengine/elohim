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
256-262 says the substrate must tell them apart. His call 2 settles it: the act is **visibly the runner's**. The
record's actor is the runner's identity, the device's did:key for a device carrier, and its steward is
`human:matthew`; the one resolver gains that case rather than a second resolver being written beside it. A
human's labour is then not inferred from a machine's act.

## The grant: his quoted words, traceable to the act that said them

His call 1: a grant needs no act only he can perform, provided the words are traceable to his own act. So the
grant **is** a covenant clause, not a new record: `epr covenant record --kind grantsCarry --party human:matthew
--quote "<his words>" --valid-until <RFC3339>` (covenant.rs:993-1060 already holds quote, validity, revocation
and the prior steward affirmation, and asks through the main conversation). Its traceability is what makes it
answerable:

- **Recorded only from the main conversation.** The subagent-stamp guard refuses it from a subagent, so the
  quote is tied to the session where the steward spoke, not to whoever typed it.
- **Auditable.** The clause names the session and the time it was said; an agent auditing the grant can read
  that session's record and find the words, or not find them. A quote that is not there is a service caught in
  a lie: recorded as an algedonic breach and as feedback naming the steward, and the grant is void from then.
- **Extension and revocation** are new clauses by the same route; nothing writes a successor on its own.
- **Its address** is the clause's CID. On the carrying device it is mirrored, with the role, the device, the
  covered projects and the commit rule, into `<device config>/carry-grant.json` so the carrier and capture read
  it from a file without the store lock.
- **Crosses** in `env:` tags on every carry claim (`env:grant=<cid>`, `env:grant-doc=<base64 canonical body>`,
  the precedent being `env:requires-doc`), which an older epr folds unchanged. Covenant notes stay local by
  standing ruling, so the crossing copy is the grant's body, not the note.

## The record a carry act leaves, and the trace

| Record | Names | Trace to the steward |
|---|---|---|
| carry claim (crosses) | actor = the runner; steward = human:matthew; signer = the device did; `env:grant`, `env:grant-doc`, `env:carry=<id>`, the miss, commit, closure | the grant clause names the steward and the session he spoke in |
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
follow can sit for hours without reading a note): a carry claims only after its answering session has
**acknowledged** the claim note within a bound (`berth inbox --reader … --ack`), or holds a live follow that
printed it. No acknowledgement within the bound declines the carry, recorded. If the answering session goes
away mid-carry, the job in flight finishes (it holds a seat and a worktree; stopping it would waste the work
without making it safer) and records `carry-unanswered@1`; no new carry starts until an answering session
acknowledges again. A note addressed to the carrier needs an address berth does not have today (berth
addresses sessions): `carrier@<device>` routed to the acknowledged answering session is new berth work, listed
below.

## One attribution resolver

The read feedback capture needs and the carrier needs are one: for whom does this act, and under what
authority, from files only (the session envelope and claim, the roster, the grant mirror), within capture's 2 s
and without the store lock. It is the existing resolver extended, not a second one beside it. A read that
cannot resolve refuses the ACT (the carrier does not carry), not the record of a problem: capture stores an
unresolved one as "no participant" within its 2 s, so a tool's report of a problem is not lost.

## The carry-watch, its isolation, and which commits it may run

- `epr flow carry-watch` is an ark `processes[]` child under `epr device supervise`. Correction to the first
  draft: the supervisor does not hold `device-ark`; the session that runs `epr device supervise` claims it
  with a TTL, and the sessionless path refuses (supervise.rs:1434-1462; berth:4030 "no P1 session"). A cargo
  seat held under a grant with no session is therefore a **new berth holder kind**, scoped to `cargo`, live
  while the grant is valid and the supervised process is alive.
- **Isolation, kept from the script it replaces** (held-tool-watch:47-60, :1064-1090): the candidate's closure
  is recomputed with the RUNNER's trusted orchestrator code from a private execution root, not the
  candidate's own gate runner; checkout hooks, LFS and attribute filters are disabled; the Cargo home stays
  outside the candidate; the gate runs inside `ark task run` in an isolated worktree. The candidate's code
  first runs when its vouched-for recipe starts.
- **Which commits:** only one a miss names, where (a) the miss was posted by a sibling device standing for the
  same steward, (b) the commit is fetched by exact id and is reachable from a ref the grant names, and (c) the
  closure recomputed as above equals the miss's key.
- **Residual risk, accepted by the steward ("Acceptable, bounded"):** anyone who can push a branch the grant
  names and post a miss gets that branch's recipes run unattended under the grant, on his own devices. Bounded
  by the rule above, the grant's expiry, the isolation, and the audit trail.
- It yields to any held cargo lease that is not its own, to `dev-integration`, and to an open interactive gate
  run (`gate-live`). The lander sets `GATE_LANDING_INTENT` in pre-push.

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
gate-grade.mjs:14-20 already counts sponsored results zero. Wiring it is its own station.

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

- Whether cross-device recording (a grant said on one device for another) stands, or only the carrying
  device's own session may record it.
- The liveness standard: the acknowledgement bound, and whether a live follow without an acknowledgement is
  enough.
- The runner-spawned accountability above: whether a child workload's grant-forwarding is the shape he means
  by "epr-service compliant".
- Whether a granted Jenkins result should ever count toward sufficiency.

## Story first, then slices, and what rolls

The scenario is written before code: `genesis/a2o/features/devflow/carry-under-steward-grant.feature` (the
graduation trigger above, plus a declined carry with no acknowledged answering session, and a grant whose quote
is not found being voided). Slices, each its own commit: (1) the attribution case and the grant clause with its
device mirror; (2) the lander's intent, the yields and the lander's grant check; (3) carry-watch under the
supervisor with the isolation, the grant-held cargo lease, the acknowledged answering agent and the three
measures with their registry declarations and headline slot, together, so no carrier ships without an
answering agent; (4) carry show; (5) the `carrier@<device>` berth address. device-runtime/** is an Edge watch
glob (holochain build-manifest.json:126), so slices that touch it ride an authorised roll bundle; epr-cli is the
eprfs gate; berth is agentic tools; epr-rea is untouched.
