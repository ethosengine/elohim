#!/usr/bin/env bash
# happ-roll-key.sh — the content key the DNA pipeline stamps on the hApp OCI
# artifact (`oras push --annotation elohim.host/roll-key=<key>`), read back at
# edge deploy time by scripts/ci/conductor-happ-stamp.sh to decide whether the
# conductors must ROLL for this hApp.
#
# The question a conductor roll answers is "does a running conductor need to
# restart to run what this bundle contains?". Two answers:
#
#   dna-set:sha256:<hex>   sha256 over the normalised packed-DNA hash set
#                          (dna-hashes.actual, one `role=hash` per DNA — the
#                          set the DNA Hash Guard enforces) plus happ.yaml
#                          (role modifiers/properties fold into the INSTALLED
#                          hash but not the packed one, so they are keyed too).
#                          Emitted ONLY when this same build's coordinator
#                          hot-swap reported SUCCESS for this exact bundle: the
#                          coordinator bytes are then already on every peer via
#                          update_coordinators, so an identical DNA set needs
#                          no restart. This is what stops an identical-hash DNA
#                          rebuild — or a coordinator-only change — from
#                          walking a restart across the whole fleet.
#   bundle:sha256:<hex>    the dna-set material plus the .happ bytes. Emitted
#                          whenever the hot-swap did not provably complete (not
#                          run, DEFERRED, INCOMPLETE, stale result file, main
#                          branch): any byte change — coordinators included —
#                          still reaches the conductors the old way, by a roll.
#
# Usage: happ-roll-key.sh <dna-hashes-file> <happ.yaml> <bundle.happ> [coordswap-result-file]
#   coordswap-result-file: written by fleet-coordswap-dispatch.sh when
#   COORDSWAP_RESULT_FILE is set — "<VERDICT> sha256:<bundle-sha>".
# Prints the key; exits 1 with empty stdout if an input is missing/empty (the
# pipeline then pushes WITHOUT a key and the deploy falls back to the manifest
# digest — today's behaviour).
set -uo pipefail

HASHES="${1:?usage: happ-roll-key.sh <dna-hashes-file> <happ.yaml> <bundle.happ> [coordswap-result-file]}"
HAPP_YAML="${2:?usage}"
BUNDLE="${3:?usage}"
RESULT="${4:-}"

die() { echo "happ-roll-key: $*" >&2; exit 1; }
for f in "${HASHES}" "${HAPP_YAML}" "${BUNDLE}"; do
    [ -s "${f}" ] || die "missing or empty input: ${f}"
done

# Normalise: drop comments/blank lines/whitespace, sort by role.
SET="$(grep -vE '^[[:space:]]*(#|$)' "${HASHES}" | sed -e 's/[[:space:]]//g' -e '/^$/d' | LC_ALL=C sort)"
[ -n "${SET}" ] || die "no role=hash lines in ${HASHES}"
printf '%s\n' "${SET}" | grep -qvE '^[a-z0-9_]+=uhC0k[A-Za-z0-9_-]+$' && die "malformed role=hash line in ${HASHES}"

YAML_SHA="$(sha256sum "${HAPP_YAML}" | awk '{print $1}')"
BUNDLE_SHA="$(sha256sum "${BUNDLE}" | awk '{print $1}')"
MATERIAL="roll-key/v1"$'\n'"${SET}"$'\n'"happ.yaml ${YAML_SHA}"$'\n'

VERDICT=""
if [ -n "${RESULT}" ] && [ -s "${RESULT}" ]; then
    read -r VERDICT SWEPT < "${RESULT}" || true
    if [ "${SWEPT:-}" != "sha256:${BUNDLE_SHA}" ]; then
        echo "happ-roll-key: coordswap result is for a different bundle (${SWEPT:-<none>}) — ignoring it" >&2
        VERDICT=""
    fi
fi

if [ "${VERDICT}" = "SUCCESS" ]; then
    echo "happ-roll-key: coordinator hot-swap SUCCESS for this bundle — keying on the DNA set (identical DNAs will not roll conductors)" >&2
    printf 'dna-set:sha256:%s\n' "$(printf '%s' "${MATERIAL}" | sha256sum | awk '{print $1}')"
else
    echo "happ-roll-key: coordinator hot-swap ${VERDICT:-not run} — keying on the bundle bytes (any byte change rolls conductors)" >&2
    printf 'bundle:sha256:%s\n' "$(printf '%s' "${MATERIAL}bundle ${BUNDLE_SHA}"$'\n' | sha256sum | awk '{print $1}')"
fi
