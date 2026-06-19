# HANDOFF 2026-06-17 — Upstream tx5/go-pion transport-buffer pin (the conductor-leak CURE)

> ## ⛔ CORRECTED — 2026-06-19 — RCA WRONG; CURED locally (allocator swap), no upstream wait needed
> The off-heap diagnosis attributed the leak to tx5/go-pion (CGo) per-send buffer retention and concluded
> WAIT on an upstream transport fix. REFUTED: go-pion exonerated (Go heap flat ~52MB); the growth was
> glibc-malloc arena retention serving Rust/C allocations (0x77xx). The "WAIT for a holo-host ≥0.6.1-rc.8 /
> iroh image" gate never had to be cleared. **CURE (shipped):** conductor global allocator glibc→jemalloc —
> FLAT ~2.1–2.9GB past the OOM cadence; DNA hash unchanged. The §3.1 "lower arc factor" FALSIFICATION still
> holds (arc=0 leaks the same shape). Read below as the investigation trail; the conclusion is superseded.
> Truth: .claude/data/conductor-leak-jemalloc-cure-verdict-2026-06-19.md · conductor-leak-jemalloc-prod-changeset-2026-06-19.md · conductor-leak-rca-native-heap-reframe-2026-06-18.md


**For:** whoever takes on the actual cure for the conductor anon-heap leak (operator-involved).
**Status:** **TASK 1 (go/no-go) RE-VALIDATED 2026-06-17 — verdict in §3.1 below.** Cure = **WAIT** (no go-pion buffer fix exists in any tagged release; tx5 is being *removed* upstream, not fixed). The actionable move is a **holo-host edgenode rebuild ≥0.6.1-rc.8**, which is NOT in our tree and NOT yet published (their images top out at `hc0.6.1-rc.1-dev`, pre-fix). Tasks 2–3 are blocked on that. The §6 "lower arc factor" mitigation is **falsified** (see §3.1). Everything we shipped so far (F-BOOTSTRAP, self-healing handles) was either the wrong layer or symptom-masking. The leaking code is in the holochain/tx5 binary we **spawn but do not compile**.
**Read first:** §3.1 (this re-validation) · `HANDOFF-2026-06-17-fbootstrap-deploy-gate.md` §6 (the post-deploy refutation) + memory `project_storage_metrics_surface_and_leak_verdict` (the corrected mechanism) · `.claude/data/conductor-leak-upstream-research-2026-06-17.md` (the prior pass this re-validation diffs against).

---

## 1. The settled diagnosis (one paragraph)

