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

3. **Unignore tests:** the baseline tests carry `#[ignore]` until the Jenkins
   pipeline wires pack-then-test. Run with `--ignored` to exercise them
   locally once DNAs are packed.

## Adding a new test

1. Extend an existing per-DNA file with a new `#[tokio::test(flavor = "multi_thread")] async fn …`.
2. If you need cross-agent behavior, pull `two_agent_conductors()` from
   `common::conductors`.
3. If you need to wait on gossip, use `common::mirrors::settle_dht` or
   `wait_for`.
4. For a brand-new DNA, add a new `src/tests/<dna>.rs` and a matching
   `[[test]]` entry in `Cargo.toml`.

## Pipeline integration (pending — Sprint 1.B Jenkins follow-up)

Each DNA gets its own Jenkins stage so failures are attributed crisply. All
five stages run in parallel; all must be green for the holochain pipeline to
gate green. That wiring is deferred to the Wave 1 close-out with
`feedback_shift_measure_jenkins.md` as the bar — tests must be green on
Jenkins, not merely locally.
