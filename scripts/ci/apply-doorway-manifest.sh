#!/bin/bash
# apply-doorway-manifest.sh — kubectl apply a rendered doorway manifest, tolerating
# ONLY the known jenkins-deployer RBAC drift on `podmonitors` (loudly), and failing
# on anything else.
#
# WHY THIS EXISTS (edge #1306/#1308/#1309, 2026-08-05)
# Wave-2 added an iroh-relay PodMonitor to each doorway manifest. jenkins-deployer
# has no grant on monitoring.coreos.com/podmonitors (backlog: ci-rbac-jenkins-deployer,
# 5th recurrence), so `kubectl apply -f <doorway>-rendered.yaml` now exits non-zero
# even though every core resource (Secret/Deployment/Service/Ingress) applied fine.
# In the Jenkinsfile that non-zero ABORTED deployDoorwayManifest before its
# `rollout restart` + `rollout status --timeout=300s` ever ran — so every edge deploy
# reported "2/2 peers did not reach Ready" while nobody actually waited on, or
# reported, the real rollout. Doorway-A was healthy the whole time; doorway-B was NOT,
# and the poisoned exit is precisely what hid that for hours.
#
# THIS TOLERANCE IS A DATED WORKAROUND (2026-08-05), NOT THE FIX. The fix is the
# jenkins-deployer RBAC grant committed in-tree and applied by the operator —
# genesis/data/timeline/backlog/ci-rbac-jenkins-deployer.md. Until that lands this
# script exists only to keep the drift loud instead of load-bearing.
#
# CONTRACT
#   - Fail-closed by construction: every line kubectl printed must be accounted for
#     as either benign (a per-resource success, an advisory warning) or a line of the
#     known podmonitors-Forbidden block. One unaccounted line fails the apply — which
#     covers kubectl's failure shapes that carry no `Error from server` prefix, such
#     as `error: unable to recognize …` (CRD absent) or `error: error validating …`,
#     whether they occur alone or alongside the tolerated Forbidden.
#   - A tolerated apply is reported with a loud RBAC DRIFT banner and exit 0, so the
#     caller proceeds to the rollout wait — the honest readiness signal.
#   - When the operator lands the grant, this script silently becomes a plain apply
#     (no Forbidden lines → nothing to tolerate). It needs no follow-up removal, but
#     the banner disappearing IS the evidence the grant landed.
#
# Usage: apply-doorway-manifest.sh <rendered-manifest.yaml>
set -uo pipefail

# --- classification ---------------------------------------------------------

# A line that carries no failure signal: blank, a kubectl advisory warning (never a
# cause of a non-zero exit), or a per-resource success line.
is_benign_line() {
    local line="$1"
    local re_warning='^[Ww]arning: '
    local re_success='^[a-z0-9][a-z0-9.-]*/[^[:space:]]+ (created|configured|unchanged|patched|replaced|deleted|pruned|serverside-applied)( \((server )?dry run\))?$'

    [[ -z "${line}" ]] && return 0
    [[ "${line}" =~ ${re_warning} ]] && return 0
    [[ "${line}" =~ ${re_success} ]] && return 0
    return 1
}

# A line of the known podmonitors-Forbidden block, in either of the shapes kubectl
# emits: the four-line "retrieving current configuration" block, or the one-line
# create/patch denial.
is_podmonitor_line() {
    local line="$1" re
    local shapes=(
        '^Error from server \(Forbidden\): error when retrieving current configuration of:$'
        '^Resource: "monitoring\.coreos\.com/[^"]*Resource=podmonitors"'
        '^Name: "[^"]*", Namespace: "[^"]*"$'
        '^from server for: "[^"]*": podmonitors\.monitoring\.coreos\.com "[^"]*" is forbidden: .*resource "podmonitors" in API group "monitoring\.coreos\.com"'
        '^Error from server \(Forbidden\): error when (creating|patching|updating) "[^"]*": podmonitors\.monitoring\.coreos\.com "[^"]*" is forbidden: .*resource "podmonitors" in API group "monitoring\.coreos\.com"'
    )

    for re in "${shapes[@]}"; do
        [[ "${line}" =~ ${re} ]] && return 0
    done
    return 1
}

# Verdict on a failed apply. Reads kubectl's combined output on stdin, prints the
# report, returns 0 ONLY when every failure kubectl reported is the known
# podmonitors-Forbidden. Failure text is grouped into blocks and a block is tolerated
# only if EVERY one of its lines is a known podmonitors shape — so an unforeseen
# message is unaccounted for, and unaccounted is fatal.
apply_verdict() {
    local manifest="$1"
    local re_boundary='^(Error from server|error: )'
    local -a lines=() unaccounted=()
    local tolerable=0
    local line

    while IFS= read -r line; do
        lines+=("${line}")
    done

    local i=0 j n=${#lines[@]}
    while [ "${i}" -lt "${n}" ]; do
        line="${lines[$i]}"
        if is_benign_line "${line}"; then
            i=$((i + 1))
            continue
        fi

        local -a block=("${line}")
        j=$((i + 1))
        while [ "${j}" -lt "${n}" ]; do
            is_benign_line "${lines[$j]}" && break
            [[ "${lines[$j]}" =~ ${re_boundary} ]] && break
            block+=("${lines[$j]}")
            j=$((j + 1))
        done

        local known=1 names_podmonitors=0 b
        for b in "${block[@]}"; do
            is_podmonitor_line "${b}" || known=0
            [[ "${b}" == *podmonitors* ]] && names_podmonitors=1
        done

        if [ "${known}" -eq 1 ] && [ "${names_podmonitors}" -eq 1 ]; then
            tolerable=$((tolerable + 1))
        else
            unaccounted+=("${block[@]}")
        fi
        i="${j}"
    done

    if [ "${#unaccounted[@]}" -eq 0 ] && [ "${tolerable}" -gt 0 ]; then
        echo "=================================================================="
        echo "⚠  RBAC DRIFT (tolerated): jenkins-deployer cannot apply PodMonitors"
        echo "   manifest: ${manifest}"
        echo "   ${tolerable} PodMonitor(s) NOT applied — the workload rolled,"
        echo "   but Prometheus scrape config for it is MISSING."
        echo "   Grant needed: monitoring.coreos.com/podmonitors {get,create,patch}"
        echo "   Backlog: genesis/data/timeline/backlog/ci-rbac-jenkins-deployer.md"
        echo "   Proceeding to rollout wait — core resources applied cleanly."
        echo "=================================================================="
        return 0
    fi

    echo "=================================================================="
    echo "✖  kubectl apply FAILED — NOT tolerating"
    echo "   manifest: ${manifest}"
    echo "   tolerable podmonitors-Forbidden: ${tolerable}"
    if [ "${#unaccounted[@]}" -gt 0 ]; then
        echo "   failures this script will not tolerate:"
        printf '     %s\n' "${unaccounted[@]}"
    else
        echo "   kubectl exited non-zero but reported no failure this script"
        echo "   recognises — treating the unexplained exit as fatal."
    fi
    echo "=================================================================="
    return 1
}

# --- main -------------------------------------------------------------------

MANIFEST="${1:?usage: apply-doorway-manifest.sh <rendered-manifest.yaml>}"

OUT="$(kubectl apply -f "${MANIFEST}" 2>&1)"
RC=$?
echo "${OUT}"

if [ "${RC}" -eq 0 ]; then
    exit 0
fi

if printf '%s\n' "${OUT}" | apply_verdict "${MANIFEST}"; then
    exit 0
fi

exit "${RC}"
