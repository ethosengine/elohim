---
name: project_serde_wall_escapable_via_hsb_057
title: Two independent gates on the dep-advisory campaign
description: "Dep-advisory remediation is gated by TWO separate things — Nexus can't fetch uncached artifacts, and holo_hash =0.6.0 pins serde =1.0.219; don't conflate them."
metadata: 
  node_type: memory
  title: Two independent gates on the dep-advisory campaign
  type: project
  originSessionId: 2aa7678c-5ff4-41ac-8332-036d8e06f44c
  modified: 2026-07-30T03:20:44.615Z
---

Established 2026-07-29/30 during the vulnerability-cluster campaign
(`VULNERABILITY_CLUSTER_{01..10,12}_*.md` at repo root — there is no cluster 11). Two gates,
independent, both real. Conflating them wastes whole sessions.

## Gate 1 — RESOLVED 2026-07-30: the mirror was dropped, not fixed

The operator hit Nexus API limits and **retired the proxy/mirror role entirely** so Nexus serves
only self-published artifacts. `.cargo/config.toml` keeps just `[registries.elohim]`
(`cargo-internal`, hosted — where brit/rakia/elohim-epr publish); the `[source.crates-io]
replace-with` and `[source.elohim-mirror]` blocks are gone. `.npmrc` is
`registry=https://registry.npmjs.org/` with `@elohim:registry=…/npm-hosted/`.

**It required zero lockfile churn** — source replacement preserves canonical source strings, so
cargo locks still read `crates.io-index` (0 nexus refs) and both pnpm locks carry integrity
hashes with no registry hosts. Verified after the switch: every previously-404 target fetches
(holo_hash 0.6.1, hdi 0.7.1, jsonwebtoken 10.3.0, async-nats 0.47.0, libp2p 0.56.0 + subcrates,
mongodb 3.8.0, hickory-proto 0.26.1, rpassword 7.5.4, crossbeam-epoch 0.9.20, quick-xml 0.41.0,
multihash-codetable 0.2.0; npm @angular/core 20.3.25, vite 6.4.2, kad-dht 16.2.6, uuid 11.1.1).

**What the drop did NOT unblock** — these were never mirror problems, so don't re-plan them as
newly-available: `sharks 0.5.0` (no fix ever published), `lz4_flex` (`cozo 0.7.6` → `swapvec ^0.3`
pins `^0.10`, no semver escape), `serde_with` >3.14.1 (needs `serde_core ^1.0.225` — behind the
serde ceiling), `lru 0.13.0` (frozen iroh 0.92 pin), `memmap2 0.6.2` (`holochain = "=0.6.0"` pin).

The history below is kept because the *diagnostic method* recurs whenever a proxy sits in front
of a registry.

### How it presented while the proxy was live (cargo hard, npm partial)

**The two proxies differ and the distinction matters** — verify before planning any bump; the
failure is silent and looks like semver trouble.

- **cargo: hard-down for anything uncached.** Four real crates absent from every lock in this
  repo (`ferris-says 0.3.1`, `cowsay 0.1.0`, `ripgrep 14.1.1`, `fastrand 2.1.0`) all 404, while
  in-lock versions serve 200.
- **npm: unreliably populated, not frozen.** Some out-of-lock artifacts DO fetch (`uuid@14.0.1`
  → 200) while patched versions of `vite`/`@angular/*`/`axios` 404. So "only serves cached" is
  too strong for npm; probe per package rather than assuming.

**Control-test discipline:** pick a control that actually exists in the ecosystem you're probing.
A `leftpad` "control" against the *cargo* repo 404s because there is no such crate — it's an npm
package. That invalid control briefly propped up an over-broad conclusion here.

Cargo: the sparse index is healthy (200), so `cargo update` *resolves* happily and proves
nothing — the artifact fetch is what 404s. Get the real endpoint from the index's own
`config.json` (`dl` = `.../repository/cargo/crates`, cargo appends `/{crate}/{version}/download`)
and probe: `serde 1.0.219` and `hickory-proto 0.25.2` (already in our locks) → 200;
`hickory-proto 0.26.1`, `mongodb 3.8.0`, and a `leftpad 0.1.0` control → 404. `static.crates.io`
serves all of them. **Do not read "cargo downloaded 30 crates" as mirror health** — those are
cached artifacts materializing into the local registry src dir. That mistake was made here.

