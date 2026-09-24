#!/bin/bash
# scripts/ci/lib/bundle-zip.sh — the ONE way CI turns an app dist into the
# content-addressed archive it publishes. Sourced, never executed.
#
# Extracted from stage-spa-blob.sh (2026-09-24, native-delivery sprint Lane A)
# so every caller that needs "the bytes this build would publish" asks the same
# function: stage-spa-blob.sh (upload), fleet-write-readiness.sh (the bundle
# sha256s a deploy intent records) and publish-app-release.sh (Lane N). Two
# callers that zip the same dist with two recipes would name two different
# CIDs for one build — the intent would point at bytes nobody uploads.
#
# DETERMINISM is the packager's contract, not this file's: the SDK's checked
# archive operation (elohim/sdk/scripts/package-app.mjs) writes a normalised
# zip, re-hashes it after the adapter's checks, and refuses an archive the
# adapter modified. The source dist is never written to, so every call on an
# unchanged dist yields the same bytes and the same sha256.
#
# Requires: bash, coreutils (sha256sum, awk, du), and `node` on PATH for the
# SDK packager — the same interpreter stage-spa-blob.sh has always needed.
#
# Usage:
#   . "<repo>/scripts/ci/lib/bundle-zip.sh"
#   bundle_zip <dist-dir> <kind> <out-dir> || handle-refusal
#   # sets BUNDLE_ZIP_ARCHIVE (<out-dir>/<kind>.zip), BUNDLE_ZIP_HASH
#   # (sha256-<hex>) and BUNDLE_ZIP_SIZE (du -h) on success.
# Returns 0 on success, 2 when the SDK's local package checks refuse the dist
# (the caller names the refusal; nothing was uploaded), 1 on a usage error.

BUNDLE_ZIP_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUNDLE_ZIP_SDK_PACKAGE="${BUNDLE_ZIP_SDK_PACKAGE:-$(cd "${BUNDLE_ZIP_LIB_DIR}/../../.." && pwd)/elohim/sdk/scripts/package-app.mjs}"

# sha256-<hex> of one file, the CID form the doorway's X-Blob-Hash expects.
bundle_zip_hash() {
    printf 'sha256-%s' "$(sha256sum "$1" | awk '{print $1}')"
}

bundle_zip() {
    local dist_dir="$1" kind="$2" out_dir="$3" app_dir=""
    BUNDLE_ZIP_ARCHIVE=""
    BUNDLE_ZIP_HASH=""
    BUNDLE_ZIP_SIZE=""
    if [ -z "${dist_dir}" ] || [ -z "${kind}" ] || [ -z "${out_dir}" ]; then
        echo "bundle_zip: usage: bundle_zip <dist-dir> <kind> <out-dir>" >&2
        return 1
    fi
    app_dir="$(cd "${dist_dir}/../../.." 2>/dev/null && pwd)" || app_dir=""
    if [ -z "${app_dir}" ]; then
        echo "bundle_zip: '${dist_dir}' is not <app>/dist/<name>/<kind> — cannot resolve the app dir" >&2
        return 2
    fi
    local -a package_args=(--adapter "$(dirname "${BUNDLE_ZIP_SDK_PACKAGE}")/package-angular.mjs"
        --dist "${dist_dir}" --kind "${kind}" --out "${out_dir}" --app-dir "${app_dir}")
    # Server/browser artifacts are published separately, but their build
    # context is checked together. The server build must carry the same build
    # stamp as its browser sibling.
    if [ "${kind}" = server ] && [ -f "${dist_dir}/../browser/version.json" ]; then
        package_args+=(--version "${dist_dir}/../browser/version.json")
    fi
    if ! node "${BUNDLE_ZIP_SDK_PACKAGE}" "${package_args[@]}"; then
        return 2
    fi
    BUNDLE_ZIP_ARCHIVE="${out_dir}/${kind}.zip"
    if [ ! -f "${BUNDLE_ZIP_ARCHIVE}" ]; then
        echo "bundle_zip: the packager reported success but wrote no ${BUNDLE_ZIP_ARCHIVE}" >&2
        BUNDLE_ZIP_ARCHIVE=""
        return 2
    fi
    BUNDLE_ZIP_HASH="$(bundle_zip_hash "${BUNDLE_ZIP_ARCHIVE}")"
    BUNDLE_ZIP_SIZE="$(du -h "${BUNDLE_ZIP_ARCHIVE}" | cut -f1)"
    return 0
}
