# Elohim Sweettest

Rust-native integration tests for all 5 Elohim DNAs (`imagodei`, `mishpat`,
`lamad`, `node_registry`, `infrastructure`), using Holochain's
[sweettest](https://docs.rs/holochain/latest/holochain/sweettest/index.html)
framework.

## Why a separate workspace

- Sweettest pulls in `holochain` (native) and `tokio` — these must not land
  in the WASM build graph for the DNAs themselves.
- Each DNA is its own Cargo workspace. This directory is **not** a member of
  any DNA workspace, so default `cargo build` inside a DNA does not pull
  native deps.
- Invoked explicitly:
  ```
  cd elohim/holochain/tests/sweettest
  CARGO_TARGET_DIR=target/native-tests cargo test -p elohim_sweettest
  ```

## Layout

```
elohim/holochain/tests/sweettest/
├── Cargo.toml                 # standalone workspace
├── README.md                  # this file
└── src/
    ├── lib.rs                 # re-exports `common::*`
    ├── common/
    │   ├── mod.rs
    │   ├── conductors.rs      # SweetConductor spin-up + DNA load with modifiers
    │   ├── fixtures.rs        # DNA path resolution + network seed defaults
    │   └── mirrors.rs         # DHT sync/propagation wait helpers
    └── tests/
        ├── imagodei.rs        # bootstrap-steward is configured + non-steward check
        ├── mishpat.rs         # governance baseline
        ├── lamad.rs           # content-store baseline
        ├── node_registry.rs   # node admission baseline
        └── infrastructure.rs  # federation-native baseline (no bootstrap steward)
```

The `[[test]]` targets in `Cargo.toml` map each test file to its own binary
so `cargo test` runs them in parallel.

## Running

1. **Pack the DNAs first.** Sweettest loads `.dna` bundle files from
   `dna/<name>/workdir/<name>.dna`. Ensure a full `just build` in each
   DNA workspace (or the packaged happ) has run.

2. **Invoke:**
   ```
   cd elohim/holochain/tests/sweettest
   CARGO_TARGET_DIR=target/native-tests cargo test -p elohim_sweettest --release
   ```

3. **Unignore tests:** baseline tests carry `#[ignore = "requires packed DNA
   artifact"]` until each DNA's Jenkins run proves green with
   `--include-ignored`. Flips are staged (one commit per DNA, behind its own
   Jenkins-green proof). Run `cargo test -- --include-ignored` locally to
   exercise them once DNAs are packed.

## Adding a new test

1. Extend an existing per-DNA file with a new `#[tokio::test(flavor = "multi_thread")] async fn …`.
2. If you need cross-agent behavior, pull `two_agent_conductors()` from
   `common::conductors`.
3. If you need to wait on gossip, use `common::mirrors::settle_dht` or
   `wait_for`.
4. For a brand-new DNA, add a new `src/tests/<dna>.rs` and a matching
   `[[test]]` entry in `Cargo.toml`.

## Pipeline integration — baseline coverage

Sprint 1.B scaffolded the workspace; the Wave 1 bodies sprint (2026-04-24)
filled the per-DNA baselines. Each file at `src/tests/<dna>.rs` covers:

| DNA | Scenarios |
|---|---|
| `imagodei` | `bootstrap_steward_is_identifiable`, `second_agent_is_not_bootstrap_steward`. Scenario 3 (validator rejection of bootstrap-only actions from non-stewards) is deliberately absent — see the test file's header comment; the bootstrap-steward pattern is identity, not a capability gate. |
| `mishpat` | `bootstrap_steward_is_configured`, `proposal_round_trips_across_agents` (two-agent create/read via `create_proposal` + `get_proposal_by_id`). |
| `lamad` | `content_store_is_reachable`, `content_publishes_and_retrieves_by_id` (single agent create + read by id + read by action hash), `content_visible_across_agents` (two-agent create + cross-agent read). |
| `node_registry` | `node_registry_has_bootstrap_steward`, `register_node_round_trips` (single agent), `admission_visible_across_agents` (two-agent `register_node` + cross-agent `get_nodes_by_region`). |
| `infrastructure` | `infrastructure_installs_without_bootstrap_steward`, `doorway_self_registers`, `doorway_visible_across_agents_and_operator_only_can_update` (self-registration succeeds, cross-agent read after settle, second-agent `update_doorway` rejected by coordinator). |

Each DNA has at least one cross-agent scenario that waits on
`common::mirrors::settle_dht` to verify DHT propagation. Fixture factories
(`common::fixtures::node_registration`) keep the 26-field `NodeRegistration`
construction out of the test body.

The Jenkins stage `DNA Integration (bootstrap-steward)` runs all five DNAs
with `--include-ignored` until the staged unignore flips complete (one
commit per DNA, each behind its own Jenkins-green proof). Measures live in
Jenkins, not locally — `feedback_shift_measure_jenkins.md`.

## Build environment

Sweettest pulls in `holochain` (native) and builds `libdatachannel` from
source via `datachannel-sys`. That build chain needs `cmake`, `clang-libs`,
`zlib-devel`, and a one-line patch to libdatachannel's CMakeLists injecting
`find_package(ZLIB REQUIRED)` before `find_package(OpenSSL)`. Three surfaces
to be aware of:

### Eclipse Che (recommended for contributors)

The `ethosengine/che-devworkspaces` image is preconfigured. See
https://github.com/ethosengine/che-devworkspaces/blob/main/containers/rust-dev/claude.md
for the container spec. Includes the libdatachannel CMakeLists patch and
sets `BINDGEN_EXTRA_CLANG_ARGS` so bindgen finds its clang resource dir
without a `clang` driver binary installed.

### Jenkins Nix build

The `holochain/dna/*.nix` dev shell provides `cmake`, `clang` (full driver),
`libsodium`, `openssl`, and the holochain toolchain. No workarounds needed —
`nix develop --command cargo test ...` just works. See the Jenkins stage
`DNA Integration (bootstrap-steward)` in `elohim/holochain/dna/Jenkinsfile`.

### Bare laptop (not recommended; contributors should use Che)

If you must build outside Che and outside Nix:

```bash
# RHEL/Fedora
sudo dnf install -y cmake clang-libs zlib-devel

# Debian/Ubuntu
sudo apt-get install -y cmake libclang-dev zlib1g-dev

# Apply the libdatachannel CMakeLists patch in the cargo registry:
CML=$(find ~/.cargo/registry/src -path '*datachannel-sys-0.23.0+0.23.2/libdatachannel/CMakeLists.txt' -print -quit)
grep -q 'find_package(ZLIB REQUIRED)' "$CML" || \
  sed -i 's|^\tfind_package(OpenSSL REQUIRED)$|\tfind_package(ZLIB REQUIRED)\n\tfind_package(OpenSSL REQUIRED)|' "$CML"

# Set bindgen's clang resource path (Linux location may vary):
export BINDGEN_EXTRA_CLANG_ARGS="-I/usr/lib/clang/20/include"
```

The devcontainer applies the patch and env var automatically. Contributors
on bare systems maintain the patch themselves.

## Husky compile-check gate

Any push that touches `elohim/holochain/dna/*/zomes/**/*.rs` or
`elohim/holochain/tests/sweettest/**` triggers a push-time
`cargo check -p elohim_sweettest`. This catches extern-signature drift
before Jenkins runs. See `.husky/pre-push` and
`elohim/holochain/dna/build-manifest.json` for the wiring.

## Jenkins stage

The Jenkins stage `DNA Integration (bootstrap-steward)` runs
`cargo test --release -- --test-threads=1 --include-ignored` after the
`Build DNA` stage (packed `.dna` artifacts are a prerequisite). Output
is filtered to summary-on-pass, full panic context on fail. See
`elohim/holochain/dna/Jenkinsfile`.
