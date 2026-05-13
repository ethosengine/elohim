---
name: Docker include_str! requires repo path mirroring in COPY
description: Rust include_str! resolves relative to source file at compile time; Dockerfiles with WORKDIR /app must COPY referenced sibling/parent dirs to matching paths or builds fail with "couldn't read"
type: feedback
originSessionId: c423684a-b162-42c6-b5cf-177683da9ed0
---
Rust's `include_str!` and `include_bytes!` resolve their string-literal path at compile time, relative to the source file containing the macro. In Docker builds with `WORKDIR /app + COPY elohim/elohim-storage/src ./src`, a source file at `/app/src/services/foo.rs` calling `include_str!("../../../sdk/schemas/...")` will look for `/sdk/schemas/...` (absolute root of the container) — NOT relative to the build context root.

**Why:** Shift `2026-05-03T18-19-orchestrator-805` discovered `elohim/elohim-storage/src/services/bootstrap_manifests.rs:26` had been broken for 2 days because the Dockerfile copied `src/` and `migrations/` but not `elohim/sdk/`. The fix was one line: `COPY elohim/sdk /sdk` placed after the `COPY src` directive. Bug was masked by an upstream pipeline failure that prevented edge from being attempted at all.

**How to apply:** When adding any new `include_str!`/`include_bytes!` reference that goes UP from the source file (`../..`), grep the relevant Dockerfile (`elohim/elohim-storage/Dockerfile`, etc.) for COPY directives that put the referenced files at the path the compiler expects. The path math: count the `..` segments from the source file location IN THE CONTAINER (typically `/app/src/...`), then verify the destination exists in the COPY-merged tree. `#[cfg(test)]` references are exempt (release builds skip them) but production code references will silently break the build with `couldn't read … No such file or directory`.

Existing convention: source/test crates in this repo expect to be co-located with `elohim/sdk/` and `elohim/constitution/` (which the Dockerfile DOES copy as `./constitution`). The asymmetry (constitution copied as `./constitution` to /app/constitution; sdk needed at /sdk) is a Dockerfile structural quirk, not a Rust language thing.
