#!/bin/bash
# conductor-workload-pin.sh — echo the conductor fork pin this commit means.
#
# Prints `conductor-<hc12>-<tx512>` on stdout, exactly the tag suffix
# scripts/ci/build-storage-image.sh derives for CONDUCTOR_SOURCE_IMAGE. That
# script is the authority on the derivation; this one exists so the DEPLOY side
# can answer a different question with the same fact:
#
#   "does the conductor StatefulSet already running for this human embed the
#    conductor this commit means, or does it need to roll?"
#
# Rung 2 of the upgrade-velocity debt snowball split the conductor into its own
# k8s workload precisely so a storage-only commit stops restarting conductors.
# That only buys anything if the conductor StatefulSet's pod template is
# byte-stable across storage commits — which means its image tag must NOT track
# the per-commit storage tag. deployHumanConductor keeps the image the live
# StatefulSet already runs, and rolls it only when this pin changes (or when an
# operator asks with [conductor-roll] / CONDUCTOR_ROLL, or when the hApp digest
# annotation moves).
#
# `rev-parse HEAD:<path>` reads the gitlink from the commit object, so it works
# for these `update = none` submodules that CI never clones.
#
# Exits non-zero with an empty stdout if either pointer is unreadable — the
# caller must treat that as "cannot judge, do not roll" rather than guessing.
set -euo pipefail

CONDUCTOR_SHA="$(git rev-parse "HEAD:elohim/holochain-conductor" 2>/dev/null || true)"
TX5_SHA="$(git rev-parse "HEAD:elohim/tx5" 2>/dev/null || true)"

if [ -z "${CONDUCTOR_SHA}" ] || [ -z "${TX5_SHA}" ]; then
    echo "conductor-workload-pin: cannot read fork submodule pointers (holochain-conductor='${CONDUCTOR_SHA:-<unreadable>}' tx5='${TX5_SHA:-<unreadable>}')" >&2
    exit 1
fi

echo "conductor-$(echo "${CONDUCTOR_SHA}" | cut -c1-12)-$(echo "${TX5_SHA}" | cut -c1-12)"
