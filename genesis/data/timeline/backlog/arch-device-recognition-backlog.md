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
  - arch-confidentiality-plane-backlog
  - genesis/a2o/features/auth/device-provisioning-paths.feature
  - genesis/a2o/features/auth/device-consent-grant.feature
cites:
  - crates/consent-grant/src/lib.rs
  - crates/consent-grant/src/declaration.rs
  - crates/consent-grant/src/pending.rs
  - crates/consent-grant/src/decide.rs
  - crates/consent-grant/src/signin.rs
  - elohim/elohim-storage/src/services/node_account.rs
  - elohim/elohim-storage/src/services/device_consent.rs
  - elohim/elohim-storage/src/services/device_carrier.rs
  - elohim/elohim-storage/src/services/device_affirmation.rs
  - crates/consent-grant/src/controller.rs
  - crates/consent-grant/src/witness.rs
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
| 1 | A remote session that proves the person | Built: sign-in to the node's own person; must-have before floor readiness: no secret in the clear. **Worse than recorded** (review 2026-10-06): anything on loopback is the person with no session; a dial (row 19, dial 1) |
| 2 | A declared pair of nodes completes the ceremony with nobody carrying anything (carriers: private-network discovery, a doorway's relay/signal) | Carrier 1 built; carrier 2 not |
| 3 | A device's root key is bound to the person | Recorded in the consent only |
| 4 | A revoked device re-enrolls | `supersedes` always None |
| 5 | A declared approvals count above one is enforced | Superseded by row 12: a policy above one counts distinct devices that speak and is verified; no write sets such a policy yet |
| 6 | Who is on the node's own machine when it is asked to sign | Reframed: a signed-in device acts with the person's authority; a witness may pause for re-authentication. Row 1's sign-in is built |
| 7 | A node with an identity of its own knows whose device it is, without its own work being re-attributed | Enrolled but not registered |
| 8 | The first carrier's remaining gaps | Built on libp2p mDNS; gaps listed. **Worse than recorded** (review 2026-10-06): a rogue node could hand back its own code and hijack an ask; fixed (rows 17, 18), reconnaissance a dial (row 19) |
| 9 | A sign-in session bound to a key the browser holds | Built (RFC 9449 DPoP, adapted): signing routes and sign-out need the key's proof; reads do not yet. **Worse than recorded** (review 2026-10-06): loopback skips the key's proof altogether; jti in memory (row 19, dials 1 and 10) |
| 10 | A second device of the person as a human witness ("is this you?") | Recorded; nothing designed |
| 11 | A node serves its own native portal | Not built: the browser run served it from a front at the same origin |
| 12 | Every device of a person speaks for them, and any may approve the next | Built (coordinator only, DNA hash unchanged); affirmation automatic; sweettest and live run. **Worse than recorded** (review 2026-10-06): a revoked record came back by rewrap; fixed (row 16) |
| 13 | Who may withdraw a device's voice | Open question: root controllers only, as before. **Worse than recorded** (review 2026-10-06): affirmation must never be the anchor; bounds turn approvals into liabilities (row 19, dials 5 and 7) |
| 14 | A device of the person contests another's joining ("I saw it; I do not affirm it") | Missing node: no contest exists |
| 15 | The identity routes answer only under this node's names; a proxied loopback request is not local | Fixed 2026-10-06 (the cheap part of critical 2; the rest is a dial) |
| 16 | A revoked joining record cannot come back by being republished | Fixed 2026-10-06 |
| 17 | A rogue node on the private network cannot hand back a code in another's name | Fixed 2026-10-06 (signing; the confirmation prompt is a dial) |
| 18 | No one lists or replaces an ask in a device's name | Fixed 2026-10-06 |
| 19 | Dialing up: the bars kept low for now | Recorded: each with its attack, fix and cost to the person |

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
- **Current state (2026-10-05): built, by the OAuth model**, a native proof that needs no
  doorway: the node signs in its own person. `epr identity secret` (this machine only) sets an
  Argon2id verifier in the node-local account; `POST /auth/login` opens a session proven by
  sign-in; the signing routes and the pending list and decide accept a caller on this machine or
  such a session, nothing else (`consent_grant::may_make_node_sign`). Assertion 1 holds: the
  session routes answer only this machine. Assertion 3 holds for sign-in sessions. Assertion 2 is
  not taken: an exchange-made session stays no proof (by design; the exchange keeps its own job).
  Live from 10.1.19.170: sign-in, approval of a pending device from that session, sign-out, the
  same cookie refused. Details:
  [security-node-session-routes-unauthenticated](epr:security-node-session-routes-unauthenticated).
- **Must-have before floor readiness (required, not optional hardening):** a sign-in secret and a
  session cookie must not cross a network in the clear. Allowed today by operator ruling and
  logged at warn; one line closes it (`consent_grant::PLAIN_SIGN_IN_ALLOWED`). Satisfied by the
  node terminating TLS, or a sign-in that never sends the secret (a password-authenticated key
  exchange, or a key-bound sign-in). Also recorded as
  [arch-confidentiality-plane-backlog](epr:arch-confidentiality-plane-backlog) row 13.
- **What remains:** passkeys; a second factor; sign-in from a device that is itself enrolled (its
  key proving the person); TLS on a home LAN; a proxy on the node's own machine looks local.

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
- **Superseded (2026-10-05) by row 12.** The rule no longer needs the authority rewritten per
  join: a policy above one counts distinct devices that speak (root controllers or joined devices
  whose records stand), and `classify_approvers` verifies it. What remains of this row is the
  policy write alone: nothing yet writes an authority whose policy asks for more than one, below.
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
- **Reframed (operator ruling 2026-10-05, the OAuth model).** Three roles stay distinct: whose
  node it is (the person it speaks for); who operates it (whoever runs the machine and answers for
  what happens to it: the person, a relative, an elohim or the commons); and whether it may
  approve other nodes (every device that speaks for the person may: row 12 superseded the earlier
  "joining does not make a node one"). Operating a node confers no say over the identity it speaks for. But "the node
  cannot tell the person from whoever is at the machine" is **not** a gap to close by asking the
  person for proof on each act. A signed-in device acts with the person's authority, as in OAuth.
  Noticing that something is off and asking for re-authentication is the witness's job, at the
  witnessed moment (`consent_grant::witness`: an attending witness may pause and the agree path
  answers 401 `consent_reauthentication_asked`; nobody attends by default, so nothing pauses
  today). No added load for the person.
- **What remained real is now built (2026-10-05):** the session routes answer only this machine,
  and remote approval needs a real sign-in (row 1). An attending witness's "sign in again" now
  has somewhere to go: `POST /auth/login`, itself a witnessed moment (`MomentKind::SignIn`).
- **"Operator of record"** for a node is its own record, on the standing side, and is not
  designed.
- **The witnessed moment is one of many.** The device-authorization moment is one witnessed
  validation moment at the sign-in and authorization floor; a reach gate is another. A shared home
  for witnessed moments (the kind, the claims handed over, the outcomes proceed / proceed with a
  signature / pause for re-authentication) is not designed; the shape lives in
  `crates/consent-grant/src/witness.rs` until it is.
- **The sign-in word's home (closed 2026-10-05).** It is shown wherever the person is named and
  never used to decide or join. It is not on the Human record (public on the DHT). Its lasting,
  private home is now the node-local account (`node_account`), written at begin (route or
  declaration) and by `epr identity secret`; the node reads it there first, then a session, then
  the declaration. An identity begun before the account existed gains one at its first
  `epr identity secret`.

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
    enrolled and verifiable by any peer, its key keeps resolving to the identity it began.
  - since row 12 it also names the person it joined: it speaks for both identities, approvals on it
    are for the one it began, and standing (`alsoSpeaksFor`), the pending list's words and `epr
    identity standing` name the other, never guessing which.

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
  speaks for the identity it began and lists asks for it, named but not otherwise guarded; a
  controller removed by another node's
  successor authority keeps listing for up to the 30 s refresh (lists only; the agree path
  re-checks the authority).
- **Signed in means the no-cookie fallback, on this machine only (narrowed 2026-10-05).** A local
  `epr device approve` still counts the node's person as signed in through the single active
  session; a caller from another machine now needs a session proven by sign-in (row 1).
- **CLOSED 2026-10-05: a decided ask was listed again.** The asking device's next announce round
  could arrive between the decision and the code reaching it (seen live, 1.4 ms after the
  decision) and was listed anew. Decided asks are now remembered by their state token for the
  ask's life and dropped as `ask_already_decided`.
- **Not built:** a clean start (a new key) for a node with an identity of its own; the joining
  node's operator confirming what will happen to its machine; a portal view of the pending list
  (`GET /auth/consent/pending` and `POST /auth/consent/pending/decide` are ready for one);
  carrier 2.

## Row 9 — a sign-in session bound to a key the browser holds

- **Chain:** sign-in / the routes that make the node sign.
- **Built (2026-10-05):** a sign-in may bind its session to a public key (`sessionKey: {alg, jwk}`,
  ES256 or EdDSA), proven at binding by a `DPoP` proof on the sign-in request itself; over TLS the
  key is required (`signin_needs_session_key`); over plain http from another machine it may be
  absent, logged once as unbound. A bound session must then present a valid proof from that key
  (compared by RFC 7638 thumbprint) on agree, bootstrap, begin, pending decide and sign-out:
  `session_proof_missing`, `session_proof_invalid`, `session_proof_stale`, `session_proof_replayed`.
  Rules: `consent_grant::dpop` (`session_proof_verdict`, `ReplaySet`); one question in storage,
  `node_account::signing_verdict`. A copied cookie is worth nothing on those routes.
- **Proofs on reads (open):** `/auth/me`, `GET /auth/identity/standing` and `GET
  /auth/consent/pending` ask only for the session. A stolen cookie can still read the person's
  sign-in word, name, human id and agent key, the identity's root, authority and controllers, and
  the devices asking with their keys and labels. It cannot approve, decline, bootstrap, begin or
  sign out. Missing node: between "a request carries a bound session" and "the node answers a
  read": the key's proof on reads too; probe: a bound cookie with no proof gets 401 from each read.
- **A witnessed-moment fact, not built:** a first-seen session key on a new address is the kind
  of fact an attending witness would look at (`MomentKind::SignIn`), and could pause on.
- **Platform authenticator (passkey) sign-in, not built:** the path to biometrics and the
  direction for replacing the password: a WebAuthn credential whose public key the node keeps on
  the account and whose assertion proves the person at sign-in, with no secret sent at all (it
  also answers the floor-readiness must-have of row 1). The verifier interface
  (`consent_grant::dpop::AlgVerifier`, keyed by `alg`) is where a platform-held key under another
  algorithm is an addition.
- **A local-only channel for same-machine acts, not built:** today "this machine" is a loopback
  peer address, which any proxy on the machine passes. A loopback-only listener or a socket file
  would make same-machine acts this machine's alone.
- **One unlock for the keystore and the sign-in, not built:** the conductor's keystore passphrase
  and the sign-in secret are separate today; one unlock that opens both is recorded, not designed.

## Row 10 — a second device of the person as a human witness

- **Chain:** witnessed moments (row 9's sign-in moment, device authorization; `consent_grant::witness`).
- **Between:** "a witnessed moment is attended by nobody, or by an elohim" → "the person themselves
  attends from another of their devices".
- **Missing node:** the familiar "X is trying to do this, is this you?" check, as one more
  attendant of a witnessed moment: at sign-in or device authorization, a second device of the
  person is asked and answers, and its answer is a witness signature (`Witnessing::
  ProceedWithSignatures`) or a pause (`PauseForReauthentication`). The answering device need not
  run a full conductor: a spoke-type device (the HTTP/WS participation track) is enough. Optional
  and opt-in; it strengthens and is never a condition for what works alone. It rides whichever
  carrier reaches the second device (private-network discovery, row 2's carrier 1, or a doorway's
  relay, carrier 2).
- **Probe:** with a second device opted in, a sign-in waits for its answer; its "yes" adds a
  witness signature, its "no" pauses with its reason; with none opted in nothing waits.
- **Current state:** nothing designed. Related: row 9's witnessed-moments note and its passkey
  (platform authenticator) row.

## Row 11 — a node serves its own native portal

- **Chain:** sign-in / approving from another machine.
- **Between:** "the person opens their node's address in a browser" → "the native portal
  (`app/imagodei-portal`, `<base href="/auth/portal/">`) loads from the node".
- **Missing node:** elohim-storage serves no `/auth/portal`; sign-in redirects there
  (`redirect: "/auth/portal"`), but on a node only a doorway (an EPR projection) or a front serves
  it. Probe: `GET <node>/auth/portal/` answers the bundle's `index.html` and its assets at the
  node's own origin.
- **Current state (browser run, 2026-10-05):** served from a small front at the node's origin
  (`genesis/local-dev/device-consent/front.mjs`: static bundle under `/auth/portal/`, everything
  else forwarded byte for byte).
- **Also found there:** `epr device redeem` speaks only `http://`, so a code shown by an https
  portal is redeemed with `--approver <the node's http address>`; the CLI keeps the request when
  the node was unreachable, so nothing is spent.

## Row 12 — every device of a person speaks for them, and any may approve the next

- **Chain:** controller policy / device lifecycle (story: `device-provisioning-paths.feature`,
  commit `b14f31cff`).
- **Between:** "a device joins the person's identity" → "it approves the person's next device,
  with the first device away".
- **Built (2026-10-05), coordinator only.** The authority record stays the root and the policy and
  is never rewritten by a join. Each joining record stands alone, verified by walking its
  approvers' own joining records back to the authority. The rule, in four sentences
  (`device_enrollment.rs` module header):
  1. A joining record stands when it names a current authority of its identity, the joining device
     signed it, and the authority's root controllers have not revoked it.
  2. At least as many distinct approvers as the person's policy requires (one by default) signed
     it, never the joining device itself, and each approver is a root controller of the authority
     or a device whose own joining record, which this record names, stands by this same rule for
     the same identity.
  3. The walk back to the authority goes at most 8 records deep and reads at most 24 records; a
     record met again on its own way back refuses as a cycle, and one reached by two approvers'
     ways is read once.
  4. Read now, an approver whose joining record was revoked no longer counts, so what it approved
     stops verifying too; read at an authenticated earlier moment
     (`verify_historical_device_binding`), it counts if its revocation came after that moment.
- **Wire.** `DeviceBinding.approved_via` (omitted when every approver is a root, so earlier records
  keep their bytes), `DeviceApprovalProofs.via`, `ConsentStanding.speaks_via` / `also_speaks_for`,
  and the read `identity_devices(identity_root)` (who approved each device, when it joined, who
  affirmed it). Standing's `controllers`, `controllerCount`, `thisNodeIsController`,
  `restsOnThisNodeAlone` and the agree answer's `controllers: {required, signed}` keep their names
  and now count devices that speak, not root controllers only; standing gains `devices` and
  `alsoSpeaksFor`.
- **Revocation, as built.** Unchanged in who: the authority's root controllers. Read now it
  cascades (rule 4). Read at an authenticated earlier moment, a device a since-revoked device
  approved still verifies. The ruling asked that what a revoked device approved earlier keep
  verifying at the current read too; that is not sound on the evidence the zome holds, because a
  joining record's time is chosen by its author, so a stolen and revoked key could approve a new
  device with a backdated record. The sound anchor, not built: an affirmation, made before the
  revocation, by a device whose own way back does not pass through the revoked one, and never an
  automatic one (row 19). Row 13 holds the who. Since the review (row 16), revocation withdraws
  the device for the identity, found from `(identity_root, device_key)`, not one record's bytes.
- **Affirmation (automatic).** Each speaking node, a minute after start and then every 30
  minutes, reads its standing and `identity_devices` and affirms at most 4 devices it did not
  approve, has not affirmed, and is not, oldest first (`services::device_affirmation`); a node
  away catches up on its next passes. Per pass: one standing read and one devices read; per
  affirmation: one mandate grant, one affirmation record and its discovery link. Affirming is not
  approving: no voice, no policy count, no verdict changes.
- **The witnessed moments' context.** Device authorization hands the witness, as data and never
  a threshold: whether the approver is a root, how long it has spoken, how many others affirmed its
  own joining, how many devices speak, and whether the asking device began an identity of its own
  (when it said so over the carrier). Sign-in hands the first four about this node. Both come from
  the last devices read kept, never a network read at the moment.
- **Probe:** sweettest `device_enrollment` stage `every_device_speaks_for_its_person` (A begins; B
  approved by A; C approved by B alone, verified by a fourth agent; two approvals at once both
  stand; self-approval, double counting and an unshown record refuse; required = 2 refuses one and
  accepts two distinct; revoked B cannot approve, C verifies before the revocation and not now),
  unit tests on the walk's depth, cycle and work bounds, and a live single-conductor run (a third
  device approved from the second).
