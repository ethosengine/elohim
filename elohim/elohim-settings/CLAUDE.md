# elohim-settings — the storage node's settings layer

Two modules that are mutually recursive and therefore move as one unit:

- `config` — the boot `Config` (env → TOML → struct), the process-wide `OnceLock` mirrors the
  reconcile sweeps read, and the `set_*` publishers `main` calls once the config is assembled.
- `runtime_config` — the WATCHED-FILE registry (upgrade-velocity rung 4), so a flag flip applies
  to a RUNNING node instead of costing a pod roll. `config`'s publishers seed its boot values;
  its `SPECS` read `config`'s `DEFAULT_*` constants. Neither can be extracted without the other.

Carved out of `elohim-storage/src/config.rs` + `src/runtime_config.rs` byte-identically. Both
paths stay alive as MODULE-SCOPED re-export shims (`pub use elohim_settings::config::*;` and
`pub use elohim_settings::runtime_config::*;`), so `crate::config::X` and
`crate::runtime_config::Y` still resolve upstream at ~28 files. **Two shims, never one flattened
glob** — a single namespace would collide `Key`, `Kind` and `config_path`.

Design: `genesis/docs/superpowers/specs/2026-09-19-storage-crate-decomposition-design.md`
(§3 principles, §4 row 3 — this crate, §5 proof, §8a the cut list).

## What it refuses to know

No service, no transport, no database, no HTTP route — and **no async runtime**.

The runtime-config watcher's *decisions* live here and are synchronous `std`: the registry, the
parse, the apply, the provenance, the signature comparison, the WARN line on every live change.
Its CLOCK does not: `runtime_config::begin_watch()` arms it and `runtime_config::poll_once(path)`
is one tick, while the `tokio::spawn` + `tokio::time::interval` that drives them lives one layer
up in `elohim_storage::runtime_config_watch` (one call site, `main.rs`). A settings layer that
pulled an async runtime would colour every crate above it with that choice for the sake of one
poll loop.

**The boundary is the dependency graph, not a lint.** A direct `use tokio;` fails to compile; a
*transitive* pull is invisible until link time, so `tests/boundary.rs` asserts it over this
crate's own `Cargo.lock`: no `tokio`, no `libp2p`, no `iroh`, no `holochain*`, no `diesel`, no
`reqwest`, no `hyper`/`axum`, and exactly ONE first-party dependency. Do not widen that denylist.

`elohim-cache-core` is that one first-party dep, and it is the type, not a convenience:
`Config::extraction_cache` IS `ExtractionCacheConfig`. It is taken with DEFAULT features
(`default = []`) so its optional `native` feature — which pulls tokio — stays out of this graph;
`elohim-storage` enables `native` on the same single package and features union.

## The one inverted cut

`ALTERNATE_ADVERTISER_CAP` is defined HERE and re-exported by
`elohim_storage::services::head_adoption`, which is where the harvest that enforces it reads it.
The direction is not an accident: `config::evidence_fallback_max_alternates()` clamps to the cap,
so a definition in that service would make the settings layer depend upward on a service — the
single outward reference the decomposition inverts (spec §8a). Every existing
`services::head_adoption::ALTERNATE_ADVERTISER_CAP` path keeps resolving through the alias.

## Adding a setting

Three shapes, three obligations:

1. **A new `Config` field.** Additive. `#[serde(default = "…")]` plus a line in
   `impl Default for Config`. Nothing else in the stack changes.
2. **A new boot publisher (`pub fn set_*`).** Add it to the `BOOT_PUBLISHERS` const in the same
   commit — `tests::the_publisher_registry_matches_this_module` fails otherwise — and give it a
   call site in storage's `main.rs`, which
   `config::tests::every_boot_publisher_is_called_from_main` checks THERE against the same list.
   The list is declared rather than scanned across the crate boundary because the storage
   Dockerfile flattens sibling crates into `/app` and rewrites only Cargo paths: a cross-crate
   `include_str!` resolves in the dev tree and breaks in the image.
3. **A new hot-reloadable `Key`.** Add the `Key` variant, its `SPECS` entry (the array length is
   a compile-time check) and its `Key::ALL` entry. Then decide `unpublished_by_design`: `None`
   means a boot publisher runs BEFORE the watcher starts, and
   `runtime_config::assert_boot_published()` refuses the boot if it did not. `Some(reason)` is
   the documented escape hatch — use it only when the key genuinely has no boot env read.

A setting whose read site captures its value once (a struct field moved into a closure, an
interval built at spawn) is declared BOOT-ONLY in `BOOT_ONLY` instead. A lever that reports
"applied" and changes nothing is worse than no lever.

## The boot assertion, both halves

`provenance` cannot serve as the check: every setting is seeded `BootEnv` at construction, so it
reads identically for "`main` published the env value" and "nothing ever touched this key".

- **Static** — `BOOT_PUBLISHERS` × `main.rs`: every publisher has a call site.
- **Runtime** — `Setting::published` × `assert_boot_published()`: every call actually landed on
  its key, checked once before the watcher starts.

Both exist because the failure is silent by construction, and it has already shipped once
(2026-09-19, `REANCHOR_HELD_BACKOFF_SECONDS` and `CONTEST_REMINT_WINDOW_SECONDS`).

## Gate

```bash
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/<branch>/elohim-settings/dev \
  just --justfile elohim/elohim-settings/justfile gate
```

`gate` = `cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`
(which runs `tests/boundary.rs`). `RUSTFLAGS` is baked EMPTY in the justfile: this is a native
build with no wasm-facing dependency, so the ambient `--cfg getrandom_backend="custom"` the
Holochain path exports would only break linking. `just gate elohim-settings` from the repo root
resolves the pool slot from `elohim/holochain/build-manifest.json`.

A change here rebuilds the storage image, so this crate is COPY'd in
`elohim/elohim-storage/Dockerfile` and watched by that manifest's `cargo-build-storage` sources.
`genesis/orchestrator/storage-build-inputs.test.mjs` holds both.

**Verify a settings change at runtime**, not only at the gate: `GET /admin/runtime-config` is the
one surface where a silent default shows up — 9 `SPECS` + 1 `TEXT_SPECS` + 5 `BOOT_ONLY`, each
with its effective value, its boot value and its provenance.
