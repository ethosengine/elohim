---
title: "Coherent transport-identity resolver — resolve the agent↔transport namespace mismatch the resilience join is blind to"
id: coherent-transport-identity-resolver-design
date: 2026-06-15
status: design
author: agentic-developer (matthew-edge resiliency session)
supersedes_practice: "direct string-equality identity joins in household_resilience.rs"
relates:
  - genesis/docs/superpowers/specs/2026-06-13-conductor-authority-arc-memory-scaling.md
  - genesis/docs/superpowers/specs/2026-05-28-mutual-storage-replication-dwelling-hub-design.md
  - .claude/data/matthew-edge-resiliency-rca-fanout-2026-06-15.md
---

# Coherent transport-identity resolver

## 0. Red-team verdict & scope correction (2026-06-15)

An adversarial review (rust-architect, verified at the *writers*, primary source) **falsified the
resolver's load-bearing premises.** Corrected scope: **the STOPGAP (§3.4 / §5 step 1 — populate
`humans.agent_pub_key`) is the actual fix for the dark card; the resolver is DEMOTED to speculative
infrastructure pending (a) a real signed self-emit path and (b) a real transport-id `provider` writer —
neither of which exists today.** This §0 supersedes contradicting claims in §§1, 3.3, 4, 5 (retained
below as the original exploration). Findings:

- **The resolver feed does NOT exist (§3.3 was false).** No edge node emits its own `AgentPeerBinding`;
  the only production caller of `create_agent_peer_binding` is the seeder (`seed-agent-bindings.ts:251`).
  Existing rows are split-brained: handshake rows (`source='handshake'`) carry a real `12D3Koo…` peer_id
  but a placeholder `agent_cid` (`AGENT_PUBKEY` unset → fabricated `uhCAk_<uuid>`, `main.rs:1325-1331`);
  seeder/dht rows carry a real notarized `agent_cid` but a fabricated `12D3KooW+sha256(humanId)` peer_id
  that "can NEVER match a running pod" (`peer-id.ts:11-46`). No row carries a real `(transport_id ↔ agent_cid)` pair.
