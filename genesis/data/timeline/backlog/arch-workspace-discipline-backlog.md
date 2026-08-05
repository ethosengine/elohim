---
id: "backlog-arch-workspace-discipline-backlog"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Workspace/crate discipline backlog — compose well, play nicely with others (p2panda-derived program)"
slug: "arch-workspace-discipline-backlog"
written: "2026-08-04"
author: "claude (p2panda cross-pollination mint pass, operator-directed clustering)"
status: "backlog"
priority: "high"
tags: [architecture, workspace, crates, discipline, licensing, versioning, ci, extraction, research-derived]
cites:
  - genesis/research/p2panda-cross-pollination-2026-08-04.md
  - genesis/data/timeline/backlog/arch-dataplane-refactor-backlog.md
  - crates/seam-contracts/Cargo.toml
---

# Workspace/crate discipline backlog (p2panda mint pass, 2026-08-04)

The operator's named pain: we struggle with a modularity discipline that would "compose well / play
nicely with others." The [p2panda survey](epr:p2panda-cross-pollination-2026-08-04) grounded it: our
50+ small crates already compose (Nexus registry, workspace inheritance, `seam-contracts`' lockfile
boundary test), but the three services carrying 336k of ~400k first-party Rust LoC sit outside every
mechanism. Items graduate to shifts/sweeps individually; this cluster is the subject's single
re-surfacing point. **Fold new workspace-discipline concerns here — do not mint siblings.**

| # | Item | What + why (grounded) | Effort | Owner shape |
|---|------|----------------------|--------|-------------|
| 1 | **`[workspace.lints]` declaration** | Move the `-D warnings` + clippy-threshold contract from the 59KB `.husky/pre-push.bash` into `[workspace.lints]` + `[lints] workspace = true` in all 8 first-party workspace roots + 3 services. Hook keeps enforcing; manifests start declaring. p2panda does it in 3 lines. | S (hours) | quality-sweep |
| 2 | **License coherence decision + sweep** | AGPL-3.0 / CAL-1.0 / Apache-2.0 / **20 unlicensed crates**; `steward/node` (Apache) consumes `elohim-storage` (AGPL) consumes `elohim-epr` (CAL-1.0). Needs ONE operator decision (outward blocks → `MIT OR Apache-2.0`? copyfarleft where intentional, *stated*), then a mechanical sweep. | Decision + S | **operator decision**, then quality-sweep |
| 3 | **Versioning that means something** | 41 crates frozen at 0.1.0, zero CHANGELOGs, zero release tags; Nexus publish scripts idempotently no-op. Adopt Keep-a-Changelog (per-crate prefixes) + bump-on-publish + tags, starting with already-published crates (`elohim-epr`, eprfs quintet, `seam-contracts`). Model: p2panda's 13-step `RELEASE.md`. | M (a day) | rust-architect shift (small) |
| 4 | **`cargo-deny` + `cargo-hack --feature-powerset` CI gates** | No license/advisory allowlist anywhere; feature-bearing crates (`seam-contracts`, `elohim-cache-core`, `elohim-peer-fabric`, storage `p2p`/`graph-native`) have no gate proving feature combos compile. Both are drop-in workflows on the Rust pipeline stages. | M (a day) | ci/quality shift |
| 5 | **Extraction sequence — services out of building blocks** | Ranked by measured coupling: `elohim-blob` (blob_store+sharding+dag_store+content_server, ~1.9k LoC, zero diesel/libp2p/holochain — free win) → `elohim-govern` (tally 7 mechanisms + sensemaking, row-type param only) → `elohim-admission` (shed/backoff/advertiser-health) → `elohim-transport` (retrofit libp2p onto the 7 existing iroh-side `*Backend` traits — **compose with, don't duplicate,** [arch-dataplane-refactor](epr:arch-dataplane-refactor-backlog) #14 shared-swarm-config and its #10→#12→#15 mod.rs chain) → `elohim-conductor` (invert the `hc_client`→projection callback first) → `elohim-reconcile` (P1 controller pattern; 61 diesel refs to invert). Method proven by `elohim-facings` (byte-identical golden); enforcement proven by `seam-contracts` (lockfile boundary test). Each extraction = its own bounded shift, never a mid-edit refactor. | M-L each | rust-architect shifts, sequenced |
| 6 | **I/O-free-core invariant, tested** | `elohim-epr` and `seam-contracts` are I/O-free de facto; state it as a boundary test (seam-contracts' lockfile-check shape) so it survives contributors. Apply to every crate born from item 5. | S | quality-sweep |
| 7 | **`anyhow`-in-public-API fix** | `elohim-bitswap/src/behaviour.rs:59-65` — `anyhow::Result` in the public `ContentStore` trait makes error variants unmatchable for consumers. One-afternoon thiserror conversion. | S | lint-fixer/quality-sweep |
| 8 | **Crate-tier ceiling policy** | `source-file-loc-ceiling@1` sees files, not crates — it makes monoliths tidier, not smaller. Author a crate-tier companion policy (concern-count / first-party-dep-direction signal) in `.claude/epr-meta/policies.yaml` once item 5's first extraction proves the remedy shape. | S (after #5 starts) | librarian + rust-architect |
| 9 | **The monorepo has no root LICENSE** | `find -maxdepth 2 -iname 'LICENSE*'` returns only `sophia/LICENSE`. Surfaced by the [Playnet survey](epr:playnet-free-association-cross-pollination-2026-08-05) §5.3: any contamination analysis against an external AGPL-3.0 artifact is *undefined on our side of the boundary*, and `elohim-storage`/`doorway` both serve over HTTP (where AGPL is a real copyleft event). Independent of Playnet — it blocks every future "can we link this?" question, and pairs with item 2's license-policy operator decision. | S (decision-bound) | **operator decision**, then librarian |
| 10 | **Policy relaxation has no working mechanism** | Doc-vs-mechanism drift found while building the research sovereignty membrane (2026-08-05). `sovereignty-ontology-guard`'s own `why:` promises an escape hatch — *"a nearest-wins `.epr-meta` override on this rule id — and the guard goes quiet."* Neither path works: a `policy:` binding **may not redeclare class** (so binding cannot relax), and inlining the id to relax it trips the `bind, don't redefine` advisory on **every** write in the subtree — tolling. Either wire a first-class relaxation (a `class:` override on a binding, or a `relaxes:` key) or correct the policy text so the next author does not rediscover it. | S | librarian |

**Below the line (not backlog-ready):** docs.rs metadata + `#![warn(missing_docs)]` rollout (after #2 settles which crates are outward-facing); crates.io publication fork-in-the-road for genuinely-general crates (after #2 + #3); repointing `placement-audit.py`'s `RESEARCH` surface from the dead `genesis/docs/research` to `genesis/research` (needs a status-vocab mapping for `Capture` first, else +25 no-status debt noise).

**Sequencing note:** 1, 4, 6, 7 are independent and afternoon-to-day sized. 2 gates 3's outward half. 5 is the long arc — one extraction per shift, `elohim-blob` first (zero measured coupling), transport leg only after the dataplane cluster's mod.rs decomposition chain reaches #12.