- **Live run (2026-10-05, one isolated conductor, four apps):** A began; B asked over a link and A
  approved; C asked with B's portal and **B approved alone**; C redeemed with `--approver` B and a
  fourth app verified C's binding (`verify_device_binding`, identity root and authority A's). After
  a restart each node's first pass affirmed what it did not approve (A affirmed C, C affirmed B),
  and `epr identity standing` on A and on B reads "3 nodes speak for you" with each device, its
  approver and "affirmed by one other device".
- **Also found there:** a joined device's terminal has no way to sign in from the CLI. `epr
  identity begin` is the only verb that keeps a session; on a joined node the person signs in by
  the portal or `POST /auth/login`, and the terminal then holds no cookie (the run wrote the one
  login returned into the CLI's session file). Missing node: `epr identity signin` on the node's
  own machine.
- **Not built:** the policy write (row 5); depth and cycle bounds are unit-tested, not reached in
  the sweettest (a chain nine devices deep is not built there, and content addressing makes a real
  cycle unconstructible: the refusal is defence in depth).

## Row 13 — who may withdraw a device's voice

- **Chain:** device lifecycle / revocation.
- **Between:** "a device of the person is lost or stolen" → "its voice is withdrawn, and the
  person knows what that withdraws".
- **Open question.** Today only the authority's root controllers may revoke a joining record, and
  that is all existing revocation verification can check. The risks either way: a stolen joined
  device can approve new devices but cannot revoke, so it cannot lock the person out, but it can
  add devices until a root revokes it; losing the root (the first node) leaves no one able to
  revoke; and revoking a device withdraws, at the current read, every device whose way back runs
  through it, including the person's own later devices. Letting devices revoke each other would
  let a stolen device revoke the person's real ones. Not settled; the record asks.
- **Probe:** a decided rule, then a sweettest that revokes by it in both orders (revoke then
  approve, approve then revoke) with historical reads.

## Row 14 — a device of the person contests another's joining

- **Chain:** affirmation (row 12) / witnessed moments.
- **Between:** "a device of the person saw another join" → "it says it does not affirm it, and
  that reaches the person and the device".
- **Missing node:** a contest, the counterpart of `affirm_identity_device`: "I saw it; I do not
  affirm it". Nothing in `device_enrollment` records one (the zome's only "contested" is an
  authority branch). Mishpat's generic `create_challenge` names any entity id but is not linked
  from a joining record and `identity_devices` does not read it. Counter-evidence always reaches
  its subject: a contest must be discoverable from the contested joining record and shown with it,
  to the person and to the contested device, never only to its author.
- **Probe:** device D contests C's joining; `identity_devices` shows C with D's contest and C's
  own standing names it.
- **Current state:** nothing; affirmations only.

## Adversarial review 2026-10-06 — what is fixed

A read for attackers (reviewer's report; coordinator's brief, then the operator's ruling: keep the
bar low for now, fix what costs the person nothing and closes a true bug, record the rest as
dials). Each fix names the test that reproduces the attack.

## Row 15 — the identity routes answer only under this node's names (critical 2, the cheap part)

- **Attack (reviewer):** DNS rebinding to 127.0.0.1, any localhost page with XSS, any process in
  the pod, any reverse proxy on the machine: `caller_is_local` was the TCP peer IP alone,
  `foreign_origin_refusal` accepted `Origin == Host` with no Host validation and any localhost
  port, and `ELOHIM_TRUSTED_PROXIES` was consulted for TLS only, never for locality.
- **Fix:** every identity, consent, device, sign-in and session route answers only under
  `localhost`, `127.0.0.1`, `[::1]` or a name in `ELOHIM_ALLOWED_HOSTS` (`421
  host_not_this_node`; `GET /auth/me` is left out: the doorway proxies it under its upstream name,
  and it only reads). Origin must equal Host exactly: another port on this machine is another
  origin. A loopback peer carrying `Forwarded`/`X-Forwarded-*`/`X-Real-Ip`, or listed as a trusted
  proxy, is not local (`consent_grant::network_local`), for signing and for the sign-in channel.
- **What it closes:** a rebound name reaches no identity route; a page on another localhost port
  cannot agree or read standing; a proxy on the machine no longer makes its clients local.
- **Left open, by ruling:** a process or page that reaches loopback directly under a loopback name
  still acts as the person with no session (dial 1).
- **Tests:** `identity_routes_answer_only_this_nodes_names`,
  `only_a_loopback_connection_counts_as_this_machine` (each forwarding header),
  `a_forwarded_or_proxied_loopback_request_is_not_local`, `only_this_nodes_names_are_hosts_it_
  answers`, `a_page_on_another_site_cannot_read_the_standing_either` and `only_this_nodes_own_
  portal_may_ask_it_to_sign` (`http://localhost:4200` refused), the `channel` cases for a proxy on
  loopback. A dev portal on another localhost port is now served through a same-origin front.

