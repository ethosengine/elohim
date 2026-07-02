---
name: dna-hash-blind-to-coordinator-zomes
title: DNA hash blind to coordinator zomes
description: "Holochain DNA hash covers only integrity zomes + modifiers — coordinator-only changes need the update_coordinators hot-swap path, not reinstall"
metadata: 
  node_type: memory
  type: project
  originSessionId: 22183d22-e97a-41cd-b6ae-863ea1b29ce9
---

A Holochain **DNA hash covers only integrity zomes + modifiers** (network seed, properties). Coordinator zomes are explicitly excluded (`DnaDef.coordinator_zomes` doc: "zomes that do not affect the DnaHash"). Consequences (root-caused 2026-06-11, attestation-fix delivery stall):

- A coordinator-zome-only fix produces a new happ bundle whose per-role **DNA hashes are byte-identical** to the installed app. Any DNA-hash-based drift/staleness check reads "no drift"; `ALLOW_DNA_REINSTALL` never fires; conductors keep serving the OLD coordinator wasm from the PVC indefinitely.
- The `uhCok…` prefix in conductor errors is a **WASM hash** (`uhC0k…` is a DNA hash) — old-wasm-hash in rejections with a fresh bundle deployed is the signature of this class.
- The heal is `happ_manager.rs::sync_coordinators` (elohim-storage): compares per-zome coordinator wasm hashes (bundle `DnaFile` via `resolve_cells` vs conductor `get_dna_definition(cell_id)`) and applies the conductor's **`update_coordinators` hot-swap** — no uninstall, no new agent key, no DHT churn, prod-safe. Gate: `ALLOW_COORDINATOR_UPDATE` (defaults to `ALLOW_DNA_REINSTALL`'s value).
- A forced reinstall is the WRONG remedy for coordinator-only drift: it re-keys the agent, which can chicken-and-egg against guards in the still-old coordinator (the 2f02879d `register_doorway` lockout shape).

**Why:** every "fix shipped but conductors still run old behavior" investigation must first ask *which zome class changed* — integrity (DNA hash moves → reinstall/migration path) vs coordinator (hash does NOT move → hot-swap path).

**How to apply:** when a DNA-pipeline fix doesn't land on running conductors, check the fix commit's diff paths: `zomes/<x>/` (coordinator) vs `zomes/<x>_integrity/`. Related: [[local-stack-dht-anchor-gap]].
