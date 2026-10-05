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
  - crates/consent-grant/src/pending.rs
  - crates/consent-grant/src/decide.rs
  - elohim/elohim-storage/src/services/device_consent.rs
  - elohim/elohim-storage/src/services/device_carrier.rs
  - elohim/elohim-storage/src/services/identity_declaration.rs
  - elohim/eprfs/epr-cli/src/device.rs
  - elohim/eprfs/epr-cli/src/approver.rs
  - elohim/holochain/dna/mishpat/zomes/mishpat/src/device_enrollment.rs
  - genesis/docs/superpowers/specs/2026-09-30-claim-backed-credentials-and-contextual-authority-design.md
---

# Device recognition: the missing nodes

A person's identity begins on one node and grows to others one approval at a time
(`device-provisioning-paths.feature`). The human is the steward of their identity; a node only
speaks for them. These are the nodes the device-consent ceremony walks:

1. the identity begins (`POST /auth/identity/begin`, `epr identity begin`, or the declaration);
2. the asking device makes a request (`epr device ask`, by link or by announcing on the private
   network);
3. a node that speaks for the person approves, on its own machine (`epr device approve`, from a
   link or from `epr device pending`);
4. the device redeems the code, checks it, and enrolls itself (`epr device redeem`, or over the
   private network);
5. any peer verifies the binding (`verify_device_binding`).

All five ran live on one isolated conductor on 2026-10-05. The rows below are the nodes the chain
still lacks, each in the shape `chain / between A→C / missing node B: assertion + probe / current
state`.

