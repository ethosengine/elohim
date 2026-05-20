# bridges/valueflows — hREA / VF-GraphQL Bridge

The bridge of VF/hREA → elohim EPR-REA. Sibling architectural pattern to
doorway's web2 → elohim P2P bridge.

## Consumed by

`elohim-storage` mounts this bridge at `/api/v1/vf-graphql`.

## Crates

- `valueflows-types` — stable type definitions (TranslationPoint, etc.)
- `valueflows-bridge` — library; GraphQL schema + handler
- `valueflows-tests` — integration tests against a mounted endpoint

## Build

```bash
cd bridges/valueflows
cargo check --all
cargo test --all
```

## Reference spec

`genesis/docs/superpowers/specs/2026-05-20-wave3-valueflows-hrea-interop-design.md`
