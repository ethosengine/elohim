#!/bin/bash
# a2o-cucumber-sites-build-storage-client.test.sh
#
# THE INVARIANT: every CI entrypoint that runs the a2o cucumber suite must build
# @elohim/storage-client first — or name the earlier stage that did.
#
# WHY IT IS A TEST AND NOT A COMMENT: the a2o support tree is loaded wholesale
# (`require: ['steps/**/*.ts']` in genesis/a2o/cucumber.mjs, inherited by every
# profile), and one of those files imports @elohim/storage-client, whose entry
# point is a BUILT dist/index.js. `pnpm install --filter '@elohim/a2o...'` links
# the package but never builds it. So a site that installs and runs cucumber
# without the build dies at SUPPORT LOAD with MODULE_NOT_FOUND — and because no
# scenario ever starts, the failure arrives as "0 scenarios", which every
# downstream tally reads as a MEASUREMENT rather than a load failure.
#
# It has now been missed once per site, never shared:
#   - genesis/scripts/ci/install-substrate-runner.sh  — built it (genesis)
#   - scripts/ci/run-dataplane-validation.sh          — MISSED; edge #1464
#   - scripts/ci/run-mesh-quiesce-stage.sh            — MISSED; same class
# Three sites, one required step. This test is the shared thing.
#
# Deploy-container discipline (scripts/ci/.epr-meta): bash + coreutils only.

set -euo pipefail

REPO_ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"

# Sites that legitimately do NOT build it themselves because a stage EARLIER in
# the same pipeline (same Jenkins workspace) already did. Each entry must name
# that stage — an allowlist without a reason is just a hole.
#   genesis/scripts/ci/e2e-verify-api.sh
#   genesis/scripts/ci/e2e-verify-browser.sh
#     -> genesis/Jenkinsfile runs genesis/scripts/ci/install-substrate-runner.sh
#        (which installs + builds the SDK) in an earlier stage of the same run.
ALLOWLIST="genesis/scripts/ci/e2e-verify-api.sh
genesis/scripts/ci/e2e-verify-browser.sh"

failures=0

check_site() {
    local rel="$1" abs="${REPO_ROOT}/$1"

    # Comment lines mention cucumber constantly (profile discipline notes,
    # RCA headers). Only an executable line counts as a site.
    if ! grep -v '^[[:space:]]*#' "${abs}" | grep -q 'cucumber-js'; then
        return 0
    fi

    if grep -v '^[[:space:]]*#' "${abs}" \
        | grep -Eq -- "--filter[[:space:]]+'?\"?@elohim/storage-client'?\"?[[:space:]]+build"; then
        echo "  ok       ${rel} (builds @elohim/storage-client)"
        return 0
    fi

    if printf '%s\n' "${ALLOWLIST}" | grep -Fxq "${rel}"; then
        echo "  ok       ${rel} (allowlisted — an earlier stage in the same pipeline builds it)"
        return 0
    fi

    echo "  FAIL     ${rel} runs cucumber-js but never builds @elohim/storage-client" >&2
    failures=$((failures + 1))
}

echo "a2o cucumber sites — storage-client build invariant"

for f in "${REPO_ROOT}"/scripts/ci/*.sh "${REPO_ROOT}"/genesis/scripts/ci/*.sh; do
    [ -f "${f}" ] || continue
    case "${f}" in
        *.test.sh) continue ;;
    esac
    check_site "${f#"${REPO_ROOT}"/}"
done

if [ "${failures}" -ne 0 ]; then
    echo "" >&2
    echo "${failures} site(s) run the a2o suite against an unbuilt @elohim/storage-client dist." >&2
    echo "Add this before the cucumber invocation (see scripts/ci/run-dataplane-validation.sh):" >&2
    echo "    pnpm --filter @elohim/storage-client build" >&2
    echo "Or, if an earlier stage of the SAME pipeline run already built it, add the" >&2
    echo "script to ALLOWLIST in this test WITH the stage that does it." >&2
    exit 1
fi

echo "a2o-cucumber-sites-build-storage-client: invariant holds"
