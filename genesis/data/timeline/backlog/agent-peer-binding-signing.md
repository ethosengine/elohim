---
id: "backlog-agent-peer-binding-signing"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Sign the AgentPeerBinding — replace STAGE1_SIGNATURE_SENTINEL so transport-id bindings are cryptographically verified (identity-head C2)"
slug: "agent-peer-binding-signing"
written: "2026-07-18"
author: "identity-head arc (Wave C2, operator: defer as security follow-on)"
status: "open"
priority: "high"
area: "imagodei/identity-transport-binding"
domain: "D2"
jobs: [elohim-edge]
cites:
  - genesis/docs/superpowers/plans/2026-07-17-identity-head-key-lineage-plan.md
  - genesis/docs/superpowers/specs/2026-07-17-identity-head-key-lineage-design.md
  - genesis/data/timeline/backlog/keyrotation-mint-path-witness-backed.md
tags: [identity, agent-peer-binding, transport-id, signature, security, did-elohim, alsoKnownAs, witnessed-binding]
---

# Sign the AgentPeerBinding — the deferred witnessed-binding leg (identity-head C2)

## Why this exists
The identity-head arc (Waves A–C1) ships the primitive: `did:elohim` resolves real
controllers + lineage. It also emits transport ids (libp2p PeerId / iroh NodeId) as
`alsoKnownAs` entries — but those are **self-asserted / unverified today**
(`STAGE1_SIGNATURE_SENTINEL = "c3RhZ2UtMS1zaWduYXR1cmU="` at
`elohim/elohim-storage/src/p2p/identity_binding_gossip.rs:129`; the binding gossip
payload carries the sentinel instead of a real signature). The DID document reflects
this honestly (phase-1 behavior C1 preserves): a transport-id `alsoKnownAs` is a
claim, not a proof.

## The work (C2 — security-critical, design-first)
Replace the sentinel with a real signature so a transport-id binding is
cryptographically verified before it becomes a `did:elohim` `alsoKnownAs`:
- **Design the scheme FIRST** (this is why it was deferred from the execution loop):
  which key signs `(agent_cid ‖ transport_id)` — the conductor-held head key, the
  transport key, or a cross-signature of both? How does elohim-storage reach the
  conductor-held head key to sign (admin/app-interface signing call)? Replay
  protection / freshness (a nonce or the existing `identity_handshake.rs`
  challenge/response scaffold — `elohim/elohim-storage/src/p2p/identity_handshake.rs`).
- Verify on receipt in the binding gossip path (`identity_binding_gossip.rs`,
  `reconcile/controller.rs:630`) and gate the `did:elohim` `alsoKnownAs` on a valid
  signature (the assembly currently populates from `store.transport_ids` with no
  verification — `bridges/did/did-bridge/src/did_elohim.rs`).
- **Red-team review before landing** — a weak scheme is worse than the honest
  sentinel (identity-transport binding is a spoofing surface).

## Scope note
Bounded to elohim-storage p2p (`identity_binding_gossip.rs` + `identity_handshake.rs`
+ `reconcile/controller.rs`) + the did:elohim assembly gate — NOT a DNA change (the
`AgentPeerBinding` entry type already exists in imagodei; this is the signing/verify
layer). Pairs with `keyrotation-mint-path-witness-backed` as the two identity-head
hardening follow-ons.

---

## Design review (2026-07-18) — build-ready design + 4-lens adversarial red-team

A design pass produced a build-ready cross-signature design; four independent red-team lenses
(transport-id-spoofing, replay-freshness-rotation, verification-placement/hash-move,
weak-scheme-vs-honest-sentinel) attacked it. **Verdict: the signature ALGEBRA is sound and
genuinely stronger than the sentinel — but the durable/temporal model, classification, and one
signed field are broken. C2 is SALVAGEABLE with the revisions below.**

### What HELD under attack (build on this, don't relitigate)
- **Bidirectional cross-signature** — transport key signs `agent_cid`, agent (head) key signs
  `transport_id`; both ids in both halves ⇒ neither side unilaterally claimable.
- **Domain separation** — distinct tags (`agent-attests-transport` vs `transport-attests-agent`)
  block lifting one half into the other; cross-transport (libp2p↔iroh, same ed25519 key) lift
  blocked because `transport_kind`+`transport_id` are inside both signed halves.
- **Length-prefixed canonical encoder** (NOT `serde_json::Value`/MessagePack map-ordering) — right
  call, avoids the WASM-boundary serialization trap.
- **`verify_strict`** (not `.verify()`) for malleability/small-order rejection.
- **Signing primitive already exists** — agent half via `ConductorSigningClient::sign` →
  `sign_for_agent` zome fn (`sign_for_agent.rs:86-122`); transport half signed locally (storage owns
  the libp2p `Keypair` / iroh `SecretKey`). No new conductor surface. **Verification lives in the
  coordinator gate + storage (DNA-hash-neutral); integrity hardening is a separate Tier-2 hash-move.**