## Row 16 — a revoked joining record cannot come back by being republished (critical 1)

- **Attack (reviewer):** root R approves D, record B (controllers=[R], approved_via=[]); R revokes
  B; anyone builds B′ = B with approved_via=[{agent X, binding Y}] and publishes it; the signatures
  pass (the intent is unchanged), classify_approvers passes (R is a root, so the junk entry is
  never read), and the revocation lookup read lifecycle(hash_entry(&entry)): B′ has a new entry
  hash and no links, so B′ stood, register_device_identity(B′) re-resolved D, and E approved by
  revoked D came back as E′ via B′.
- **Fix (coordinator only, DNA hash unchanged):** revocations are found on an anchor per device of
  an identity, from `(identity_root, device_key)`, the two signed facts every joining record of the
  device carries (and on the entry, for revocations made before). Keyed on the device rather than
  the intent's bytes because revoking a device withdraws its voice: an intent anchor would let a
  second joining record of the same key keep speaking and its approvals return. `device_revoked`
  compares the revoked target's identity and device, never bytes; `approved_via` must name exactly
  the device approvers, in proof order; a record stands only if its native author is its device;
  `enroll_identity_device` refuses a caller that is not the device and a device already revoked
  (re-enrolment waits for `supersedes`, row 4).
- **Tests:** sweettest rewrap stage: (a) B with a junk approver record refused at enroll and,
  published around the zome, not standing; (b) a new root-approved record of revoked B's key
  refused; (c) C, approved by revoked B, rewrapped to name the rewrapped B, refused; (d) a copy of T
  published by another chain not standing. Against the pre-fix coordinator it failed at (a): "a
  revoked device cannot rejoin by a rewrapped record" (`genesis/local-dev/device-consent/
  sweettest-fix1-before.log`). Unit: one root proof with a junk via refused.

## Row 17 — a rogue node cannot hand back a code in another's name (high 3, the signing part)

- **Attack (reviewer):** the attacker answers Listed{approver: any key}; note_listed recorded it;
  the attacker sends Code; on_code checked only that the source listed the ask and discarded the
  approver; the last code won; the CLI redeemed at the attacker; check_delivered never checked whose
  key signed nor approverKey; finish enrolled; the device resolved to the attacker's Human.
- **Fix:** Listed, Code and Declined are taken only with a proof by the claimed key over (kind,
  the sender's transport id, and for Code and Declined the request's state); the key must be the
  one that listed and equal a declared `approverKey`; `check_delivered` takes the expected approver
  (declared, or proven with the code) and requires it among the consent's and enrollment's signers
  (`delivered_not_the_expected_approver`); `/auth/device/enroll` checks against the carrier's
  proven approver. The asking terminal prints the approving node's and the identity's
  fingerprints before it enrolls. Signing: coordinator extern `sign_carrier_statement` under the
  `elohim:device-carrier:v1:` domain, one mandate grant per signature.
- **Deviation:** the Listed proof names the transport id but not the ask's state: Listed is answered
  inside the transport's event loop, where a zome call per ask would stall it, so it is signed once
  per key and transport id. Code and Declined are state-bound.
- **Left open, by ruling:** with no approver declared, a proven rogue that is a person of its own
  can still approve and the device enrolls into that identity after printing it (dial 2).
- **Tests:** `a_rogue_node_on_the_network_cannot_capture_the_asking_device`,
  `a_proof_holds_only_for_its_own_signer_kind_state_and_transport`,
  `each_way_a_delivery_can_be_wrong_is_refused_by_name` (a consent by another key refused).

## Row 18 — no one lists or replaces an ask in a device's name (high 4)

- **Attack (reviewer):** the attacker sends an Ask with the victim's device key and its own
  challenge, state, acts and device_root_key; PendingAsks::admit replaced by device key alone and
  moved the source; the attacker redeemed: not enrolling its own key, but denying the real device
  and able to swap in its own device.bind-root key.
- **Fix:** an ask carries the device key's proof over (ask, state, the device's transport id, the
  PKCE challenge), signed by the device's node when it starts announcing; unsigned or sent from
  elsewhere it is dropped (`ask_unsigned`); it is replaced only from the source it was listed from
  (`ask_listed_from_elsewhere`). The test that asserted the cross-source replacement now asserts
  the same-source one.
- **Test:** `no_one_lists_or_replaces_an_ask_in_a_devices_name`.

## Also fixed: discovery floods and authorship (medium 7, half of medium 8)

- **Attack (reviewer):** discovery() did not filter link authors: 64 device| or 32 affirmed| links
  from anyone hid the real ones, each costing a walk; binding_stands never checked native author
  == device key.
- **Fix:** device| links are followed only when their author is the device the record joins,
  affirmed| only from a speaker, each capped per author; a record stands only if its device
  published it.
- **Test:** sweettest flood stage (65 device| and 33 affirmed| links from a non-speaker; a device
  joining after them is listed and a real affirmation counts); pre-fix it failed: "a device that
  joins after a link flood is still listed" (`sweettest-fix5-before.log`); the copy-by-another-
  chain case in row 16.

## Row 19 — Dialing up: the bars kept low for now

Each dial: the reviewer's attack, the fix that would close it, and what it would cost the person.

1. **Loopback is the person, with no session** (rows 1 and 9, worse than recorded).
   - *Attack:* any process on the machine, a page on another app's localhost origin that a person
     visits (once it can name a loopback host: DNS rebinding is now refused by the Host gate), or
     an XSS in anything served on loopback sends JSON to `/auth/identity/secret`,
     `/auth/consent/agree` or `/auth/consent/pending/decide` with no Origin (a non-browser
     process) and is the person: `signing_caller` returns early for local callers and
     `person_signed_in` falls back to the single active session. Rebinding steps (reviewer):
     attacker.example resolves to the attacker, then to 127.0.0.1; the page fetches
     `http://attacker.example:8191/auth/identity/secret` with a new secret; before row 15 the node
     took it, and `GET /auth/device/announce` handed back `code#state`.
   - *Fix:* a per-install token (random, in a user-only file in the node's data dir, read by `epr`
     and the shell, sent in a header) or a signed-in session for every this-machine act, even on
     loopback; begin on a node with no identity the one exception; `/auth/identity/secret` and
     agree never without a session.
   - *Cost:* nothing visible on the CLI; the Tauri shell and its webview must read and send the
     token; a browser that begins an identity must then sign in before it approves.
