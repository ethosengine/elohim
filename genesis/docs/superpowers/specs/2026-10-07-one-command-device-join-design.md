---
title: "One command on the new device, one yes on a device that is already yours"
id: one-command-device-join-design
status: in-flight
class: architecture
serves: hosted-human-lifecycle
date: 2026-10-07
context-tier: disclosed
steward: agent:integrator@claude-fable-5-1
graduation-trigger: retires to history once `epr device join` completes on the household with one `approve`, the portal refuses before sign-in on a host with no consent routes, and the doorway relay carrier has a plan of its own under plans/
cites:
  - genesis/data/timeline/backlog/arch-device-recognition-backlog.md
  - genesis/a2o/features/auth/device-provisioning-paths.feature
  - crates/consent-grant/src/return_path.rs
  - elohim/eprfs/epr-cli/src/device.rs
  - elohim/elohim-storage/src/services/device_consent.rs
  - app/elohim-elements/elohim-imagodei/src/device-consent/logic.ts
---

# One command on the new device, one yes on a device that is already yours

## What the operator went through on 2026-10-07

Registering a Che workspace with alpha took six things from the person that the system could
have done itself: a `--node` flag on every command because the terminal assumed port 8090; an
`ask` and then a separate `redeem` with a pasted code under a five-minute clock; a link carried
by hand between two workspaces; a portal that sent the person through sign-in and then said
`consent_unavailable`, because the alpha doorway serves the approval page and has no `/auth/consent/*`
behind it; working out which node is the asker and which the approver; and stale `epr` and
storage binaries that failed mid-flow instead of before it.

## What it should feel like

On the new device: `epr device join --label che-shem`. It prints "Asking to join as che-shem.
This device is uhCAk0t54…DucibY. Confirm it on a device that is already yours." and waits.

On a device that is already the person's, in a terminal or a portal: "che-shem is asking to act
for you. Does it show uhCAk0t54…DucibY?" One yes. The new device finishes on its own: no link
carried, no code pasted, no flag naming a port.

## Local-first reading: device join

- **Alone, key on the device:** the ask is prepared, signed by this node, and kept on disk
  (`consent-requests/<state>.json`) with the PKCE verifier; the device's fingerprint is shown.
  Nothing else is needed until a node that speaks for the person is reachable. The one step that
  waits is the person's yes, which is someone else's by definition.
- **Alone, key held by a doorway:** a hosted person's approving node *is* the doorway. The same
  `/auth/consent/*` routes the person's own node runs must be mounted by the doorway (today they
  are not: the alpha doorway answers 404 on `/auth/consent/view`). The doorway can see the ask
  and can sign the consent as the person; both are said on the screen.
- **Alone, no key:** does not apply; a visitor has nothing to join to.
- **Working version:** none; the ask is prepared in one act and either completes or expires.
- **Each kind of state:** the pending ask on the asking device (private, on disk, gone when
  used); the held code on the approving node (ephemeral, in memory, swept after five minutes,
  class C — no DHT entry); the joining record (Notarized, already designed in row 12 of the
  device-recognition backlog; unchanged here).
- **Arrivals:** the person's other device adds the yes and learns the asking device's label,
  key and network; when it is gone, the ask waits or expires, nothing is lost. A doorway adds
  the same yes for a hosted person and learns everything the approving node learns, standing
  inside the circle; when it is gone, a person with a node of their own is untouched. No pool.
- **Shown to the person:** on the asking device: waiting, and on what; approved by whom, for
  which identity; enrolled, with the joining record. On the approving device: the asking
  device's label and fingerprint, and the acts asked. On a portal that cannot take approvals:
  that, before sign-in, and what to do instead.
- **Refusals:** the portal that sends a person through sign-in to a dead end is a red flag
  (a request to a server before the person's own node has acted, and a wait that is not a
  wait). Fixed by probing availability before sign-in and by refusing at `ask` time when the
  named approver cannot be reached. A `--node` that must be typed is a request for something
  the machine already knows; fixed by discovery with an honest refusal when two nodes answer.
