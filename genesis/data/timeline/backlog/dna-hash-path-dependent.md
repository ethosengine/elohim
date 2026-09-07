---
id: "backlog-dna-hash-path-dependent"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "DNA hash is path-dependent — identical lamad integrity source packs to three different hashes from three checkouts (worktree, main tree, fleet receipt), because rustc's per-crate metadata hash bakes in the absolute source path"
slug: "dna-hash-path-dependent"
written: "2026-09-07"
author: "sprint 2026-09-08 T9a (reproduce and file, do not fix — D-F)"
status: "open"
priority: "high"
jobs: [elohim-holochain, elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "habit:happ-lineage-migration"
tags: [dna-hash, path-dependence, rustc-metadata, worktree, ci, happ-lineage-migration]
---

**Reproduced 2026-09-07, sprint T9a (D-F: reproduce and file, do not fix).**

## Measurement

Packed `dna/elohim` (the `lamad` DNA — single-DNA content_store zomes) from two checkouts of
the SAME integrity source, no build config touched:

| Checkout | `hc dna hash workdir/lamad.dna` (fork hc, `/projects/.claude-config/tools/hc-fork-25dd2d0be144/bin/hc`) | integrity wasm sha256 (`content_store_integrity.wasm`) | packed |
|---|---|---|---|
| worktree `/projects/elohim/.claude/worktrees/sprint-0908` (HEAD `ef6ae9120`) | `uhC0kEZTgq_RVas2rLzUsEDu17TIvJePpqfnTZWh2ZE5_GrPqdpHy` | `a9373f6b4eb79478d482be7f76226baa5d127a4e0446270ccca00d258a4ddef` | 2026-09-07, this task (`just pack` from `elohim/holochain/dna/elohim`, no cargo/git run in the other checkout) |
| main tree `/projects/elohim` (pre-existing pack, read-only — not rebuilt) | `uhC0kLJHygE_XFy1DFnLyMQMaWmpzR1hMX7gAFBmfWrnRUyI-iYlt` | `2b970c9ad486f90e86d9b82e78024ea49b1fc49eddc78ecba573ee4f59a874c` | 2026-09-07 09:03/09:04 (a prior session's pack; unrelated to T7a's in-flight coordinator-only edit to `feedback_signal.rs`, since that file is in the coordinator zome and does not move the DNA hash) |
| alpha fleet receipt `genesis/a2o/reports/workspace-release/2026-09-06/workspace-release-20260906043835.json` (`dnaHash` field, older source commit — context only, not a same-source comparison) | `uhC0kRGwtzMN--M7GzoA25aSqEaKEskTFelXj_ByCkx7DpNX9AdFr` (one of several `dnaHash` rows in that receipt; every DNA-role hash differs, as expected) | not captured (no matching wasm artifact retained) | 2026-09-06 |

Same integrity source (worktree HEAD `ef6ae9120` carries no diff against the coordinator-affecting
files under `dna/elohim/`), two different checkout **paths**, two different `.dna` files, two
different DNA hashes. This is not the fleet-receipt row (that one is from an older commit and is
listed only as a third data point that a real network identity existed at yet another hash) — the
load-bearing comparison is worktree-vs-main-tree, same day, same source.

## Mechanism confirmed

`strings` found no literal embedded path in either wasm (`grep -c '/projects'` = 0 in both) — the
dependence is NOT a debug-info path string. Diffing the full sorted `strings` output of the two
`content_store_integrity.wasm` files shows every Rust-mangled symbol in the binary carries a
per-crate disambiguator hash that differs between checkouts, e.g.:

- worktree: `..._RNvCsfY6IeLnF72V_23content_store_integrity8validate`
- main tree: `..._RNvCsd3ffmjXaO7p_23content_store_integrity8validate`

(`CsfY6IeLnF72V_` vs `Csd3ffmjXaO7p_` — same length, same position, only the hash differs; this
pattern repeats across all ~8,100 unique symbols in the module, plus every crate the module links
against that is compiled fresh — `holochain_wasmer_guest`, `hdi`, `serde_json`, etc., since their
metadata hash is also derived per-build-graph.) Cargo's per-crate "stable crate id" / metadata
hash factors in the absolute path of the workspace/crate root among its inputs; two checkouts at
different absolute paths (`/projects/elohim/.claude/worktrees/sprint-0908/...` vs
`/projects/elohim/...`) therefore produce different disambiguator hashes for every crate compiled
from source in that build graph, which propagates into every exported/mangled symbol name in the
resulting wasm, which changes the final wasm bytes globally (not a localized diff — the two
`content_store_integrity.wasm` files differ in size by 36 bytes and differ throughout), which
changes the DNA hash `hc dna pack` computes over the integrity zome.

`dna/elohim/justfile:7`'s `RUSTFLAGS` export and the DNA `Jenkinsfile:154,857` `RUSTFLAGS=""`
export are the confirmed reason a `.cargo/config.toml` `[build] rustflags` would never be
consulted on either path (both invocation paths set the env var directly, which always wins over
config.toml) — that was the original hypothesis for a *different* class of hash-moving change
(a `--remap-path-prefix` flag), and it is confirmed dead as a config.toml-based fix. The
mechanism found here (crate-metadata-hash-from-absolute-path) is a distinct, additional cause:
it moves the hash even with RUSTFLAGS held byte-identical across both invocations, purely from
where the checkout sits on disk.

## Consequence

A DNA packed from a worktree is a **different network** from one packed from `/projects/elohim`,
and both are different again from whatever CI/Jenkins packs at its own checkout path (and,
per the DNA justfile's own recorded 2026-09-02 lesson, a `RUSTFLAGS` remap flag change in CI has
independently moved every DNA hash before). Concretely:

- "Prove on local mesh first" (CLAUDE.md's cadence discipline) is undermined for anything
  DNA-hash-sensitive done from a worktree: a worktree-packed conductor cannot talk to a
  main-tree-packed conductor on the same DHT (different network_seed-plus-hash identity), so a
  worktree rehearsal of an integrity crossing is not evidence about what CI/the fleet will
  produce — the hash itself, not just the content, differs.
- Release-adoption's `appliesTo` binding (`services/release_adoption/`) and the
  `happ-lineage-migration` habit's whole invariant (a crossing is refused unless the release
  names what it migrates from, matched by hash) are keyed on this hash. A path-dependent hash
  means the SAME logical integrity change can mint two different "migrates from" identities
  depending on which checkout produced the release artifact — a lineage-tracking hazard, not
  just a CI-triggering annoyance.
- This is exactly the class of event the DNA justfile's own comment already names: "moved every
  DNA hash in CI" — this backlog item generalizes that from "a stray RUSTFLAG" to "any checkout
  whose absolute path differs from the one that produced the last comparison point," which
  includes every worktree by construction.

- **Measured on the running mesh, 2026-09-07 20:09Z (T7a):** `just mesh coordswap` from this
  worktree against a mesh installed from the main-tree bundle was refused by
  `happ_manager::lineage_mismatch_error` for the lamad role only —
  `dnaHashMismatch (installed=uhC0keuLMYBe0sj4ZqLvIyuVqCXcxPL3PbHyiqx40UGgBh-bTbBOl,
  bundle=uhC0k8pow3EM3IVN9lZhCBlgnwyKrReAAXLKTEKnEFcPJQxjPZLNm)` — while the four roles whose
  DNAs were copied unchanged showed zero drift, which isolates the cause to the path. The guard
  is right (it cannot tell path drift from an integrity change) and the operational rule
  follows: **rung-1 coordinator hot-swap is a main-tree vehicle; a worktree needs a full
  `just mesh stop` / `MESH_HAPP_PATH=<its bundle> just mesh start`** (fresh DHT, fresh keys;
  ready in 194 s that night). The coordinator-only change itself was proven hash-neutral by
  packing before/after from ONE path (`uhC0kEZTgq…` identical, only `content_store.wasm`
  moved) — the comparison discipline this atom prescribes.

## Remedy (held — do not implement this sprint, per D-F)

**Correction (2026-09-08, from the measurement above):** the mechanism is the rustc
stable-crate-id / `-C metadata` disambiguator, which Cargo derives from the package id of a
*path* dependency — and that id carries the absolute checkout path. `--remap-path-prefix`
rewrites paths embedded in debuginfo and panic locations; it does **not** change the
`-C metadata` hash, so a remap alone leaves every mangled symbol different across checkouts
and the DNA hash still moves. The remedy that actually pins the hash is a
**checkout-independent package path for the DNA crate graph** at pack time — pack from one
canonical path on every builder (a bind-mount or symlink such as `/elohim/dna` that the DNA
justfile enters before `cargo build`, and that CI enters identically), so Cargo's package ids
are byte-identical everywhere. A remap flag may still be added for the debuginfo strings, but
it is secondary. The verification for any candidate remedy is the table above: pack from two
different checkout paths and require identical wasm sha256 and DNA hash.

The original remedy text is kept below as the record of the first hypothesis. Whatever the
final shape, it must be applied uniformly in **both** places that export `RUSTFLAGS` as env
for this crate graph —
`elohim/holochain/dna/elohim/justfile:7` (local/worktree builds) **and**
`elohim/holochain/dna/Jenkinsfile:154,857` (CI) — since a `.cargo/config.toml` `[build]
rustflags` entry would never be read on either path (both set the env var directly, and env
always wins). Applying the remap in only one of the two would reintroduce exactly this
divergence between local and CI hashes, so the fix is one atomic change to both call sites,
never a partial one.

**Placement rule (sealed decision D5):** this remap moves every existing DNA hash once (it
changes what previously-produced hashes will look like when anything is repacked with the flag
in place), which is by definition a `happ-lineage-migration`-relevant event — a hash-moving
change of this shape rides the **next integrity crossing already needed**, never as a
standalone push. Do not schedule this as an isolated CI hygiene commit; fold it into whatever
integrity-zome change next needs a fresh DNA hash anyway, and record the before/after hash pair
in that crossing's receipt so the lineage record shows the move was the remap, not the content
change.

## Verification for this sprint (T9a)

- Both hashes above exist and differ for byte-identical integrity source. ✓
- No build config, RUSTFLAGS, Cargo.toml, `.cargo/`, justfile, or Jenkinsfile touched by this
  task. ✓ (only this backlog atom was written)
- Mesh untouched. ✓
- Cargo berth claimed before packing, released after (`berth claim cargo` /
  `berth release cargo`, this session). ✓
