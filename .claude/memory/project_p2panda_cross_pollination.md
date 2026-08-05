---
name: project_p2panda_cross_pollination
title: p2panda survey — crate-discipline transfer program
description: p2panda surveyed 2026-08-04; 7-item discipline adoption program + extraction sequence; p2panda-encryption candidate (audit-gated); truth-plane line holds.
metadata:
  type: project
---

Surveyed the p2panda org 2026-08-04 → `genesis/research/p2panda-cross-pollination-2026-08-04.md` (repos cloned: `p2panda`, `p2panda-aquadoggo`, `p2panda-reflection`). p2panda = the SSB descendant that deprecated its own monolithic node (aquadoggo) in 2024 and rebuilt as 10 composable Rust crates over iroh (I/O-free core, trait-per-domain stores, pluggable sync `Protocol`/`Manager`, ractor supervision). Materially the closest stack to ours (Rust, iroh, CBOR, Ed25519, BLAKE3, SQLite).

**The load-bearing verdict (operator's axis):** their whole project is 65k LoC; our `elohim-storage` alone is 243k / 23 concerns / 38-field god-struct. We don't lack discipline — 36 small crates + Nexus registry + `seam-contracts` (which out-disciplines p2panda via its lockfile boundary test) — we lack its application to the services. "They made every unit a building block; we made the building blocks and kept writing services around them instead of out of them."

**The 7-item adoption program (survey §adopt-now):** (1) `[workspace.lints]` in manifests (contract now lives only in 59KB pre-push bash); (2) license coherence (AGPL/CAL-1.0/Apache/none ×20 unlicensed — our one published crate is CAL-1.0, reads as accident); (3) CHANGELOGs + version bumps + tags (41 crates frozen at 0.1.0, publish scripts no-op); (4) cargo-deny + cargo-hack feature-powerset CI; (5) extraction sequence by measured coupling: elohim-blob (free) → elohim-govern → elohim-admission → elohim-transport (retrofit libp2p onto the 7 existing iroh-side `*Backend` traits) → elohim-conductor → elohim-reconcile; (6) ephemeral-gossip-for-presence / durable-sync-for-content as named idiom; (7) I/O-free-core as tested invariant.

**Protocol-level:** `p2panda-encryption` (I/O-free, MIT/Apache, DCGKA/2SM/Double-Ratchet, fuzz-tested) is a serious candidate for our unbuilt encryption layer — GATED on unconfirmed security audit (announced Feb 2025, never verified published) + ed25519→X25519 substrate + p2p-design-gate for `KeyEnvelope`. PSI confidential topic discovery is the private end of the locate-token design space.

**Minted 2026-08-04** into backlog clusters (cluster-first discipline, operator-directed): discipline program → `arch-workspace-discipline-backlog`; borrows → `arch-dataplane-borrows-backlog`; encryption → `arch-confidentiality-plane-backlog`; map = `genesis/data/timeline/backlog/CLUSTERS.md`; provenance chain documented there (research → cluster row → spec → code+scenario → chronicle).

**Cardinal line (3rd survey running):** writer-signed validity / key-as-identity / ACL-CRDT-as-authority never cross into the truth plane. **Watch:** `p2panda-blobs` = 4th sighting of flag-with-no-reader (README claims working feature; lib.rs is 5 lines, code orphaned behind missing `mod`) — extends the Freenet lint. Related: [[project_holepunch_cross_pollination]], [[project_cross_pollination_surveys]], [[project_iroh_dataplane_actual_state]].
