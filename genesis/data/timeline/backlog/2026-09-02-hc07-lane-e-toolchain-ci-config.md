---
id: "backlog-hc07-lane-e-toolchain-ci-config"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Holochain 0.7 — Lane E: toolchain, conductor image, CI, and every conductor-config surface"
slug: "hc07-lane-e-toolchain-ci-config"
written: "2026-09-02"
author: "holochain 0.7 upgrade guide (Lane E)"
status: "open"
priority: "high"
tags: [holochain-0.7, ci, conductor-image, conductor-config, che-devworkspaces, codex-claimable, lane-e]
cites:
  - genesis/docs/superpowers/plans/2026-09-02-holochain-0-7-upgrade-guide.md
---

# Lane E — toolchain, CI, images, config templates (claimable by any agent; no session context assumed)

Part of the Holochain 0.7 upgrade guide (see cite; read **§Global Constraints**, **§Version Table**
and **§Lane E** — the lane section lists every file with line numbers; this item is the executable
summary). No cargo builds in this lane except one `cargo check` in step 8.

## Write-set (nothing else)

- Submodule `che-devworkspaces/` (its own git repo; work on a branch `holochain-0.7` inside it):
  `containers/elohim-edgenode/Dockerfile`, `containers/elohim-edgenode/conductor-config.yaml`,
  `jenkins/Jenkinsfile-elohim-edgenode`, `containers/rust-dev/Dockerfile`,
  `containers/udi-plus-mem-rust-nix/Dockerfile`.
- Monorepo: `elohim/conductor-image/{README.md,build-manifest.json}`, `scripts/ci/build-storage-image.sh`,
  `scripts/ci/validate-conductor-config.sh`, `scripts/ci/conductor-workload-pin.sh`,
  `genesis/orchestrator/Jenkinsfile`, `genesis/orchestrator/commit-tag-parser.mjs`,
  `elohim/holochain/Jenkinsfile`, `elohim/holochain/edgenode/{conductor-config.yaml,README.md}`,
  `genesis/orchestrator/manifests/edgenode/{alpha,staging,prod}.yaml`,
  `genesis/orchestrator/manifests/humans/{_edgenode-conductor.template.yaml,adam-firstman-conductor.yaml,_edgenode-consolidated.template.yaml}`,
  `app/elohim-app/scripts/hc-mesh.sh`, `app/elohim-app/scripts/hc-start.sh`,
  `elohim/holochain/dna/elohim/flake.nix`, `steward/device/src-tauri/src/{doorway,lib}.rs`.

## The facts you need

- 0.7.0 removed the tx5/WebRTC transport, the SBD signal server, and the conductor-config keys
  `signal_url` and `webrtc_config`. iroh is the only transport (no cargo feature). Unknown config keys
  hard-fail conductor startup. `relay_url` and `bootstrap_url` stay, and **per-doorway relays stay**
  (operator ruling 2026-09-02: doorways are different operators).