2. **No confirmation before enrolling** (row 17).
   - *Attack:* with no approver declared, a node of another person on the same network that is
     faster to approve hands back a proven code; the device enrolls into that identity, having
     printed it.
   - *Fix:* ask the person to confirm the approver and identity shown before
     `/auth/device/enroll` (`--yes` for scripts).
   - *Cost:* one more answer at every undeclared join.
3. **Historical reads take the moment they are given** (the other half of medium 8).
   - *Attack:* `verify_historical_device_binding` takes a caller-chosen `witnessed_at`;
     identity_devices keeps the earliest-linked copy, so `joined_at` is forgeable within the
     device's chain; Mode::At's "joining postdates the witnessed moment" uses the native time alone.
   - *Fix:* use `max(native time, intent.issued_at)` and either keep the read off public paths or
     say in its docs that it is evidence only.
   - *Cost:* none; deferred to keep historical semantics unchanged in this slice.
4. **Signing oracles on the person's key** (high 5).
   - *Attack (reviewer):* `imagodei::sign_for_agent` signs arbitrary bytes for any chain-author
     caller (sign_for_agent.rs:143); the enrollment message is plain JSON (device_enrollment.rs);
     `mishpat::create_lineage_commitment` signs an arbitrary `signing_payload_cid`
     (commitments.rs:373). All three use the agent key the device ceremonies sign with, so a caller
     that may call either can sign an enrollment intent or approval as the person.
   - *Fix:* domain-prefix what they sign and refuse input that parses as an intent, revocation,
     consent, authority or carrier statement; coordinator-only for the refusal half (the prefix
     half moves sign_for_agent's verifiers in storage and the seeder, and lineage signatures are
     checked by integrity validation, so it would move the DNA hash).
   - *Cost:* none to the person; a coordinator change in two DNAs.
