# eprfs — Native Filesystem Projection Layer

This workspace owns the native filesystem projection seam: collapsing distributed EPR-governed data into ordinary local files without making the filesystem the source of truth.

## Boundary

- Do not put external protocol translation here. That belongs in `bridges/`.
- Do not embed git semantics in `eprfs-core`. Git/repository meaning belongs in brit or a brit-facing adapter.
- Do not make `elohim-storage` understand working trees. Storage remains the data plane.

## Dependency Direction

`domain adapter -> eprfs -> elohim-storage`.

The core crate must stay small, pure, and runtime-agnostic. Storage-specific and filesystem-specific behavior belongs in sibling crates.