npm: same root cause, different symptom — stale cached *packuments*. `npm view
@angular/core@20.3.25` fails against Nexus while `npm view @angular/core` against Nexus returns
`21.2.14`: a newer major is visible but the needed **backport** to an older line is not, because
the packument was cached before the backport published. Advisories point precisely at such
backports. Blocked-lists built this way carry false positives — re-probe each one
(`immutable@5.1.4` was listed blocked and is actually present).

Fix is operator-side (restore proxy upstream fetch / invalidate metadata cache). **Do not route
around it** by pointing at `registry.npmjs.org` or `static.crates.io`: resolved URLs bake into
the lockfile and become what CI fetches — a supply-chain policy change, not a workaround.

## Gate 2 — the serde ceiling is `holo_hash`, not the holochain_* pins

The advisory class needing serde ≥1.0.220 (`time 0.3.47`, `jsonwebtoken 10.3.0`,
`serde_with 3.19`) is blocked by a `serde_derive` exact-pin collision. Mechanism: every
`serde_core` carries a *normal* `serde_derive` exact-pin behind `target = "cfg(any())"` —
serde's deliberate version-lock — so `serde_core` being a separate crate is **not** an escape
hatch (tested).

The binding pin is **`holo_hash = "=0.6.0"` inside `elohim/sdk/domains/*/types`** (imagodei,
infrastructure, lamad, shefa, …), which pins hsb `=0.0.56` → serde `=1.0.219`. It is NOT the
services' `holochain_*` requirements — those are wide carets already resolving ahead of their
stated minimums. Setting hsb `0.0.57` in a service just makes cargo name `holo_hash v0.6.0`
explicitly.

So **the unlock is `holo_hash >= 0.6.1` — a minor 0.6-line move, not a dev→rc jump.** But all
six `domains/*/types` crates carry the pin, they are shared with three DNA zomes, and those
build on `hdi 0.6.x` which also pins hsb 0.0.56 (first `hdi` on 0.0.57 is **0.7.1**). That
carries a DNA-hash-moving integrity migration → `ALLOW_DNA_REINSTALL`, new agent keys, prod
migration/lineage (see [[project_dna_hash_blind_to_coordinator_zomes]]). Operator-gated.

**And the unlock is itself Gate-1 blocked:** `holo_hash` 0.6.1/0.6.2/0.6.3/0.7.0 are ALL 404 and
`hdi 0.7.1` is 404 (only `hdi 0.7.0`, still on hsb 0.0.56, serves). The serde crates it would
need *are* served (`serde 1.0.228`, `serde_core 1.0.228`, `num-conv 0.2.1`), so Gate 2 looks
independently attackable and is not. **Gate 2 cannot be touched until Gate 1 clears.**

**There is no "Tier 1 of native services that can move independently"** — an earlier plan here
assumed one and it was wrong. `elohim-storage` inherits the ceiling via `lamad-types`/`shefa-types`.
And **`steward/node` is NOT exempt either**, though its manifest declares no `holochain_*`: the
pin arrives in its *resolved graph* via `elohim-storage → holochain_types`. Judge this ceiling
from the resolved graph, never from the manifest.

## Gate 3 — `libp2p 0.54.1` family: no semver escape

A dozen advisories are second copies of a crate alongside an already-fixed version, pulled by
the libp2p 0.54 tree (`ring 0.16.20` via `libp2p-tls`→`rcgen 0.11.3`, `yamux 0.12.1`,
`libp2p-gossipsub 0.47.0`, `rustls-webpki 0.101.7`, `hickory-proto 0.24.4`, `lru`). Each is the
**last release on its minor line**, so no patch bump escapes them — only a libp2p major does.
That major is itself Gate-1 blocked (`libp2p 0.56.0` is cached but 17 of its 20 required
subcrates 404; `0.55.0` 404s outright). Cost if it ever clears: ~125 files in
`elohim-storage/src/` plus 8 in `steward/node/src/` — and steward must move in the same change
or the shared wire types split. `request_response` alone is 141 call sites across 4 custom
codecs whose `Codec` trait moved to native `async fn`. Clear Gate 1 first; it's the precondition
anyway.

## Gate 4 — no upstream fix exists

`sharks 0.5.0` is a **direct** dependency at its newest release with an open advisory — needs
replacement or a fork, and it's secret-sharing code in the custody path, so it's the highest-value
non-mirror item. `lz4_flex 0.10.0` is pinned transitively by `cozo 0.7.6` → `swapvec ^0.3`.

## During a mirror outage, REMOVAL beats upgrade

The two biggest wins of the campaign were deletions, not bumps — reach for this first when an
upgrade is Gate-1 blocked:

