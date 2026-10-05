---
id: "backlog-arch-device-recognition-backlog"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Device recognition: the missing nodes between a person's identity and every device that acts for it"
slug: "arch-device-recognition-backlog"
written: "2026-10-05"
author: "claude (device-authorization-grant branch, operator-directed)"
status: "backlog"
priority: "high"
tags: [device-consent, recognition, identity, mishpat, imagodei, elohim-storage, epr-cli, story-maintainer]
relatedNodeIds:
  - security-node-session-routes-unauthenticated
  - genesis/a2o/features/auth/device-provisioning-paths.feature
  - genesis/a2o/features/auth/device-consent-grant.feature
cites:
  - crates/consent-grant/src/lib.rs
  - crates/consent-grant/src/declaration.rs
  - elohim/elohim-storage/src/services/device_consent.rs
  - elohim/elohim-storage/src/services/identity_declaration.rs
  - elohim/eprfs/epr-cli/src/device.rs
  - elohim/eprfs/epr-cli/src/steward.rs
  - elohim/holochain/dna/mishpat/zomes/mishpat/src/device_enrollment.rs
  - genesis/docs/superpowers/specs/2026-09-30-claim-backed-credentials-and-contextual-authority-design.md
---

# Device recognition: the missing nodes

A person's identity begins on one node and grows to others one approval at a time
(`device-provisioning-paths.feature`). These are the nodes the device-consent ceremony walks:

1. the identity begins (`POST /auth/identity/begin`, `epr identity begin`, or the declaration);
2. the asking device makes a request (`epr device ask`);
3. a steward agrees on its own machine (`epr device approve`);
4. the device redeems the code, checks it, and enrolls itself (`epr device redeem`);
5. any peer verifies the binding (`verify_device_binding`).

All five ran live on one isolated conductor on 2026-10-05. The rows below are the nodes the chain
still lacks, each in the shape `chain / between A→C / missing node B: assertion + probe / current
state`.

| # | Missing node | Current state |
|---|---|---|
| 1 | A remote session that proves the person | Remote callers refused |
| 2 | A declared pair of nodes completes the ceremony with nobody carrying anything (carriers: private-network discovery, a doorway's relay/signal) | A person or script carries the link and code |
| 3 | A device's root key is bound to the person | Recorded in the consent only |
| 4 | A revoked device re-enrolls | `supersedes` always None |
| 5 | A declared approvals count above one is enforced | Read and reported; not enforceable |

## Row 1 — a remote session that proves the person

- **Chain:** device consent / the native portal.
- **Between:** "the person signs in on their node's portal from another machine" → "the node signs
  as the person".
- **Missing node:** a session that proves the person. The assertion needs all three of:
  1. `POST /session`, `GET /session` and `GET /session/all` answer only on loopback, so session
     ids stay secret and the exchange allowlist cannot be seeded remotely;
  2. `/session/exchange` marks its session as proven and checks the redeemed agent equals the
     node's cell agent (or a native proof-of-key sign-in exists; the two-portals spec names it out
     of scope);
  3. agree and bootstrap accept a remote caller only with a cookie naming such a session.
- **Probe:** from a non-loopback address, an exchange-proven cookie approves and a forged session
  is refused.
- **Current state:** remote callers are refused 403 `consent_caller_not_local`, so a person
  approves on the node's own machine only (`epr device approve`, or a browser there). The session
  surface itself is open: [security-node-session-routes-unauthenticated](epr:security-node-session-routes-unauthenticated).

## Row 2 — a declared pair completes the ceremony with nobody carrying anything

- **Chain:** declared provisioning.
- **Between:** "the steward declares the device's key and acts" plus "the device declares its
  steward" → "the device is enrolled".
- **Missing node:** a carrier for the request and the code between the two nodes, so no person or
  script carries them.
- **Probe:** two isolated nodes with matching declarations reach a verified binding with no
  command after start.
- **Current state:** the declaration spares the person the approval question
  (`Declaration::agreed_for`). The link and the code are still carried by a person or a script.
  No carrier is built.

**Two carriers, named (operator ruling 2026-10-05).** Neither is built yet. The p2p-design-gate
runs before either is.

