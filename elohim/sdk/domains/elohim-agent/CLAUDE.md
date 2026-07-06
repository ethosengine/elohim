# Elohim Agent Domain

This domain declares the canonical agentic capability vocabulary for Elohim SDK
manifests. V1 is schema and manifest only.

## Source Of Truth

The canonical source is the EPR/app-manifest vocabulary in this directory:

- `manifest.json`
- `manifest/content-types/*.json`
- `schemas/*-metadata.schema.json`

Generic EPR `Manifest` atoms can carry these app-manifest artifact shapes.
Generic EPR `Agent` atoms can later point at or embody the executable agent
contracts declared here. This slice does not add new core EPR kinds.

## Projection Boundary

Runtime surfaces are future projections, not canonical sources:

- `.claude/*`
- `CLAUDE.md`
- `AGENTS.md`
- `.agents/*`
- `.codex/*`
- hook files
- plugin manifests and plugin folders

Those files may be generated from `projection-binding` artifacts in a later
slice, and they may be scanned to report projection drift, but they do not own
the vocabulary.

## V1 Non-Goals

- No changes to `elohim/epr/src/kind.rs`.
- No changes to `epr-kind.schema.json`.
- No changes to `manifest-epr.schema.json`.
- No `eprfs` materialization or Claude/Codex semantics.
- No Claude or Codex projection compiler.
- No runtime agent services.

The purpose of V1 is to make agentic capabilities legible as app-manifest
content types with shallow metadata validation.
