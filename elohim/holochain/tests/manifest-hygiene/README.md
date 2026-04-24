# Elohim Manifest Hygiene

Fast schema contract test for the 5 elohim DNA manifests + the elohim.happ
workdir `happ.yaml`. Enforces the conventions laid out in wave-1 execution
plan §7.

## Why a separate crate

- No holochain / tokio / conductor deps. Pure YAML parsing via `serde_yaml`.
- Compiles in a few seconds cold, empirical run time: **0.01s**.
- Lets husky run it on every push that touches a manifest, instead of
  making the developer wait for a full `cargo test -p elohim_sweettest`
  (which has native libdatachannel + conductor dep chain).

## What it checks

```
1.  Every dna.yaml is manifest_version "0" (Holochain 0.6 DnaManifest tag).
2.  dna.yaml `name` field matches the expected DNA name.
3.  integrity.network_seed follows elohim_<dna>_alpha stability contract.
4.  (Removed — `lineage:` field is gated behind HC 0.6's `unstable-migration`
    feature and rejected by the stable `hc dna pack`. Reintroduce once the
    feature stabilizes upstream.)
5.  happ.yaml is manifest_version "0" and declares all 5 expected roles.
6.  Every happ role has `clone_limit: 0` (default-deny).
7.  happ role network_seeds match their dna.yaml counterparts (no drift).
8.  Bootstrap-steward DNAs (lamad, imagodei, mishpat, node_registry)
    declare `progenitor_pubkey` in modifiers.properties.
9.  Infrastructure does NOT declare progenitor_pubkey (federation-native
    per wave-1 §1.2 Q2).
10. No bare "progenitor" vocabulary in dna.yaml files outside the schema
    field name `progenitor_pubkey` (catches surface-language leaks).
```

## Running

```
RUSTFLAGS="" cargo test --manifest-path elohim/holochain/tests/manifest-hygiene/Cargo.toml
```

`RUSTFLAGS=""` is only needed in workspaces that have the Holochain WASM
`--cfg getrandom_backend="custom"` flag sticky in the environment —
doesn't hurt to pass it unconditionally.

## Husky pre-push integration

Registered as project `manifest-hygiene` in
`elohim/holochain/dna/build-manifest.json`. The graph walker emits it
whenever files under these globs change:

- `elohim/holochain/dna/*/dna.yaml`
- `elohim/holochain/dna/elohim/workdir/happ.yaml`
- `elohim/holochain/tests/manifest-hygiene/**`

`.husky/pre-push` then invokes `cargo test` via the `manifest-hygiene`
project entry.

## Adding a new assertion

1. Add a `#[test] fn …() -> Result<()>` to
   `tests/manifest_hygiene.rs`.
2. Reuse `load_dna_manifest()` / `load_happ_manifest()` helpers.
3. If the assertion requires a new cross-file coherence check, keep it
   standalone — don't weave it into existing tests.

## Related

- Plan: `genesis/docs/plans/2026-04-21-rno-lessons-wave-1-execution-plan.md` §7
- Spec: `genesis/docs/superpowers/specs/2026-04-21-bootstrap-steward-authority-frame-design.md`
- Sibling crate (slow, holochain-dep): `elohim/holochain/tests/sweettest/`
