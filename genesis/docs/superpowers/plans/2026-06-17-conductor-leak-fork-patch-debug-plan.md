---
title: Conductor anon-leak — fork-patch-debug (amplifier brake + floor R&D)
id: conductor-leak-fork-patch-debug-plan
status: Draft
domain: D5
sprint: conductor-leak-cure
cites:
  - HANDOFF-2026-06-17-upstream-tx5-transport-pin.md
  - HANDOFF-2026-06-17-fbootstrap-deploy-gate.md
  - conductor-leak-upstream-research-tx5-pin-verdict | Conductor anon-heap leak | sha256:ccbf95a2af47c660 | path: genesis/docs/content/elohim-protocol/history/2026-06-17-conductor-leak-upstream-research-tx5-pin-verdict.md
  - arc-factor-feasibility-spike-findings | Arc-factor feasibility findings (for Pillar 3 spec) | sha256:1c8521e12ee40ecd | path: genesis/docs/content/elohim-protocol/history/2026-06-14-arc-factor-feasibility-spike-findings.md
  - matthew-edge-resiliency-rca-fanout-synthesis | Matthew Edge Resiliency | sha256:a4fafb4f91612eba | path: genesis/docs/content/elohim-protocol/history/2026-06-15-matthew-edge-resiliency-rca-fanout-synthesis.md
# MIXED plan (CLAUDE.md scope convention): no doc-level requires_env.
# Build/source/instrument legs are in-tree/household-testable; deploy+canary+measure
# legs are tagged inline @requires:alpha-cluster-6peer (HELD while alpha is degraded).
---

# Conductor anon-leak — fork-patch-debug Implementation Plan

> ## ⛔ SUPERSEDED — 2026-06-19 — WRONG RCA; the cure was the allocator, not a fork connection/buffer patch
> This whole plan (fork the conductor, cherry-pick #5719 amplifier brake, bisect tx5/go-pion for the
> "off-heap floor") rests on a refuted premise. The conductor OOM was a **native glibc-malloc heap
> retention** in the embedded `holochain` child — freed-but-pinned memory in glibc's 64MB secondary
> arenas (0x77xx). NOT go-pion (Go heap flat ~52MB; 0x77xx anon is the Rust/C norm — Rust's default
> allocator IS glibc malloc). The amplifier/floor split and the "off-heap Go/CGo" key-unblock are dead.
> **THE CURE (shipped):** swap the conductor global allocator glibc→jemalloc (tikv-jemallocator +
> unprefixed_malloc_on_supported_platforms). Verified FLAT ~2.1–2.9GB >7.5h, past the old ~5h OOM
> cadence. Allocator-only binary change; DNA hash unchanged. The fork infra
> (ethosengine/holochain@elohim-0.6) became the *vehicle for the jemalloc build* (b477ca7), so Stage 0/1.1
> are not wasted — but Stages 2–4 target the wrong mechanism and are DONE-DIFFERENTLY. Status → SUPERSEDED.
> Truth: genesis/docs/content/elohim-protocol/history/2026-06-19-conductor-leak-jemalloc-cure-verdict.md · genesis/docs/content/elohim-protocol/history/2026-06-19-conductor-leak-jemalloc-prod-changeset.md · genesis/docs/content/elohim-protocol/history/2026-06-18-conductor-leak-rca-diverse-eyes-synthesis.md · genesis/docs/content/elohim-protocol/history/2026-06-18-conductor-leak-rca-native-heap-reframe.md


> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Use the new `ethosengine/holochain` fork to (a) ship the #5719 receipt-amplifier brake on our exact 0.6.0/tx5 base via a custom edgenode image, and (b) debug the per-send tx5/go-pion buffer-retention floor in source — the two components of the alpha conductor anon-heap leak the WAIT verdict could not pin to any upstream release.

