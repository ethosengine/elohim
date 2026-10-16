---
title: Graduating onto your own node's key
id: graduate-onto-own-node-key-design
status: Draft
class: architecture
date: 2026-10-16
context-tier: disclosed
steward: human:matthew
author: "Claude (orchestrator), with Codex surveys, at the operator's direction"
tags: [identity, graduation, doorway, agency, imagodei, hosted-human]
graduation-trigger: >-
  The graduates-onto-own-key habit goes green on a household run of
  genesis/a2o/features/auth/graduation/01-onto-my-own-node.feature, and the witnessed key-lineage
  rotation either lands (the doorway follows the identity head) or is ruled out of scope by Matthew.
cites:
  - genesis/a2o/features/auth/hosted-human/05-leaving.feature
  - genesis/a2o/features/auth/recovery/identity-key-lineage-recovery.feature
  - genesis/docs/superpowers/plans/2026-07-17-identity-head-key-lineage-plan.md
  - doorway/doorway-service/.epr-meta/hosted-human-lifecycle.habit.md
  - .claude/skills/p2p-design-gate/SKILL.md
---

# Graduating onto your own node's key

## Why

A hosted human who now runs their own node still has a doorway account pointing at the key the
doorway minted for them. On 2026-10-16 one such account (`hCAkmO-…`, whose cell vanished with a
retired pod) was re-pointed at its owner's node key (`hCAkJH75…`) by a hand edit of the doorway's
Mongo row. That was safe only because the operator and the account's owner were the same person.

Two acts were fused in that edit, and they belong to two parties:

1. **Which key is mine** is the human's act. It belongs on their account page and is proven by a
   signature from the key they claim, never by an operator or an agent writing a row.
2. **What the doorway hosts** is the operator's concern (uninstalling orphaned apps, registry rows
   for conductors that no longer exist, capacity, sleeping idle cells). Those are actuators behind
   the operator seat; they are backlog rows here, not this design.

Rule: the operator manages what the doorway hosts; the human manages which key is theirs; k8s and
Mongo are never the way to do either.

## The ceremony

Three requests, each authorized by the party whose word it carries.

| # | Who answers | Request | Authority checked |
|---|---|---|---|
| 1 | doorway | `POST /auth/node-key/challenge` `{ password }` | the doorway session (JWT) **and** the account password, re-entered now |
| 2 | the human's node (elohim-storage) | `POST /api/v1/identity/account-bind/sign` `{ challenge }` | a proven, **key-bound** node sign-in session (DPoP); the loopback bypass of `signing_caller` does NOT apply |
| 3 | doorway | `POST /auth/node-key/bind` `{ challengeId, agentPubKey, signature }` | the same doorway session, a live unspent challenge it issued to this account, and an Ed25519 signature by `agentPubKey` over the challenge bytes |

Neither credential alone moves anything. An operator with Mongo access but not the human's node
session cannot produce step 2; an agent on the node's machine without the human's doorway password
cannot produce step 1.

### Where the person does it (three surfaces, one redirect round trip)

Two portal rules fix the placement. The doorway portal is the only surface that ever sees a hosted
human's password (`doorway/CLAUDE.md`, two portals), and the node signs only for a session proven
on its own origin (`same-origin.ts`, DPoP over the exact body). So the ceremony crosses the two
portals the way an OAuth hop does:

