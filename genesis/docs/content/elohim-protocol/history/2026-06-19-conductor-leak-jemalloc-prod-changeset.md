---
title: "jemalloc-PRODUCTION conductor change set — verified, ready-to-apply (2026-06-19)"
id: conductor-leak-jemalloc-prod-changeset
type: history-gotcha
status: noted
tier: history
created: 2026-06-19
topic: [conductor-leak, jemalloc, production, changeset, edgenode]
---

# jemalloc-PRODUCTION conductor change set — verified, ready-to-apply (2026-06-19)

*The production form of the leak cure. Companion to `2026-06-19-conductor-leak-jemalloc-cure-verdict.md`
(the verdict: jemalloc allocator swap cured the OOM) and the RCA chain (`2026-06-18-conductor-leak-rca-diverse-eyes-synthesis.md`,
`2026-06-18-conductor-leak-rca-native-heap-reframe.md`, `2026-06-18-conductor-leak-canary-runbook.md`). Authored + adversarially verified via workflow
(map→author→3-lens verify→revise); the edited Cargo feature graph PARSE OK offline.*

## Current state (verified 2026-06-19)
- feat/frontend-eyes-sprint HEAD = **option B** (profiler kept): `elohim/elohim-storage/Dockerfile:17`
  `CONDUCTOR_SOURCE_IMAGE=...elohim-edgenode-prof:e87a680`; `_edgenode-consolidated.template.yaml:253`
  `_RJEM_MALLOC_CONF`. `b8481f090` is an ancestor of HEAD (the `-X ours` merge ran). **Push B as-is —
  the profiler is the live bridge cure; do NOT push the glibc `:latest` (option A re-leaks).**