**Architecture:** The leaking conductor is holo-host's `edgenode` binary (holochain `0.6.0-dev.28`, tx5/go-pion), which we previously only consumed as a base image. The fork makes us a *compiler* of that binary, unlocking the verdict's PATCH path. The leak splits: an **AMPLIFIER** (~1.8 GB/h on 3 pods, driven by the validation-receipt re-drive storm — fixed by holochain #5719, which lives *in the fork*) and a **FLOOR** (~0.2 GB/h universal, per-send transport-buffer retention — lives in tx5/kitsune2, *below* the fork, and is unresolved upstream #5664). We bank the amplifier first (high-confidence), then time-box the floor as R&D.

**Tech Stack:** git submodules; holochain `holochain-0.6.0` @ `a6d4e805` (NOT `develop` — that carries the 0.7/iroh reframe); tx5 0.8.1 / `kitsune2_transport_tx5` 0.3.2 / go-pion; holonix/nix conductor build; Docker (`elohim/holochain/edgenode/Dockerfile`); the edge CI pipeline (`elohim/holochain/Jenkinsfile`) as the heavy-build host.

## Global Constraints

- **Base on `a6d4e805` (holochain-0.6.0), never `develop`.** `develop` carries the BREAKING 0.6.1 transport reframe (iroh default, kitsune2 0.4.0-dev.10, hdi 0.7.1). Staying on a6d4e805 keeps us tx5/go-pion + hdi 0.7.0 → **DNA-hash-stable, no genesis re-key** (verdict §3.1). This is the whole point of forking vs. bumping.
- **No DNA rebuild.** A conductor-only change does not move the DnaHash (hash surface byte-identical 0.6.0→0.6.1; we don't even cross to 0.6.1). `ALLOW_DNA_REINSTALL` stays false. Touch zero integrity-zome/`hdi` deps.
- **Transport stays tx5/go-pion.** Any feature-flag drift to iroh partitions the genesis pair by a non-DNA mechanism. Verify the built binary links tx5 0.8.1 / go-pion, matching the live mesh.
- **Deploy/canary/measure is BLOCKED-BY-ENV** (`@requires:alpha-cluster-6peer`, currently degraded; cluster is operator-owned — no `kubectl` from dev). In-tree deliverables stop at *a built, verified image*; the operator deploys.
- **Pushing to the fork (`ethosengine/holochain`) is outward-facing** — `git push` to the fork only on explicit operator approval (GH_TOKEN = EthosengineBot has repo scope, but the repo is the user's).
- **Disk:** volume at 85% (138G free), cargo pool only 1.7G. The full nix conductor build is hungry → run it in the **edge CI pipeline** (its normal build host), not the dev container. The dev container does source work + `cargo check`-scale validation only.

---

## Stage 0 — Submodule integration

### Task 0.1: Add `ethosengine/holochain` as a submodule

**Files:**
- Modify: `.gitmodules` (append a `[submodule "elohim/holochain-conductor"]` block)
- Create: `elohim/holochain-conductor/` (the submodule checkout)

**Interfaces:**
- Produces: a local holochain source tree at `elohim/holochain-conductor`, checked out at `a6d4e805`, on a working branch `elohim-0.6`, for Stages 1–3 to patch/instrument.

- [ ] **Step 1: Add the submodule** (follows the existing `elohim/<fork>` pattern — `elohim/brit`, `elohim/rakia`, `elohim/rust-ipfs`). Avoid the `elohim/holochain/` path (already the DNA+edgenode dir).

```bash
cd /projects/elohim
git submodule add https://github.com/ethosengine/holochain.git elohim/holochain-conductor
```
Expected: clones; `.gitmodules` gains the block; `elohim/holochain-conductor` populated.

- [ ] **Step 2: Pin to our 0.6.0 base and create the working branch**

```bash
cd /projects/elohim/elohim-holochain-conductor 2>/dev/null || cd /projects/elohim/elohim/holochain-conductor
git fetch origin
git checkout a6d4e805a0971ccbc0dcb3f3ed6a9e2fac980a3b
git switch -c elohim-0.6
git log -1 --oneline
```
Expected: HEAD at `a6d4e805 … create a release from branch release-20251119` on branch `elohim-0.6`. (a6d4e805 is in the fork's inherited history — no push needed yet.)

- [ ] **Step 3: Commit the submodule wiring** (parent repo)

```bash
cd /projects/elohim
git add .gitmodules elohim/holochain-conductor
git commit -m "build(conductor): vendor ethosengine/holochain fork at 0.6.0 for leak patch/debug"
```

---

## Stage 1 — De-risk GATES (cheap, no heavy build; outcomes gate everything below)

> These three gates decide whether the fork approach is viable. Run them BEFORE the build pipeline. A failure on Gate B is the one surfaced to the operator (the build-fidelity blocker the advisor flagged).

### Task 1.1: GATE A — prove #5719 cherry-picks clean onto a6d4e805 (pure git)

**Files:**
- Modify (in submodule, branch `elohim-0.6`): `crates/holochain/src/core/workflow/validation_receipt_workflow.rs`

**Interfaces:**
- Consumes: the #5719 backport merge commit `6923effd507e771ae2d59a3ada6d5ce182d54ac1` (base `develop-0.6`) — verified present at rc.8, absent at a6d4e805 (verdict §3.1, "Code presence").
- Produces: branch `elohim-0.6` carrying the offline-author receipt-clearing block; or a recorded conflict if it doesn't apply clean.

- [ ] **Step 1: Confirm the block is ABSENT at base** (the precondition that makes the cherry-pick meaningful)

```bash
cd elohim/holochain-conductor
grep -c "was_agent_recently_online" crates/holochain/src/core/workflow/validation_receipt_workflow.rs
```
Expected: `0` (absent at a6d4e805).

- [ ] **Step 2: Fetch the fix commit and cherry-pick it**

```bash
git fetch origin 6923effd507e771ae2d59a3ada6d5ce182d54ac1 || git fetch https://github.com/holochain/holochain.git develop-0.6
git cherry-pick -x 6923effd507e771ae2d59a3ada6d5ce182d54ac1
```
Expected: applies clean. If it conflicts (the backport may pull adjacent context), record the conflicting hunks — a small manual port of just the `was_agent_recently_online` → `set_require_receipt(txn, …, false)` offline-clearing arm is the fallback (the change is self-contained per the research, point 3).

- [ ] **Step 3: Verify the block is now PRESENT**

```bash
grep -c "was_agent_recently_online" crates/holochain/src/core/workflow/validation_receipt_workflow.rs
```
Expected: `≥1` (the offline-clearing path landed). **GATE A outcome:** CLEAN (proceed) / MANUAL-PORT (small, proceed) / BLOCKED (record + surface).

### Task 1.2: GATE B — build FIDELITY: does from-source 0.6.0 reproduce holo-host's `go-pion-custom` leaking stack?

**Files:**
- Read: `elohim/holochain/edgenode/Dockerfile`, `elohim/holochain/edgenode/README.md`, `elohim/holochain/edgenode/scripts/`
- Read (submodule): `Cargo.lock` (tx5/go-pion/kitsune2 versions), `flake.nix`, `nix/` (holonix build inputs)

**Interfaces:**
- Produces: a GO / NEEDS-RECIPE / BLOCKER verdict on whether a vanilla holonix build of `a6d4e805` yields the SAME transport stack (tx5 0.8.1 / go-pion) as the live leaking binary. If it doesn't, we'd debug a *different* conductor — the single most likely thing to sink the approach.

- [ ] **Step 1: Pin down what `-go-pion-custom` actually means.** Determine whether the base image's `custom` is (a) only conductor-config (then vanilla holonix reproduces the binary) or (b) holo-host build patches/feature-flags beyond holonix.

```bash
# Our base + their feature surface
grep -n "FROM" elohim/holochain/edgenode/Dockerfile
# In the fork: confirm the tx5/go-pion transport stack our build would link
cd elohim/holochain-conductor
grep -E '^name = "(tx5|tx5-go-pion|kitsune2_transport_tx5|kitsune2)"' -A1 Cargo.lock | grep -E 'name|version'
```
Expected: `tx5 0.8.1`, `tx5-go-pion 0.8.1`, `kitsune2_transport_tx5 0.3.2` (matches the live leaking stack per the research). The transport backend is a compile-time Cargo feature — confirm which feature the holochain binary build enables for go-pion.

- [ ] **Step 2: Find or reconstruct the edgenode build recipe.** holo-host's edgenode build source was NOT an obvious public repo (verdict research). Check: the edgenode README/scripts for a build reference; the holochain fork's `flake.nix` for a conductor/`holochain` package output; whether `holochain` binary + `hc` build cleanly from the fork with the go-pion feature.

```bash
cat elohim/holochain/edgenode/README.md
ls elohim/holochain/edgenode/scripts/
cd elohim/holochain-conductor && ls flake.nix nix/ 2>/dev/null && grep -n "go-pion\|tx5\|packages" flake.nix | head
```
**GATE B outcome:**
  - **GO** — `custom` is conductor-config only; vanilla holonix build of a6d4e805 + go-pion feature reproduces the binary.
  - **NEEDS-RECIPE** — must obtain holo-host's build config/patches; identify where (ask operator / holo-host).
  - **BLOCKER** — cannot reproduce the leaking stack from the fork. **Surface to operator before any build investment** (the advisor's named user-decision point).

### Task 1.3: GATE C — build capacity + host decision

**Files:**
- Read: `elohim/holochain/Jenkinsfile` (the edge pipeline — confirm it's the right heavy-build host), `elohim/holochain/build-manifest.json`

- [ ] **Step 1: Confirm the heavy build runs in CI, not the dev container.**

```bash
df -h /projects | tail -1     # 138G free, 85% warn — too tight + slow for a full nix holochain build locally
grep -niE "edgenode|docker build|FROM|holochain build|nix" elohim/holochain/Jenkinsfile | head
```
Expected: the edge pipeline builds/pushes the edgenode image → the custom conductor build belongs there. **GATE C outcome:** dev container does source work + `cargo check`-scale validation; the full nix/image build is a CI leg (Stage 2.2). Note any disk reclaim the operator must do on the CI build host.

---

## Stage 2 — Amplifier brake: build + image (the headline win — gated on 1.1 CLEAN + 1.2 GO/NEEDS-RECIPE + 1.3)

### Task 2.1: Compile-validate the patched conductor source

- [ ] **Step 1: `cargo check` the patched workflow crate** (dev-container-scale, no full build)

```bash
cd elohim/holochain-conductor
RUSTFLAGS="" cargo check -p holochain --no-default-features --features <go-pion-feature-from-1.2>
```
Expected: compiles; the #5719 change type-checks against a6d4e805. (Use the go-pion feature confirmed in Gate B; do NOT enable iroh.)

- [ ] **Step 2: Commit the patched branch** (local; push to fork is operator-gated per Global Constraints)

```bash
git add -A && git commit -m "fix(conductor): cherry-pick holochain #5719 receipt-offline-clear onto 0.6.0 (amplifier brake, tx5)"
```

### Task 2.2: Custom edgenode image from the patched conductor (CI leg)

**Files:**
- Modify: `elohim/holochain/edgenode/Dockerfile` (build the patched conductor from `elohim/holochain-conductor` instead of, or layered over, the holo-host base — exact form depends on Gate B's recipe)
- Modify: `elohim/holochain/Jenkinsfile` / `build-manifest.json` (wire the from-source build leg)

- [ ] **Step 1:** Implement the image build per Gate B's recipe (holonix build of the patched binary → edgenode image with the SAME conductor-config overlay). Keep tx5/go-pion feature explicit.
- [ ] **Step 2:** Build the image in CI; verify it links tx5 0.8.1 / go-pion (not iroh) and starts a conductor that loads the existing a6d4e805-built DNA with an UNCHANGED DnaHash.
Expected: image builds; transport = tx5/go-pion; DnaHash stable. **Deliverable = the verified image.**

### Task 2.3 `@requires:alpha-cluster-6peer`: deploy + measure the brake (HELD — operator)

- [ ] Canary the image to ONE non-genesis pod; over a clean multi-hour window confirm the receipt-error rate (`could not find url for peer`) drops on that pod and its `elohim_node_conductor_smaps_anon_bytes{class="other"}` slope falls from ~2 GB/h toward terrance's ~0.2 GB/h floor. This is the brake's proof. **Held while alpha is degraded; the operator runs it. The live diagnostic (handoff §3.1) already shows the amplifier concentrates in matthew/james/jessica — those are the canary targets.**

---

## Stage 3 — Floor RCA: per-send tx5/go-pion buffer retention (TIME-BOXED R&D — now CLUSTER-INDEPENDENT)

> #5664 is unresolved upstream ("awaiting clarification", not reproduced in Wind Tunnel). This is genuine R&D — TIME-BOX it. **KEY UNBLOCK (from the "what else to fork" question, confirmed 2026-06-17):** the floor leak is **off-heap Go/CGo** (Rust `[heap]` dead-flat, anon-mmap growing) → it lives in the **`tx5-go-pion-sys` Go layer**, NOT in Rust. And `holochain/tx5` ships a **standalone repro harness** (`crates/tx5/benches/throughput.rs`, `tx5-demo`) that drives the WebRTC send path under load with NO conductor and NO alpha cluster. So the floor RCA is **decoupled from the degraded alpha cluster AND from the edgenode build** — we reproduce + bisect it locally.

### Task 3.0: Clone tx5 + kitsune2 at the conductor's PINNED versions (not main)

**Files:**
- Create: `elohim/tx5/` (submodule of `holochain/tx5` — or `ethosengine/tx5` once we patch), pinned at **tag `v0.8.1`** (the version the leaking conductor links; `main` is moving to iroh/0.9 — wrong, same lesson as a6d4e805-not-develop).
- Create: `elohim/kitsune2/` (submodule of `holochain/kitsune2`), pinned at **tag `v0.3.2`** (the conductor's `kitsune2_transport_tx5`).

- [ ] **Step 1:** Clone both at the pinned tags. Fidelity: the repro must link tx5 0.8.1 / `kitsune2_transport_tx5` 0.3.2 / pion-webrtc v4.1.3 — matching the live binary. (Cloning is enough to read+repro+instrument; FORK to `ethosengine` only when carrying a fix to push — outward-facing, operator-gated.)
- [ ] **Step 2:** Confirm the suspect surface is present: `tx5/crates/tx5-go-pion-sys/buffer.go` (the `GoBuf` lifecycle — #1 off-heap suspect), `datachannel.go`/`peerconnection.go`, vendored `pion/webrtc/v4 v4.1.3` (`go.mod`); `kitsune2/crates/transport_tx5/src/lib.rs` (`data.to_vec()` per send).

### Task 3.1: Reproduce the off-heap leak locally with the tx5 throughput harness

**Files:**
- Run (tx5 clone): `crates/tx5/benches/throughput.rs` (and/or `tx5-demo` with its influxive dashboards) under a sustained send loop.

- [ ] **Step 1:** Build tx5 (needs Go 1.24 + CGo; `tx5-go-pion-sys/build.rs` compiles the vendored Go). Run `throughput.rs` under sustained send traffic.
- [ ] **Step 2:** Monitor `/proc/self/smaps` anon-mmap **count** + `other_anon_bytes` vs send count (the same telemetry that fingerprinted the conductor leak). **Expected if the hypothesis holds:** the discrete >128 KB anon-mmap count climbs monotonically with sends while RSS-anon grows — reproducing terrance's ~0.2 GB/h floor in isolation, no cluster. If it does NOT reproduce here, the leak is above tx5 (back up to kitsune2 / holochain_p2p — Task 3.3).

### Task 3.2: Bisect the layer + catch the retained buffer (Go-side profiling, NOT Rust heaptrack)

**Files:**
- Modify (tx5 clone): `tx5-go-pion-sys/buffer.go` + `datachannel.go` (instrument `GoBuf` alloc vs free balance at the CGo boundary).

- [ ] **Step 1:** Because the leak is off-heap Go, profile the **Go** side: Go `pprof` / `GODEBUG=madvdontneed=1` / a per-`GoBuf` alloc-free counter at the FFI boundary. Rust heaptrack is blind to the Go mmap heap — do not lead with it.
- [ ] **Step 2:** Determine whether the retention is (a) tx5-go-pion-sys's own `GoBuf` not freed (CGo lifetime bug — fixable in our fork), or (b) `pion/webrtc/v4` v4.1.3 retaining per-DataChannel/PeerConnection send buffers (then check pion's issue tracker + consider a pion bump/patch). **This localization IS the real RCA #5664 never reached.**

### Task 3.3: If tx5 alone does NOT reproduce — climb back up through kitsune2 + holochain_p2p

- [ ] Instrument `kitsune2/crates/transport_tx5/src/lib.rs` (`data.to_vec()` per send) then the in-fork `holochain_p2p` send path (`BytesMut::put_slice` accumulation, research point 2), `[patch]`-wiring the holochain workspace to the local tx5+kitsune2 checkouts. Only needed if the leak proves to live above the go-pion layer — the off-heap signature makes that less likely, so try Task 3.1 (tx5-only) FIRST.

---

## Stage 4 — `@requires:alpha-cluster-6peer`: upstream contribution (after floor located)

- [ ] Once the floor's retained buffer is located, file/PR upstream: against `holochain/tx5` or `kitsune2` if it's below holochain_p2p, or attach to #5664 with the tx5/off-heap CGo evidence its iroh-only dumps lack (handoff §3.1 rec 3). Outward-facing — operator sends. Our fork carries the fix in the meantime (the PATCH path becomes a real contribution).

---

## Self-Review notes

- **Spec coverage:** Stage 2 = amplifier (verdict Path-A-via-fork, no 0.6.1 minor); Stage 3 = floor (the cure #5664 never pinned); Stages 2.3/3.3/4 = the BLOCKED-BY-ENV deploy/measure/upstream legs, tagged, not planned-as-now. Gate B = the advisor's build-fidelity blocker. The falsified §6 arc lever is intentionally NOT a task (it's a dead end — recorded in the verdict, not re-planned).
- **Env split:** in-tree/household = Stages 0, 1, 2.1, 2.2 (CI build), 3.1, 3.2. HELD `@requires:alpha-cluster-6peer` = 2.3, 3.3, 4.
- **Sequencing risk:** Stage 3 (open R&D) must not block Stage 2 (the high-confidence win). Bank the brake first.
