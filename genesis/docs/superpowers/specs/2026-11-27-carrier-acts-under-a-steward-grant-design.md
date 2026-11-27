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
  a miss posted by one device's hand-over is taken, gated and answered by a second device's supervised
  carry-watch with no note sent and no session cued; the carry claim, the verdict and any feedback it
  captured each name human:matthew as participant and cite the grant by CID; and from the first device
  `epr flow carry show` answers who carried it, under whose grant, until when, and which live agent
  answered for it
cites: []
---

# The carrier acts as its device under a steward's expiring grant, and a live agent answers for it

Plan station F4-ask-unattended (`2026-10-29-native-dev-tooling-over-peers-plan.md`) wants a device to take a
posted miss with no session told. The script that does it today (`held-tool-watch carry`) has three defects
(no build role read, yields only to `dev-integration`, nothing sets `GATE_LANDING_INTENT`) and could not hold a
cargo seat because no session stands behind it. This page settles who acts, under what, and how it is asked.

## The steward's ruling

Said by human:matthew on 2026-11-27, about 17:35 to 17:45Z, in ethosengine session
`d6cfb7e1-ee3c-4e45-bfb2-ae04ec4b50a3`, as typed:

> a service agent sponsored by the device/working under the steward's authorization... is the right instict
> there, the point though is it has to traceback to the steward of record somewhere.. so if something is
> happening somewhere, it can be asked.. what are you doing here, and acconutability is always there.. and
> it's possible the question/feedback can get get back to the steward if warrented/carried back by the
> chain. I don't think we have a direct consumer for this yet beside ourselves.. but that's a deliverable of
> the elohim protocol so that wholistic limits can be understood in relationship to the human stewards

> service agents need epr-level acconutability.. they just don't have an "agent" of themselves.. they need to
> be in communication with a live agent that can review/answer for th stward.

> oh just a note.. there's feedback and algedonic signals..

Read together: the carrier is a service, not an agent, and this design names it as none; its acts are accountable at
the epr level and trace to the steward of record; a live agent reviews it and answers for the steward; what
must travel back to the steward travels by the channels that exist, feedback and algedonic signals.

## Who acts: the actor options and what each costs

| Option | What it is | Cost | Verdict |
|---|---|---|---|
| A. new kind `device-agent:<role>@<did>` | a fifth `ParticipantRef` in epr-rea | a new kind makes an older epr refuse the WHOLE flow log (`FlowRecord` has no catch-all, store.rs:22-32; an unknown prefix parses as a malformed agent, actor.rs:310); epr-rea selects an Edge fleet roll (holochain build-manifest.json:131); and it names the service as an agent, against the ruling | rejected |
| B. `agent:carrier@<build>` | the agent form with a build for a model | misstates the actor: no AI model stands behind it; the ruling says it has no agent of its own | rejected |
| C. `service:carrier` | the provenance form | `service:` is not an actor by definition (actor.rs:269-272; enforced: elohim/epr-rea/src/actor.rs `parse_acting_participant`, which refuses it in acting position at :321); changing that invariant reopens the Jenkins boundary | rejected |
| D. **the device as itself, under a steward grant** | the device's did:key acts; attribution resolves through the roster to the steward | no ontology change: with no session, attribution already falls back to the device's roster standing, actor and steward = the standing human (`resolve_attribution_on` → `Attribution::standing`, flow/note.rs:1419-1452), and carry claims already use that path (flow/carry.rs:349-350); needs a home for the grant (below) | **recommended** |
| E. the Holochain `delegates-compute` grant | the one native grant today (elohim-storage api/compute_grants.rs:85-161) | its recipient is a Holochain AgentPubKey, not a did:key, and it lives behind the storage daemon a device may not run; the right graduation target once grants are DHT records, not today's home | later |

## The grant: what it is and where it lives

The carry switch becomes a **grant**: a small signed document the steward of record's words create.

```json
{ "grant": "carry@1",
  "granted_by": "human:matthew",
  "to_device": "did:key:z6Mk…",
  "role": "carrier",
  "quote": "Carry 3h, move checkout",
  "said_in": "session d6cfb7e1-…",
  "valid_from": "2026-11-26T18:00:00Z",
  "valid_until": "2026-11-26T21:00:00Z",
  "commits": { "posted_by": "standing-sibling", "reachable_from": ["refs/remotes/origin/dev", "refs/remotes/origin/self/*"] },
  "signature": "<device key over the canonical body>" }
```

- **Home.** `<device config>/carry-grant.json` beside `build-role.json`: device-local and untracked (the device config directory is outside the checkout), read
  from a file with no store lock. Its CID (`BlobCid` of the canonical body) is the grant's address.
- **Issued** by `epr device carry-grant --until <RFC3339> --quote "<the steward's words>"` from a live session
  standing for that steward on that device; it refuses with no `--quote`, no standing, or an unbounded or past
  expiry. Revoked by `--revoke`, which writes a successor with `valid_until` now.