5. **Automatic affirmation reads as evidence** (row 13).
   - *Attack (reviewer):* device_affirmation affirms every standing device not approved by this
     node, up to 4 per pass every 30 minutes; a thief's devices read "affirmed by N" within
     minutes; building row 12's sound anchor on it would make stolen approvals survive revocation.
   - *Fix:* never use automatic affirmation as that anchor; mark affirmations automatic in the
     record and the words.
   - *Cost:* none; it stays as built and counts as it does.
6. **A policy above one counts keys, not root lineages** (medium 9).
   - *Attack:* classify_approvers counts distinct device keys toward recovery-quorum m and
     steward-set, so one root's descendants can meet a quorum meant to need two roots.
   - *Fix:* resolve each approver's way back to its root and count roots.
   - *Cost:* a person with one root can never meet a policy above one by devices alone.
7. **Bounds that refuse honest growth; approvals that become liabilities.**
   - *Attack:* the depth bound (8) refuses a 10th sequential device; check_approvals checks every
     approver, so an extra approval by a later-revoked device makes the whole record stop
     verifying.
   - *Fix:* stand when enough approvals stand; bound by root distance.
   - *Cost:* none; a wider walk.
8. **Carrier reconnaissance and the per-source cap.**
   - *Attack:* asks broadcast label and key to 16 peers every 5 s; Listed reveals the approver key;
     new PeerIds defeat the per-source cap and fill MAX_PENDING.
   - *Fix:* send a declared approver's ask to it alone; cap per proven device key (now possible,
     row 18).
   - *Cost:* an undeclared ask still has to be heard by someone.
9. **Remote sign-in lockout.**
   - *Attack:* the account limiter (FREE_FAILURES_PER_ACCOUNT=10, 15 min, signin.rs:176) lets
     anyone who can reach the node lock its person out remotely.
   - *Fix:* do not count this machine's sign-ins against the account; slow remote sources only.
   - *Cost:* none.
10. **DPoP jti in memory; htu by path only** (row 9).
    - *Attack:* a restart forgets the replay set, so a captured proof replays within its ±60 s
      window after a restart; htu is compared by path only.
    - *Fix:* persist the jti set for its window; compare the full htu when the node knows its own
      origin.
    - *Cost:* none.

## shift_objective

```
Pick the highest row that a single slice can close end to end on an isolated stack: row 1's
floor-readiness must-have (no sign-in secret in the clear: node TLS, or a PAKE / key-bound
sign-in) first, then row 4 (supersedes), then design
rows 3, 5's policy write, 7, 13 and 14 through the p2p-design-gate before any zome change. Each row closes with its
probe passing live and a one-line delta here.
```
