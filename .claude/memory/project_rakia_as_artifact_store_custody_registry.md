---
name: project_rakia_as_artifact_store_custody_registry
title: rakia = package manifest over the dataplane store
id: project-rakia-as-artifact-store-custody-registry
description: "2026-09-25: the p2p dataplane IS the store; rakia adds none — it is the package manifest giving blobs meaning over the storage SDK; latest = channel head."
metadata:
  node_type: memory
  type: project
  originSessionId: 184fb351-36f9-402f-b933-974a3cf7b37c
  modified: 2026-10-27T17:04:12.359Z
cites:
  - elohim/rakia
  - "steward-space-landing-plane-design | The steward space | sha256:53718b5898db3b15 | path: genesis/docs/superpowers/specs/2026-10-27-steward-space-landing-plane-design.md"
---

**Operator steering 2026-09-25 (compute-agreements sprint), corrected the same day:** rakia should fill the repository / registry (Nexus / Harbor) role — but *the store itself is not unbuilt*: the p2p dataplane supplies it (elohim-storage `BlobStore`, blob protocols over iroh/libp2p, `peer_blob_inventory`, `replicates-content` / `custody-blob` commitments, ≤1 MiB CIDv1 chunks). Rakia is the **installation / package manifest** that supplies meaning and relationships to particular blobs (this CID is `compute-executor` from step X, depends on Y, supersedes Z, pinned by channel `dev`), as a composition of the storage SDK. Seam: what you add is a manifest → **SDK seam** (seam map §3.5). Rakia = רָקִיעַ, the firmament: peers are stars, the build graph is their constellation, a blob at its CID is a star, a channel head is the course a dependency follows.

**Ruling R12 (plan `/projects/.claude-config/plans/we-ran-into-a-cryptic-robin.md`):** wired = chunked, leased, content-addressed transfer between peers + inventory + custody schemas; composition to write (SDK-side, small dataplane additions at most) = (a) a manifest artifact entry naming a blob CID with relationships, (b) persistent custody minted for build outputs (not task-scoped leases), (c) `brit build put` with the artifact CID as `output_cid` (today a run-record CID), (d) multi-source chunk fetch (`materialize` takes one base), (e) channel-head resolution — "latest" is a declared head, never recency. lvi Track B stays deferred and is this composition.

**The human experience of rakia (operator, 2026-10-27, steward-space design session):** *"rakia for a human is experienced like a view you can ask for.. and then experience, that cybersyn control room, and can see from your perspective.. your devices or others, in their nebulae, forging stars in the firmamemnt"*. The operator also set the destination: native p2p CI/CD, where anyone with compute in a community helps build an application and inherits the EPR primitives (governance, story, VSM, accountability, REA). He finds GitHub branches, PRs and Jenkins branches big friction that does not move toward that destination.

The operator corrected my first reading the same day: *"don't undersell it.. rakia is just the brand.. we can't abandon our primitives to pivot to a view... the 'stars' are the artifacts... the dependency tree, sort of becomes the constellation.. everything we're doing RIGHT now, has to be done right for that next-generation CI/CD, repository, registry, build-cache, experience, inclusive and honest. To be orchestrated and composed."*

How to apply:
- Rakia names the WHOLE system: CI/CD, repository, registry, build cache and experience. It is not a view.
- Stars = artifacts, content-addressed blobs on the dataplane. Constellation = the dependency graph between them.
- A stage FORGES a star. A signed verdict ATTESTS one. The asked-for, perspective-relative Cybersyn view only renders what the primitives already make true; it never owns or invents data.
- Every slice built now must emit rakia-grade primitives:
  - the gate verdict is a brit attestation whose subject is the closure and whose `output_cid` is the artifact CID (R12 c);
  - an offer is a bundle artifact by CID;
  - a landing intent references artifact and closure CIDs;
  - build outputs get custody.
- Integration avoids forge branches and PRs. Jenkins is a witness, then one provider among many.
- **Valueflows back to every steward (operator, same session):** *"attestations (ie. the valueflows...) back to all the stewards.. everyone gets to look at the sky.. not having a telescope you might not see the intricacies of any particular object (anonymity in aggregate), but they're accounted for.. trust but verifiable trust from the intimate edges of the network, to the rakia, in a clear dark sky, that is an inheritance to all mankind."*
  - Every forge, attest, review, use or custody act emits a VF event (produce, use, deliver-service, custody) alongside the artifact.
  - The event is accounted to the true PRINCIPAL. On the real network today that is human:matthew: the agents are his delegates, recorded as provenance (persona, session, device), not as separate economic parties ("vendor is provenance not payee"). Persona-level attribution and multi-human accounting are exercised in Simulacra until a second real human or earned agent standing exists. The operator ruled 2026-10-27 that challenger H7's "attribution must never default to the human" is wrong for the real network; see [[project-real-network-is-one-human]].
  - The aggregate sky is visible to all. Per-object detail needs reach or relationship (the "telescope"; SDO/RWA boundary 6, consumer-blinded census).
- Home: `genesis/docs/superpowers/specs/2026-10-27-steward-space-landing-plane-design.md` (A5, A7, A8, A10, and the S2/S4 rakia node).

**Why:** the executor binary vanished from the cargo pool on the 2026-09-24 pin move and rung H had to rebuild it from a stash; as a rakia manifest entry over the dataplane, any household peer would serve it. **How to apply:** never propose a new artifact plane, registry service or "store"; name built artifacts by CID and record sha256/CID in receipts now; design the registry as a rakia manifest composition over the storage SDK (missing node `artifact-manifest-entry`, chain build-step → artifact-fetched-by-peer). Related: [[project_rea_compute_commitment_primitive]], [[feedback_upgrade_propagation_north_star_wall_clock]], [[project_lvi_devspace_peer_runtime]], [[project_head_reach_freshness_semantics]], [[project_inventory_exchange_not_byte_replication]].