The alpha conductor OOM is an **off-heap anonymous-mmap leak in the holochain conductor child** — the **tx5/go-pion (CGo) networking transport retains a buffer per send/connection**. Proven off-heap: the Rust `heap` gauge is dead-flat (matthew = 32,313,344 B across 8 samples) while `other` anon grows GBs. It is **one mechanism at different volumes** — terrance (zero receipt errors) leaks the same monotonic shape at ~0.12 GB/h; matthew (busiest) at ~2 GB/h. **NOT** a Rust receipt *queue* (that would grow the heap), **NOT** bootstrap islanding (F-BOOTSTRAP unified the store — `backend=mongo`, 1705 agents — and matthew's `could not find url for peer` rate was **unchanged 95k/h→95k/h**; the failures are churn-driven transport staleness, not lookup misses), **NOT** RAM/cache/arc/MALLOC_ARENA_MAX (all ruled out earlier). Self-healing (admission shed, circuit breakers, memory bound) is all **wrong-axis** — it defends inbound HTTP + upstream storage, never the outbound transport path — and the memory-bound OOM-restart actively *re-storms* the loop (vicious cycle). Cure = stop the transport buffer retention upstream.

## 2. The substrate to pin (verified in-tree)

| Component | Version | Source |
|---|---|---|
| Conductor BINARY (the leaking child) | holochain `0.6.0-dev.28` | base image `ghcr.io/holo-host/edgenode:v0.0.8-alpha31-hc0.6.0-go-pion-custom` (`elohim/holochain/edgenode/Dockerfile:10`) |
| holochain source pin | `holochain-0.6.0` @ rev `a6d4e805a0971ccbc0dcb3f3ed6a9e2fac980a3b` | `elohim/holochain/dna/elohim/flake.lock` (holonix `?ref=main-0.6`) |
| kitsune2 | `v0.3.2` @ `22de6e42…` | flake.lock `kitsune2.original.ref` |
| tx5 transport | `0.8.1` (go-pion WebRTC) | the leaking layer; NOT iroh |

We **spawn** the conductor (`elohim/elohim-storage/src/conductor/process_manager.rs::start()`), we do not build it. So a "code fix" is not a lever we hold — the lever is the **base-image version** (or a patched rebuild) + conductor **config**.

## 3. TASK 1 (do first — go/no-go): identify the EXACT upstream fix

This is the open research question, and the prior verdict flagged "**KNOWN-OPEN, no clean pin-to-fix**" — re-validate that, it may have changed.

- **Distinguish two upstream fixes.** holochain **#5718/#5719** fix the **receipt trigger** (`validation_receipt_workflow` not clearing `require_receipt` → re-drive loop) — that lowers the *send rate* on busy nodes (shrinks the amplifier), but it does **not** explain terrance's zero-error floor. The **floor** is per-send **transport buffer retention** in tx5/go-pion (the non-iroh component matched to holochain **#5664**). **The cure we need is the transport-buffer fix, not (only) the receipt-trigger fix.** Both help; only the transport fix flattens terrance.
- **Deliverable:** find the holochain 0.6.x release/rev (since `a6d4e805`) that includes the tx5/go-pion buffer-retention fix, and confirm it's DNA-compatible with our current hash (integrity zomes + modifiers — a coordinator-only or transport-only bump must NOT move the DNA hash; see CLAUDE.md "DNA changes don't redeploy"). Check: tx5 changelog ≥0.8.1, kitsune2 ≥v0.3.2, holochain 0.6 release notes, and PRs #5664/#5718/#5719 merge status + which release carries them.
- **Output a go/no-go:**
  - **GO** — a clean forward rev exists and is DNA-compatible → proceed to Task 2.
  - **PATCH** — fix merged upstream but not in a compatible release → carry a patched tx5/go-pion in a custom edgenode build (heavier; holonix/nix).
  - **WAIT** — not yet fixed upstream → mitigate via conductor config (§6) and track (or file) the upstream issue.

## 3.1 TASK 1 — RE-VALIDATION VERDICT (2026-06-17, primary-source, adversarially verified)

Diffed against the prior pass (`conductor-leak-upstream-research-2026-06-17.md`, ~10h earlier). Method: 5-agent workflow (cure / brake-release / DNA-compat + 2 refuters) over the GitHub REST API + crates.io + in-tree reads; the three load-bearing facts were re-verified by hand (in-tree `dna.yaml`/`hdi` pin; ghcr edgenode tag list; rc.8 > rc.1 topology).

### VERDICT = **WAIT** on the literal cure — but two forward moves the original scoping missed, both gated on a holo-host edgenode rebuild (not our tree).

**CURE axis (terrance's universal ~0.2 GB/h go-pion buffer-retention floor) → WAIT, unchanged.** No tx5/go-pion transport-buffer fix in any tagged release: tx5 caps at **0.8.1** (2025-11-14 — the version our pinned holochain already resolves); `kitsune2_transport_tx5` still does the per-send `data.to_vec()` copy on `main`; **#5664 open and stalled since 2026-03-02** (no closing PR; its heap dumps are *iroh* builds, so it's weaker evidence for our tx5 leak than it looks); no new transport-memory issue filed since 2026-06-01. **Decisive new fact:** upstream is **removing tx5, not buffer-fixing it** — kitsune2 **#542** ("Remove tx5 support", updated 2026-06-10) ships in kitsune2 0.5.x / **Holochain 0.7.x**. So no tx5 point-release cure is coming; the structural escape is the **iroh transport**.

**Two forward moves (both require a NEW edgenode conductor image — the leaking binary we don't compile):**

| Path | What it buys | What it does NOT fix | DNA-hash safety | Runnable today? |
|---|---|---|---|---|
| **A — 0.6.1 on tx5** (image built with `transport-tx5-backend-go-pion`) | **#5719** receipt-trigger fix → cuts matthew/james/jessica's **amplifier** (~1.8 GB/h, the "could not find url for peer" re-drive). Also the **cheap diagnostic**: does the receipt-rate drop flatten the slope toward terrance's floor? | terrance's universal floor (the buffer leak) — **stays** | **HASH-STABLE** (conductor-only; DnaHash resumed from PVC, not recomputed; full hash surface byte-identical 0.6.0→0.6.1; wasmer host 0.0.101→0.0.102 is a non-breaking load-ABI). **No genesis re-key.** | **NO** — gated on a holo-host image ≥0.6.1-**rc.8** |
| **B — 0.6.1 on iroh** (default) | Abandons go-pion → eliminates *this* leak mechanism. A **minor** bump (iroh-default shipped in 0.6.1, NOT 0.7.x as first thought). Also carries #5719. | Nothing proven — **iroh is an ESCAPE, not a validated cure** (#5664's iroh dumps show `magicsock` VecDeque growth → may leak its own way). Needs a canary memory-soak. | HASH-STABLE (conductor-only) | **NO** — no iroh-built edgenode published; can't canary a *subset* (mixed tx5/iroh mesh = partition) → whole-mesh move |

### Why "not runnable today" (the real gate)
Published `ghcr.io/holo-host/edgenode` tags top out at **`hc0.6.1-rc.1-dev`** (verified: `v0.0.10/v0.0.11-…-hc0.6.1-rc.1-dev`; everything else is `hc0.6.0-go-pion-*` or `hc0.5.6`; **no rc.8, no stable 0.6.1, no iroh image**). **#5719 landed in `holochain-0.6.1-rc.8` (2026-04-17)**, which is ~2 months *after* `0.6.1-rc.1` (2026-02-08) — so even the newest published edgenode predates the brake (`compare 5719...rc.1` = behind_by 25). Our base `v0.0.8-alpha31-hc0.6.0-go-pion-custom` is two minors below it. **The lever is holo-host's image cadence (last push 2026-06-10, still rc.1) OR a custom holonix build (heavy; pool at 85% ceiling).** Nothing in our repo delivers the conductor fix.

### Correction to §6 — the "lower arc factor" mitigation is FALSIFIED
The handoff §6 names "lower arc factor / target_arc → fewer sends → lower leak rate" as the lever we hold. It is **not a lever for this leak**: (1) `target_arc_factor` is a u32 **participation switch `{0,1}`**, not a fractional dial — fractional sharding is hard-clamped upstream (`factor>1 → forced to 1`, "multi-factor sharding isn't yet implemented"; see `.claude/data/arc-factor-feasibility-findings.md` + `matthew-edge-resiliency-rca-fanout-2026-06-15.md` §8); and (2) the leak is **arc-independent** — arc=0 nodes (jessica, james) leak the *same* monotonic shape (james arc=0 is the *worst*; see `arc-shrink-ineffective-memory-soak.md`). So the only arc move available can't touch the per-send floor. **The only mitigation fully in our control today is the cgroup memory-bound crash-floor + periodic restart** (accept the sawtooth; treat OOM-restart count as the pressure gauge). Every mitigation that actually *reduces* the leak rate (Path A's #5719 brake, Path B's iroh escape) is gated on the holo-host image we don't compile.

### Live diagnostic (run 2026-06-17, free + in-tree — gates the external ask)
Before spending external-image budget on Path A, the cheap test is: does the per-pod anon-leak slope track the receipt-error rate? **It does — Path A's target is real.** Prometheus `deriv(elohim_node_conductor_smaps_anon_bytes{class="other"}[2h])` peaks at **matthew 2.12 / james 2.04 GB/h**; Loki `count_over_time(… |= "could not find url for peer" [1h])` returns **exactly 3 emitters — matthew 42,832/h, james 40,580/h, jessica 36,828/h — and zero on the other 11 pods.** The high-slope pods ARE the receipt-error pods. #5719 clears the receipt requirement for offline authors, stopping the re-drive **by a different mechanism than the refuted F-BOOTSTRAP fix** (which tried to make the URL *resolve*; #5719 stops *retrying* regardless) — so it should cut the amplifier even though bootstrap unification didn't. **Caveat:** the instant window was confounded by active pod churn (two scrape IPs/pod, uniform ~0.6–0.8 GB/h cold-refill on the fresh instances, adam at −1.7 reclaim) — the clean steady-state correlation is the §6 settled-window table; this run re-confirms the *concentration*, not a clean slope. The ~0.2 GB/h floor on the 11 zero-error pods is **untouched by #5719** — that residue is the go-pion buffer leak (cure/iroh territory).

### Recommended next actions (revised roadmap)
1. **Confirm Path A is even on holo-host's menu BEFORE asking for it.** ⚠ Every `0.6.0` edgenode tag carries `-go-pion`; the published **`0.6.1-rc.1` tags DROPPED the `-go-pion` suffix** — and 0.6.1 defaults to iroh (verified CHANGELOG). Strong signal that holo-host's 0.6.1 line is **iroh-default and may not ship a `transport-tx5-backend-go-pion` variant at all.** So the move is: (a) verify holo-host still builds a tx5-0.6.1 image (their build source wasn't a public repo I could read — operator may know, or check the image's transport at runtime); if **yes** → request a ≥0.6.1-rc.8 tx5 image (cuts the amplifier on the 3 pods, DNA-safe); if **no** → Path A is either a **custom holonix edgenode build** (heavy; pool at 85%) or it **collapses into Path B** (their only 0.6.1 image *is* iroh).
2. **Track iroh (Path B) as the structural escape — validate, don't assume.** It's a 0.6.1 *minor* (available sooner than the 0.7.x tx5-removal), but it trades go-pion's CGo-mmap mechanism for iroh's unproven profile (#5664's iroh dumps leak too). Needs a **canary memory-soak** before any fleet roll, and a **whole-mesh** cutover (mixed tx5/iroh = partition — can't canary a subset). If Path A is off-menu (rec #1), this becomes the de-facto forward path and should get the soak budget.
3. **File the upstream issue (still un-filed) — draft ready, operator sends (outward-facing).** Frame: per-send go-pion buffer retention re-driven by the validation-receipt "could not find url for peer" loop, with our >128 KB-discrete-mmap telemetry (flat `[heap]`, growing `anon_mapping_count`, off-heap CGo). Attach to **#5664** — whose dumps are *iroh*-only, so maintainers may not realize the tx5/go-pion path has the same per-send-buffer issue. #5664 stalled since 2026-03-02; a tx5-specific off-heap report is new signal that also informs whether iroh (Path B) actually escapes it.
4. **Pre-rollout gates for ANY 0.6.1 pin** (all cheap, resolve the residual unknowns): (a) test-load the `a6d4e805`-built `.dna` on a 0.6.1 binary → DnaHash unchanged + cell starts (the definitive hash check); (b) verify conductor DB/PVC schema migration 0.6.0→0.6.1 on a PVC *copy*; (c) confirm the image's transport matches the mesh (0.6.1 defaults to iroh + kitsune2 0.4.0-dev.10 — a silent switch partitions the genesis pair by a non-DNA mechanism).

### Diff against the prior pass
- **STILL HOLDS:** "KNOWN-OPEN, no clean pin-to-fix" for the literal buffer cure. #5664 open/unreleased; tx5 ≤0.8.1.
- **CHANGED / new:** #5719 is **now in a tagged release** (`0.6.1-rc.8`/`0.6.1`) — the prior pass stopped at the `develop-0.6` branch. The **iroh escape is a 0.6.1 minor, not a 0.7.x major** (refuted the "wait for 0.7.x" framing). The **DNA-compat gate the prior pass left open resolves favorably** (conductor-only 0.6.1 bump is hash-stable; re-key risk attaches only to a *deliberate* DNA rebuild against hdi 0.7.1). **New partition risk surfaced:** transport mismatch (0.6.1 iroh default) — a non-DNA mechanism that must be gated.

## 4. TASK 2–3: rebuild the edgenode image + canary deploy

- **Image:** `elohim/holochain/edgenode/Dockerfile` (+ README). The base is holo-host's. Two paths: (a) consume a newer holo-host base that carries the fix, or (b) build our own with the forward pin. Prefer (a) if it exists.
- **Canary FIRST, never a blind fleet roll.** Deploy the pinned image to ONE expendable leecher (a fresh/non-genesis pod). Do **not** lead with the matthew/adam genesis pair (a rolling restart is benign for the DHT — rejoins same key/hash — but canary-first de-risks the *pin* itself; only the genesis pair carries the re-key/partition risk IF the pin accidentally moves the DNA hash, so verify §3 DNA-compat before any genesis restart).
- **Deploy path:** land on `dev` → orchestrator → edge → alpha (the working trigger; a `claude/*` push does NOT auto-deploy — see memory `project_sprint_branch_not_orchestrator_indexed`). Cluster is operator-owned; no `kubectl` from dev.

## 5. DONE criteria (the measurement)

On the canary, over a clean multi-hour window (relative `now-Xh` — dev clock is ~6h behind cluster):
- ✅ **CURED:** `elohim_node_conductor_smaps_anon_bytes{class="other"}` **flattens** — the monotonic climb stops (terrance's ~0.12 GB/h floor → ~0). The sawtooth becomes a flat line. This is the definitive signal.
- Receipt errors (`could not find url for peer`) may **persist** — that's churn-driven transport staleness, a separate concern; the buffer-retention stopping is what matters.
- ⚠️ **If the slope persists** post-pin → the fix didn't cover the retained buffer; re-open Task 1 (wrong PR identified).

## 6. Mitigation lever we DO hold (if Task 1 = WAIT)

Reduce transport work via conductor **config/topology** — lower `arc factor`/`target_arc`, fewer peers, leaner topology → fewer sends → lower leak rate. This is config, not a code change. It's a *mitigation* (longer sawtooth period), never a cure. Keep the cgroup memory bound as a crash-floor, and treat OOM-restart count as the pressure gauge, not as health.

## 7. Risks / constraints

- **DNA re-key/partition:** alpha has `ALLOW_DNA_REINSTALL` non-prod=true. If the pin moves the DNA hash, a reinstall re-keys agents → DHT partition. Verify the fix is transport/coordinator-only (no integrity-zome/modifier change) BEFORE touching the genesis pair. A plain rolling restart on an unchanged hash is safe.
- **Base-image ownership:** the edgenode base is holo-host's; building our own is a holonix/nix job (heavy, disk-hungry — note the pool is at the 85% ceiling).
- **Operator-owned cluster:** repo is the cleanup surface; deploys go through the dev→orchestrator pipeline.
