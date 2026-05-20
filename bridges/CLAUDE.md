# bridges/ — Pluggable Interop Layers

This directory holds bridge crates — Rust libraries that translate external
protocols (web2 federation, VF-GraphQL/hREA, future) to and from elohim's
canonical EPR-REA substrate.

## Pattern

Each bridge is a library crate. Runtimes (`doorway-service`, `elohim-storage`)
consume the bridges they need:

- `doorway-service` consumes bridges that absorb web2 traffic (`atproto`,
  `activitypub`, future)
- `elohim-storage` consumes bridges that speak protocol-shaped interop
  (`valueflows`)

Bridges are libraries, not services. The runtime hosting a bridge is decided
by the kind of traffic it absorbs (web2 = doorway; protocol = storage).

## Current bridges

- `valueflows/` — hREA / VF-GraphQL interop (Wave 3)

## Adding a new bridge

1. Create `bridges/<name>/` with its own Cargo workspace.
2. Expose a single library crate `<name>-bridge` with a `mount` or
   `handle_request` entry point.
3. Document which runtime consumes it.
4. Pull `qahal-authority` (from `elohim/qahal-authority`) if the bridge
   absorbs external writes.

## Reference spec

See `genesis/docs/superpowers/specs/2026-05-20-wave3-valueflows-hrea-interop-design.md`
for the architectural pattern that produced this directory.
