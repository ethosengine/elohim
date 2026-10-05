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
  - genesis/a2o/features/auth/device-provisioning-paths.feature
  - genesis/a2o/features/auth/device-consent-grant.feature
cites:
  - elohim/elohim-storage/src/http.rs
  - elohim/elohim-storage/src/main.rs
  - elohim/elohim-storage/src/services/device_consent.rs
  - elohim/elohim-storage/src/services/session_exchange.rs
  - genesis/orchestrator/manifests/network-policies.yaml
  - genesis/orchestrator/manifests/edgenode/alpha.yaml
  - app/imagodei-portal/src/app/services/steward-login-controller.ts
  - genesis/docs/superpowers/specs/2026-09-05-two-portals-sso-consolidation-design.md
---

# A node's session routes answer anyone who reaches its port

## What is open

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

## What is closed

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

## What would close the rest

1. Answer `POST /session`, `GET /session`, `GET /session/all` and `DELETE /session` only to
   loopback callers. Their only known users (the Tauri shell, its webview and the connection
   strategy) are on the same machine. This also stops a remote caller seeding the exchange
   allowlist.
2. Drop the no-cookie fallback for any caller that is not on this machine.
3. Consider binding storage's HTTP to loopback by default where nothing off the machine needs it
   (Tauri), and keep the public Che endpoint off the person's signing surface. The latter already
   holds by the loopback rule.

What is still unproven: Che routing was reasoned from the devfile, not measured. No kubectl was
used.

## shift_objective

```
Make every node session route (POST/GET/GET all/DELETE /session) answer only callers on the
node's own machine, refuse the no-cookie fallback for any other caller, and prove both with a
handler test per route plus one live probe from a non-loopback address on an isolated stack.
Keep the Tauri shell working (it calls from localhost) and leave /session/exchange's contract
intact.
```
