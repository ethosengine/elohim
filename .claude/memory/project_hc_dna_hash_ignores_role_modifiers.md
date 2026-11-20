---
name: hc-dna-hash-ignores-role-modifiers
title: "hc dna hash ignores hApp role modifiers"
description: "A bundled .dna's hc dna hash never matches an installed cell under role properties; repack with the role's modifiers before calling a bundle lost."
id: project-hc-dna-hash-ignores-role-modifiers
metadata:
  node_type: memory
  title: hc dna hash ignores hApp role modifiers
  type: project
  originSessionId: 3e5b86fe-6ab7-4e02-acb0-77bc907448fd
  modified: 2026-10-26T21:17:44.527Z
cites:
  - elohim/holochain/dna/elohim/workdir/happ.yaml
---

`hc dna hash <bundled .dna>` hashes the DNA with its own `dna.yaml` modifiers. Holochain installs with the hApp **role's** modifiers, and elohim's happ.yaml sets `properties: {progenitor_pubkey: null}` on lamad, imagodei, mishpat and node_registry. Infrastructure has `properties: null` in both places, so only its raw hash ever matches. To compare a bundle with installed cells: unpack it, write each role's `network_seed`/`properties` into its unpacked `dna.yaml`, repack, then hash.

On 2026-10-26 this trap made the canonical household's bundle look "lost" across two sessions. The matching bundle was in another worktree all along, and is now kept at `genesis/local-dev/household-dowell/artifacts/installed-20261007/`. Its `environment.sh` had named a campaign-1.4 bundle whose lamad and infrastructure integrity differ; repointed at the recovered bundle 2026-10-26, and `hc-mesh.sh` preflight now refuses a bundle that does not reproduce a household's recorded cells. The conductor's `wasm.db` is SQLCipher-encrypted, so it can't be read offline.

**Why:** two sessions concluded "no bundle matches" and proposed recasting identities on that false evidence.
**How to apply:** before calling an install bundle lost or mismatched, hash with role modifiers applied. Related: [[native-peer-dev-channel-and-collective-memory-seed]] (the network should say which bundle it runs).
