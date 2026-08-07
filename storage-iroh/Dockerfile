# elohim-storage-iroh — the Wave-2 iroh-transport lane of the storage image.
#
# WHAT THIS IS: a per-commit binary-swap overlay. The elohim-storage SERVICE is
# unchanged from the tx5 lane (same Rust build, same `elohim/elohim-storage/Dockerfile`
# artifact this pipeline just pushed); only the EMBEDDED CONDUCTOR differs.
# alpha runs the consolidated `elohim-node` container, which IS the storage image
# and spawns /usr/local/bin/holochain as an embedded child — so the conductor's
# compiled transport is a property of the storage image, not of a separate
# edgenode container (no alpha human manifest renders EDGENODE_TAG_PLACEHOLDER).
#
# WHY A SECOND IMAGE AND NOT A FEATURE FLAG: kitsune2 selects the transport at
# COMPILE time and tx5 wins whenever it is compiled (kitsune2-0.4.1/src/lib.rs:60-68).
# There is no runtime selector, so tx5 and iroh cannot coexist in one binary
# (design doc 2026-08-05-wave2-relay-sovereignty-design.md §4.1).
#
# RECIPE PROVENANCE: this reproduces the 2026-08-06 che-devworkspaces edgenode-job
# artifact (`Jenkinsfile-elohim-edgenode` stage 'Build & Push Canary Storage',
# irohTransport branch) — the SAME `containers/elohim-storage-zombiefix` two-line
# overlay, the SAME COPY source path (/usr/bin/holochain, usr-merged holo-host base),
# with CONDUCTOR_SOURCE pointed at the isolated elohim-edgenode-iroh line. The ONLY
# deliberate difference is STORAGE_BASE: that job baked the floating
# `elohim-storage:dev-latest`, which is exactly why alpha froze at the 2026-08-06
# storage service (edge #1319 — the honesty-floor gauge could never appear because
# the code carrying it never shipped). Here STORAGE_BASE is the per-commit tag.
#
# ROLLBACK CONTRACT (design doc D9): `elohim-storage-iroh:hc-elohim-0.6.3-iroh` and
# `elohim-edgenode-iroh:hc-elohim-0.6.3-iroh` are named rollback anchors that stay
# live in Harbor for the wave. This build NEVER writes either of them — it only
# READS the edgenode anchor as a base, and it publishes under `:${STORAGE_TAG}`.
#
# FLIP ATOMICITY (design doc D10): a tx5 conductor cannot dial an iroh peer URL and
# vice-versa. Every alpha human must render the SAME storage image or the namespace
# partitions its own DHT. resolveStorageImage() in elohim/holochain/Jenkinsfile is
# env-keyed (not per-human) precisely to make a partial flip unexpressible.

# The per-commit tx5-lane storage image the edge pipeline pushed earlier in this
# same Push stage. Default is the floating dev tag so a manual/local build still
# works; CI always overrides it with :${STORAGE_TAG}.
ARG STORAGE_BASE=harbor.ethosengine.com/ethosengine/elohim-storage:dev-latest

# The iroh-transport conductor. Pinned to the NAMED rollback anchor rather than
# `:latest` (the reproducibility lesson from elohim/elohim-storage/Dockerfile's
# CONDUCTOR_SOURCE_IMAGE note) and rather than a bare digest (Harbor retention GCs
# untagged digests — edge #1261 died that way; a tag survives retention). This
# anchor moves only when the edgenode job is deliberately re-run with
# HC_BRANCH=elohim-0.6.3 + HC_FEATURES=...,transport-iroh,... — i.e. only on a
# deliberate conductor rebuild of the same branch. Override with the
# IROH_CONDUCTOR_SOURCE env var (scripts/ci/push-storage-iroh.sh) to pin a digest
# or move the lane to a new conductor branch without editing this file.
ARG CONDUCTOR_SOURCE=harbor.ethosengine.com/ethosengine/elohim-edgenode-iroh:hc-elohim-0.6.3-iroh

FROM ${CONDUCTOR_SOURCE} AS conductor

FROM ${STORAGE_BASE}
# Embedded-conductor mode spawns /usr/local/bin/holochain
# (elohim/elohim-storage/Dockerfile:413 puts the tx5 conductor there). The edgenode
# image carries its binary at /usr/bin/holochain (usr-merged base). Bare COPY, no
# --chown: COPY ownership is root:root regardless of the base image's trailing
# `USER storage`, identical to how the tx5 conductor lands in the base image.
COPY --from=conductor /usr/bin/holochain /usr/local/bin/holochain
