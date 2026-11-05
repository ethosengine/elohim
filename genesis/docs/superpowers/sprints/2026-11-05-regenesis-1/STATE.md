# Re-genesis 1 — orchestrator journal

Plan: `genesis/local-dev/regenesis-1/plan.md` (hub copy of `/projects/.claude-config/plans/foamy-zooming-owl.md`). Orchestrator: session `184beb01-43bc-4c74-9212-9f7189301998`, claude-fable-5-1, moored `task=regenesis-1`.

## 2026-11-05 — day 0 (Phase 0)

- dev head at start: `9c103746e`. Main checkout holds `[dev]` with ~185 dirty files from other sessions — untouched; all work in `.worktrees/rg1-*`.
- Sibling session `95f6fc83` (brit fingerprint slot fix + tracks E/A/D) is STALE 47 min; it held brit main for shem's ack. Pin move to `1c6ae556870bab5f30ae25cd68b2b09e5ecd7c2b` (brit-cli 0.1.12, green) taken over by note (`berth say`).
- shem device `did:key:z6Mkm5de…bowj`: last head 43,815 s ago; no session; operator will open one. Lane brief sent by note.
- `fastcdc` is on crates.io at **5.0.0** (plan said 3; kernel uses 5.0.0, v2020 module).
- Operator answers: shem lane yes; alpha in scope; `[skip ci]` on L2b/L2c approved.
- Astra challenge returned WEAKENED/WEAKENED/REFUTED/REFUTED; ruled in `design-gate.md`; plan §2.5 carries 14 amendments (elohim-address crate, full-decode validator, no persistent iroh cache, authenticated layouts + whole-blob verify on read, RS stripes ≤16 MiB with `rs-gf8-0x11d-v1`, CDC spec + protected vectors, signature transcript + HELD at every consumer, brit v2 note format, **ChannelHead excluded**, KEEP corrections, alpha recast script + state inventory, five-DNA crossing, T11 ark).
- Worktrees: all 12 now at **1051d0957** (rebased onto origin/dev 426b0bcb7; rows renumbered 9–11; brit sub-checkout c2f81931 in the Rust worktrees). Originally 12 created from local dev 9c103746e; origin/dev had moved (9 commits; brit gitlink → c2f81931, 0.1.11 — still not the 0.1.12 reuse cure). Rebase hit a backlog-row conflict (origin rows 5–8 vs my 5–7) → ruled: mine renumber to 9–11; leg resolving in rg1-integrate then propagating. `brit rebase` refuses shallow repos (brit gap, recorded).
- T0b codemod + lint built under `tools/`: census 9,558 address hits (3,526 in `genesis/blobs/manifest.json` — seeder regenerates it), dry-run 3,003 mechanical substitutions / 354 files, lint 7,404 violations today (= the Phase 3 exit metric → 0). Rulings on its 9 open classifications: markdown = prose (lint skips *.md); tracked files only; `genesis/blobs/manifest.json` = seeder-track regeneration; `.epr-meta/**/objects/bafkrei*.json` = record class (keep); `content_hash` confirmed per track; bare `bafkrei` prefixes in code = manual, per track; `sha256:` 16-hex = cite; rakia+sdk renames land with their Rust + codegen (never alone).

### Track status
| Track | Status | Worktree | Report |
|---|---|---|---|
| T0 kernel | **dispatched** (Opus) 17:20Z; brief `briefs/T0-kernel.md` | rg1-kernel | — |
| T0b codemod/lint | built; census-0 + dry-run-0 done; apply per track at start | tools/ (lint lands via rg1-seeder) | — |
| T1 DNA | brief ready (`briefs/T1-dna.md`); waits for `elohim-address` on a branch (L1) | rg1-dna | — |
| T2 storage | brief ready (`briefs/T2-storage.md`, two workers); waits for L1 | rg1-storage | — |
| T3 eprfs | brief ready (`briefs/T3-eprfs.md`); waits for L1; ChannelHead excluded | rg1-eprfs | — |
| T4 brit | blocked on publish | rg1-brit | — |
| T5 schemas | **dispatched** (Sonnet) 17:20Z; brief `briefs/T5-schemas.md` | rg1-schemas | — |
| T6 doorway | blocked on T1.4/T5.3 | rg1-doorway | — |
| T7 app | blocked on T5 | rg1-app | — |
| T8 seeder/CI | **dispatched** (Sonnet) 17:20Z; brief `briefs/T8-seeder-ci.md` (incl. recast script + lint) | rg1-seeder | — |
| T9 a2o | not started | rg1-a2o | — |
| T10 steward | not started | rg1-steward | — |

### Device gate timings
| Device | Gate | Worktree @ base | Cold | Warm | EXIT | Evidence |
|---|---|---|---|---|---|---|
| shem | `just gate sweettest-check` | rg1-dna @ a31d94403 | 1779 s (EXIT=1: preflight red, brit/rakia/sophia submodules missing; compile clean) | 52 s | 0 (warm) | not recorded (worktree dirty before the info/exclude fix) |
| shem | `just gate elohim-storage` (`CARGO_BUILD_JOBS=1`, mesh down) | rg1-storage @ a31d94403 (old base) | 6014 s | — | 0 | `bafyreidexrg6b4uopprwxif2cewp3scgy56mdouvcjgzpy57uj26bl5vge` |
| shem | `just build` in `elohim/holochain/dna/elohim` (wasm release, default stable toolchain) | rg1-dna @ a31d94403 | 275 s | — | 0 | toolchain warm for T1; a `RUSTUP_TOOLCHAIN=1.96.1` pin fails E0463 (no wasm32 target), so DNA builds take the default toolchain |

Detail: `evidence/shem-pre-l1.md` on `regenesis/shem-evidence`.

### Open operator items
- Open a session on shem (`epr peer serve` up; moor; read `berth inbox`).
- Ack the brit pin takeover from `95f6fc83` (or tell me it is still theirs).

### Next three actions
1. Read the four Astra verdicts; record the amended R1 in plan §7.4 (or amend).
2. Dispatch T0 kernel (Opus) once R1 survives.
3. Dispatch T5 schemas first among ethosengine tracks (T6/T7 wait on it).