1. **Private-network discovery.** This is for two of the person's devices that can see each other
   on a local network, modelled on setting up a home device: an unassigned device announces
   itself, and the already-assigned device (the steward) sees it and registers it. No doorway is
   involved. It would ride the storage node's existing local discovery: libp2p mDNS
   (`elohim/elohim-storage/src/p2p/behaviour.rs`, the `mdns::tokio::Behaviour` field and
   `Mdns(mdns::Event)`) or the iroh endpoint's discovery (`elohim/elohim-storage/src/p2p_iroh/endpoint.rs`).
2. **A doorway's relay or signal servers.** This is for two nodes that cannot see each other
   directly. A doorway may host the relay or signalling that carries the same exchange. It is
   optional, it confers nothing, and it is only a carrier. It would ride the doorway's signal
   function (`doorway/doorway-service/src/signal/`, routed at `/signal/*` in
   `doorway/doorway-service/src/server/http.rs`, `handle_signal_request`), or the iroh relay the
   nodes already home to.

**Why either carrier is safe to treat as untrusted.** All of this already holds:
- The request carries only a PKCE challenge and public keys (the device key, the device root
  `did:key`, the network DNA hashes), plus a label and a state token. Nothing in it is secret.
- The code is useless to anyone but the terminal holding the PKCE verifier. Redemption also needs
  the matching device key, and a wrong verifier burns the code (`RedemptionRefusal::burns_delivery`).
- A carrier never causes a signature. The person, or a declaration naming the exact device key and
  no more than the acts asked, still approves on the steward's own machine. The signing routes
  answer only loopback callers.
- When the key is not declared, the person compares the device key's fingerprint by eye on both
  ends: `epr device ask` prints it, and `epr device approve` and the portal show it.
- The device checks what it collects before signing anything (`check_delivered`).

**What a carrier adds as new surface, so the next slice designs against it:** a bounded list of
pending asks on the steward. It needs a count cap, a five-minute life (the code window) and a
per-source limit. It must be visible to the person (in `epr identity standing` or the portal) and
must never be acted on by itself. An ask arriving through a carrier is shown, not approved; only a
declaration's exact key-and-acts match or the person approves it.

## Row 3 — a device's root key is bound to the person

- **Chain:** provenance (spec §12.1 planes).
- **Between:** "the consent agrees to `device.bind-root`" → "bytes the device signs with its root
  key trace to the person".
- **Missing node:** a zome act that binds a device root key (`did:key`) to the device binding.
- **Probe:** a peer resolves a root-signed artifact to the person's identity.
- **Current state:** nothing in any zome binds a root key. Agree records the act and the root in
  the consent record only. `epr device redeem` says so plainly.

## Row 4 — a revoked device re-enrolls

- **Chain:** device lifecycle.
- **Between:** "a device binding is revoked" → "the same device is enrolled again under the same
  identity".
- **Missing node:** the ceremony names the revoked binding. `DeviceIntent.supersedes` exists in the
  zome (`validate_intent` checks it).
- **Probe:** re-enroll a revoked device; `verify_device_binding` accepts the new binding.
- **Current state:** `EnrollmentIntent::agreed_in` always sets `supersedes: None`.

## Row 5 — a declared approvals count above one is enforced

- **Chain:** controller policy.
- **Between:** "the person declares `approvalsNeeded = N`" → "a device is recognized only after N
  stewards approve".
- **Missing node:** a successor authority write path. Nothing yet creates an authority with
  `previous_authority` set, so the controller set and policy cannot change from the bootstrap's
  `self` policy. `current_successor` in `device_enrollment.rs` reads successors; nothing writes
  them.
- **Probe:** with N = 2 declared, the first steward's approval issues no enrolling code and the
  second's does.
- **Current state:** the declaration's `approvalsNeeded` is read and reported. `epr identity
  standing` says the node cannot hold to it yet. One steward's approval is enough.

## shift_objective

```
Pick the highest row that a single slice can close end to end on an isolated stack: row 1's
session-surface half (loopback-only session routes) first, then row 4 (supersedes), then design
rows 3 and 5 through the p2p-design-gate before any zome change. Each row closes with its probe
passing live and a one-line delta here.
```
