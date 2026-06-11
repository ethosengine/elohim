# elohim/holochain/ — Holochain Infrastructure

This directory is the Holochain layer of the Elohim Protocol: the DNAs that form
the distributed truth layer, plus the toolkits, tests, and packaging that keep
them buildable and upgradeable. The architecture prose that used to live in this
`docs/` folder (ARCHITECTURE, P2P-DATAPLANE, SYNC-ENGINE, COMMUNITY-COMPUTE,
REACH, and siblings) was harvested and retired to git history on 2026-06-11 —
this README is a thin pointer index, not a design document.

**Why Holochain?** The protocol needs infrastructure that households can own
rather than rent: data lives with its owners, identity is cryptographic,
validation is distributed math rather than corporate policy, and every entry
type in `dna/` is a notary anchor no single party controls. That agency framing
is canonical in the protocol specification
(`genesis/docs/content/elohim-protocol/protocol-specification.md`) — this
directory implements it; read the canon for the why.

## What this directory contains

| Path | What it is |
|------|------------|
| `dna/` | Multi-DNA hApp — elohim, imagodei, mishpat, infrastructure, node-registry, hrea (lamad-v1 is a v1 archive for healing migration). Per-DNA Jenkinsfiles + build manifests. |
| `rna/` | DNA migration toolkit — Rust + TypeScript validators, templates |
| `tests/` | `sweettest/` integration suite + `manifest-hygiene/` manifest checks |
| `edgenode/` | Deployment packaging — Dockerfile, conductor config, compose |
| `elohim-wasm/` | WASM utility crate |
| `local-dev/` | Deployed bundles for the local dev stack |

## Where the truth lives

| Concern | Canonical home |
|---------|----------------|
| Integrity-layer rails — entry types, link budget, upgrade rails | `elohim/holochain/dna/CLAUDE.md` (the holochain-integrity-layer gospel; read it before any zome change) |
| DNA upgrade governance — forward-compat rules, network-seed ladder, lineage | `genesis/docs/content/elohim-protocol/architecture/2026-06-11-dna-upgrade-governance.md` |
| Protocol canon | `genesis/docs/content/elohim-protocol/protocol-specification.md` |
| Trust/data split — what the DHT notarizes vs. what storage carries | `genesis/docs/content/elohim-protocol/architecture/2026-05-11-tiered-quilt-stewardship-design.md` + `genesis/docs/content/elohim-protocol/history/2026-06-01-dht-is-a-notary-not-a-byte-store.md` |
| Founding vision — family nodes, community compute, agency stages | `genesis/docs/content/elohim-protocol/history/2026-06-11-community-compute-founding-vision-arc.md` |
| P2P dataplane + sync-engine design history | `genesis/docs/content/elohim-protocol/history/2026-06-11-p2p-dataplane-sync-engine-design-arc.md` |

## Live how-to (skills, in `.claude/skills/`)

- `hc-dev-orchestrator` — start/manage the local conductor + storage + doorway trio
- `holochain-import` — DHT seeding, hc-rna fixtures, snapshots
- `automerge-sync` — CRDT sync engine, stream positions, conflict resolution
- `libp2p-discovery` / `libp2p-transport` / `libp2p-protocols` — P2P networking reference
- `holochain-storage-api` — storage HTTP surface + Rust→TS type pipeline