- **8 `openssl` advisories closed by making openssl absent.** `steward/node` was the last manifest
  in the monorepo on `reqwest = "0.11"` with default features, and `default-tls` on 0.11 *is*
  `native-tls` *is* `openssl`. Every sibling was already `reqwest 0.12` + `rustls-tls`, so 0.12.28
  was already in the tree and cached — feature unification made the switch free, at a cost of nine
  API-compatible `Client::new()` sites. Upgrading was impossible anyway (of `openssl`
  0.10.73→0.10.81, only 0.10.75 serves).
- **Yanked `core2` removed structurally.** Its holder was never `cid` as written up — it was
  `libsodium-sys-stable → libflate 2.2.x → core2 ^0.4`. `libflate 2.3.x` swapped `core2` for
  `no_std_io2`, so pinning `libflate 2.3.0` + `no_std_io2 0.9.3` (both served; 2.3.1 and 0.9.4 are
  404) drops `core2` from the tree entirely. Also: a yanked crate already in a lock still resolves
  under `--locked`, so yanked ≠ blocking.
- Similarly `keccak 0.1.5` was *evicted* by `multihash-codetable 0.1→0.2`, which drops a duplicate
  sha3/ripemd tree.

Two related traps: a `cfg(windows)`-gated 404 (e.g. `windows 0.52.0` under `sysinfo`) is never
fetched by a Linux check — don't "fix" it. And several committed locks in this repo are already
stale against their path-dep manifests, so `--locked` fails at HEAD too; regeneration is then
mandatory, not agent breakage.

## Audit the closures — alert-set triage systematically under-reports

Two failure shapes, both confirmed repeatedly. **Sweep every lock against OSV directly; never let
the Dependabot alert set define the surface.**

1. **Multi-version masking.** A lock holds several copies of one crate; an agent sees the fixed
   version and calls the advisory closed while a vulnerable older copy remains. Three confirmed
   instances (`rustls-webpki 0.101.7` in sweettest, `rand 0.9.2` in doorway+storage,
   `rustls-webpki 0.102.8` in doorway). **Rule: a resolved row claiming another copy is
   out-of-range must quote the advisory's fixed-versions list, or it's an assumption.** Check with
   `grep -A1 '^name = "<crate>"$' Cargo.lock | grep version`.
2. **Untracked exposure.** Advisories Dependabot never raised. `anyhow` <1.0.103
   (RUSTSEC-2026-0190) was live in five locks and named by no cluster. Only a full-lock OSV sweep
   finds these — a per-alert decomposition structurally cannot.

Also: **an availability pin can land on an affected version.** When the mirror forces a downgrade,
check the target against advisories, not just for HTTP 200 — `crossbeam-epoch 0.9.18` and
`quick-xml 0.38.4` sit below their fix floors (0.9.20, 0.41.0).

## Practical consequence

Split remediation by *cache residency*, not by semver — and **probe the version the resolver
actually picks, not the range floor.** `multihash-codetable 0.2.0` 404s while **0.2.1 serves**,
so "0.2 is blocked" was wrong; the real 404 in that lock was `libflate 2.2.2`→`no_std_io2 0.9.4`.
Likewise a *yanked* crate already in the lock still resolves under `--locked` — yanked ≠
unfetchable, so yanked `core2 0.4.0` was never the hard blocker it was written up as.

Cached and landable: `prometheus 0.14.0` (closes RUSTSEC-2024-0437, which survived in production
via elohim-storage's `prometheus = "0.13"` after doorway and cluster 10 fixed their copies —
`src/metrics.rs` needed **zero** call-site edits across the major), `bytes 1.12.1`, `diesel
2.3.11`, `tar 0.4.46`, `rustls-webpki 0.103.13`, `quinn-proto 0.11.16`, `yamux 0.13.10`,
`lru 0.16.3`, `rand 0.8.6`, `multihash-codetable 0.2.1`. Gate-1 blocked: `jsonwebtoken 10.x`,
`serde_with` (>3.14.1 all 404, though semver-compatible with `holochain_types`'s `^3` — no
holochain bump needed), `rpassword 7.5.x`, `holo_hash 0.6.1`. `time 0.3.47` serves but its deps
`num-conv 0.2.0` and `serde_core 1.0.220` 404 — **Gate 1 first, Gate 2 second**.

Also true here: piping cargo output can mask a red run as green, and pool slots can be dangling
symlinks into a cleared `/tmp`. Check `EXIT=` explicitly. See
[[project_cargo_pvc_disk_discipline]] before fanning out cargo agents, and cap
`CARGO_BUILD_JOBS` — this container's 62GB is shared with the IDE.