- **Expires** by its own `valid_until`: the carrier reads it before every claim and every pass, and an
  expired, missing or unverifiable grant means no carry: the carrier refuses rather than guesses. The build role stays the eligibility
  (`carrier` or `builder`; `neither` refuses), so an expiring opt-in sits beside it with no schema change to
  `build-role.json`, whose schema forbids `until` for `carrier`.
- **Crosses** without a schema change: every carry claim already carries `env:` tags in `classified_as`;
  it adds `env:grant=<cid>` and `env:grant-doc=<base64 canonical body>`, the precedent being
  `env:requires-doc` on a produce record. An older epr reads it unchanged.

## The record a carry act leaves, and the trace

| Record | Names | Trace to the steward |
|---|---|---|
| carry claim (crosses) | actor = steward via device standing; signer = device did; `env:grant=<cid>`, `env:grant-doc`, `env:carry=<id>`, the miss, commit, closure | grant `granted_by` = the roster standing of the signing device; both must agree or the claim refuses |
| gate verdict (`gate-verdict@1`, crosses) | the device signer, the closure, `env:grant=<cid>` | same |
| feedback item (device-local) | `participant` = the steward the grant traces to; detail key `grant` = the CID (the feedback lifecycle spec's allow-list adds it); `EPR_FEEDBACK_INVOCATION` = the carry id, so one record per carry | the closing rule reads `participant`, so the item can close |
| algedonic observation | measure `carry-unanswered@1` when a carry ran with no live answering session, `carry-grant-expired@1` at expiry mid-carry | read by `epr flow report`, which every session's SessionStart headline prints |

The answering session is deliberately in none of these records (sessions are replaced; the grant resolves who
answers now), and the device did appears only on what leaves the device.

**Asked from another device**, `epr flow carry show [--device <did>] [--carry <id>]` answers from the folded
carry claims alone: the device, the steward it stands for, the grant (decoded from `env:grant-doc`, its CID
recomputed and its signature checked against the device did), its expiry, the miss, commit and closure, since
when, the outcome so far, and which live agent answers for it now (below).

## The live agent that answers for it

The ruling's "in communication with a live agent": a carry starts only when at least one live session is moored
on the carrying device standing for the same steward (`berth status`). That session is the carrier's answering
agent for the duration. At claim the carrier sends it one berth note (what it took, under which grant, until
when); at the end, one with the verdict. A berth note addressed to the carrier is delivered to its answering
agent, which answers from `epr flow carry show` and the carry's log. When no live answering session exists, the
carrier does not start; if the answering session ends mid-carry, the carry finishes, records
`carry-unanswered@1`, and the next session's headline shows it. What the answering agent cannot settle it
captures as feedback naming the steward, which is how a question is carried back.

## One read: for whom does this act, under what authority

Designed once, used by feedback capture and by the carrier alike: `acting(root) -> { participant, via:
session <id> | grant <cid>, device, expires }` in device-runtime, from files only (the session envelope and
claim, the roster, the grant), inside capture's 2 s budget and without taking the store lock. A session resolves
through its envelope and claim, a carrier through its grant; neither resolving is a refusal, not a guess.

## The supervised carry-watch, and which commits it may run

- `epr flow carry-watch` is an ark `processes[]` child under `epr device supervise`, not a session hook's
  detached runner; it holds its cargo seat as the device process the supervisor already is (the supervisor
  holds the `device-ark` lease today), with the grant as its authority. Berth's live-P1-session check stays
  for sessions; the grant-held lease is the one new holder kind, scoped to `cargo`.
- It yields to any held cargo lease that is not its own, to `dev-integration`, and to an open interactive
  gate run (`gate-live`), not only to `dev-integration`.
- It runs only a commit that a miss names AND (a) the miss was posted by a sibling device standing for the
  same steward, (b) the commit is fetched by its exact id and reachable from a ref the grant's `commits`
  names, (c) the recomputed closure equals the miss's key. Anything else is refused and recorded.
- The lander sets `GATE_LANDING_INTENT` in pre-push, so its bounded wait for a carried verdict is reachable.

## What changes where, and what rolls

device-runtime (the `acting` read, the grant document): an Edge input, so it rides a landing that already
pays those gates. epr-cli (`carry-grant`, `carry show`, `carry-watch`, capture's grant hand-off): the eprfs
gate. berth (the grant-held cargo lease): agentic tools. epr-rea: nothing (no new kind, no new field), so no
pin-lag refusal and no roll from it. Slices, each its own commit: the grant and `acting`; the lander's intent
and the yields; `carry-watch` under the supervisor; `carry show`; the answering agent and the two measures.

## Working postures on the open questions

The steward, asked these on 2026-11-27, said he was not sure and asked for each to be green- and red-teamed to
a working posture, adding (as typed): "i'm thinking jenkins sholud have it's grants.. as long as it's results
can carry the accountability, and it's runtime can be asked.. and can reach the elohim network to consider and
answer the question if necessary then I think it's ok." And then: "because jenkins has access to pods and
such.. whatever compute it has available.. as long as it's own limits are anserable to the elohim network..
then it can be honestly subject to governance and trust if it does something that an agent with an
acconutable steward can undrstand, answer and respond to." Those are the test every grant must pass, whoever
holds it: **(1) its results carry the accountability** (each cites the grant, and the grant names the
steward); **(2) its runtime can be asked** (a question about any result reaches something that can answer from
the records); **(3) it can reach the elohim network** to consider and answer a question when one comes; and
**(4) its own limits are answerable to the network**: the grant declares, or cites a declaration of, the
runner's compute (cores, memory, concurrency, the pods it can schedule) and its reach (what it can touch:
namespaces, registries, deploy rights, secrets), so that an agent with an accountable steward can understand
what the runner can do, answer for what it did, and respond when it goes beyond its declaration. A run beyond
its declared limits is recorded as an algedonic breach, and that is what makes it honestly subject to
governance and trust rather than trusted blind. For a device carrier the declaration is its capacity vector
(plan row 16: declared once in its ark RuntimeManifest, read by dispatch). A posture below is a working
default until the steward rules otherwise.

### Q1. May a grant cover only some gate projects?

- **Green.** Least privilege, and the ruling's "wholistic limits" in practice: a steward can let a small
  device (a repurposed Chromebook) carry the cheap gates and leave the 45-minute storage gate to a larger one; it mirrors the
  scope list the one native grant already has (`GRANT_SCOPES`, elohim-storage api/compute_grants.rs:46); and a
  narrowing can only reduce what runs.
- **Red.** A narrower grant leaves misses nobody carries, and a silent skip would look like an idle carrier;
  gate projects are renamed, and a list of stale names would quietly cover nothing; and capacity admission
  (ram-guard, io-guard, the build role) already limits what a device takes, so two filters can disagree.
- **Posture: yes, narrowing only.** `projects` is `"*"` by default or a list of gate project names, checked
  against the gate registry when the grant is issued (an unknown name refuses the issue) and again at claim
  (a name that no longer exists is reported). Capacity admission still applies on top; in this design a grant does not add
  what the build role or the device's capacity refuses. A miss the grant does not cover is recorded as
  `declined-by-grant`, visible in `epr flow carry show`, rather than skipped silently.

### Q2. May one of the steward's devices issue a grant to another?

- **Green.** It is how a headless device (the Chromebook fleet the steward opened on 2026-11-25, a media PC)
  joins without anyone opening a session on it; the roster already proves both devices are the steward's;
  and it adds little attack surface, because a sibling standing for the same steward can already post misses
  the other device would carry.
- **Red.** The ruling wants a live agent answering for the service, and a headless recipient has none of its
  own; a recipient whose roster has not folded the issuer's binding cannot verify it; a revocation issued
  while the recipient is offline does not reach it until it reconnects; and one compromised device could
  grant itself work on another.
- **Posture: yes, bounded.** The issuer is a live session standing for the same steward on any of the
  steward's bound devices; the grant is signed by the issuing device's key and names exactly one recipient
  device. The recipient admits it only when its own roster fold shows the issuer bound to the same steward
  (it does not fetch anything to decide). The answering live agent may be on the issuing device, so a headless carrier
  is answered for by the issuer's session, and with no live answering session on either device it does not
  start. A cross-device grant is short (at most 24 hours), a revocation crosses as a successor grant, and a
  recipient that has folded no head of the issuer within berth's `freshness_s` stops taking new carries.

### Q3. Jenkins has its grants too

- **Green.** It is the same shape: a service with no agent of its own, acting under a steward's grant. It
  gives Jenkins's results the accountability they lack today: `gate-grade` counts a sponsored runner's result
  as zero (genesis/orchestrator/gate-grade.mjs:17-19) because nothing traces it to a principal.
- **Red.** `service:jenkins` is provenance and not an actor (actor.rs:269-272), and making it one would
  reopen that boundary; counting its results toward sufficiency would change what work is skipped, which is
  integrity-class; and Jenkins today reaches the network only through what the jenkins bridge harvests.
- **Posture: yes, as a grant, not as an actor.** A grant may name `service:jenkins` as its runner, with the
  steward as grantor, the pipelines or gate projects it covers, and an expiry. Its results cite the grant
  (condition 1), it is asked through the answering live agent that reads its harvested results (condition
  2), and it reaches the network through the bridge that already harvests and offers its records (condition
  3). Its grant declares its limits (condition 4): the compute its agents and pods have, and its reach, which
  for Jenkins is the larger part (it schedules pods, pushes images, deploys to the alpha fleet). That
  declaration is read from the repository's manifests (`genesis/manifests/`, the Jenkinsfiles' agent and pod
  templates), never from the live cluster, since cluster reads are the operator's; a result the bridge
  harvests from beyond the declared reach is an algedonic breach and does not count. `service:jenkins` stays a non-actor: the grant is the authority, the steward the principal. Whether a
  granted Jenkins result then counts toward sufficiency is a separate ruling, not taken here; it counts zero
  until the steward rules.
