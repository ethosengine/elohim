# Edge build-manifest under-watches elohim-storage's sibling path-dep crates

**Status:** OPEN · **Discovered:** 2026-06-20 (shift babysit-dev-ci-facings, iter 2) · **Domain:** D5 / CI-orchestration

## Concern

`elohim/holochain/build-manifest.json`'s `cargo-build-storage` step watches only
`elohim/elohim-storage/{src,benches,migrations,Cargo.toml,Cargo.lock,Dockerfile}` as
its change-detection source globs. But the storage binary is compiled in Docker from a
build context that ALSO includes its **sibling path-dep crates** — `elohim/elohim-views`,
`elohim/epr`, `elohim/constitution`, `elohim/elohim-cache-core`, `elohim/elohim-compute`,
the `bridges/valueflows/*`, the `sdk/domains/*/types`, and (new) `elohim/elohim-facings`.

A change to **only** a sibling crate (e.g. a fold edit in `elohim-facings`, or a View
change in `elohim-views`) does NOT match the storage globs → the orchestrator does **not**
re-trigger the edge build → the deployed edge node runs stale storage. This is an
**under-build** (principle-7 class). It bit nobody tonight only because the facings landing
also touched `elohim-storage/Cargo.toml` + `Dockerfile` (which ARE watched).

## Fix sketch

Add the sibling path-dep crate source globs to the `cargo-build-storage` step's source list
(at minimum `elohim/elohim-facings/**`, `elohim/elohim-views/**`, `elohim/epr/**`, and the
other path-deps in `elohim-storage/Cargo.toml`). Keep it in sync with the Dockerfile's COPY
set — the two are the same dependency surface and drift apart silently (this gap + tonight's
missing-COPY regression are two faces of the same un-DRY'd list). Consider deriving both from
`elohim-storage/Cargo.toml`'s `path =` deps so a new crate wires itself into both.

## Related
- Tonight's sibling regression: `fix(edge): COPY elohim-facings into the storage Docker build context` (922a11acb).
- Pattern: the Dockerfile COPY set and the build-manifest source globs are the SAME list maintained in two places.
