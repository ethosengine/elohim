#!/usr/bin/env bash
set -euo pipefail
# Package prebuilt native tools into the SAME compatible runtime userspace.
# Usage: build-compute-worker.sh BASE@sha256:DIGEST EXECUTOR ARK OUTPUT_TAG
compute_base=${1:?immutable compatible runtime image required}
compute_executor=${2:?prebuilt compute-executor required}
compute_ark=${3:?prebuilt ark required}
compute_tag=${4:?output image tag required}
[[ "$compute_base" =~ @sha256:[a-f0-9]{64}$ ]] || { echo 'Base image must be pinned by digest' >&2; exit 2; }
compute_context=$(mktemp -d)
trap 'rm -rf "$compute_context"' EXIT
cp "$compute_executor" "$compute_context/compute-executor"
cp "$compute_ark" "$compute_context/ark"
cp genesis/agentic/compute/{common,worker,workspace,payloads,retention}.mjs "$compute_context/"
cp genesis/agentic/compute/Dockerfile "$compute_context/Dockerfile"
docker build --build-arg "WORKER_BASE_IMAGE=$compute_base" --tag "$compute_tag" "$compute_context"