- **Degrade-to-sentinel** is the correct failure direction (unverified ⇒ routing-only).

### The BLOCKER — the "pincer" (found independently by two lenses)
Freshness was placed on the self-asserted payload `issued_at` with a ±300s window
(`parse_iso_and_check_window`, `identity_handshake.rs:308-329`). That is a **liveness** primitive
applied to a **durable, gossiped, notarized credential**:
- **Keep ±300s at consume →** every durable binding is `cross_signed` only on the minting node and
  `unverified` on every remote node (dead-on-arrival exactly where attribution/`alsoKnownAs` needs it).
- **Remove ±300s →** a compromised *retired* key mints fresh `cross_signed` standing by **backdating
  `issued_at`** before its supersession (rotation is the compromise-recovery path, so leaked-then-
  rotated keys are exactly the live population).
**Resolution:** anchor BOTH freshness AND the supersession-cutoff on the **notarized DHT action
`Timestamp`** of the `AgentPeerBinding` entry (`must_get_action(...).timestamp`) — author-asserted
but source-chain-monotonic and DHT-validated, un-backdatable below a real supersession. Reserve
±300s + a **verifier-issued challenge nonce** (own domain tag, length-prefixed) strictly for the
*interactive handshake* liveness path.

### Other required revisions (all in-scope, DNA-hash-neutral)
- **Remove `binding_action_hash` from the signed core** — it's carried INSIDE the entry's own
  `signature` field, so signing over the entry's own ActionHash is self-referential/unconstructible;
  the anti-lift property is already fully held by the other signed fields.
- **Fail-closed classification (type-level, not a remembered `WHERE`):** `proof_status TEXT DEFAULT
  'unverified' NOT NULL` + backfill all existing rows to `unverified`; a **single writer chokepoint**
  that calls the shared `verify_cross_signature` (every other insert/update hard-codes `unverified`);
  positive-match `='cross_signed'` gates only (never `!=unverified`/`IS NOT NULL`); `transport_ids()`
  returns verified-by-construction with a distinctly-named `routing_transport_ids()` for the
  unverified cut; a `cross_signed`-only SQL view for `reciprocity_view`/`cluster_view`.
- **Durable vs ephemeral is the real safe/unsafe cut** (not care-vs-compute): only ephemeral,
  self-correcting ops (one CID-verified blob fetch, a re-authenticating connect) may consume
  `unverified`. Durable decisions — replica placement, diversity accounting, `peer_map`
  cross-transport mappings — MUST gate on `cross_signed` (else replica-capture / diversity-defeat /
  eclipse via spoofed `(agent→transport)` gossip).
- **Per-row panic-free decode** of the attacker-controlled proof bytes (one poisoned DHT entry must
  not empty the projection — the `EprRouter`-poisoned-row class); poisoned-row test.
- **Close the `sign_for_agent` no-EPR carve-out** for this use (`sign_for_agent.rs:94-111` skips the
  signer-match gate when no Agent EPR exists) — and state as a hard invariant that a `sign_for_agent`
  output is never on its own proof of `agent_cid` control; the verifier's head/lineage check is
  load-bearing and mandatory on every consume path, failing closed when no head/EPR resolves.
- **Reject `valid_until: None`** (clamp to a protocol max, e.g. 30–90d) so every credential expires
  and is re-minted under present key control; bound the nonce/dedup store by the validity window.
- **Encoder Option collision:** emit a 1-byte present/absent flag before `valid_until` (else
  `None` vs `Some("")` produce identical bytes); pin ONE canonical `transport_id` encoding per kind
  and store/consume only the pubkey-derived canonical form; reject non-ed25519 transports to `unverified`.
- **Revocation authorization:** supersede/revoke a binding for `agent_cid` only under a proof from
  `agent_cid`'s current head (blocks force-revoke DoS; enables compromised-transport-key cut).

### Bite-sized session decomposition (each independently executable to clean delivery)
- **C2-S1 — algebra core (no deps, DNA-neutral):** `BindingCore` + length-prefixed domain-separated
  encoder (present/absent flag on `valid_until`) + `CrossSignatureProof` (WITHOUT `binding_action_hash`)
  + shared pure `verify_cross_signature(core, proof, head_resolver)` + unit tests (injectivity,
  domain-swap/forged-half rejection). This is the red-team-validated sound core.
- **C2-S2 — signing wiring:** transport-half (libp2p/iroh by active backend) + agent-half via existing
  `ConductorSigningClient::sign`→`sign_for_agent` (carve-out closed). Deliverable: storage assembles a
  complete valid proof for its own `agent_cid`.
