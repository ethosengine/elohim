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

**Words this page uses.** A **miss** is a gate result a device posts when it holds no evidence for the exact
inputs of a gate; it names the commit, the gate project and the closure. The **closure** is the fingerprint of
everything that gate reads, and its **key** is the verdict's address. To **carry** a miss is for another device
to run that gate on that commit and record the verdict, so the posting device can reuse it. A **carry claim** is
the signed record a carrier writes when it takes a miss (and, declined, when it refuses one); it is valid for
a bounded time (at most 2 hours today, `held-tool-watch`'s `MAX_CARRY_VALID_S`). The **lander** is the
pre-push step that moves work onto `dev`; it may reuse a carried verdict instead of running the gate. A **cargo
seat** is one of the device's slots for heavy cargo builds, held as a berth lease on `cargo`. To **fold** is to
read another device's signed records into this device's view; a record **crosses** when the device exchange
carries it to the steward's other devices, which `berth.channel.crosses` in the pool policy allows kind by kind.
A **station** is a named step of the plan (`2026-10-29-native-dev-tooling-over-peers-plan.md`); a **slice** is
one commit-sized piece of this design, numbered 0 to 5 at the end of the page.

The tools it names, one line each: a **gate** is a named check a change must pass before it lands (`just gate
<project>`); **berth** is the device's tool for session moorings, leases on shared resources such as cargo
seats, and notes between sessions; **brit** is this repository's git implementation, which also stores signed
covenant clauses as notes; **epr** is the repository's flow, session and covenant command line; **ark** is the
device runtime that supervises background processes and runs placement tasks; **REA** is the
resource-event-agent accounting model whose records (here a Commitment) epr keeps in the flow store; an **Edge
roll** is a deploy of the Edge pipeline that restarts the fleet's pods (about twenty minutes of churn), which
some paths select; the **roster** is the tracked list of devices Matthew has admitted as his own.

Every command this page names that does not exist yet is marked with the slice that builds it at its first use.

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

- ~18:40Z, two more calls asked of him in this session (his choices among offered options, quoted as the option
  labels he chose): on checking a quote, "Digest at the turn" (nothing reads conversation text later: when he
  speaks, the harness records only a digest of his turn on the device, and a grant's quote is checked against
  that digest); on covenant content, "Both, quote stays home" (a carry grant may outlive its session, at most
  24 hours, and its signed summary crosses to his other devices without his quoted words, which stay on the
  recording device).
- ~19:15Z, asked whether grants are EPRs and offered the grant as a native REA Commitment instead of tags on
  other records: "yes that sounds resonable".
- ~20:00Z, told that "Digest at the turn" fails twice (an agent with a shell can append and sign a forged turn,
  since the device key is a plain file of the same user, and the chain catches only rewrites; and the log is a
  standing, easily reversed record of every turn he types), he chose the offered option "Approve in the portal":
  only grants are witnessed, by his approval in the consent portal on one of his devices, and no log of his
  turns exists. This replaces the ~18:40Z "Digest at the turn".
- ~22:20Z, on the integrator's staging question (portal consent done properly is its own work: a presence
  factor, a field binding the record to the grant, resolving the signer against his controllers, a node on each
  device, an Edge roll and a brit release; option 1, make it a station and keep the supervised script until then;
  option 2, ship the native carrier on a same-device covenant clause asked through the main conversation, the
  residual he already accepted as bounded, and upgrade to portal consent when that station lands): "go with
  option 2 then."

## Who acts: the options and what each costs