| # | Missing node | Current state |
|---|---|---|
| 1 | A remote session that proves the person | Remote callers refused |
| 2 | A declared pair of nodes completes the ceremony with nobody carrying anything (carriers: private-network discovery, a doorway's relay/signal) | Carrier 1 built; carrier 2 not |
| 3 | A device's root key is bound to the person | Recorded in the consent only |
| 4 | A revoked device re-enrolls | `supersedes` always None |
| 5 | A declared approvals count above one is enforced | Read and reported; not enforceable |
| 6 | Who is on the node's own machine when it is asked to sign | The machine is trusted; the person is not told apart from whoever operates it |
| 7 | A node with an identity of its own knows whose device it is, without its own work being re-attributed | Enrolled but not registered |
| 8 | The first carrier's remaining gaps | Built on libp2p mDNS; gaps listed |

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
- **Current state:** remote callers are refused 403 `consent_caller_not_local`, so approving
  happens on the node's own machine only (`epr device approve`, or a browser there). The session
  surface itself is open: [security-node-session-routes-unauthenticated](epr:security-node-session-routes-unauthenticated).

## Row 2 — a declared pair completes the ceremony with nobody carrying anything

- **Chain:** declared provisioning.
- **Between:** "the approving node's declaration names the device's key and acts" plus "the device
  declares its approving node" → "the device is enrolled".
- **Missing node:** a carrier for the request and the code between the two nodes, so no person or
  script carries them.
- **Probe:** two isolated nodes with matching declarations reach a verified binding with no
  command after start.
- **Current state (2026-10-05):** carrier 1 is built (`services::device_carrier`; `epr device ask`
  with no portal, `epr device pending`, `epr device approve <n>`). The request and the code travel
  over the private network; someone still runs `ask` and `approve` (a declared device needs no
  answer, but the approve command is still typed). Carrier 2 is not built. Row 8 holds carrier 1's
  remaining gaps.

**Two carriers, named (operator ruling 2026-10-05).** The p2p-design-gate ran for carrier 1 before
it was built; it runs again before carrier 2 is.

1. **Private-network discovery.** This is for two of the person's devices that can see each other
   on a local network, modelled on setting up a home device: an unassigned device announces
   itself, and an already-assigned device (a node that speaks for the person) sees it and
   registers it. No doorway is involved. Built on libp2p mDNS
   (`elohim/elohim-storage/src/p2p/behaviour.rs`, the `mdns::tokio::Behaviour` field), carried as
   one `DeviceCarry` variant of the existing `/elohim/epr/1.0.0` request/response protocol. The
   iroh endpoint has no local discovery (`elohim/elohim-storage/src/p2p_iroh/endpoint.rs`
   registers only pkarr), so iroh-only mode has no carrier.
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
  no more than the acts asked, still approves on the approving node's own machine. The signing
  routes answer only loopback callers.
- When the key is not declared, the fingerprint of the device key is compared by eye on both ends:
  `epr device ask` prints it, and `epr device pending`, `epr device approve` and the portal show it.
- The device checks what it collects before signing anything (`check_delivered`).

**The new surface carrier 1 added:** a bounded list of pending asks on the approving node
(`consent_grant::PendingAsks`): 32 at most, four per source, five minutes each; over the cap is
dropped, not queued; the same device asking again replaces its earlier ask; every outcome is
logged with its reason. It is visible through `epr device pending` and `GET /auth/consent/pending`
and is never acted on by itself.

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
  of the nodes that speak for the person approve".
- **Missing node:** a successor authority write path. Nothing yet creates an authority with
  `previous_authority` set, so the controller set and policy cannot change from the bootstrap's
  `self` policy. `current_successor` in `device_enrollment.rs` reads successors; nothing writes
  them.
- **Probe:** with N = 2 declared, the first approving node's approval issues no enrolling code and
  the second's does.
- **Current state:** the declaration's `approvalsNeeded` is read and reported. `epr identity
  standing` says the node cannot hold to it yet. One approving node is enough.

## Row 6 — who is on the node's own machine

- **Chain:** device consent / the signing rule.
- **Between:** "someone on the node's own machine asks it to sign" → "the node signs as its
  person".
- **Missing node:** who that someone is. Three roles are distinct: whose node it is (the person it
  speaks for); who operates it (whoever runs the machine and answers for what happens to it: the
  person, a relative, an elohim or the commons); and whether it may approve other nodes (one of
  the controllers the person's authority names; joining does not make a node one). Operating a
  node confers no say over the identity it speaks for. The loopback rule trusts the machine, so
  it cannot tell the person from whoever operates the machine. The assertion: an act for the
  identity is taken only from the person (or an elohim attending them), and a confirmation about
  the machine only from its operator.
- **Probe:** an operator who is not the person, at the node's own machine, is refused an approval
  for the person's identity and can still confirm what happens to the machine.
- **Current state:** every caller on the node's own machine can approve for its person. Approval
  answers are taken as the person's (`consent_grant::ByAnswer`), and the code says the gap is open.
  **"Operator of record" for a node is its own record, on the standing side, and is not
  designed.**

## Row 7 — a node with an identity of its own knows whose device it is

- **Chain:** joining "as it is" (`device-provisioning-paths.feature`).
- **Between:** "a node that began an identity of its own is bound as a device of someone else's
  identity" → "the node knows whose device it is, while what it made still traces to the identity
  it began".
- **Missing node:** a resolution of the node's key that names both relationships: the identity it
  began (authored by its key) and the person whose device it now is.
- **Probe:** after joining as it is, the node's own content resolves to the identity it began, and
  the node reports the person it joined.
- **Current state (sweettest `device_enrollment`, stage
  `a_node_with_its_own_identity_joins_as_it_is`, 2026-10-05):**
  - the zomes accept the binding (`enroll_identity_device` on a node that is the sole controller
    of its own identity), and another agent verifies the node as the other person's device;
  - before `register_device_identity`, the node's key still resolves to the Human it began;
  - after `register_device_identity`, the key resolves to the person it joined, so its earlier
    work would trace there, which the story forbids;
  - so the device's own node registers only when it had no Human of its own. A node with one is
    enrolled and verifiable by any peer, its key keeps resolving to the identity it began, and it
    does not itself report the person it joined.

## Row 8 — the first carrier's remaining gaps

- **iroh-only mode has no carrier.** Its only discovery is pkarr, which is not local. The announce
  answers `carrier_absent` and the link-and-paste path remains. Dual mode carries over libp2p.
- **The ask is evidence, not proof.** The transport id that carried an ask is not bound to the
  device key the ask names; the key is proven only when the device signs its possession at
  enrollment. A forged ask can at most be listed and shown; it cannot redeem.
- **Older peers fail to decode** the new `DeviceCarry` variant of `/elohim/epr/1.0.0`. They answer
  with an error and are simply not approving nodes.
- **Multicast must reach.** Discovery is libp2p mDNS; a host or network that blocks multicast never
  lists the ask.
- **CLOSED 2026-10-05: a node that may not approve still listed asks.** A joined device that was
  not a controller listed a later ask, and the asking terminal showed it as a node that lists it.
  Now a node lists only when it speaks for a person (`consent_grant::Speaks`: its own cell is a
  controller of an identity's authority). Whom it speaks for is read from its own cell into
  memory every 30 s and at once after begin, bootstrap, enroll or an applied declaration; a read
  older than 120 s counts as unknown and lists nothing. A node that speaks for nobody drops the
  ask (`ask_node_speaks_for_nobody`, logged) and answers nothing, so the asking terminal never
  shows it; `GET /auth/consent/pending` and `epr device pending` say it speaks for nobody. Every
  listed ask names whose identity an approval would be for (`forIdentity`). Live re-run: the
  joined device no longer appears as a lister. **What remains:** a node that joined as it is still
  speaks for the identity it began and lists asks for it, named but not otherwise guarded; the
  node cannot tell the person from its operator (row 6); a controller removed by another node's
  successor authority keeps listing for up to the 30 s refresh (lists only; the agree path
  re-checks the authority).
- **Signed in means the no-cookie fallback.** `epr device approve` on the approving node carried
  no session cookie; the decide route counted the node's person as signed in through the session
  fallback that row 1 and `security-node-session-routes-unauthenticated` describe.
- **Not built:** a clean start (a new key) for a node with an identity of its own; the joining
  node's operator confirming what will happen to its machine; a portal view of the pending list
  (`GET /auth/consent/pending` and `POST /auth/consent/pending/decide` are ready for one);
  carrier 2.

## shift_objective

```
Pick the highest row that a single slice can close end to end on an isolated stack: row 1's
session-surface half (loopback-only session routes) first, then row 4 (supersedes), then design
rows 3, 5, 6 and 7 through the p2p-design-gate before any zome change. Each row closes with its
probe passing live and a one-line delta here.
```
