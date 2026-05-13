---
name: Quilt is the elohim-native S3 surface — don't reach for AWS S3 for build cache
description: Build-side caching needs (sccache, blob backends, etc.) should target the EPR quilt surface as the dogfooding path; defer until iroh substrate matures
type: project
originSessionId: 91882765-aece-476c-a49a-85b618774d32
---
When designing build-cache or blob-cache infrastructure (e.g. sccache backend, build artifact stores), the default elohim-native answer is the EPR **quilt** surface — the S3-shaped abstraction over EPR content addressing. It's part of the resiliency social compute epic.

**Why:**
- Reaching for AWS S3 (or any external vendor) couples the protocol's CI to a centralized substrate the protocol exists to subsume. That's the wrong dependency direction.
- The quilt is exactly an S3-shaped surface for content-addressed blobs — sccache wants `bucket/key→blob`; quilt provides `cid→blob` with the same affordances.
- Dogfooding the quilt for our own dev tooling tightens the feedback loop on the substrate that real users will rely on for content distribution.
- Premature S3 commitment would have to be unwound when quilt graduates.

**How to apply:**
- Don't propose AWS S3 / GCS / R2 as a build-cache backend in iterations. Park those proposals.
- Wait for the iroh parallel-stack work (see `project_iroh_parallel_stack_phases3_7_landed.md` and `project_iroh_phase11_sync_first_plane_landed.md`) to land its remaining backend wirings before designing quilt's CI integration.
- When the substrate is ready, schedule a brit-rakia brainstorm to revisit quilt's S3 surface API and nominate the first dogfood target (likely sccache).
- This memory governs items like the deferred sccache layer for elohim-holochain DNA Integration — that work parks behind quilt readiness, not behind external S3.
- Distinguish quilt from weave (Moss collision) and lattice (governance) per the storage vocabulary memory; quilt is the storage-distribution name, S3-shape is the API surface.
