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

## Open for the steward

Whether a grant may name a narrower set of gate projects than every miss, and whether a second device of the
same steward may issue a grant to the first (today only the device's own live session can).
