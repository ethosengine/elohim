---
id: "backlog-security-doorway-auth-required-unenforced"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Manifest route auth_required is stored but never enforced by the doorway forwarder — canonical-head POST reaches the zome unauthenticated"
slug: "security-doorway-auth-required-unenforced"
written: "2026-07-26"
author: "claude (resiliency-saga overnight cure sprint, operator-directed)"
status: "open"
priority: "critical"
ci_status: none
jobs: [elohim-edge]
tags: [security, authorization, doorway, route-registry, canonical-head, declare, auth-required]
cites:
  - doorway/doorway-service/src/server/http.rs
  - elohim/elohim-storage/src/http.rs
  - genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md
---

# auth_required is a dead flag on the doorway forwarding path

## The finding (2026-07-26, overnight saga sprint)

Storage's `build_manifest()` marks write routes `.auth_required()` (POST
`/api/v1/commitments`, PATCH `/api/v1/commitments/{id}`, and as of tonight the
`/api/v1/pins` POST/DELETE pair). The doorway's `RouteRegistry` stores the flag
on `CompiledRoute` — and **no code on the forwarding path ever reads it**
(verified by sweep of `server/http.rs`: the registry-routed dispatch attaches
`ForwardCtx`/`X-Agent-Cid` and forwards; there is no auth check keyed on
`route.auth_required`).

Live proof, both doorways (including elohim.host, which does not set
`DEV_MODE`): `POST /db/content/{id}/canonical-head` — the **Declare-mode
canonical channel that is authorized to move any content head anywhere** —
reached the zome with no credentials (edge evidence: the zome itself answered
with a Guest error, meaning admission passed). Control: the legacy hardcoded
gate on `POST /db/content/{id}/head` correctly 401s. Anyone on the internet can
currently stage a canonical-head declare for any content id on any doorway; the
only thing that saved the fabric tonight is that B's conductor refuses declares
for ids it holds no entry for.

## Why it wasn't patched overnight

The App pipeline's own propagation leg (`authorHeadOnce` → `DECLARE_ONLY`
stage-spa-blob calls with `STORAGE_API_KEY_ADMIN`) uses this exact route.
Enforcing `auth_required` at the doorway without first verifying the pipeline's
X-API-Key acceptance path end-to-end could break the deploy pipeline's
canonical-head propagation — the very leg the convergence arc depends on. The
fix needs one deliberate change with the pipeline verified against it, not a
3am patch.

## The fix shape

1. Doorway forwarder: read `route.auth_required` on registry-routed dispatch;
   enforce the same credential class the hardcoded `/db/content/{id}/head` gate
   uses (admin API key today; delegates-compute commitment per
   `project_rea_compute_commitment_primitive` as the destination state).
2. Verify `stage-spa-blob.sh` sends `X-API-Key: $STORAGE_API_KEY_ADMIN` on the
   canonical-head POST (it sets the env; confirm the header actually rides the
   request) and that a full App-pipeline deploy stays green with enforcement on.
3. Regression: an a2o scenario asserting unauthenticated POST
   `/db/content/{id}/canonical-head` returns 401 on both doorways.

## 2026-09-27: the content body joined this channel
`PATCH /db/content/{id}` still checks no caller identity before re-notarizing through the storage
node's agent. That already let a caller move a row's `reach` and `blobHash`; since the coordinator
update now carries the body, format and tags (and a body-only PATCH on an anchored row routes
through the conductor), it also lets a caller replace the body that peer serves. Network-wide, the
earned-head election still refuses a version its root author didn't declare, so another peer keeps
serving the author's body; the exposure is the row on the peer that took the PATCH. Found by the
code review of the body-carry change (FCT v2 recomposition, 2026-09-27). The cure is this item's:
enforce `auth_required` on storage write routes, so only a steward of the atom can PATCH it.

## 2026-09-27 (later): the graph joined it too
Authored edges now ride an atom's signed metadata and each peer replaces the atom's `explicit`
edges from the adopted head. So an unauthenticated `PATCH {"metadata":{"relationships":[]}}` on a
peer re-signs the entry there with no edges and removes that atom's outbound graph on that peer
(a lesson detached from its course, a prerequisite chain hidden). Bounded to the patched atom's own
`explicit` rows; it forges no other agent's signature and the earned-head election keeps other
peers on the author's version. Same cure: only a steward of the atom may PATCH it. The sibling
`POST /db/relationships` route is equally unauthenticated; its caller-stated edge reach is being
clamped to the source atom's reach (code review, FCT v2 recomposition), but the route still needs
the same authorization.
