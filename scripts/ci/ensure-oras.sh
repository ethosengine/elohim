#!/usr/bin/env bash
# ensure-oras.sh — print the path of an oras CLI, installing the pinned
# v1.1.0 release when none is on PATH (the same pin the DNA and edge pipelines
# install inline). Installs into /usr/local/bin when writable (the builder
# container runs as root), otherwise into a fresh temp dir. Callers capture
# the path: ORAS="$(bash scripts/ci/ensure-oras.sh)".
set -euo pipefail

if command -v oras >/dev/null 2>&1; then
    command -v oras
    exit 0
fi

DEST=/usr/local/bin
[ -w "${DEST}" ] || DEST="$(mktemp -d)"
TMP="$(mktemp -d)"
echo "ensure-oras: installing oras v1.1.0 into ${DEST}" >&2
curl -sSL -o "${TMP}/oras.tar.gz" \
    https://github.com/oras-project/oras/releases/download/v1.1.0/oras_1.1.0_linux_amd64.tar.gz
tar -xzf "${TMP}/oras.tar.gz" -C "${TMP}" oras
chmod +x "${TMP}/oras"
mv "${TMP}/oras" "${DEST}/oras"
echo "${DEST}/oras"
