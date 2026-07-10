#!/usr/bin/env bash
# resolve-happ-digest.sh — resolve the immutable content digest of the floating
# elohim-happ tag (dev-latest / latest) from the Harbor registry.
#
# WHY THIS EXISTS
#   HAPP_TAG is a MUTABLE floating tag: a DNA-pipeline rebuild moves dev-latest
#   (or latest) to new hApp bytes WITHOUT an edge commit. The edgenode manifests
#   render that constant string into the `happ-version` ConfigMap, so the
#   StatefulSet pod-template is byte-identical across a tag move → `kubectl apply`
#   reports "unchanged" → the genesis-peer surge guard skips the rollout restart
#   for adam+matthew while non-genesis peers restart and pull the new DNA →
#   DNA-hash divergence on the bootstrap pair → DHT partition.
#   Resolving the tag to its content digest and stamping it into the pod-template
#   spec makes a DNA move surface as "configured", so the restart proceeds.
#
# USAGE
#   resolve-happ-digest.sh <happ-tag>
#
# ENV (optional — the happ repo currently allows anonymous pull)
#   HARBOR_USER / HARBOR_PASS   harbor-robot-registry credentials (reused from
#                               the same credentialsId the hApp fetch path uses).
#
# CONTRACT
#   success → prints the digest (sha256:<64 hex>) to stdout, exit 0.
#   failure → prints nothing to stdout, logs a warning to stderr, exit 1.
#   The Groovy caller MUST treat a non-digest / empty result as FAIL-SAFE: stamp
#   a unique per-build marker so the STS reads "configured" and the restart still
#   happens. NEVER render an empty or stale digest — that silently reopens the
#   skew hole this script closes.

set -uo pipefail

TAG="${1:-}"
REGISTRY="harbor.ethosengine.com"
REPO="ethosengine/elohim-happ"
REF="${REGISTRY}/${REPO}:${TAG}"

if [ -z "${TAG}" ]; then
  echo "resolve-happ-digest: no tag argument given" >&2
  exit 1
fi

# oras is the canonical registry client for the hApp OCI artifact (the
# happ-fetcher init container and the Jenkinsfile fetchHappFromHarbor helper both
# use it). Install it to a per-invocation temp dir if the deploy container lacks
# it — same pinned version as fetchHappFromHarbor, no shared-path write so
# parallel per-human branches can't race.
if ! command -v oras >/dev/null 2>&1; then
  echo "resolve-happ-digest: installing oras CLI (v1.1.0)..." >&2
  ORAS_TMP="$(mktemp -d)"
  if ! curl -sSL -o "${ORAS_TMP}/oras.tar.gz" \
      https://github.com/oras-project/oras/releases/download/v1.1.0/oras_1.1.0_linux_amd64.tar.gz; then
    echo "resolve-happ-digest: failed to download oras CLI" >&2
    exit 1
  fi
  if ! tar -xzf "${ORAS_TMP}/oras.tar.gz" -C "${ORAS_TMP}" oras; then
    echo "resolve-happ-digest: failed to extract oras CLI" >&2
    exit 1
  fi
  chmod +x "${ORAS_TMP}/oras"
  export PATH="${ORAS_TMP}:${PATH}"
fi

# Authenticate when robot creds are present. The happ repo allows anonymous pull
# today, but logging in when we can means a future ACL tightening doesn't silently
# break digest resolution (which would fail-safe to per-deploy restarts). A login
# failure is non-fatal — fall through and try the fetch anonymously.
if [ -n "${HARBOR_USER:-}" ] && [ -n "${HARBOR_PASS:-}" ]; then
  printf '%s' "${HARBOR_PASS}" | oras login "${REGISTRY}" -u "${HARBOR_USER}" --password-stdin >/dev/null 2>&1 \
    || echo "resolve-happ-digest: oras login failed; attempting anonymous fetch" >&2
fi

# Resolve the manifest descriptor and extract its sha256 digest. `--descriptor`
# emits an OCI descriptor JSON: {"mediaType":...,"digest":"sha256:...","size":...}.
# grep the digest so we don't depend on jq being present in the deploy container.
DESCRIPTOR="$(oras manifest fetch --descriptor "${REF}" 2>/dev/null)" || true
DIGEST="$(printf '%s' "${DESCRIPTOR}" | grep -oE 'sha256:[a-f0-9]{64}' | head -n1)"

if [ -z "${DIGEST}" ]; then
  echo "resolve-happ-digest: could not resolve digest for ${REF} (registry unreachable or tag missing)" >&2
  exit 1
fi

printf '%s\n' "${DIGEST}"
exit 0