| Option | What it is | Cost | Verdict |
|---|---|---|---|
| A. a new `ParticipantRef` kind (today there are three: agent, human, service) | `runner:<role>@<did>` | a new kind makes an older epr refuse the WHOLE flow log (`FlowRecord` has no catch-all, store.rs:22-32; an unknown prefix parses as a malformed agent, actor.rs:310); epr-rea selects an Edge roll (holochain build-manifest.json:131); and it names the service as an agent, against the ruling | rejected for now (decision 1) |
| B. `agent:carrier@<build>` | the agent form | no AI model stands behind it | rejected |
| C. `service:carrier` | the provenance form | `service:` is not an actor by definition (actor.rs:269-272; enforced: elohim/epr-rea/src/actor.rs `parse_acting_participant`, :321) | rejected |
| D. **the runner as itself, under a steward grant** | a device's did:key (or a system's identity, below) acts; the steward answers | no ontology change; needs one change to attribution (the next section) and a home for the grant | **recommended** (the author's derivation, not a ruling) |
| E. the Holochain `delegates-compute` grant | elohim-storage api/compute_grants.rs:85-161 | names a Holochain AgentPubKey, lives behind the storage daemon | later home |
| F. the affiliation record (eprfs-agent memory.rs `sponsor` / `acts_for`) | a sponsored line in the local Steward fold | it models membership in a collective, not authority to run work; weighed and not reused, but its sponsor-chain check is the precedent for the grant's | not reused |

## Attribution: visibly the runner's, answered for by the steward

Today, with no session, `resolve_attribution_on` (flow/note.rs:1419-1452) gives actor = steward =
`human:matthew`, so unattended machine work reads as his own labour, told apart only by a tag, while actor.rs:
256-262 says the substrate must tell them apart. His call 2 ("visibily the devices") asks that the act read as
the runner's. What this design can give without option A's cost is less than "visibly": the device is named,
but not in the actor slot.

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
option A, with its roll. He ruled on that cost (2026-11-28, choosing among offered options): "Beside now, slot
later": the device is named beside the actor slot in these slices, and moving it into the slot is its own later
station, with its roll.

## The grant: his quoted words, traceable to the act that said them

His call 1: a grant needs no act only he can perform, provided the words are traceable to his own act. His call
of ~22:20Z ("go with option 2 then") sets the first stage: a **same-device covenant clause asked through the main
conversation**, with portal consent as a later upgrade (station F4-portal-consent, below).

**Two records, one act.** The words are recorded as a covenant clause on the carrying device (the
`grantsCarry` kind exists in brit 0.3.13, pinned in f07f678d8; epr's side of it is slice 1, not built): `epr covenant
record --kind grantsCarry --party human:matthew --quote "<his words>" --valid-until <RFC3339>` (covenant.rs:993-1060:
quote, validity, revocation, the prior steward affirmation; the clause is a brit-signed note and stays on the
device). The **grant itself is a native REA `Commitment`** in the same device's flow store (epr-rea
model.rs:352-371), minted by the same command: slice 1 extends `epr covenant record` so that, for
`grantsCarry` only, it stores the clause and then writes the Commitment naming it. The carrier reads the
Commitment, so a clause whose Commitment was not written grants nothing: a failure between the two fails
closed. The contract (slice 1): if the Commitment write fails, the command exits non-zero naming the stored
clause's CID, `verify-grant` reports such a clause as an orphan, and the same command given that clause CID
writes the missing Commitment once (the words are already recorded, so the new-words rule does not apply to it). The Commitment's fields:

| Field | Value |
|---|---|
| `action` | `use` (the steward lends the use of a runner's compute and standing) |
| `provider` | `human:matthew` |
| `receiver` | the repository agent, as on every carry commitment (flow/carry.rs): the work is received by the repository's scope, and both forms are ones every reader already parses |
| `resourceSpec.classifiedAs` | a list of strings (epr-rea's `classified_as: Vec<String>`), in this order: `carry-grant@1`, `runner:<did>`, one `project:<name>` per covered gate (or the single `project:*`), one `ref:<glob>` per covered branch, `words:<clause cid>` |
| `validUntil` | the expiry, at most 24 hours after `validFrom`, the time it was recorded |
| `state` | `Active`; revocation is a later record that settles it, as a released carry does |
| `inScopeOf` | the repository scope, as every carry record |

Its CID is the grant's address. The record binds his words **by hash only** (`words:<clause cid>`): the quote
itself stays on the device (his ruling "Both, quote stays home"): no record that crosses includes it.

- **Where it may be recorded.** Only in the main conversation of a session on the device the grant names as its
  runner, and the recording device's did must equal `runner:<did>`. What enforces that today, and what does not:
  `epr covenant record` names the recording session in the clause and, through `live_guard`
  (session_envelope.rs:409-444), requires that session's envelope on this device to be live and bound to this
  checkout; the `covenant-subagent-stamp` hook refuses `covenant record` or `covenant put` from a tool call that
  carries an `agent_id`, so a subagent's attempt is refused. The hook records nothing (nothing in the clause says
  "main conversation"), and it is a string match (its own docstring: it guards against honest drift, not a
  determined forger). Nothing tells a Codex session or a terminal command from a main conversation: each can
  name a live session. Slice 1 adds the one check the device holds: a `grantsCarry` clause is refused unless its
  session is moored on this device with the runtime `claude-code`, saying why. That runtime is a declaration:
  the Claude Code start hook moors each session with it (ram-guard.py:502), and any process can declare it, so
  the check stops an honest Codex session or terminal and not one that lies. A shell that names a borrowed
  live session still passes, and so does any main-conversation session on the runner: that is the accepted
  residual below. What of
  the trace survives a workspace restart: the clause (a brit note at `refs/notes/brit/covenant/<session>/<cid>`
  in the repository's git store), the Commitment (the flow sidecar `.eprfs/status/flows.jsonl`) and the
  recording session's own transcript (under the harness's config directory) all live on the persistent
  volume; `/tmp` holds none of it.
- **This changes what a clause means.** A clause is honoured within its own session today (covenant.rs:1-5: the
  steward answers once per agent session, and each answer is a clause for that session). `grantsCarry` is
  honoured beyond its session: it authorises a sessionless process after the session that recorded it ends,
  until it expires or is revoked (his ruling "Both, quote stays home" allows this).
- **The clause's payload** is a closed set, agreed with the brit lane (`deny_unknown_fields`, so it cannot
  grow once shipped): `runner`, a `did:key:z…` whose key is ed25519 (brit refuses any other key type);
  `projects`, a non-empty list of gate project names, or exactly `["*"]`; `refs`, a non-empty list of branch
  globs without the `refs/heads/` prefix (`self/*`), `*` the only metacharacter, matching any run of characters
  including `/`. Under his ruling of 2026-11-28 ("Refuse dev and main"), slice 1 refuses at record time a grant
  any of whose `refs` would match `dev` or `main`, saying so (reason `shared branch`); brit's shape check is
  unchanged. The quote is the clause's own quote field, the validity its signed body's `validFrom` and
  `validUntil`, the steward its party. The clause does not name the Commitment (the Commitment names the
  clause). Brit checks shape only; whether the runner stands for the steward and whether a run falls inside the
  coverage stay with epr.
- **Expiry.** `--valid-until` is required for `grantsCarry` and at most 24 hours ahead. The reader-side check is one
  validator, `verify-grant`'s, which refuses a grant whose `validUntil` is more than 24 hours after its
  `validFrom` whoever wrote it; the carrier (slice 3), the lander and `carry show` (slice 4) each call it rather
  than check on their own. In brit the bound is a well-formedness check on the
  clause's signed body (`CovenantClauseBody`), beside the payload check. A grant that expires mid-carry stops
  nothing already claimed: the carry finishes and records its verdict, but no new claim starts under it, and a
  lander does not reuse a verdict recorded after `validUntil` (it runs the gate itself). That compute is spent
  knowingly, for the record. To keep it rare (the author's derivation, for slice 3): the carrier does not claim a
  miss whose gate's p90 is longer than the time the grant has left.
- **Renewal needs new words.** An extension is a new clause whose quote differs from every earlier `grantsCarry`
  clause of the same party on that device, checked against those clauses only (there is no turn log); reusing an
  old quote is refused (today `covenant record` refuses only an empty quote, covenant.rs:1005-1009). A renewal is
  a new grant with its own Commitment. A **revocation** is a clause by the same route whose `revokesClause`
  names the grant's clause CID (brit's field, which takes any clause CID); slice 1 then writes the record that
  settles the Commitment, and the grant is void from that moment, before its expiry. In today's code no
  path writes a `grantsCarry` successor or revocation except `covenant record` run in a session, and slice 1's
  design adds no other writer.
- **How it crosses.** A Commitment stays on the device that wrote it, so the grant crosses as one signed `Cite`
  event on the Commitment, a new kind `grant` under `dev-landing@1` built by the landing module's one event builder
  as the carry claim is (flow/carry.rs, "Two records"), with the Commitment's fields in its env slots (slice 1):
  `env:grant=<commitment cid>`, `env:runner=<did>`, one `env:project=` and one `env:ref=` per entry,
  `env:valid_from=` and `env:valid_until=`, `env:words=<clause cid>`, `env:clause_session=` and `env:clause_at=`;
  and carry claims carry `env:grant=<commitment cid>`. The Cite also carries the clause's session and recording time, so
  a second device has them without the clause. The kind joins `KINDS` (flow/intend.rs) and the policy's
  `berth.channel.crosses` entry. A receiver whose policy lacks `grant` drops the event and counts it, and
  `just status device` shows the Degraded crossing-drop line, whose cure is to refresh the policy and restart
  through `epr device supervise` (device/arrival.rs:2860-2940). Every receiver needs that before its lander can
  check a grant; until then it finds no grant and reuses no carried verdict, so it fails closed.
- **Addresses.** `words:<clause cid>` is brit's own stored clause CID, over the signed clause's canonical bytes
  (brit-epr/src/elohim/attestation/covenant.rs:228-236), signature included. Without the signature every other
  field of the clause is known or crosses (kind, session, party, payload, validity, agent), so a short quote
  ("Carry 8h") would be recoverable by guessing, which would break "quote stays home". The new-words rule reads
  clauses across sessions with `brit-build-ref covenant list --kind grantsCarry` (no `--session`), which brit
  adds in the same release.

**The residual of this stage, accepted by the steward.** Nothing witnesses that he said the quoted words: a
session that types a quote he did not say writes a grant that verifies. The false grant is not caught; it is
**attributable**: the clause names the session that recorded it and that session's claim, so asking that device
which session wrote it, and reading that session's own records, traces the lie to its writer. His rulings accept
this for now ("quoted words suffice, because as long as the words are traceable.. there's a chain of
acconutability.. back to my own act"; the residual "Acceptable, bounded"; "go with option 2 then"). Station
F4-portal-consent removes it.

**Who audits the grant, and when.** The verifier is one read, `epr flow carry verify-grant <cid>` (slice 1, not built), run (1) by the
answering session before it acknowledges the first claim under a grant (the carrier's claim note names the
grant's CID, so a grant the session has not verified is how it learns of a new one) and (2) by the lander before
it reuses a carried verdict.
On the recording device it checks that the clause exists in the covenant notes, is signed, names the steward,
names a session that has an envelope on this device and was moored here as `claude-code` (the most the
device records, and a declaration; nothing records "main conversation"), and matches the Commitment (runner, coverage, expiry); a
lander elsewhere checks the `Cite` event's signature, the recording device's standing for the steward, the expiry
and the coverage. A grant that fails is carried under by nobody: the verifier emits `carry-grant-unverified@1`
naming the session that recorded it, captures feedback naming the steward, and voids the grant with a revocation.

**Station F4-portal-consent (the upgrade, its own work).** A grant is then witnessed by his approval in the
consent portal (`crates/consent-grant`'s ceremony), which needs, per the integrator's sixth read: a presence factor
only he can give for the `carry.grant` act (a passkey, or a secret re-entered with no echo), and refusing
`epr device approve --yes` for it (approver.rs:46 lets any shell on a signed-in device approve today); a field the
`ConsentRecord`'s address covers that holds the Commitment's CID or body hash (optional, so existing record CIDs
hold); resolving the signer against his identity's controllers (a DHT read, or a controller set the roster pins),
since a signature alone proves only the key (verify.rs); a node that approves for the real steward on each device
(the household mesh's matthew is a fixture); the Edge roll that `crates/consent-grant/**` selects, with every
portal upgraded before the new act is asked (an older node refuses an unknown act); and a brit release. It also
moves the crate's boundary from recognition to authority (act.rs:8-9; lib.rs:33-34). Cross-device grants belong to
that station, not this stage: a grant approved on whichever of his devices he is at names exactly one recipient
device; the recipient admits it only when its own roster fold shows both devices bound to the same steward; carry
show verifies the approval against the device it names as issuer. At most 24 hours; a revocation crosses as a new
record; a recipient that has folded no head of the issuer within berth's `freshness_s` takes no new carries.

## The record a carry act leaves, and the trace

| Record | Names | Trace to the steward |
|---|---|---|
| carry claim (crosses) | actor empty; steward = human:matthew; signer = the device did; `env:runner=<did>`, `env:grant=<commitment cid>`, `env:carry=<id>`, the miss, commit, closure | the grant clause names the steward and the session that recorded it |
| gate verdict (`gate-verdict@1`, crosses) | the device signer, the closure, `env:grant` | same |
| feedback item (device-local) | `participant` = the steward; detail key `grant` = the clause CID (the feedback lifecycle spec adds it); `EPR_FEEDBACK_INVOCATION` = the carry id | its closing rule reads `participant` |
| algedonic observation | `carry-unanswered@1`, `carry-grant-expired@1`, `carry-grant-unverified@1` (the clause and the Commitment disagree) | each needs a declaration in the measure registry and a bound, and a slot in the headline, which today renders a closed set (report.rs:100-140, 2367-2385): both are in the carry-watch slice |

The answering session is in none of these records (sessions are replaced; the grant resolves who answers now),
and the device did appears only on what leaves the device.

`epr flow carry show [--device <did>] [--carry <id>]` (slice 4, not built) answers from the folded carry claims: the runner, the
steward, the grant (decoded, its CID recomputed, the clause's session and time), its expiry, the miss, commit,
closure, since when and the outcome. **Who answers now** is not in those records, so carry show reports it only
on the carrying device (from its berth state); from another device it says so and names the device to ask.

A **lander** checks the grant before reusing a carried verdict: a verdict whose grant is expired, void or does
not cover the project is not reused, and the lander runs the gate itself. Without that check the grant would be
accountability only.

## The live agent that answers for it

His "in communication with a live agent", made checkable (today proved twice that a moored session with a
follow can sit for hours without reading a note). Nothing starts or keeps alive an answering session for the
carrier: it is any session Matthew has open on the carrying device, moored by the start hook, that arms the
`berth inbox --wait` wake and acknowledges the carrier's notes. With none, carries decline. The answering
session is **on the carrying device** (a berth
acknowledgement is visible only on the reader's device), and a carry claims only after that session has
**acknowledged** the claim note within 120 seconds (decision 3). The session acknowledges by reading the note
on its own `main` reader (the hook or the `--wait` wake does this). carry-watch learns of it through a berth read
that slice 3 adds, `berth acked --session <S> --note <cid>`, which exits 0 once any reader of that session has
acknowledged that note. No acknowledgement in time declines the carry, recorded. If the answering session goes away
mid-carry, the job in flight finishes (it holds a seat and a worktree; stopping it would waste the work without
making it safer) and records `carry-unanswered@1`; no new carry starts until an answering session acknowledges
again (step 3 below is that check). A note addressed to the carrier needs an address berth does not have today (berth addresses sessions):
`carrier@<device>` routed to the acknowledged answering session is new berth work, listed below.

Three cases a stranger asks about. When several sessions are open on the carrying device, the first to
acknowledge a claim note answers for that carry. A grant voided (revoked, or failing its check) while a carry
under it runs is treated as an expiry: the carry finishes and records, and no lander reuses its verdict. Two
devices each holding a grant for the same miss: whether a second device's claim on a miss already claimed is
refused is not established here (the claim is `epr flow intend --carry`, held-tool-watch:1447); slice 3
confirms it, and if it is not, carry-watch skips a miss for which the fold already holds another device's
unexpired carry claim.

## One attribution resolver

The read feedback capture needs and the carrier needs are one: for whom does this act, and under what
authority, from files only (the session envelope and claim, the roster, and the grant Commitment read from the flow sidecar without taking its lock), within capture's 2 s
and without the store lock. It is the existing resolver extended, not a second one beside it. A read that
cannot resolve refuses the ACT (the carrier does not carry), not the record of a problem: capture stores an
unresolved one as "no participant" within its 2 s, so a tool's report of a problem is not lost.

The same gap already exists in a landed store: of the eighteen `epr feedback` write acts, nine take no named actor
(capture, ingest-genesis, record, drain-spool, link, repair, reclaim, migrate, qualify), an ordinary append can
start a reclaim with no actor named (generation.rs:437-440), and where `--as` is required the check is only that
it is non-empty (mod.rs:285-290): nothing under the feedback code calls `parse_acting_participant` (actor.rs:321),
so `service:jenkins` passes as an author (the feedback lane's acts table,
`genesis/docs/superpowers/sprints/2026-11-27-feedback-contract/fc5-acts-and-authority.md`, confirmed by an
independent reader). It is cited here, not fixed here.

## The carry-watch, its isolation, and which commits it may run

- `epr flow carry-watch` (slice 3, not built) is an ark `processes[]` child under `epr device supervise`. It finds misses by polling,
  as `held-tool-watch carry` does today: it reads the device fold with `epr peer intents --json` every 60 s
  (`DEFAULT_CARRY_EVERY_S`), takes at most one miss per pass, and between passes holds nothing. Correction to the first
  draft: the supervisor does not hold `device-ark`; the session that runs `epr device supervise` claims it
  with a TTL, and the sessionless path refuses (supervise.rs:1434-1462; berth:4030 "no P1 session"). A cargo
  seat held under a grant with no session is therefore a **new berth holder kind**, scoped to `cargo`: its
  holder names the grant's Commitment CID and the carry-watch pid instead of a session, and it is live while the
  grant is valid and that pid is alive (berth's existing pid-liveness check). Its record (slice 3, not built):
  `{"kind": "grant", "commitment": "<cid>", "pid": <n>, "since": <time>, "ttl": <s>}`, and berth is to refuse
  that holder for any resource but `cargo` (slice 3, not built).
- **The bounds he accepted, each with where it is enforced, in this order.** Every refusal records a declined
  carry claim: the same carry-claim record with `env:declined=<reason>`, which crosses like any carry claim and shows in `carry show`. An older
  epr ignores the `env:` tag, so a declined claim also carries a `valid_until` at or before its own time: every
  reader, old or new, sees it lapsed and does not wait on it.
  0. *At record time* (slice 1): a grant longer than 24 hours (`too long`) or whose refs would match `dev` or
     `main` (`shared branch`) is not recorded at all.
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
     a private execution root (a read-only worktree of the commit `origin/dev` named at the runner's last
     fetch, outside the candidate's tree; trusted because every commit on `dev` passed the full gate when it
     landed, and it is not the candidate's; refreshed at the start of each pass, never during a carry; the
     author's derivation, for slice 3), reading the worktree as data (the closure includes cargo-derived and directory
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
  run (`gate-live`). Yielding is what slice 2 built into `held-tool-watch carry` (landed in 59725bf2c): before
  claiming, and repeatedly during the run, it reads `berth who cargo` and the open gate-live runs
  (held-tool-watch:1430-1514); if a foreign holder or an interactive gate appears it stops the gate with
  `gate-live stop --now`, which records no verdict, and releases its claim unrecorded (so a stop is not yet
  recorded once: see the known gaps below). A gate-live run left open by a killed `just gate` reads as live until its
  6-hour staleness window passes (`STALE_MS`, gate-live.mjs:25). The lander sets `GATE_LANDING_INTENT` in
  pre-push.
- **`GATE_CARRY_WAIT=on` waits for the lander's grant check.** Slice 2 (landed in 59725bf2c) gives every dev
  gate its landing intent, but `GATE_CARRY_WAIT` stays `shadow` until the lander checks the grant: turned on
  before that, a matched intent could hold a landing up to the wait's hard cap (1800 s) on a carry no grant
  vouches for. It is turned on by a later commit that flips its default, made by the integrator once every
  lander runs an epr with the grant check (the pin lag below has ended).
- **The lander's check has a pin lag:** a lander running an epr older than this change ignores `env:grant` and
  reuses a carried verdict without checking it, until every lander is upgraded. Until then the grant is
  accountability, not a gate, on those landers. It ends when every device that lands has moved its epr pin past
  the grant check (`just status device` shows each device's epr against the head).

## Runners other than a device, and the work they spawn

His call 2: the acting authority "CAN be more than just a device": Jenkins holds an RBAC service grant of its
own to operate on k8s pods, and the containers such a system creates should be "epr-service compliant, so the
accountability is automatically extendable". Derivation, for his ruling: a **runner** is whatever identity acts
under a grant (a device did today; a system identity such as Jenkins's later). Work a runner spawns (a pod, a
container, an ark task) carries its grant forward: the spawned workload receives the grant's CID and its
parent runner in its environment or labels, its results cite both, and a child grant is a narrowing of its
parent's (a check to build with it: a child naming a project or ref its parent does not cover is refused). Then a result from any depth traces to the runner that started it and to the steward,
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
- **Jenkins:** acts under a grant as a runner, not an actor; its results cite the grant; counting them toward
  sufficiency is a separate ruling, and they count zero until then.

## Open for the steward

Ruled and no longer open: a grant outliving its session and its signed summary crossing without the quote
("Both, quote stays home", ~18:40Z); no log of his turns ("Approve in the portal", ~20:00Z, which also retired the
turn digest); the first stage being a same-device clause, portal consent its own station ("go with option 2 then",
~22:20Z). A grant said on one device for another is not possible in this stage. On 2026-11-28, choosing among
offered options: decision 1, "Beside now, slot later", and decision 4, "Refuse dev and main" (both below, kept for
the record).

Open, each a decision for him, with the options, what each costs, and the author's recommendation. One more
point is not a decision but a gap to know of: whether two devices that each hold a grant can both claim one miss
is not established (above, "Three cases a stranger asks about").

1. **Ruled: the device is named beside the actor slot now, and moving it into the slot is its own later
   station** ("Beside now, slot later"). The options were: (a) beside the slot only, for good; (b) the new
   participant kind now, which makes every older epr refuse the whole flow log until each device upgrades,
   rides an Edge roll (a fleet restart, about twenty minutes of churn), and names the device as an agent; (c)
   (a) now and (b) later, whose cost is the same work later plus a migration of the records written meanwhile.
2. **Ship the first carrier before station F4-limits-answerable?** Nothing observes a carry exceeding its
   declared limits (condition 4, unwired). This is not the risk he already called "Acceptable, bounded", which
   is about a branch's code running on his device; this one is about the carrier's compute going unobserved.
   (a) Ship it, with condition 4 stated as unwired in `carry show`. Cost: a carry could use more of a device
   than intended and nothing would say so; the runner is one of his own devices, whose compute he knows, and the
   grant ends within 24 hours. (b) Wait for F4-limits-answerable, which is not started. Cost: until it lands,
   every missed gate is run by the device that posted it, as today. Recommended: (a); the limits question bites
   first for a runner whose compute someone else affords (Jenkins, a provider).
3. **The liveness standard.** (a) A carry claims only after the answering session acknowledges its note within
   120 seconds, else it declines `unanswered`. The basis is the author's: the `--wait` wake delivered a note in
   9 seconds when first used, and a session busy in a long tool call acknowledges at its next call, so 120 s
   covers ordinary work and declines when nobody is there. Cost: a session in one tool call longer than that
   misses the window, and the posting device then runs the gate itself, which is what happens with no carrier.
   (b) A session with a live wake on the device is enough, with no acknowledgement. Cost: this is what failed on
   2026-11-27: a moored session with a follow read nothing for hours. (c) (a) with a longer bound, e.g. 10
   minutes. Cost: a miss can wait that long before it is declined and the posting device runs it. Recommended: (a).
4. **Ruled: a grant cannot cover `dev` or `main`** ("Refuse dev and main"). In plain words, under the matching
   agreed with brit a `*` crosses `/`, so `self/*` covers every hand-over branch however deep and `*` alone would
   cover every branch. The options were: (a) keep that, so a grant can cover any branch, `dev` and `main`
   included; (b) make `*` stop at `/`, so `self/*` covers `self/x` and not `self/x/y`; (c) keep the matching and
   refuse at record time a grant whose refs would match `dev` or `main`. Slice 1 builds (c) (bound 0 above).
5. **Grant-forwarding for work a runner spawns** ("epr-service compliant", above). (a) As derived: the child
   carries the grant's CID and its parent runner, and a child grant is a narrowing. Cost: a forwarding check in
   every runner that spawns work. (b) Each spawned workload is enrolled on its own. Cost: an enrolment per pod or
   container, and a roster that grows with them. (c) Decide when the first non-device runner is built. Cost:
   none now; the choice is made later, with that runner in view. Recommended: (c); nothing here spawns workloads.
6. **Should a granted Jenkins result ever count toward sufficiency?** (a) It counts zero until condition 4 is
   wired for Jenkins. Cost: a Jenkins result never spares a landing its own gate run until then. (b) It counts as
   a sibling device's does. Cost: a result from compute whose limits nobody can see would count as evidence.
   Recommended: (a), which is what gate-grade.mjs does today.
7. **When station F4-portal-consent starts.** It moves the consent crate's boundary from recognition to
   authority and must answer an agent that drives the browser where his controller is signed in. (a) After
   slice 3 ships and has carried under real grants. Cost: the unwitnessed-quote residual stays until then.
   (b) Now, in parallel. Cost: the portal work, which also rolls the fleet, competes with slices 1 to 5 for the
   same hands. Recommended: (a).

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

Slice 0 is the scenario, before any code: `genesis/a2o/features/devflow/carry-under-steward-grant.feature`,
through the blind-reader loop. Its scenarios: the graduation trigger above; a carry declined with no acknowledged
answering session; a miss declined at each of steps 2, 4 and 5; a grant recorded outside the runner's main
conversation refused; and a grant that does not verify voided and traced to the session that wrote it. Then, each
its own commit: (1) the attribution case, the `grantsCarry` clause kind with its required expiry and new-words
renewal, the grant Commitment and its signed `Cite` event of kind `grant`, and `carry verify-grant`; (2) the
lander's intent and the yields, LANDED in 59725bf2c; (2b) the lander's grant check, NOT built: the lander calls
`verify-grant` before reusing a carried verdict, in the lander's reuse code that the integrator owns
(`gate-carry.mjs`), so it is the integrator's change, after slice 1 exists; (3) carry-watch under the supervisor with steps
1-6, the grant-held cargo lease, the acknowledged answering agent and the three measures with their registry
declarations and headline slot, together, so no carrier ships without an answering agent; (4) carry show; (5)
the `carrier@<device>` berth address. device-runtime/** is an Edge watch glob (holochain build-manifest.json:126),
so slices that touch it ride an authorised roll bundle; so does slice 3's entry in `processes[]` of
`genesis/orchestrator/manifests/runtime/workspace-device.manifest.json`, an Edge watch glob too (holochain
build-manifest.json:127); epr-cli is the eprfs gate; berth is agentic tools; epr-rea is untouched. Slice 1 started in brit, and that part is landed: brit refuses a clause kind it does not know (covenant.rs:14-52,
the closed kind enum and its `FromStr`) and validates each kind's payload (`validate_payload`, :94-115), so the
`grantsCarry` kind, its payload, the body's 24-hour check and the cross-session list came first, in brit 0.3.13,
pinned in f07f678d8. Slice 1's epr work is next. An older brit decodes the clause as unverifiable and lists it so,
which fails closed. Station F4-portal-consent rolls the fleet on its own.
