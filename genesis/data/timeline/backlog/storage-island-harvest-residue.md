---
id: "backlog-storage-island-harvest-residue"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Storage island harvest residue — encryption-at-rest vision + unconsumed sovereignty/cluster scaffolding + identity serving stub"
slug: "storage-island-harvest-residue"
written: "2026-06-11"
author: "storage island recompose (backlog-items harvest agent, code-verified)"
status: "backlog"
priority: "medium"
tags: [elohim-storage, island-recompose, harvest, reach, encryption, sovereignty, security]
derived_from:
  - elohim/elohim-storage/REACH.md             # retired to git 2026-06-11 (storage island recompose)
  - elohim/elohim-storage/P2P-ARCHITECTURE.md  # retired to git 2026-06-11 (storage island recompose)
  - elohim/elohim-storage/EDGE-ARCHITECTURE.md # retired to git 2026-06-11 (storage island recompose)
cites:
  - elohim/elohim-storage/src/sovereignty.rs
  - elohim/elohim-storage/src/cluster.rs
  - elohim/elohim-storage/src/identity.rs
  - elohim/elohim-storage/src/services/sealed_against_self.rs
  - tiered-quilt-stewardship-design | the canonical substrate model any encryption-at-rest answer must compose WITH (temperature/floor, custody commitments) — currently silent on encryption | sha256:9f9c6a1c391712b3 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-11-tiered-quilt-stewardship-design.md
  - genesis/data/timeline/backlog/http-reach-enforcement-gap.md
  - genesis/data/timeline/backlog/reach-vocabulary-frontend-strand.md
---

# Harvest residue from the retired elohim-storage island

Code-verified residue from the retired `elohim/elohim-storage/{REACH,P2P-ARCHITECTURE,EDGE-ARCHITECTURE}.md`
(git history). Everything else still-true in those files is already homed (tiered-quilt §2 dual-plane,
DHT-notary history record, cache-core gospel + extraction design, doorway gospel, storage gospel
§Design Vocabulary, the two reach backlog entries). Only the three items below are homed nowhere else.

## 1. Encryption-at-rest by reach — vision, never built, no canonical home

REACH.md designed per-reach content encryption: `private` = encrypt to the beneficiary's key,
`invited` = symmetric shared key distributed to the invite list, household-tier (`local`) = family
cluster key, `neighborhood`+ = cleartext; key distribution via Holochain DNA (its Migration Phase 3).

- **Never built**: `elohim/elohim-storage/src/blob_store.rs` contains zero encryption — every blob
  is stored cleartext regardless of reach. No reach-keyed encryption anywhere in elohim/, doorway/,
  or steward/ (`encrypt_for_agent`/`encrypt_symmetric`/reach-keyed `cluster_key`: zero Rust hits;
  steward's `cluster_key` config is join-key discovery, unrelated).
- **No canonical home**: the tiered-quilt stewardship design (`2026-05-11-tiered-quilt-stewardship-design.md`)
  is silent on encryption (zero matches). The only at-rest-encryption designs in the corpus are
  recovery-domain: sealed-against-self 2-of-2 sealing (trust-compute-gradient brainstorm §10.1, live
  at `src/services/sealed_against_self.rs`) and the deferred custodian-share encryption note
  (`src/db/models.rs:3418-3421`, homed in code + migration comment). Those protect recovery records,
  not reach-restricted content.
- **Distinct from the live recovery/shamir machinery** (`src/p2p/shamir_transport.rs`,
  `src/p2p/recovery_*.rs`) — that is key/record RECOVERY, not content encryption.
- **Framing**: when restricted-reach content lands on the quilt substrate, the encryption question
  reopens — shards of `private`/household-tier content held by non-beneficiary custodians are
  readable by their hosts today. Compose the answer WITH the tiered-quilt model (temperature/floor,
  custody commitments) rather than re-deriving a parallel scheme from the retired doc.
- **OPEN QUESTION**: which reach vocabulary would even key the tiers — the retired design used the
  geographic 8, which matches no live vocabulary (see `reach-vocabulary-frontend-strand.md`); the
  reconciliation must land first.
- **OPEN QUESTION**: key distribution via Holochain DNA (as designed) vs reusing the live recovery
  primitives (shamir custodian shares, sealed-box nesting) as the key-escrow substrate.

## 2. Unconsumed sovereignty/cluster scaffolding — wire or delete (record, do NOT bless as live)

These modules implement REACH.md's "Sovereignty × Reach" two-gate (node-level `should_serve` ×
content-level reach check) and its reach→trust replication mapping. They compile, are declared, and
have ZERO consumers outside their own files:

- `src/sovereignty.rs` — `SovereigntyMode` (:19, Laptop/HomeNode/HomeCluster/Network),
  `ClusterRole` (:64), `should_serve()` (:117, correctly checks `family_members`). Declared at
  `src/lib.rs:179`; only external reference is `src/cluster.rs:35` importing `ClusterRole`.
- `src/cluster.rs` — `TrustLevel` (:44, Family/Extended/Community/... ordering = REACH.md's
  `reach_to_minimum_trust` target), `ClusterManager` (:115), its own `should_serve` (:239,
  exercised only by in-module tests). Zero consumers outside the module (`#[cfg(feature = "p2p")]`,
  lib.rs:178).
- The live serving path never calls any of this; its actual reach-gate state is tracked in
  `genesis/data/timeline/backlog/http-reach-enforcement-gap.md` (not duplicated here). The live
  enforcement philosophy also inverted vs REACH.md: `src/p2p/reach_authorization.rs` does
  author-side earning + receiver-side pre-authorization, not delivery-side trust filtering.
- **Decision needed**: either wire `SovereigntyMode`/`TrustLevel` into the live serving/replication
  path (as the node-level gate composing with the reach gate) or delete both modules. Leaving them
  declared-but-dead invites a future agent to "discover" and bless them as the live model.

## 3. identity.rs `should_serve` permissive stub — security-adjacent flag

`src/identity.rs:310` — `NodeIdentity::should_serve()` returns `serve_public || serve_family` with
`// TODO: Check if requester is in family list` (:315): when `serve_family` is set it serves ANYONE,
ignoring the requester entirely. Calibration: repo-wide grep finds zero callers — a latent trap, not
a live hole today. But it sits parallel to `sovereignty.rs:117`, which implements the same concept
CORRECTLY (checks `family_members`); whoever wires item 2 must not pick the permissive twin.
Tag-worthy as [security] because the failure mode is silent: wiring it "works" in every happy-path
test while granting family-scope serving to the world.

## 4. Own-sweep verdict: nothing further

Independent re-read of all three island files found no additional still-true-but-unhomed fragments
beyond the dossier's verdict. Notable already-homed items deliberately dropped: acknowledgment tiers
(superseded by REA commitment/attestation model, tiered-quilt §4), eviction-by-reach (superseded by
tiered-quilt temperature/floor), ContentLocation DHT entry + signal-server bootstrap (superseded by
live Kademlia provider records + `p2p_bootstrap_nodes` config), sha256-hex convention (storage
gospel), ReachAwareCache geographic ordinals (recorded as site 4 in `reach-vocabulary-frontend-strand.md`).