- **Stands as is:** the consent-grant rules, the private-network carrier, the joining record,
  the approver's `pending`/`approve` terminal flow. New homes: the held return path in
  `crates/consent-grant`; collect-by-verifier in `elohim-storage`; discovery, capability refusal
  and `join` in `epr-cli`; the availability probe in the `elohim-imagodei` consent element.

Back-fill: (1) with every doorway deleted, the person's own node runs ask, approve, collect
and enroll; (2) closing the laptop mid-ask leaves the kept request on disk and the held code on
the approving node until its window ends; reopening with no network, `join` says it is
waiting on the approving node; (3) the first thing that waits on someone else is the yes.

## The contract (what each lane builds)

### 1. `crates/consent-grant` — done 2026-10-07

`ReturnPath::Hold` and `ReturnTarget::Held` / `ReturnTargetView::Held` (wire `{"kind":"held"}`);
`Collection { state, codeVerifier, clientId, deviceKey }`, `CollectionRefusal`, and
`MemoryStore::collect` / `holds_for_state`. The raw code is never stored (the crate's
digest-only rule stands): collect finds the held delivery by the `state` bound at issuance and
checks the verifier, client and device against the fields bound then, never against a request
seen since; a mismatch spends nothing; only a delivery whose terminal chose `Hold` is
collectable. 117 crate tests pass.

### Rulings from the second-model read (Codex, 2026-10-07), binding on every lane

- `view` never overwrites an issued delivery; a second well-formed request with the same state
  does not change what collect authenticates against.
- No per-state lockout: a lockout would be the link-holder's denial attack. Mismatches are
  counted in a metric and logged; the verifier is 256 bits of randomness and the challenge is
  already in the link, so guessing is not a practical attack.
- `consent_declined` is produced only by an authenticated decide (`/auth/consent/pending/decide`
  or a signed-in portal decline that reaches it). There is no state-only decline route.
- The waiting window starts at `view`; the delivery window starts at issuance. Terminal
  answers (`consent_spent`, `consent_declined`, `consent_expired`) stay answerable for the rest
  of the state's window, then the state is forgotten.
- `join` uses `Hold` only on the http-approver branch. The private-network carrier keeps
  `Paste` on both sides, unchanged.
- The capability probe for collect is `POST /auth/consent/collect` with `{}`: `404` means the
  approving node predates it (the CLI then falls back to the paste path and says so); `400`
  means it exists. `/auth/device/self` stays the probe for "this node can join at all".
- Discovery's single-responder message says so: "using the only node of this machine that
  answers, at 127.0.0.1:<port>", because a stopped or slow peer is not an absence of ambiguity.
- A `200` from the availability probe means the host takes approvals, not that the person is
  signed in: the sign-in step still follows when no session exists.
- The witness pause and the undeclared-approver risk (backlog row 19) stand: "one yes" is the
  normal path, not an unconditional guarantee.

### 2. `elohim-storage` — the approving node holds and hands over

- When an admitted request's `returnPath` is `{kind: "hold"}`, `agree` keeps the issued code
  keyed by the request's `state` beside the existing held delivery (same five-minute window,
  same sweep), and answers the consent screen with `returnTarget: {kind: "held"}`.