- **`shard_locations.peer_id` is `agent_cid` (uhCAk), NOT a libp2p id (§1 table was false).** Verified at
  both writers: `seed_shard_manifest.rs:55-58` ("MUST be the steward's agent_pub_key… a libp2p PeerId
  will NOT join") and `p2p/mod.rs:1527` via `peer_selection.rs:253-255` (`SelectedPeer.peer_id == agent_pub_key`).
  So the stewarding join is already agent-keyed → **the stopgap fixes it directly**; the resolver's
  transport-id arm is a no-op there. (Same error was duplicated in `elohim-storage/CLAUDE.md` — now fixed.)
- **No production writer puts a transport id into `rea_commitments.provider`.** Seeder writes `uhCAk`
  (`seed-provide-rows.ts:239`); the runtime projects `provider` verbatim from the DHT payload
  (`mishpat_projection.rs:409-413`) with no located writer wiring `self_cid` there, and projects
  `state='proposed'` (`:473`) which the `state='active'` join filters out. The resolver's sole surviving
  consumer (the direct `commitment_backed` join) has no real transport-id provider to resolve.
- **LIVE SECURITY ISSUE (not deferred): `AgentPeerBinding` is self-asserted.** Integrity validation checks
  only non-empty fields + ordered window + non-empty signature (`imagodei_integrity/src/agent_peer_binding.rs:155-190`);
  the gossip publisher writes a `STAGE1_SIGNATURE_SENTINEL` (`controller.rs:623`); the "proof seam"
  `synthesise_dht_anchor_hash` is `sha256(peer_id+agent_cid)` — a row-key, not auth; the libp2p keypair
  and Holochain agent key are independent (no cross-signature). A gossiped spoof `(agent_cid=X, peer_id=Y)`
  credits victim Y's flows to attacker X — and **bindings are already consumed for attribution today**
  (`reciprocity_view.rs:48-58`, `cluster_view.rs:252`). → Routed to a security backlog item; the binding
  substrate must NOT be extended to economics/custody until a cross-signed control proof lands.
- **`peer_identity_bindings` is the right canonical store** (it alone carries `dht_anchor_hash` / `source` /
  validity / `superseded_by`) — but NOT because `peer_transport_manifest` is "iroh-only" (it has
  `lookup_by_libp2p_peer_id`); the real reason is provenance + rotation lineage. The iroh fallback arm
  (§3.1 step 3) reads a table with **zero production writers** (all `record_*` callers are `#[cfg(test)]`).

**Reordered plan:** ship step 1 (stopgap) now; **new step 0** = build the node self-emit path WITH a
cross-signed control proof (the unbuilt precondition); resolver steps 2-3 stay shelved until step 0
yields real signed rows AND a real transport-id provider exists. The §3.5 social-recovery design stands
as the *target* for that future emit/lineage path.

## 1. Problem

A node has **three non-interchangeable identity namespaces**:

| Namespace | Form | Where it lives |
|---|---|---|
| **agent identity** (`agent_cid`) | Holochain agent pubkey `uhCAk…` | `humans.agent_pub_key`; `rea_commitments.provider` (seeder); **`shard_locations.peer_id`** (misnamed — agent-keyed at both writers, see §0) |
| **libp2p transport id** | `12D3Koo…` peer id | `peer_identity_bindings.peer_id` (handshake rows); the libp2p swarm. *(No confirmed writer puts it in `rea_commitments.provider` — §0.)* |
| **iroh transport id** | iroh `NodeId` | `peer_transport_manifest.iroh_node_id` (zero production writers — §0) |

The resilience-card snapshot joins these by **raw string equality**:
`household_resilience.rs:172-174` (`humans.agent_pub_key = rea_commitments.provider`) and
`:74` / `:447-449` (`humans.agent_pub_key = shard_locations.peer_id`). When the two sides
hold different namespaces the INNER JOIN silently empties → **the card reads all zeros even
when the data exists.** This is not hypothetical; it is the standing cause of the dark alpha
card and it has bitten repeatedly:

- The **seeder** writes `provider = uhCAk` (agent key) — `genesis/seeder/src/seed-provide-rows.ts` header.
- The **runtime provide-loop** writes `provider = self_cid = transport peer id` (libp2p `12D3Koo…`
  / iroh NodeId) — `elohim/elohim-storage/src/node_transport.rs` doc.
- `humans.agent_pub_key` is **NULL in production** anyway (seeder sends `agentPubKey: null`;
  `reconcile/controller.rs:1103-1136 on_membership_projected` stamps only `household_id`).

So the join is empty for **two independent reasons** — a missing key (`agent_pub_key` NULL) and a
**namespace mismatch** (runtime provider is a transport id, never an agent key).

There are **four uncoordinated agent↔transport bridges** and the join uses none of them:

1. **`peer_identity_bindings`** (`db/peer_identity_bindings`) — `peer_id ↔ agent_cid ↔ dht_anchor_hash`,
   multi-source (`source ∈ {dht, handshake, gossip}`), `superseded_by`, validity window. Written by
   `ReconcileController::on_agent_peer_binding` (`controller.rs:566-579`) from the **notarized**
   `AgentPeerBinding` DHT signal (`is_integrity_kind`, `write_through.rs:211`), and also by handshake +
   gossip writers. `list_active_for_agent` exists (GraphQL consumes it). **This is the canonical resolver substrate.**
2. **`peer_transport_manifest`** (`p2p_iroh/peer_map.rs`) — `agent_cid ↔ libp2p_peer_id ↔ iroh_node_id`,
   the iroh-stack tri-identity map (`lookup_by_iroh_node_id`, `select_transport`). Iroh plane only.
3. **`node_transport.rs`** (iroh sprint) — the **self** seam (`self_cid()` / `status_peer_id()` per transport).
4. **`p2p/identity_handshake.rs::synthesise_dht_anchor_hash(peer_id, agent_cid)`** — handshake-time binding.

The mismatch is an **identity-resolution gap**, not a typo. Hard-coding one vocabulary re-breaks
the moment another writer uses a different namespace (exactly what the iroh sprint did to the
runtime provider). The fix is a **coherent client that resolves any transport identity → agent_cid**,
fed by the notarized binding the dataplane already gossips.

## 2. P2P Design Gate

### Entity: AgentPeerBinding (agent ↔ transport identity)
- **Classification**: **Notarized (A)** — *already exists*. `is_integrity_kind("AgentPeerBinding")`
  (`write_through.rs:211`); `AgentPeerBindingCreated` (`signals.rs:1194`); projected by
  `on_agent_peer_binding` (`controller.rs:549`). The community must verify "agent X owns transport
  peer Y." **No new entry type — DHT capacity untouched.**
- **Content Address**: **Agent-Scoped Composite** `(agent_cid, peer_id)` — the agent's signed claim
  over a transport id. `dht_anchor_hash` = `binding_action_hash`.
- **Source of Truth**: Holochain DHT. `peer_identity_bindings` is a Category-C operational projection.
- **Coordinator / Signal / Projection**: zome emits `AgentPeerBindingCreated` → `on_agent_peer_binding`
  upserts `peer_identity_bindings` (`source='dht'` authoritative on `superseded_by`) **and** gossips
  `PublishIdentityBinding`. Handshake + gossip arrivals also upsert (`source` distinguishes).
- **HTTP Route**: none new. The resolver is internal (consumed by the snapshot + custody + provide-loop).
- **Anti-pattern caught**: the current snapshot join IS "three address formats left undefined" /
  cross-namespace string-equality. The resolver corrects it. (Captured as a new gate anti-pattern row.)

### Design constraints discovered
- **Two projections of the binding**: `peer_identity_bindings` (peer_id↔agent_cid, multi-source,
  the libp2p-facing one fed by the DHT signal) vs `peer_transport_manifest` (tri-identity, iroh-plane).
  **Canonical = `peer_identity_bindings`** (it is the notarized-signal projection with provenance/validity);
  `peer_transport_manifest` remains the iroh transport-selection map and a secondary resolve source for
  `iroh_node_id → agent_cid`.
- **`node_transport.rs` is the *self* seam; this resolver is the *peer* seam.** Complementary.
- **Canonical join key = `agent_cid` (`uhCAk`).** A household is an agent-level concept; transport ids
  resolve *to* it.

## 3. Design

**A read-side identity resolver over `peer_identity_bindings`, that the resilience join resolves
*through* — leaving every write side transport-native.**

### 3.1 Resolver
Add a pure resolve function (storage `db/peer_identity_bindings.rs` + a thin service wrapper):

```
resolve_agent_cid(conn, identity: &str) -> Option<String>
  // 1. If `identity` is already an agent_cid (matches a humans.agent_pub_key
  //    OR a peer_identity_bindings.agent_cid), return it.            ← seeder path (uhCAk)
  // 2. Else treat as a transport id: SELECT agent_cid FROM peer_identity_bindings
  //    WHERE peer_id = identity AND superseded_by IS NULL
  //    AND (valid_until IS NULL OR valid_until > now)
  //    ORDER BY source-precedence (dht > handshake > gossip), observed_at DESC.  ← runtime path (12D3Koo)
  // 3. Else (iroh) consult peer_transport_manifest: iroh_node_id → agent_cid.
  // 4. None → unresolvable (the row simply does not count; honest, not a crash).
```

Identity-namespace tolerant: the same function resolves a uhCAk, a libp2p peer id, or an iroh
NodeId to the canonical `agent_cid`. The resolve is content-addressed by the notarized binding, so
it survives transport toggles.

### 3.2 Join rewrite (`household_resilience.rs`)
The two joins stop string-comparing and resolve through the resolver. Because diesel cannot call a
Rust fn inside SQL, the snapshot pre-resolves: load the candidate `(provider | peer_id, household
context)` rows, map each identity through `resolve_agent_cid`, then aggregate distinct households.
For the modest per-content row counts this is fine (the card is a per-request read, not hot-path).
A pure SQL alternative (resolve via a `peer_identity_bindings` LEFT JOIN with a COALESCE on the
direct-match) is the optimization if row counts ever grow — noted, not built (YAGNI).

Net: a commitment authored with `provider = 12D3Koo…` (runtime) **or** `uhCAk` (seeder) **both**
resolve to the same agent → the household lights, and stays lit when the iroh dataplane is toggled.

### 3.3 Feed (already exists)
No new dataplane plumbing: `on_agent_peer_binding` + handshake + gossip already populate
`peer_identity_bindings`. The spec only requires that **libp2p nodes actually emit the binding**
(the `AgentPeerBinding` for `agent_cid ↔ its libp2p peer id`) — verify on alpha; if a node never
emits its binding, its provide rows stay unresolvable (graceful — same as today, never a crash).

### 3.4 The humans side (the stopgap = step 1 of this design)
The join's *other* side is `humans.agent_pub_key`, which must hold `agent_cid` (`uhCAk`). Populating it
is the **stopgap** shipped first: extend `on_membership_projected` to also stamp
`humans.agent_pub_key = member_agent_key` (it already has the uhCAk key — `controller.rs:1070-1073` —
and currently drops it), NULL-guarded like the `household_id` stamp. This lights the **seeder-uhCAk**
path immediately (provider=uhCAk matches agent_pub_key=uhCAk directly). The resolver then completes it
by also lighting the **runtime-peerId** path. The stopgap is a strict subset of the durable design —
they compose, the stopgap is not thrown away.

### 3.5 Identity continuity & social recovery (avoid the lost-keys anti-pattern)
The canonical join key is `agent_cid`, but `agent_cid` (a Holochain `uhCAk…` keypair) is
**device-bound** — making it the *terminal* identity would reintroduce the cryptobro "lost keys in
the city dump" failure: lose the device, lose your standing, commitments, and household membership
forever. We reject that. **The durable, socially-recoverable identity is the human/household, not the
key:**

- The resilience card already aggregates by `household_id` (a social unit), not by key — **keep that.**
  The resolver maps any (rotatable) key or transport id → *current* `agent_cid` → `human.id` → household.
- **Lineage-aware resolve.** `peer_identity_bindings.superseded_by` + the existing `KeyRotation` /
  `KeyRevocation` / `RevocationAttestation` substrate form an identity lineage. `resolve_agent_cid`
  follows `superseded_by`, so a commitment authored under an old or lost key still resolves to the
  *current* agent, and a socially-recovered new key **inherits the prior standing** (commitments,
  household membership, custody) instead of starting from zero.
- **Recovery is a social quorum, as private & resilient as our storage.** Key/identity recovery rides
  the same RS-sharded encrypted-storage substrate (Shamir-style social shares distributed across the
  household/trust graph — the recovery-quorum instance of the REA compute-commitment primitive):
  household-attested, shares encrypted (private), sharded (resilient). No central key escrow.
- **Optional cryptographic hardening (opt-in, not default).** Default = social recovery (low friction,
  the protocol's resilience model). A vulnerable individual (activist, abuse survivor) may opt into
  stronger custody — hardware-key-only, no social recovery, maximal anonymity — explicitly accepting
  the lost-keys risk. A per-identity tier, never forced on everyone.

**Design test (the operator's bar):** identity is socially recoverable AND as resilient/private as our
sharded encrypted storage, with optional crypto hardening for the vulnerable. The resolver meets it by
anchoring on the household and following the binding lineage — never on a single terminal key. *(The
stopgap's NULL-guarded `agent_pub_key` stamp seeds the initial key; rotation/lineage is the resolver's
job, step 3 — the stopgap must not block a later supersede.)*

## 4. Self-red-team (questions I asked myself)

- **Canonical key — agent_cid or peerId?** agent_cid. Household/stewardship is an agent concept; a
  commitment by matthew's household must key to matthew's *agent*, not to whichever transport's peer id
  (which changes per transport). Keying to a transport id couples economics to transport — wrong.
- **Resolve-at-read vs reconcile-at-write?** Resolve-at-read. Write sides stay transport-native; the
  resolver is the single coherence point; new transports just register a binding. Reconcile-at-write is
  brittle (every writer must know the canonical vocab — the iroh sprint already diverged) and would
  require rewriting the runtime provide-loop the iroh sprint just shipped. Read-side is non-invasive.
- **Does it fix BOTH zeros (commitment_backed AND stewarding)?** Yes — both joins route through the
  resolver, so `shard_locations.peer_id` resolves too. The stopgap alone fixes only commitment_backed
  (and only the uhCAk subset); the resolver fixes the whole card. (Honest: ship the stopgap knowing it
  is partial.)
- **Security / spoofing.** The binding is notarized + signed by the agent; an agent can only assert its
  own `(agent_cid, peer_id)`. Risk: agent X falsely claims a `peer_id` it does not control, so a
  commitment with `provider = that peer_id` resolves to X. Mitigation: the `AgentPeerBinding` validation
  must prove control of the transport id (the handshake `synthesise_dht_anchor_hash(peer_id, agent_cid)`
  is the proof seam). **Open item: confirm binding validation requires a control proof, not just
  self-assertion** — see §6. The resilience card is low-stakes display, but the same resolver will gate
  custody/economics, so the proof matters before broader reliance.
- **Migration / backcompat.** Resolve-through *subsumes* direct equality (step 1 returns the identity
  when it is already an agent_cid). Empty resolver ⇒ same all-zeros as today (no regression); as bindings
  populate, it lights. Graceful, monotonic.
- **Performance / DHT.** No per-request DHT calls — `peer_identity_bindings` is the local projection.
  Per-content row counts are small; pre-resolve in Rust is fine. SQL-COALESCE path is the escape hatch.
- **Two projections — fold or coexist?** Coexist with a declared canonical (`peer_identity_bindings`);
  the resolver reads it first, falls back to `peer_transport_manifest` for iroh ids. Folding them is a
  follow-on hygiene item, not a blocker.
- **Lost-keys / social recovery (operator constraint).** `agent_cid` is device-bound; making it the
  *terminal* identity is the cryptobro lost-keys anti-pattern. Resolved by anchoring durable identity on
  the human/household and making `resolve_agent_cid` lineage-aware (follow `superseded_by` +
  key-rotation/recovery) so a recovered key inherits prior standing. Default social recovery; opt-in
  crypto hardening for the vulnerable. (See §3.5.)

## 5. Staged plan

| # | Step | Surface | Lights |
|---|------|---------|--------|
| 1 | **Stopgap** — `on_membership_projected` also stamps `humans.agent_pub_key = uhCAk` (NULL-guarded) + TDD | repo (`reconcile/controller.rs`, `tests/household_resilience.rs`) | seeder-uhCAk provide rows → commitment_backed (partial) |
| 2 | `resolve_agent_cid()` over `peer_identity_bindings` (+ iroh fallback) + unit tests | repo (`db/peer_identity_bindings.rs`) | — (library) |
| 3 | Snapshot joins resolve through the resolver (both `provider` and `shard_locations.peer_id`) | repo (`household_resilience.rs`) | runtime-peerId rows + stewarding → whole card |
| 4 | Verify libp2p nodes emit their `AgentPeerBinding`; backfill if absent | repo/seeder + verify on alpha | resolver has data to resolve |
| 5 | (hygiene) fold `peer_transport_manifest` ↔ `peer_identity_bindings` resolve into one entry point | repo | coherence |
| 6 | tune→document→report: a2o `@regression` "resilience card lights across transport namespaces"; shape-report dim | a2o + repo | regression guard |

## 6. Open questions / risks
- **Binding control-proof**: does `AgentPeerBinding` validation require proof the agent controls the
  `peer_id`, or is it self-asserted? Decides spoofing exposure once the resolver gates economics.
- **Do alpha libp2p nodes actually emit their binding today?** If not, step 4 is load-bearing for the card.
- **`peer_id` namespace in `shard_locations`**: confirm it is the libp2p peer id (so the resolver's
  peer_id arm covers it) vs some other form.
- **Two-planes tie-in**: the dwelling-hub storage-plane resilience (the durable household-resilience
  home, per the arc-memory-scaling spec) will key custody to the same identities — it needs this resolver
  too. Build it once, here.
- **Lineage depth & recovery authority (load-bearing for §3.5)**: does `peer_identity_bindings.superseded_by`
  chain across a *key rotation* (a brand-new keypair), or only within one key's peer-id changes? Does the
  `KeyRotation` / `RevocationAttestation` path write a `superseded_by` link the resolver can follow, and is
  the household the attesting authority for a recovery-rotation? If lineage stops at the key boundary,
  social recovery does not actually carry standing — confirm before relying on §3.5.
