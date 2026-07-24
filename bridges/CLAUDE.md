# bridges/ — Pluggable Interop Layers

This directory holds bridge crates — Rust libraries that translate external
protocols (web2 federation, VF-GraphQL/hREA, future) to and from elohim's
canonical EPR-REA substrate.

## Seam map — you are here

This surface owns the **Bridge** seam (atlas §3.6 — translate outward, add a
CRATE; integrity by routing translated writes through the notary).

Any "where does this go?" concern routes through the concern-routing atlas:
`genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md`.

Confusion-to-avoid: bridge vs mod/plugin — the discriminator is **direction +
bind-time** (the bridge is the compile-time form of native extension), not
"two different things."

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
- `did/` — W3C DID resolution and `did:key`/Holochain key translation
- `pkarr/` — deterministic signed doorway endpoint records; the
  infrastructure DHT is truth and pkarr publication is a later projection

## Adding a new bridge

1. Create `bridges/<name>/` with its own Cargo workspace.
2. Expose a single library crate `<name>-bridge` with a `mount` or
   `handle_request` entry point.
3. Document which runtime consumes it.
4. Pull `qahal-authority` (from `elohim/qahal-authority`) if the bridge
   absorbs external writes (planned for M2 of Wave 3 — not yet available;
   see Wave 3 spec §2.3).

## Reference spec

See `genesis/docs/content/elohim-protocol/architecture/2026-05-20-wave3-valueflows-hrea-interop-design.md`
for the architectural pattern that produced this directory.