- `POST /auth/consent/collect` with body `{"state": "...", "codeVerifier": "...", "clientId":
  "...", "deviceKey": "..."}` answers:
  - `200` with the same `Delivered` body `POST /auth/consent/redeem` answers, when a held code
    for `state` exists AND `pkce::challenge(codeVerifier)` equals the held request's
    `codeChallenge` AND `clientId`/`deviceKey` match the held request. The code is spent exactly
    as a redeem spends it; a second collect answers `410 consent_spent`.
  - `202` with `{"status": "waiting", "secondsLeft": n}` when the ask is known (the request's
    state was seen by `view`) but not yet agreed. Known states come from `view`, kept with the
    same window.
  - `404 consent_unknown` when nothing is held or known for that state; `403 verifier_mismatch`
    when the verifier does not match (the code is NOT spent: a wrong verifier must not let a
    link-holder burn the real device's code); `410 consent_expired` past the window; `410
    consent_declined` when the person said no (decline records the state for the window).
  - No session, cookie or proof is required: possession of the verifier is the authority, as it
    is for redeem. Rate-limit by state as redeem is.
- Nothing is written to the DHT or the content DB by this route. Entry class: Ephemeral (C).
- `GET /auth/device/self` already exists and is the capability probe the CLI uses; leave it.
- **A caller on this machine approves with no session.** Found in the live proof: on a node
  whose identity began by declaration and where nobody has ever signed in, `epr device approve`
  from the node's own terminal was refused `consent_not_signed_in`, because `caller_signed_in`
  resolved a session that `signing_caller` had already said a this-machine act does not need.
  The recorded ruling (device-recognition backlog row 1, dial 1: a caller on this machine acts
  as the person with no session) governs: a local caller on a node that holds a person is
  signed in for agree. Every non-local path is unchanged. A seventh friction, then: the
  approving terminal could not say yes until someone had signed in through a browser.

### 3. `epr-cli` — discovery, honest refusals, `join`

- **Node discovery** (replaces `DEFAULT_NODE` in `device.rs` and `approver.rs`), in order:
  `--node`; `ELOHIM_NODE_URL`; the ports file `.hc_ports` of the launcher when it records a
  storage port (today it records only conductor ports, so this is the slot for when it does);
  then probe `GET /auth/device/self` on `127.0.0.1` ports 8090, 8095, 8091, 8092, 8093 within
  one second each. Exactly one answers → use it and say which (`using this machine's node at
  127.0.0.1:8095`). None → refuse: "no node of this machine answers on 8090/8095/…; start one
  (`just dev start`) or name it with --node". More than one → refuse naming them all; never
  guess.
- **Capability refusal before anything is asked**: a node that answers `/health` but 404s
  `/auth/device/self` is refused with "this node's build predates device joining; rebuild and
  restart it (`just mesh build storage`)". The same probe guards `approve`, `pending`,
  `identity` subcommands against the approving node.
- **Approver reachability at ask time**: when the approver is an `https` origin (a doorway
  portal), refuse before printing a link: "this terminal reaches nodes over plain http only; a
  doorway portal's node must be named with --approver <http url>, or approve on a device that
  is already yours with `epr device approve '<link>'`". When it is `http`, probe `POST
  /auth/consent/view` with an empty body: a 404 is refused as "the node at <origin> does not
  take device approvals"; a 400 means the route exists.
- **`epr device join [--label L] [--portal P] [--approver A] [--node N]`**: one command that
  asks with `returnPath: {kind: "hold"}` when the approver is reachable over http, prints the
  link (or announces on the private network when no portal/approver is known, exactly as `ask`
  does today), then polls `POST /auth/consent/collect` every two seconds until `200`, `410` or
  the window ends, and finishes with the existing `finish` (check, enroll, report). Status
  lines name what it waits on. `ask` and `redeem` stay for the paste path.
- **Role suggestion**: `epr device` with no subcommand, and `join` on a node whose
  `/auth/identity/standing` says it already speaks for a person, say so: "this node already
  speaks for Matthew; to approve another device run `epr device pending`; to join it to a
  different person's identity run join with --portal".

### 4. `elohim-imagodei` consent element + `doorway-app` — no dead end after sign-in

- Before routing to sign-in, the consent page probes `POST /auth/consent/view` on its own
  origin with the request body. `404` (or no answer) → render the `consent_unavailable` refusal
  immediately, with the way through: "Approve on a device that is already yours:
  `epr device approve '<this link>'`" (the link is the page's own URL). A `401` means the host
  takes approvals and the person must sign in → the existing sign-in redirect. A `200` →
  review as today.
- `returnTarget: {kind: "held"}` after agreement renders "Done. <label> will finish joining on
  its own; nothing to copy." with the device fingerprint, instead of a code or a redirect.
- Selectors used by a2o stay unchanged.

### Rulings from the second-model read of the diff (Codex, 2026-10-07, after the live proof)

- One state, one held delivery: `MemoryStore::insert` replaces by state as well as by challenge,
  so collect can never pick the wrong delivery for a state.
- Terminal answers stay terminal: once collect has answered spent, expired or declined for a
  state, every later collect answers the same for the rest of the window; `view` never revives a
  terminal state, and a second `view` does not extend a shown record's window.
- The terminal's poll deadline follows the node's `secondsLeft`, not the ask's creation time.
- A kept ask or kept consent is resumed only when label, approver, acts and mode match the
  current flags.
- The collected consent is written to disk before enroll is attempted, atomically, and is kept
  for five minutes from when it was kept on this device (never from the approver's clock).
- Capability probes accept only conclusive answers (400 = exists, 404 = absent, 421 = own names);
  anything else is "could not tell; try again shortly", never a Hold request or a build verdict.
- A 421 during polling keeps the ask so the person can fix `ELOHIM_ALLOWED_HOSTS` without a new yes.
- Deferred by name: a **No in the portal does not reach a waiting `join`**; the portal's decline
  stays tab-only, and the asker waits out its window with an honest line. The authenticated
  decline that would answer `consent_declined` for a link ask is a small follow-up (an agree with
  nothing agreed, or a signed-in decline route), not a state-only route.

### Out of this pass, named

- **The doorway relay carrier** (device-recognition backlog row 2, carrier 2): a doorway relays
  the sealed ask and the sealed answer between a person's nodes that cannot see each other on a
  private network, and mounts `/auth/consent/*` for the people it hosts. Until it lands, two Che
  workspaces still need the link carried by hand once; `join` then finishes on its own.
- **Two ids for one person** (row 22): unchanged; join works under either.

## Proof (2026-10-07)

- On the fleet: pushed as e87ee7c69; edge #1572 rolled alpha; the morning's approval link now
  refuses before sign-in with the command (render: `genesis/a2o/reports/look/alpha-consent-after-roll`).
  The household serving receipt is owed on this tip (the recast household reproduced the
  dead-anchor wedge; see that backlog entry).

- Tests: consent-grant 117; epr-cli 314 (clippy clean); elohim-storage focused 52 (clippy
  clean); elohim-imagodei consent specs 223 (29 pre-existing failures in two untouched files);
  doorway-app 115 + production AOT build; imagodei-portal 87 + build. Portal states rendered
  headless (`genesis/a2o/reports/look/join-refused`, `join-held`).
- Live, on the i1006 network on one machine: the approver was the i1006 workspace peer
  (`:8093`, speaks for Matthew); the asker a new conductor+storage generated from the repacked
  happ (`:8098`, device `uhCAkFbHc…XU0hn2`). `epr device join --label fresh-device-3` →
  `epr device approve '<link>' --yes` once → "This device is enrolled. Joining record:
  uhCkkAh97F_w5OijD61Wy_EOm0fC-nCFZW3yWOH5Gz-gRBhKIvDR6"; the new device's standing reads
  `hasIdentity: true`. The code was never shown; nothing was carried by hand. `--node` was typed
  only because three nodes answer on this one machine and discovery refuses to guess.
- The che workspace node (`:8095`) turned out to hold no person at all (`hasIdentity: false`):
  Matthew's alpha identity was minted by the doorway, so on alpha the only node that speaks for
  him is the doorway, which mounts no consent routes. That is the hosted slice named below.
- What the proof found that was not in the design: (1) a terminal on the approving node's own
  machine was refused on a node nobody had ever signed in to (fixed, §2 last bullet); (2) an
  enroll failure after a collected consent needed a second yes (fixed: the consent is kept for
  its window and the next `join` enrolls from it); (3) a device whose packed happ carries stale
  coordinators fails with "zome function doesn't exist" (`sign_device_enrollment`,
  `get_human_root_evidence`) — the capability probe cannot see this, since it is the conductor's
  coordinator, not the storage's route; a `join` that fails there should say "this device's
  conductor runs older zomes than the network; update it" (open); (4) with no bootstrap or
  relay the zome cannot read the identity commitment — the join needs the network, and the
  wait it reports is honest.
- Second cycle on the final bytes (device `uhCAkMIUu…`): the first enroll came before the new
  conductor could read the identity commitment, the consent was kept, and `join` run again with the
  same flags enrolled "without asking again" (joining record `uhCkkhW8B…`): one yes total.
- Read two minutes later: the approver's own device list names both devices, so the joining
  record travelled over the network as well. The
  scenarios in `device-provisioning-paths.feature` stay `@wip` until their step definitions
  land; the blind-reader loop closed READY in two rounds (0/7/2 → 0/5/3).