- **C2-S3 — notarized-timestamp anchoring (resolves the pincer):** durable proofs verify timelessly
  (validity window only, cutoff on notarized action `Timestamp`); ±300s + verifier-issued challenge
  nonce (own domain tag, length-prefixed) reserved for the interactive handshake.
- **C2-S4 — verify-on-consume + fail-closed projection:** `proof_status` migration (default unverified
  NOT NULL + backfill); single-writer verify chokepoint; per-row panic-free decode + poisoned-row test.
- **C2-S5 — type-level consumer enforcement:** verified-by-construction `transport_ids()` +
  `routing_transport_ids()`; `cross_signed`-only view for `reciprocity_view`/`cluster_view`; move
  durable placement + `peer_map` cross-transport to the `cross_signed` cut; did-bridge gate.
- **C2-S6 — ship R1, gate R2:** R1 (current-head signed) delivers as `cross_signed`; R2 (lineage-
  inherited) stays fail-closed to `unverified` until its deps land.
- **C2-S7 — Tier-2 integrity hardening (MOVES DNA hash — but that's fine in dev):** fold the two
  self-contained ed25519 verifies into integrity `validate_create_agent_peer_binding` — the only
  path to a *notarized* (not receiver-local) guarantee. The DNA-hash move is a normal dev
  `ALLOW_DNA_REINSTALL` reinstall (both genesis peers together), NOT a scheduled prod re-key — so
  Tier-2 is not a scary deferred arc. **Given that, reconsider the Tier-1/Tier-2 split itself:** the
  two-tier dance existed only to avoid the hash move; in dev, C2 may just do the integrity
  verification from the start (still keeping lineage-currency in the coordinator, since HDI has no
  `get_links`). If bindings migrate across the reinstall, still `scheme_version`-gate so legacy
  shape-only entries don't retro-invalidate; else fresh DHT + re-mint-under-current-head.

### Dependencies + honest state
- **C2-S1…S5 + S6-R1 are buildable now, DNA-hash-neutral, deliver R1** (current-head bindings verified).
- **R2 (lineage inheritance) ships INERT** until: (a) the B1b **redesign** (see
  `keyrotation-mint-path-witness-backed.md` "Design review" — B1b as originally specced is unsound),
  (b) pubkey-timeline populated by a real `on_key_rotation` (today a no-op stub, `controller.rs:277`),
  and (c) a new `find_head_by_chain_root` resolver — all keyed on DHT-canonical `KeyRotation.rotated_at`,
  never node-local arrival order.
- **Tier-1 verification is receiver-local, NOT notarized** — a DHT-direct third party can still be
  fooled by a shape-only forged entry until Tier-2. This is inherent to hash-neutrality; state it,
  don't oversell the coordinator gate as a trust boundary.

---

## Confirmed review finding (2026-07-23) — fallback-dial redirects shard PUSH bytes

A subsequent code review named a concrete consequence of the unsigned binding that this doc's
"replica-capture / diversity-defeat / eclipse via spoofed `(agent→transport)` gossip" line
(above, "Durable vs ephemeral is the real safe/unsafe cut") gestures at generally but had not
pinned to a file:line or traced through to its blast radius. Recording it here as its own
confirmed finding so it isn't lost inside the general framing:

`elohim/elohim-storage/services/transport_resolve.rs:78` — `resolve_agent_cid_to_libp2p`'s
source-2 fallback (`peer_identity_bindings::list_active_for_agent`) is, per the module's own
docs, "the load-bearing fallback when the manifest is empty (as in prod today)" — i.e. the
common path, not an edge case. Its rows are exactly the self-asserted, `STAGE1_SIGNATURE_SENTINEL`
bindings this doc is about. Consequence: an attacker who upserts a `peer_identity_bindings` row
claiming their own libp2p `PeerId` for a victim `agent_cid` gets that `PeerId` dialed by
`distribute_shards`/`push_shard` the next time that agent is selected as a shard custodian —
**silently**, because `shard_locations.peer_id` keeps recording the victim's `agent_cid` (per the
module's "Transport-layer only" contract, the resolver never writes a transport id back into the
`agent_cid` column). So the resilience card / placement/diversity accounting still shows the
legitimate steward as custodian while the actual shard bytes were pushed to the attacker's peer —
the redirect is invisible to every reader of `shard_locations`, not just unauthenticated.

This is the same root cause C2 already fixes (durable placement decisions must gate on
`cross_signed`, not `unverified` — see "Durable vs ephemeral is the real safe/unsafe cut" above):
`resolve_agent_cid_to_libp2p`'s source-2 fallback is a durable-decision consumer of
`peer_identity_bindings` and belongs on the `cross_signed`-only cut (C2-S5's
`routing_transport_ids()`/`transport_ids()` split), not general re-scoping. No new work item —
this subsection exists so the shard-push-redirect consequence is named and file:line-anchored
against the existing plan rather than rediscovered later.
