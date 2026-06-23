# Conductor zombie-leak fix — deploy recipe (2026-06-17)

How to get the verified tx5 zombie-connection fix (+ #5719 amplifier brake) onto the alpha conductors. RCA + verification: `conductor-leak-rca-tx5-gopion-backpressure-2026-06-17.md`. The fix is **proven** (tx5 teardown tests pass with it / time out without) and lives in the pushed forks.

## The mechanism: a binary SWAP (not a full image rebuild)
The holo-host edgenode base does **not** compile holochain — it `wget`s a **prebuilt** binary and `mv`s it to **`/bin/holochain`** (+ `hc` → `/bin/hc`):
- source: `github.com/zo-el/holochain-binaries` release `holochain-binaries-chore-downgrade-kitsune2-to-0.3.0-dev.3`, asset `…-x86_64-unknown-linux-gnu` (**glibc**, not musl).
- base entrypoint: `/usr/local/bin/entrypoint.sh` → launches `/bin/holochain`.

So our custom image = build OUR patched `holochain` and `COPY` it over `/bin/holochain`. No need to replicate holo-host's whole build. Wired in **`elohim/holochain/edgenode/Dockerfile.zombie-fix`** (multi-stage: builder clones the two forks side-by-side, compiles, swaps).

## Build base + inputs
- Conductor fork: `ethosengine/holochain@elohim-0.6` = `a6d4e805` (holochain-0.6.0) + #5719 + the tx5 `[patch.crates-io]`.
- tx5 fork: `ethosengine/tx5@elohim-0.8.1-zombie-fix` = v0.8.1 + #194 + #199.
- The conductor `[patch]` routes the tx5 0.8.1 family → `../tx5/crates/*` (forks must be siblings).
- **`[patch]` VALIDATED 2026-06-17**: `cargo` locked all six tx5 crates to our fork paths (`Adding tx5 v0.8.1 (…/elohim/tx5/crates/tx5)` …). The go-pion feature set was accepted.

## Build command (validated invocation)
```
cargo build --release --bin holochain --no-default-features \
  --features sqlite-encrypted,wasmer_sys,backend-go-pion
```
**Build-env gotchas (all hit + solved this session):**
- **Go 1.24** must be on PATH — `tx5-go-pion-sys` is CGo (vendored pion/webrtc v4.1.3). gcc suffices as CC.
- **`--no-default-features`** is mandatory: the default tx5 backend is `backend-libdatachannel` → pulls `datachannel-sys`, which builds its own OpenSSL via CMake + bindgen and **fails on `stdbool.h` not found** (clang include-path). We want go-pion anyway.
- **`RUSTFLAGS=""`** — native build; the repo's WASM `getrandom` flag breaks the link.
- **`RUSTC_WRAPPER=""`** — the dev-container sccache intermittently fails to spawn (`could not execute process sccache … No such file or directory`); disabling it is the fix (see memory `feedback_sccache_spawn_enoent_rca`).
- **`/dev/null`** — the Bash sandbox doesn't expose it; the Go toolchain needs it → run the build with the sandbox disabled (or on a real host/CI).
- `CARGO_TARGET_DIR=/tmp/…` to avoid the container's pool-fingerprint quirks; the build wants ~15-25 GB + network (crates from elohim-mirror; Go deps vendored).

## TWO things to verify before fleet use (not yet closed)
1. **Exact `go-pion-custom` production feature set.** The set above built/resolved, but holo-host's release recipe is authoritative for the *production* binary (wasmer variant, any `unstable-*`/`chc` flags). A wrong set = a subtly-different conductor. Confirm against holo-host's build.
2. **kitsune2 wire-compat.** Our binary links kitsune2 **0.3.2** (a6d4e805); the live mesh runs **0.3.0-dev.3** (holo-host's downgrade branch). Both 0.3.x → expected wire-compatible, but **canary first** and confirm gossip/DHT before the genesis pair. (DNA-load compat is fine: our binary loads our a6d4e805-built DNA natively.)

## CANARY IMAGE READY (2026-06-17)
Built from the verified binary (no 40-min rebuild) + pushed:
**`ghcr.io/ethosengine/elohim-edgenode:zombie-fix-canary-5a73e400d`**
= holo-host base `v0.0.8-alpha31-hc0.6.0-go-pion-custom` + our `holochain` binary at `/bin/holochain`
(elohim-0.6: a6d4e805 + #5719 + tx5 [patch] #194/#199, go-pion, glibc). Only the binary differs from
prod — same base, same config handling. (Dev container can't reach harbor creds / kubectl, so it's on ghcr;
copy to harbor or point the cluster at ghcr.) ⚠ canary-only — the two checks below still gate fleet roll.

## ⚠ CORRECTED DEPLOY TARGET — it's the STORAGE image, not the edgenode image (2026-06-17)
The LIVE alpha runs the **consolidated** model (`genesis/orchestrator/manifests/edgenode/_edgenode-consolidated.template.yaml`, NOT the separate-container `alpha.yaml`): one **`elohim-node` container = the `elohim-storage` image**, which runs the conductor as an **embedded child** (`process_manager.rs` `Command::new(conductor_binary)`). The `elohim-storage` Dockerfile **extracts the conductor binary** from a `conductor-source` stage (`COPY --from=conductor-source /usr/bin/holochain …`). So the leaking conductor is carried by the **elohim-storage image**; the edgenode image is only the binary SOURCE.

**Wiring (landed on dev `b33ff524a`):** `elohim/elohim-storage/Dockerfile` now takes `ARG CONDUCTOR_SOURCE_IMAGE` (default = holo-host base → production byte-identical). Override it to embed the patched conductor.

**Canary storage build — run in PROPER infra (NOT the dev container: podman `/dev/null` is read-only → `apt-get` fails in the Rust builder):**
```
docker build -f elohim/elohim-storage/Dockerfile \
  --build-arg CONDUCTOR_SOURCE_IMAGE=harbor.ethosengine.com/ethosengine/elohim-edgenode:<edgenode-job-tag> \
  -t harbor.ethosengine.com/ethosengine/elohim-storage:zombie-fix-canary <repo-root>
docker push harbor.ethosengine.com/ethosengine/elohim-storage:zombie-fix-canary
```
(`<edgenode-job-tag>` = a tag from the che-devworkspaces `elohim-edgenode` job. Cleanest durable form: a `CONDUCTOR_SOURCE_IMAGE` param on the edge pipeline's storage-build step, default holo-host.)
Then deploy that **storage** image to one non-genesis leecher's `elohim-node` container; watch `smaps_anon{class=other}` flatten + stay in-mesh; roll, genesis pair last.

(NOTE: the earlier "point a leecher at the elohim-edgenode image" runbook below is superseded by this — the conductor is embedded in the storage image. The `ghcr …elohim-edgenode:zombie-fix-canary` image is still useful as the `CONDUCTOR_SOURCE_IMAGE`.)

## Deploy (operator / cluster-owned — no kubectl from dev)
**Canary runbook (alpha = per-human StatefulSets `elohim-{human}-alpha`, ns `elohim-alpha`):**
1. Get the image where the cluster pulls (harbor): `skopeo copy docker://ghcr.io/ethosengine/elohim-edgenode:zombie-fix-canary-5a73e400d docker://harbor.ethosengine.com/ethosengine/elohim-edgenode:zombie-fix-canary-5a73e400d` (your creds) — or make the ghcr package public and point the canary at ghcr.
2. Pick ONE non-genesis leecher (`nodeTypes:['remote']`: susan/frank/pete/eve/nancy… — NOT matthew/adam/james/jessica). `kubectl set image statefulset/elohim-<leecher>-alpha <container>=<that-tag> -n elohim-alpha` then `kubectl rollout status`.
3. Verify over a clean multi-hour window (`now-Xh`):
   - **CURE signal:** `elohim_node_conductor_smaps_anon_bytes{class="other",pod="elohim-<leecher>-alpha-0"}` **FLATTENS** vs the unpatched leechers still climbing ~0.2 GB/h. (Best witness: a quiet leecher with 0 receipt errors.)
   - **kitsune2 wire-compat check (the 0.3.2↔0.3.0-dev.3 risk):** the leecher stays in the mesh — peerCount stable, gossip/DHT healthy, no partition. If it islands → roll back (revert image) and investigate.
   - DNA-load: the conductor starts + serves (loads the a6d4e805-built DNA natively).
4. Flattens + healthy → roll wider (other leechers → anchors → **genesis pair matthew/adam LAST**). Slope persists → wrong-binary/feature-set; investigate before rolling.

### (original note retained below)
1. CI/build-host builds `Dockerfile.zombie-fix` → push to the registry.
2. **Canary ONE non-genesis leecher** (per `HANDOFF-2026-06-17-upstream-tx5-transport-pin.md` §4) — point its edgenode at the patched image; over a clean multi-hour window confirm `elohim_node_conductor_smaps_anon_bytes{class="other"}` **flattens** (the definitive cure signal) and gossip/DHT stays healthy.
3. Only then roll wider; the genesis pair (matthew/adam) last. A rolling restart on an unchanged DNA hash is safe (no re-key).

## What's confirmed vs remaining
- ✅ Fix correct + built + tested (tx5 level); `[patch]` + go-pion features validated; binary-swap path + Dockerfile wired; forks pushed; upstream comments posted.
- ✅ **Full conductor binary BUILT + RUNS (2026-06-17)** — `cargo build --release --bin holochain --no-default-features --features sqlite-encrypted,wasmer_sys,backend-go-pion` from `elohim-0.6` produced a 51 MB glibc x86-64 binary that reports `holochain 0.6.0`, with the patched tx5 (#194+#199) linked via `[patch]`. The whole chain compiles + links end-to-end. (Built at `/tmp/hc-target/release/holochain` — ephemeral; rebuild via the recipe or CI.)
- ☐ CI image build (`Dockerfile.zombie-fix`) · operator canary deploy · the two verifications above. **This is the remaining path to the actual alpha cure — operator/cluster-owned.**
