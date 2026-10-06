---
id: "backlog-security-node-session-routes-unauthenticated"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A node's session routes answer anyone who reaches its port: sessions are created for whoever asks, their ids are handed out, and any caller can sign the person out"
slug: "security-node-session-routes-unauthenticated"
written: "2026-10-05"
author: "claude (device-authorization-grant branch, operator-directed)"
status: "backlog"
priority: "high"
tags: [security, authentication, sessions, elohim-storage, loopback, device-consent, trustful-self]
relatedNodeIds:
  - arch-device-recognition-backlog
  - arch-confidentiality-plane-backlog
  - genesis/a2o/features/auth/device-provisioning-paths.feature
  - genesis/a2o/features/auth/device-consent-grant.feature
cites:
  - elohim/elohim-storage/src/http.rs
  - elohim/elohim-storage/src/main.rs
  - elohim/elohim-storage/src/services/device_consent.rs
  - elohim/elohim-storage/src/services/node_account.rs
  - crates/consent-grant/src/signin.rs
  - elohim/elohim-storage/src/services/session_exchange.rs
  - genesis/orchestrator/manifests/network-policies.yaml
  - genesis/orchestrator/manifests/edgenode/alpha.yaml
  - app/imagodei-portal/src/app/services/steward-login-controller.ts
  - genesis/docs/superpowers/specs/2026-09-05-two-portals-sso-consolidation-design.md
---

# A node's session routes answer anyone who reaches its port

## What was open (before sign-in, kept as the record)

elohim-storage keeps the person's sign-in on their node as a `local_sessions` row.
The routes that manage it trust whoever can reach the HTTP port:

- **`POST /session` asks for no proof** (`http.rs`, `handle_create_session`). It writes a session
  for whatever `humanId`, `agentPubKey` and `doorwayUrl` it is sent and makes it the active one.
  It exists for the Tauri shell, which calls it from localhost
  (`steward/device/src-tauri/src/lib.rs:868,1105`; `tauri-auth.service.ts:431`).
- **`GET /session` and `GET /session/all` return session ids** to any caller, so an
  `elohim_session` cookie is not a secret on this server.
- **`DELETE /session` from anywhere** deactivates the active session: anyone on the network can
  sign the person out.
- **With no cookie, the single active session stands in for every caller**
  (`resolve_local_session`, the same fallback `/auth/me` uses).
- **The port is reachable off the machine in every deployment**: storage binds `0.0.0.0`
  (`main.rs:4226`). For a Tauri sidecar that is the LAN. In Che, the devfile's `hc-storage`
  endpoint is `exposure: public` and not `secure`. On the household mesh it is the pod network.
  For a deployed edge pod it is same-namespace pods, the ingress namespace and Jenkins
  (`network-policies.yaml` ~60–105; the `storage-db` Service on 8090 in `edgenode/alpha.yaml:443`).

The one real proof of a person storage has is `/session/exchange`: a doorway-issued handoff token,
redeemed back-channel at the issuing doorway (the native portal's sign-in,
`steward-login-controller.ts` → `standalone-resolver.ts:154`). Today it cannot carry anything
that matters:
- its doorway allowlist is seeded from `local_sessions` history, which `POST /session` writes, so
  a remote caller can add an attacker's doorway to the allowlist;
- its session id can be read back through `GET /session`;
- it does not check that the redeemed agent is this node's own.

**Live evidence (2026-10-05, isolated single-conductor run under
`genesis/local-dev/device-consent/`):** `POST http://10.1.19.170:8191/session` from a
non-loopback address answered **201** and became the active session. `DELETE /session` cleared it.

## Closed first: the signing routes

Since commit `404962377`, the three routes that make the node sign as its person (approving a
device, bootstrapping an identity, beginning one) answer only callers on the node's own machine:
`POST /auth/consent/agree`, `POST /auth/identity/bootstrap` and `POST /auth/identity/begin`. Any
other caller gets **403 `consent_caller_not_local`** before anything else, whatever session it
presents. The device-side steps (`/auth/device/self`, `/auth/device/enroll`) and the declaration
read (`/auth/identity/declaration`) were loopback-only from birth.

Live, from 10.1.19.170: begin, bootstrap and agree were each refused 403 even after a forged
`POST /session`.

The doorway-hosted path is not exposed by this hole and was not changed by the fix. The doorway
signs through its own conductor connection, and storage's `build_manifest()` declares none of
these routes, so the doorway forwards none of them.

## Closed 2026-10-05: sign-in, and the session routes

