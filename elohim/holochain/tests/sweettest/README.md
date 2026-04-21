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

## Che compile blocker (2026-04-21)

The `holochain` crate's default features include `datachannel-vendored`,
which pulls in `datachannel-sys` and builds libdatachannel from source via
a `cmake` build script. Che's base image (`base-developer-image:ubi10-latest`)
does not ship `cmake`. Without it, `cargo check -p elohim_sweettest` fails:

```
thread 'main' panicked at cmake-0.1.58/src/lib.rs:1132:5:
failed to execute command: No such file or directory (os error 2)
is `cmake` not installed?
```

Alternatives evaluated:

- `backend-go-pion`: requires `go` compiler — not installed either.
- `default-features = false` without any webrtc backend: compile fails
  inside `tx5-connection-0.8.1/src/config.rs` because `BackendModule`'s
  `Default` impl is feature-gated and collapses to `fn default() -> Self {}`
  when no backend is selected.
- Using the installed `/opt/holochain/bin/holochain` binary directly:
  sweettest is in-process; it needs the holochain crate linked into the
  test binary, not an external binary.

Orchestration options (out of scope for this session):

1. Add `cmake` + `make` to `Dockerfile` (one-line `dnf install -y cmake
   make` add-on). This is the lightest path; only affects the image,
   rebuilds on next workspace spin-up.
2. Run sweettest exclusively in Jenkins (where cmake is presumably
   available via the holochain build agent). Reverts to the "measures
   live in Jenkins" default for this particular gate.
3. Switch to `backend-go-pion` and add `go` to the image. Wider surface
   area change.

Until this is resolved, the sweettest tests stay `#[ignore]`'d and only
compile in environments with cmake installed. The `manifest-hygiene/`
sibling crate does not depend on holochain and runs fine locally in Che —
husky pre-push uses that for the fast gate.