- Conductor cargo features at 0.7.0: default = `["encryption","schema","wasmer-sys-cranelift"]`.
  Our production set becomes **`encryption,wasmer-sys-cranelift,jemalloc`** with `--no-default-features`
  (drops `schema`; `jemalloc` is our fork's allocator cure — never drop it). Today's default string in
  `containers/elohim-edgenode/Dockerfile:48` and `jenkins/Jenkinsfile-elohim-edgenode:126` is
  `sqlite-encrypted,wasmer_sys,transport-tx5-backend-go-pion,jemalloc`.
- No Go toolchain is needed anymore (`Dockerfile:57-58` installs go1.24.4 for tx5's CGo backend).
- Image tag becomes `conductor-<hc12>` (12 chars of the `elohim/holochain-conductor` gitlink) — the
  `-<tx512>` half and every `TX5_FORK/TX5_BRANCH/TX5_REF` parameter go away
  (`genesis/orchestrator/Jenkinsfile:651-652`, `scripts/ci/build-storage-image.sh:19,89`,
  `elohim/conductor-image/README.md`, `build-manifest.json` watched paths).
- `genesis/orchestrator/commit-tag-parser.mjs:35-38` maps variant `iroh` → a feature string; keep the key,
  make it the default string, document it as a no-op.
- `elohim/holochain/Jenkinsfile`: `resolveStorageImage` (`:926-954`) repoints alpha to `elohim-storage-iroh`
  — collapse so every env uses `elohim-storage:${STORAGE_TAG}` and delete the `push-storage-iroh.sh`
  lane at `:2516-2535` (keep the relay build/push at `:2300`). `resolveRelayUrl` (`:915`) is keyed off
  the signal URL — key it off a new `relayUrl` field added beside `bootstrapUrl` in the primaryDoorway
  maps at `:535-566` (`alpha` → `https://relay.alpha.elohim.host`, `alpha-b`/shem → `https://relay.elohim.host`;
  staging/prod: same host pattern as their bootstrap, and FAIL the render if unmapped — never default
  to n0's public relay). Delete the `SIGNAL_URL_PLACEHOLDER` sed at `:1057`. Wrap the coturn deploy at
  `:554` in `if (env.RETIRE_TX5 == 'true')` (default unset) — Lane G flips it later.
- Prebuilt 0.7.0 binaries exist at
  `https://github.com/holochain/holochain/releases/download/holochain-0.7.0/{holochain,hc,hcterm}-x86_64-unknown-linux-gnu`
  (verified HTTP 200) — the same unversioned asset names the dev-container Dockerfiles already fetch
  through the Nexus proxy with `ARG HOLOCHAIN_VERSION=0.6.0`. lair 0.7.1 (`v0.7.1`) publishes **no**
  binary asset.
- holonix has `main-0.7` (2026-07-31). `elohim/holochain/dna/elohim/flake.nix:5` says `ref=main-0.6`.
  `nix` is NOT installed in the dev container, so `flake.lock` cannot be regenerated here.

## Steps

1. che Dockerfile: `ARG HC_FEATURES=encryption,wasmer-sys-cranelift,jemalloc`; keep `--no-default-features`;
   delete the Go install and all `TX5_*` args/clone steps; rewrite the `:46-48` comment (drop the tx5
   rename note, keep the jemalloc rationale).
2. che Jenkinsfile: same default string; delete `TX5_REF` handling; tag = `conductor-<hc12>` from `HC_REF`
   alone; delete the `irohTransport` isolation branch (iroh is the only transport, every build pushes
   the fleet tag); keep the `jemalloc-prof` isolation branch unchanged. Commit inside the submodule
   on branch `holochain-0.7`.
3. Monorepo pin derivation: `build-storage-image.sh` derives `CONDUCTOR_PIN` from
   `git rev-parse HEAD:elohim/holochain-conductor | cut -c1-12`; orchestrator Jenkinsfile drops `TX5_REF`;
   `build-manifest.json` drops `elohim/tx5` from watched paths; README tag shape + tx5 paragraph → one
   history line; `conductor-workload-pin.sh` — grep for `tx5|tx512` and fix.
4. Edge Jenkinsfile changes listed above.
5. Conductor configs (all six files): delete the `signal_url:` line and the whole `webrtc_config:` block
   with its comments; keep `bootstrap_url`, `relay_url`, `advanced.k2Gossip`, `data_root_path`, `keystore`.
   Check each with
   `python3 -c "import yaml,sys; c=yaml.safe_load(open(sys.argv[1])); n=c['network']; bad=set(n)-{'bootstrap_url','relay_url','advanced','target_arc_factor','request_timeout_s'}; assert not bad, bad" <file>`
   (for k8s manifests, extract the `conductor-config.yaml:` block first).
6. `scripts/ci/validate-conductor-config.sh`: invert — FAIL if `signal_url` or `webrtc_config` present
   ("tx5 keys hard-fail a 0.7 conductor"); FAIL if `relay_url` absent; keep the `*.iroh.network` check.
7. Local mesh: `hc-mesh.sh` / `hc-start.sh` stop emitting the two keys; the `HOLOCHAIN_BIN` directory
   contract (both `holochain` + `hc`) stays; update the 0.6.0 notes at `hc-mesh.sh:1103,1148`.
   Dev containers: `ARG HOLOCHAIN_VERSION=0.7.0`; for lair, grep devfiles/skills for `lair-keystore`
   usage — if nothing needs a standalone lair, delete the download and `LAIR_VERSION`; otherwise
   `cargo install lair_keystore --version 0.7.1 --locked`. Say which in the commit.
8. Flake: `ref=main-0.6` → `ref=main-0.7`; add to the DNA Jenkinsfile nix stage a guarded
   `nix flake update holonix --flake elohim/holochain/dna/elohim` (only when the lock's holonix ref is
   not `main-0.7`) OR note that the operator regenerates `flake.lock` with nix and commits it. State
   which path you took. steward device: drop the two keys from any emitted conductor config; a
   "signal URL" derived from the doorway becomes that doorway's relay URL;
   `RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/steward__node/dev cargo check` in
   `steward/device/src-tauri`; echo EXIT=$?.

## DoD

`bash scripts/ci/validate-conductor-config.sh` passes on every manifest; the YAML key assertion passes on
all six configs; `grep -rnE 'signal_url|webrtc_config|transport-tx5|go-pion|tx512|TX5_REF' <write-set>`
returns only history comments; commits path-limited (one in the submodule, one in the monorepo:
`chore(ci,config): holochain 0.7 toolchain — conductor features encryption/wasmer-sys-cranelift/jemalloc, no Go, tag conductor-<hc12>, tx5 keys removed from every conductor config, holonix main-0.7`).
**Commit-only; never push.** Do not bump the `che-devworkspaces` gitlink in the monorepo — the
integrator does that after pushing the submodule branch.
