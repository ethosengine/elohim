# Holochain Development Workflow

The repository-root `justfile` is the supported entrypoint for the local
conductor, storage service, Doorway, seeding, and health checks. Its lifecycle
commands discover dynamic conductor ports and route native Rust builds through
the cargo target pool.

## Quick Start

From the repository root:

```bash
just dev start                         # isolated conductor + storage + Doorway
just seed validate local content       # non-writing content validation
just seed apply local content 10       # explicitly seed ten content records
just dev app                           # Angular app in a second terminal
```

To build the native services before starting, or seed the full content corpus
during startup:

```bash
just dev start isolated false true     # build, then start
just dev start isolated true true      # build, start, and seed
```

The default forms are safe inspections: `just dev` reports runtime status and
`just seed` validates content without writing it.

## Components

```text
Angular app (:4200) -> Doorway (:8888) -> Holochain conductor (dynamic admin port)
                              |
                              +----------> elohim-storage (:8090)
```

- `hc-start.sh` owns the single-peer conductor/service lifecycle.
- `just dev` supplies the supported public interface.
- `build-manifest.json` supplies native gate commands and explicit cargo-pool
  workspace mappings.
- `genesis/seeder` owns schema validation, seed application, diagnostics, and
  snapshots.

## Command Reference

| Command | Effect |
|---|---|
| `just dev` | Report local runtime status |
| `just dev start` | Start the isolated local stack |
| `just dev start alpha` | Join the alpha network profile |
| `just dev conductor` | Start only the conductor |
| `just dev app` | Start the Angular app |
| `just dev stop` | Stop the local conductor and service ports |
| `just status runtime` | Probe Doorway, storage, and mesh state |
| `just seed validate local content` | Validate content without writing |
| `just seed validate local all` | Run all structured-data validators |
| `just seed apply local content [limit]` | Seed local content explicitly |
| `just seed stats local` | Read local seed statistics |
| `just seed diagnose local` | Diagnose the local seed surface |

`just seed apply alpha ...` is a deliberate remote write. Production seeding is
CI/operator-owned and is not exposed as a local profile.

## Snapshots and Clean Slates

Snapshot operations remain specialist seeder commands:

```bash
cd genesis/seeder
pnpm run snapshot:status
pnpm run snapshot:save
pnpm run snapshot:restore
pnpm run snapshot:list
```

There is intentionally no destructive root `reset` verb. If a clean conductor
is required, save anything needed first, stop the stack, and use the explicit
compatibility reset from `app/elohim-app`:

```bash
just dev stop
cd app/elohim-app
pnpm run hc:reset
```

That reset deletes local conductor and storage state. See
[`genesis/seeder/README.md`](../../../genesis/seeder/README.md) before snapshot or
seed recovery work.

## Troubleshooting

Start with the read-only probes:

```bash
just status runtime
just seed diagnose local
```

If a service is down, stop and restart the coordinated stack instead of killing
one process or launching a binary from a hard-coded `target/release` path:

```bash
just dev stop
just dev start
```

Native Rust commands outside the manifest runner must have the correct
`CARGO_TARGET_DIR`; DNA/WASM builds are the exception and remain unredirected.

## Canonical Paths

| Path | Purpose |
|---|---|
| `elohim/holochain/local-dev/` | Local conductor state and discovered ports |
| `elohim/holochain/dna/elohim/` | Lamad DNA/hApp source and workdir |
| `doorway/doorway-service/` | Doorway Rust service |
| `elohim/elohim-storage/` | Storage Rust service |
| `genesis/data/lamad/` | Content source data |
| `genesis/seeder/` | Validation, seeding, diagnostics, and snapshots |