- **The node signs its own person in** (the OAuth model: the node is the authorization server for
  its own person). `POST /auth/identity/secret` (this machine only; `epr identity secret` reads the
  secret without echo or from stdin, never from an argument) keeps an Argon2id verifier in the
  node-local `node_account` table, the lasting home of the sign-in word. `POST /auth/login` (the
  doorway's request shape) checks the word and secret for this node's own person only, with a
  per-source and per-account growing delay, constant-time word comparison, and one answer for a
  wrong word and a wrong secret; it opens a session proven by sign-in (thirty days, ended by
  `POST /auth/logout` or a new secret) in an HttpOnly, SameSite=Strict cookie, `Secure` over TLS.
- **Who may make the node sign:** a caller on this machine, or a request carrying a session proven
  by sign-in (`consent_grant::may_make_node_sign`, asked through one function,
  `HttpServer::signed_in_by_proof`). A session from `POST /session` or `/session/exchange` proves
  nobody to another machine.
- **Items 1 and 2 above are closed:** `POST /session`, `GET /session`, `GET /session/all`,
  `DELETE /session` and `POST /session/intent` answer only callers on this machine (403
  `session_caller_not_local`); the no-cookie fallback to the single active session applies only to
  callers on this machine. Their callers, all on this machine: the Tauri shell
  (`steward/device/src-tauri/src/lib.rs`, `http://localhost:8090`), its webview
  (`tauri-auth.service.ts`, `environment.client.storageUrl` defaulting to localhost) and the
  connection strategy (`tauri-connection-strategy.ts`, localhost). `/session/intent` has no caller
  in the tree.
- **Live (2026-10-05, isolated stack, from 10.1.19.170):** `POST /session` and `GET /session`
  answered 403; sign-in over plain http was accepted with the warn line logged; a pending device
  was approved from that session; after sign-out the same cookie was refused 403 by decide and
  agree, and `/auth/me` answered 401.

## Closed 2026-10-05: a copied cookie cannot make the node sign

A sign-in session may be bound to a key the browser holds (RFC 9449 DPoP, adapted to a cookie);
over TLS it must be. A bound session must present a valid proof from its key on every route that
makes the node sign and on sign-out. What a stolen bound cookie can still do is read
(`/auth/me`, standing, the pending list): device-recognition row 9.

## Worse than recorded (2026-10-06): loopback is the person, with no session

The adversarial review found that "this machine" meant any TCP peer on loopback. Closed now: every
identity route answers only under this node's names (`ELOHIM_ALLOWED_HOSTS` adds more), so a page
that rebinds its name to 127.0.0.1 reaches none of them; Origin must equal Host exactly; a
forwarded or trusted-proxy request is not local. Kept, by operator ruling (keep the bar low for
now): anything that reaches loopback under a loopback name, a local process above all, still acts
as the person with no session. The install-token dial that would close it, with its steps and
cost: [arch-device-recognition-backlog](epr:arch-device-recognition-backlog) row 19, dial 1.

## Must-have before floor readiness: no secret or cookie in the clear

**Required, not optional hardening.** A sign-in secret and a session cookie must not cross a
network in the clear. Today a sign-in from another machine over plain http is **allowed** by
operator ruling (2026-10-05: no real secrets, no real networks at risk) and logged at warn once per
sign-in (`sign-in: a sign-in secret crossed a network in the clear`). The decision is one place,
`consent_grant::sign_in_channel_verdict` with `PLAIN_SIGN_IN_ALLOWED = true`; closing it is that one
line. What would satisfy the requirement:
- the node terminating TLS itself (today it terminates none; the only TLS it can know of is a
  proxy named in `ELOHIM_TRUSTED_PROXIES` that sends `X-Forwarded-Proto: https`), or
- a sign-in exchange that never sends the secret: a password-authenticated key exchange, or a
  key-bound sign-in (the session already carries `proven_by` and an always-empty `bound_key` for
  that step).

This intersects the unbuilt encryption layer:
[arch-confidentiality-plane-backlog](epr:arch-confidentiality-plane-backlog) row 13.

## What remains

1. **The secret in the clear**, above.
2. **"This machine" includes any proxy on it.** A doorway or a reverse proxy on the node's own
   machine makes every caller it forwards look local. The doorway forwards only routes storage's
   `build_manifest()` declares, and none of the signing or session routes is declared; a reverse
   proxy the operator adds in front of storage would need `ELOHIM_TRUSTED_PROXIES` and the loopback
   rule would still trust it.
3. **Passkeys, a second factor, sign-in from a device that is itself enrolled** (the device's key
   proving the person, no secret at all), and **TLS on a home LAN** (no public name, so no
   ordinary certificate).
4. **Bind storage's HTTP to loopback by default** where nothing off the machine needs it (Tauri).
5. `/session/exchange`'s allowlist seeding is closed now that `POST /session` is local-only; its
   redeemed agent is still not checked against this node's own.

What is still unproven: Che routing was reasoned from the devfile, not measured. No kubectl was
used.

## shift_objective

```
Close the floor-readiness must-have: no sign-in secret or session cookie crosses a network in
the clear. Either the node terminates TLS, or sign-in stops sending the secret (a password-
authenticated key exchange or a key-bound sign-in), then set PLAIN_SIGN_IN_ALLOWED to false and
prove a plain-http sign-in from a non-loopback address is refused signin_needs_secure_channel.
```