1. **Account settings in the app** (`app/elohim-app` imagodei profile, agency section). While the
   account is hosted, the "Upgrade" panel offers *Move this account onto my node* and sends the
   person to the doorway portal's account page with `#move-to-node`. After the move it shows
   Node Steward (the agency stage reads the account's `agencyPhase`).
2. **Doorway portal account page** (`doorway/doorway-app`, `DoorwayAccountComponent`). The person
   types the address of their node's portal and their doorway password. The page calls step 1,
   then navigates to the node portal:
   `https://<node>/consent/account-bind?challenge=<base64url of the challenge text>&return=<this page's URL>`.
3. **Node portal** (`app/imagodei-portal`, a new `consent/account-bind` mode). If the person is not
   signed in to the node, the portal's own sign-in runs first. It then shows, in words, which
   doorway (`audience`), which account (`identifier`) and until when, and asks for approval. On
   approval it calls step 2 same-origin with the session key's DPoP proof and navigates back to
   `return` with `#challengeId=…&agentPubKey=…&signature=…` (signature base64url), or
   `#refused=<CODE>` on any refusal or on decline. The fragment never reaches a server log.
4. Back on the doorway portal account page, the fragment triggers step 3 and the page shows the
   result; on success it stores the fresh session the bind returned.

A signature that reaches the wrong page is useless on its own: the bind needs the account holder's
doorway session (`CHALLENGE_NOT_YOURS`) and the challenge is single-use.

### The signed bytes (wire contract, v1)

The challenge is UTF-8 text, LF-separated, fields in exactly this order, no trailing newline:

```
elohim:doorway-account-bind:v1
audience=<doorway id, or its gateway domain when no id is configured>
identifier=<the account identifier>
human=<UserDoc.human_id>
from=<current UserDoc.agent_pub_key, uhCAk…>
nonce=<64 lowercase hex chars, 32 random bytes>
expires=<RFC3339 UTC, seconds precision, e.g. 2026-10-16T20:15:00Z>
```

No field value may contain a newline. The doorway returns `{ challengeId, challenge, expiresAt }`;
`challengeId` is the nonce.

The node signs the challenge bytes exactly as received with `sign_raw` under its agent key. It
refuses (400) anything that does not parse as the seven lines above, that is over 1 KiB, or whose
`expires` has passed, so the route can never be used as a general signing oracle. It returns
`{ agentPubKey, signature }` — `agentPubKey` as `uhCAk…`, `signature` as standard base64 of the
64 raw bytes.

The doorway verifies with the 32 key bytes inside `agentPubKey` (bytes 3..35 of the 39-byte
decoded key, `conductor::agent_key::decode_key_bytes`) over the challenge text it stored, never
over text the client sends back.

### What the bind does

On a valid signature, in one compare-and-swap on the account row filtered by
`{identifier, agent_pub_key: from}` (a concurrent bind or close loses cleanly):

- `agent_pub_key` ← `agentPubKey`; `agency_phase` ← `node`; `conductor_id` ← null;
  `is_steward` ← true and `stewardship_at` ← now; `token_version` += 1;
- `node_key_bound_at` and `previous_agent_pub_key` are recorded on the row so the account page and
  an operator view can say what happened.

Older sessions are not revoked. JWT validation does not read `token_version` today (the admin
routes increment it the same way, to the same lack of effect), so a session minted before the bind
keeps verifying until it expires, still naming the old key. What bounds it: `/hc/connect` reads the
account row, refuses a `node`-phase account (`not-hosted-here`, 409), and the old cell is gone. Making
`token_version` a checked claim is on the frontier below.

Then, best effort and after the swap: the old hosted app is deprovisioned from its pool conductor
(`AgentProvisioner::deprovision_agent(from)`), and the custodial signing keys cached for the human
are deactivated. The response carries a fresh session minted for the new key.

The challenge is single-use (consumed on the first bind attempt, valid or not) and lives 5
minutes in a process-local store shaped like `session_transfer_store`.

`/hc/connect` must never auto-provision a hosted cell for a `node`-phase account: after
graduation the doorway is a router for that human, not their host.

### Refusals, named

`challenge`: `NO_TOKEN`/`INVALID_TOKEN` 401, `WRONG_PASSWORD` 403, `NOT_HOSTED` 409 (the account is
not in the hosted phase, or `is_steward` is already true: graduated here, or already holding the
hosted key itself after exporting it, or never hosted).
`sign` (node): `NO_SESSION` 401, `SESSION_NOT_BOUND` 403, `PROOF_REFUSED` 403, `NOT_A_BIND_CHALLENGE`
400, `CHALLENGE_EXPIRED` 400, `NO_CONDUCTOR` 503.
`bind`: `CHALLENGE_UNKNOWN` 404 (unknown, spent or expired), `CHALLENGE_NOT_YOURS` 403,
`SAME_KEY` 409, `SIGNATURE_INVALID` 403, `KEY_MOVED` 409 (the CAS lost).

## Next slice: release what this doorway still hosts for me

The bind deprovisions the one app under `from`. Earlier runs, failed registrations and re-provisioning
can leave more apps installed for the same person across the pool. Account settings offer
*Release what this doorway still hosts for me*, the human's own act on resources that are provably
theirs:

- `GET /auth/hosted-apps` lists, across every registered pool conductor, installed apps whose agent
  key equals the account's `agent_pub_key` or `previous_agent_pub_key`, or whose registry row records
  this account's `human_id`. Each row: conductor, app id, agent key, why it matched.
- `POST /auth/hosted-apps/release` `{ password, appIds }` re-checks each id against that same list and
  deprovisions it; an id not on the list is refused (`NOT_YOURS`). For a hosted account, the app its
  current cell runs on is refused (`IN_USE`) — closing the account is the way to release that one.
- Never matched by the identifier-derived app id alone: a closed account frees its name, and a new
  person may hold it now.

Apps no signed-in human can prove are theirs (the other identities' leftovers, registry rows for
conductors that left the doorway's list) stay with the operator seat:
`genesis/data/timeline/backlog/doorway-hosted-capacity-bound-from-pod-memory.md`.

## What this does NOT do (the frontier)

- **No witnessed lineage.** Nothing on the network links the old key to the new one, so anything
  the old key wrote is unlinked from the human. The deeper version is key lineage
  (`rotate_identity_key`, plan cited above): the account points at the identity's chain-root and a
  key change is a witnessed rotation the doorway follows. The bind keeps `previous_agent_pub_key`
  so that rotation can be minted later from what the doorway saw; it is not itself that record.
- **External-node routing.** When the node's key sits on a conductor the doorway does not pool,
  `/hc/connect` has nothing to route to; the human uses their node's own portal. The story's
  household case binds onto a conductor the doorway already routes to.
- **Older sessions stay valid until they expire.** `token_version` is incremented but no validator
  reads it; checking it means a claim in the JWT and an account lookup on every verify. Until then
  the bound is the `/hc/connect` refusal described under "What the bind does".
- **Password guesses at the challenge** are limited only by the membrane that shapes every request,
  the same as `/auth/login`. The challenge additionally needs a live session for that account.
- **The challenge store has no size cap**; it is pruned of expired entries on insert. One entry per
  request from an authenticated, password-proven caller.
- **Accounts with no declared `agency_phase`** (rows older than 2026-10-15) are still auto-provisioned
  by `/hc/connect`; only a declared non-hosted phase is refused. Such an account cannot graduate
  either: `NOT_HOSTED` answers until its phase is declared.
- **The node's browser approval is enforced by storage, not by the zome.**
  - `sign_account_bind_challenge` signs for any caller that holds a capability grant on the
    imagodei cell. Storage's own grant comes from `authorize_signing_credentials` with
    `functions: None` (`elohim-storage/src/hc_client.rs`), and it is all-functions.
  - The extern re-checks the challenge grammar and its expiry (at most 10 minutes ahead). It can't
    check the provenance, because storage calls it with a signing key it generated, not the agent key.
  - On a node, a new grant needs the node's admin interface, which belongs to the node's owner.
  - On a hosted conductor, the doorway can make a hosted cell sign. That signature names the
    account's current key, which is refused as `SAME_KEY`, or another hosted agent's key. Getting a
    challenge still needs the account holder's JWT and password.
  - Closing this means a function-listed grant for storage plus a consent check the zome can verify.
    Nothing has tested the bypass yet.
- **Operator remedies** (stuck-account diagnostic, registry cleanup) are backlog rows:
  `genesis/data/timeline/backlog/doorway-registry-prune-matches-app-id-only.md`,
  `genesis/data/timeline/backlog/doorway-hosted-capacity-bound-from-pod-memory.md`.

## P2P design gate

### Entity: doorway account → agent key binding (`UserDoc.agent_pub_key`)
- **Classification**: Ephemeral (C) at the doorway — a federation-plane projection. The doorway is a
  relying party; the row is reconstructable from the human's own word (a fresh bind) and is not the
  identity's truth. The notarized truth it should eventually follow is the identity-lineage head
  (A, imagodei), out of scope here.
- **Head-Plane Cost Budget**: n-a (no DHT entry, no head).
- **Network Stakes**: all four stages; the signature check is floor-protected and never cheapens,
  Simulacra included — a key binding is constitutional-shaped.
- **Content Address Strategy**: agent key (`uhCAk…`, Holochain hash) — the thing bound IS a key.
- **Source of Truth**: Mongo (operational) at the doorway; the human's node holds the key.
- **Integrity Zome + DNA-hash class**: none touched. **DNA-hash-NEUTRAL.**
- **Coordinator Zome**: `imagodei::sign_account_bind_challenge(String) -> SignedAccountBind`
  (new, coordinator-only, hot-swappable). It re-checks the purpose line in-zome so no caller of the
  cell can turn it into a general signer. `sign_for_agent` is not reused: it requires an Agent EPR a
  node may not have registered, and it signs arbitrary bytes.
- **Projections**: none.
- **HTTP Routes**: `/auth/node-key/*` in doorway-service (account auth is doorway-specific logic);
  the node route declared in elohim-storage `build_manifest()`.
- **Anti-Pattern Check**: cross-namespace identity — the binding stores and compares `uhCAk…`
  only; the doorway JWT subject is resolved to the row by identifier, never string-matched to a
  key. No sovereignty apex: the stage is "Node Steward", a key-location fact.
- **SDO/RWA Test**: the worst holder of the doorway learns which key an account moved to and when
  (it already sees which key it hosts). It cannot compel a move, because it cannot produce the
  node's signature. It can refuse to honour one; the human's node portal still works without it.
  Bounded by boundary 5: correlating the two keys is the human's act, the bind itself.

### Entity: account-bind challenge
- **Classification**: Ephemeral (C), process-local, 5-minute TTL, single-use. Losing it on restart
  costs the human one retry.
- **Address**: random nonce (Option 3 — an operational token with no content to hash).

### Concern canon (Step 4)
C1 anti-self-election: answered — the doorway never picks the key; the key signs for itself.
C4 honest absence: answered — each refusal is a named code. C6b idempotent effect: answered — CAS
on `from`; a replayed bind finds the challenge spent. C9 identity-lineage continuity: **partial** —
`previous_agent_pub_key` is kept, no witnessed rotation. C12 consent/authorization: answered — two
credentials from two parties. C14 witnessed residual: partial — the row records the move, the
network does not. The remaining classes are n-a (no gossip, no backpressure surface, no contract
evolution). Seam rows: `doorway/doorway-service` (`node_key_bind_verdict`) and `elohim/elohim-storage`
(`account_bind_challenge_parse`).
