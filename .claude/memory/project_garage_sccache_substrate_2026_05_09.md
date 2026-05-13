---
name: S3-shape sccache substrate live (2026-05-09) — actually MinIO, not Garage
description: sccache-elohim bucket live on MinIO (single replica, openebs-jiva PVC on hp-micro10); earlier sessions logged this as "Garage" — that name leaked into devfile/flake comments but the actual cluster deployment is MinIO. Endpoint abstraction is the integration point for future quilt cutover.
type: project
originSessionId: 91882765-aece-476c-a49a-85b618774d32
correctedInSession: 2026-05-11 brainstorm (tiered-quilt-stewardship spec drafting)
---

**State as of 2026-05-09, corrected 2026-05-11:**

S3-shape cache substrate is live and healthy. **Implementation is MinIO, not
Garage** — earlier session memory wrote "Garage" because that was the working
plan name, but the actual cluster deployment per
`genesis/manifests/RUNBOOK-minio-sccache-2026-05-09.md` is MinIO. The
`devfile.yaml` and `elohim/holochain/dna/elohim/flake.nix` comments still say
"Garage"; that is stale documentation drift, scheduled for cleanup.

**Substrate facts (verified 2026-05-11):**
- Bucket: `sccache-elohim`
- Endpoint: `http://minio.ethosengine.svc.cluster.local:9000`
- Namespace: `ethosengine`
- Mode: Single replica, openebs-jiva-csi-default StorageClass, 200Gi PVC, backing on hp-micro10
- Region: `garage` (legacy env value; harmless — sccache's S3 v4 client doesn't care)
- Secret: `sccache-credentials` (auto-mounted in Che user namespaces and `jenkins` ns)
  - Keys: `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `SCCACHE_BUCKET`,
    `SCCACHE_ENDPOINT`, `SCCACHE_S3_USE_SSL=false`, `SCCACHE_REGION`
- Versioning: disabled (content-addressed by cargo)
- Lifecycle/eviction: **none configured** (bucket fills indefinitely until manual TTL added — decision pending)

**Why:** Replaces the broken PVC-cache attempts (`/nix/store` mount in build
#1211, `CARGO_TARGET_DIR=/cargo-target` in build #1212). Endpoint abstraction
(single env var `SCCACHE_ENDPOINT`) is bulletproof — swapping to quilt-S3-shim
is a single env-var repoint per namespace. This is the **tiered-quilt Wave 3
dogfood substrate** per
`genesis/docs/superpowers/specs/2026-05-11-tiered-quilt-stewardship-design.md`.

**Followup status (most landed):**
- `elohim/devfile.yaml` — `RUSTC_WRAPPER=sccache` env set ✅
- `elohim/holochain/dna/Jenkinsfile` — envFrom + RUSTC_WRAPPER wiring ✅
- `flake.nix` probe for `/usr/local/bin/sccache` with graceful fallback ✅

**How to apply:**
- When referencing the substrate in new design work, **say MinIO**, not Garage.
  Devfile/flake comments are scheduled for cleanup but not high-priority.
- The bucket-fill problem is decision-#5 in the tiered-quilt delivery master
  (`genesis/docs/superpowers/plans/2026-05-11-tiered-quilt-delivery-master.md`):
  operator owns the lifecycle policy.
- Future quilt cutover: `SCCACHE_ENDPOINT` points at `quilt-s3-shim`
  (tiered-quilt Wave 3 deliverable); same bucket name, transparent to sccache.
- Don't re-introduce `/nix/store` PVC mount or `CARGO_TARGET_DIR=/cargo-target` —
  both were path-incoherent with `hc dna pack` and image-bundled binaries.
- PVCs `nix-cache-holochain`, `cargo-cache-holochain`,
  `sweettest-target-cache-holochain` still exist in cluster for sweettest
  cold-compile cases; leave them alone.