- The cure is REAL and is the jemalloc ALLOCATOR (image-attributed: glibc images OOM ~8GB/~5h;
  the `b8481f09` jemalloc image flat ~2.7GB/0-restarts/10.5h; `edgenode-prof` differs from
  `:latest` by exactly `d0f505f` = the jemalloc feature; jemalloc-only env can't cure glibc).

## ⚠ The profiler never actually profiled (operator finding, 2026-06-19)
Zero `.heap` dumps exist on any PVC (full-FS `find`, incl. daniel @ 10h/0-restarts which would have
dumped dozens of times). **The allocator took (cure), but `--enable-prof` did NOT compile into the
conductor binary** → `prof:true` is a silent no-op. Two consequences:
1. **Prod path is UNAFFECTED** — it drops profiling entirely, which never worked anyway. Pure cleanup.
2. **Naming the call-site (Path 2) needs a BUILD-LEVEL fix, not just the Cargo shape.** The revised
   `jemalloc-prof = ["jemalloc","tikv-jemallocator/profiling"]` resolves *byte-equivalent* to today's
   broken `["dep:tikv-jemallocator"]` (both → {unprefixed, profiling}) — so it would STILL not dump.
   The root cause is build-level: `tikv-jemalloc-sys` was not (re)compiled with `--enable-prof`
   (cached layer, or the feature not reaching the `-sys` build — the runbook's "confirm `Compiling
   tikv-jemalloc-sys`, NOT `Adding`" warning). Before ever trusting a profiler deploy again, VERIFY:
   - build log shows `Compiling tikv-jemalloc-sys …` (not merely `Adding …`), AND
   - `strings /usr/local/bin/holochain | grep -iE 'prof_prefix|opt\.prof'` is NON-empty.
   NB: the operator's earlier `strings` was on `/usr/local/bin/elohim-storage` (the PARENT, never built
   with jemalloc) — the conductor is `/usr/local/bin/holochain` (Dockerfile line 236
   `COPY --from=conductor-source /usr/bin/holochain`). Check the conductor.

## Two paths
- **Path 1 — ship the cure, skip naming (recommended; satisfies "cured before touching profiler").**
  Apply A+B+C below; the OOM is already out, this makes it the official prod build and removes the
  dead profiler. Root cause documented at the *layer* (glibc-arena pinning, cured by jemalloc).
- **Path 2 — name the call-site first.** Additionally fix the `--enable-prof` build (above), redeploy a
  *working* profiling conductor to one anchor, collect dumps, `jeprof --base` diff. Slower; and
  jemalloc flattened the target so the growth-diff signal is weak. Optional root-cause insurance.

---

# CHANGE SET

DNA hash UNCHANGED (allocator is a process-level binary property; covers integrity zomes + modifiers
only — no `ALLOW_DNA_REINSTALL`, no re-key). Default/native (non-jemalloc) builds UNAFFECTED
(`default` doesn't include `jemalloc`; the `#[global_allocator]` is `#[cfg]`-compiled-out). Cargo.lock
needs NO edit (no `tikv-jemallocator` entry; conductor Dockerfile builds without `--locked`).

## PART A — FORK (conductor) · `holochain-conductor` submodule, branch `elohim-0.6`

### A1. `crates/holochain/Cargo.toml` — base optional dep: drop `profiling`, KEEP `unprefixed`
```toml
# BEFORE
tikv-jemallocator = { version = "0.6", features = [
  "profiling",
  "unprefixed_malloc_on_supported_platforms",
], optional = true }
# AFTER
tikv-jemallocator = { version = "0.6", features = [
  "unprefixed_malloc_on_supported_platforms",
], optional = true }
```
`unprefixed_malloc_on_supported_platforms` MUST stay on the base — it routes C-side malloc
(SQLCipher codec, OpenSSL, CGo) through jemalloc; that interposition is load-bearing for the cure
(the leak's top unnamed candidate is C-side). Dropping `profiling` from the base is the whole game.

### A2. `crates/holochain/Cargo.toml` [features] — new prod `jemalloc`, prof becomes a superset
```toml
# BEFORE
jemalloc-prof = ["dep:tikv-jemallocator"]
# AFTER
jemalloc = ["dep:tikv-jemallocator"]
jemalloc-prof = ["jemalloc", "tikv-jemallocator/profiling"]
```
(Update the attached comments: `jemalloc` = standard production allocator cure; `jemalloc-prof` =
debug superset adding `--enable-prof`.) The prof feature resolves byte-equivalent to today's — see
the build-level caveat above for why that alone won't produce dumps.

### A3. `crates/holochain/src/bin/holochain/main.rs` — gate the allocator on `jemalloc`
```rust
# BEFORE
#[cfg(feature = "jemalloc-prof")]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
# AFTER
#[cfg(feature = "jemalloc")]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
```
`jemalloc-prof` now implies `jemalloc`, so both builds install the allocator. Only `#[global_allocator]`
in the tree (grep-confirmed).

## PART B — CHE-DEVWORKSPACES (build pipeline) · pushes to `main`

### B1. `jenkins/Jenkinsfile-elohim-edgenode` — HC_FEATURES default (the deciding knob)
```
# BEFORE defaultValue: 'sqlite-encrypted,wasmer_sys,backend-go-pion,jemalloc-prof'
# AFTER  defaultValue: 'sqlite-encrypted,wasmer_sys,backend-go-pion,jemalloc'
```
Update the description accordingly. The three `params.HC_FEATURES.contains('jemalloc-prof')` gates
(lines ~133/163/165) stay BYTE-UNCHANGED: `'…,jemalloc'.contains('jemalloc-prof')` is correctly
`false` → publishes `elohim-edgenode:latest` (prod); the `-prof` isolation branch stays dormant
unless `jemalloc-prof` is explicitly set. Harden the nearby comments (B2/B4) so nobody drops jemalloc
to the bare glibc string (that re-leaks).

### B3. `BUILD_STORAGE_CANARY` description — note prod default → `elohim-storage-zombiefix` embedding
`edgenode:latest`; prof → isolated `elohim-storage-prof`. (Comment-only.)

### B5 (judgment, defense-in-depth). `containers/elohim-edgenode/Dockerfile` ARG default
```
# BEFORE ARG HC_FEATURES=sqlite-encrypted,wasmer_sys,backend-go-pion
# AFTER  ARG HC_FEATURES=sqlite-encrypted,wasmer_sys,backend-go-pion,jemalloc
```
Not pipeline-load-bearing (Jenkinsfile always passes HC_FEATURES); stops a raw `docker build` from
silently re-leaking. Deviates from upstream holo-host's recipe — if kept, also apply C3 (twin). Revert
both together to track upstream.

## PART C — ELOHIM MONOREPO (deploy surfaces) — APPLY LATER (the profiler-out swap; reverts B)

> ⚠ Part C undoes the staged option-B. Apply ONLY after B-stage republishes `:latest` as jemalloc-prod
> AND a canary anchor proves flat (runbook steps 2+4). `:latest` is the GLIBC LEAKER until then —
> landing C1 early embeds the leaker fleet-wide.

### C1. `elohim/elohim-storage/Dockerfile` — repoint prof → `:latest`, fix the trap ROLLBACK comment
```dockerfile
# BEFORE ARG CONDUCTOR_SOURCE_IMAGE=harbor.ethosengine.com/ethosengine/elohim-edgenode-prof:e87a680
# AFTER  ARG CONDUCTOR_SOURCE_IMAGE=harbor.ethosengine.com/ethosengine/elohim-edgenode:latest
```
Replace the ROLLBACK comment that points at glibc (`ghcr.io/holo-host/edgenode … go-pion-custom`) —
rolling back to glibc RE-LEAKS. Safe rollback = the PREVIOUS pinned jemalloc dated/git tag.

### C2. `genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml` — drop the
`_RJEM_MALLOC_CONF` env block (restore the pre-b8481f090 `CONDUCTOR_DATA_DIR`→`HAPP_PATH` gap).
Hygiene (a prod manifest must not carry profiling-intent env); inert on a non-`--enable-prof` binary
regardless.

### C3 (only if B5 kept). `elohim/holochain/edgenode/Dockerfile.zombie-fix` ARG → add `,jemalloc`.
Coherence only — `build-zombie-fix.sh` pushes `:zombie-fix-canary-<hash>`, never `:latest`; residue.

## NO change: `Cargo.lock`, `containers/elohim-storage-zombiefix/Dockerfile`,
`elohim/elohim-storage/build-storage-canary.sh`, the three Jenkinsfile `.contains` gates.

## PROD jemalloc config: MALLOC_CONF = NONE
The observed cure ran on jemalloc DEFAULT decay, background_thread OFF (the prof env touched only
profiling keys). Prod with no MALLOC_CONF = byte-identical reclamation to the image that flattened.
jemalloc purges to the OS on allocator activity; the leak was monotonic-under-churn (never idle), so
`background_thread` isn't needed. Add nothing. If idle-purge insurance is ever wanted (only after a
24–48h slope check), set the NON-profiling key `_RJEM_MALLOC_CONF=background_thread:true` — ship without.

---

# RUNBOOK — build → canary-ONE-anchor → fleet → genesis-pair-LAST

1. **Fork: commit AND PUSH A1/A2/A3** to ethosengine `holochain@elohim-0.6`. The edgenode Dockerfile
   `git clone -b elohim-0.6` at build time — a local submodule commit is INSUFFICIENT; the feature
   must be on the fork HEAD before che-dw runs.
2. **che-dw: apply B, push `main`, run `elohim-edgenode` job** with defaults + `BUILD_STORAGE_CANARY=true`,
   `HC_BRANCH=elohim-0.6`. `HC_FEATURES=…,jemalloc` → publishes `elohim-edgenode:latest`+dated/git
   (jemalloc-PROD) AND `elohim-storage-zombiefix` (embeds `:latest`). Confirm `Compiling tikv-jemalloc-sys`
   in the log. **No monorepo edit yet.** This is when `:latest` stops being the glibc leaker.
3. **Canary ONE non-genesis anchor** at the `elohim-storage-zombiefix` tag (empirical confirm — the
   profiling-off prod binary has never run).
4. **Prove cure (multi-hour):** `smaps_anon_bytes{class=other}` FLAT (~2.2GB) AND cadvisor
   `working_set` FLAT (~2.7GB), 0 restarts, no monotonic climb, gossip/DHT healthy. Don't proceed
   until flat over multi-hour.
5. **Fleet roll: apply monorepo C1/C2 (Part C), push dev.** Edge rebuilds `elohim-storage`
   (`COPY --from=conductor-source /usr/bin/holochain` re-copies the jemalloc binary). Roll leechers
   first, **genesis pair (adam/matthew) LAST**. `imagePullPolicy: Always`, no re-key.
6. **Confirm fleet** flat on matthew/james. Rollback target = previous pinned jemalloc tag, NEVER glibc.

# RESIDUAL RISKS
- Slow residual leak unfalsified (10.5h flat kills the fast leak; carry a 24–48h slope check post-roll).
- `:latest`-is-glibc race — C1 before B-republish embeds the leaker (sequence enforced above).
- Fork-push dependency — feature must be on the fork HEAD before the che-dw clone.
- Silent glibc fallback — any future edit dropping `jemalloc`/`unprefixed` or repointing to glibc
  `:latest` re-leaks invisibly; the hardened comments (B1/B2/B4/C1) are the guardrail — keep them.
</content>
