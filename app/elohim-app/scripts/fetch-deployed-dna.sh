#!/bin/bash
#
# fetch-deployed-dna.sh - Fetch the DEPLOYED elohim.happ bundle (DNA-hash parity rail)
#
# NETWORK_PROFILE=join-alpha makes the local conductor join the alpha DHT — but
# installing a locally-built hApp with different DNA hashes lands the peer on a
# PARTITIONED DHT (same network endpoints, different DHTs). This script fetches
# the bundle alpha actually runs, so join-alpha installs exactly what alpha runs.
#
# This is a FETCHER only — it never refuses anything. The join-alpha refusal
# (no deployed bundle, no FORCE_LOCAL_HAPP) lives in hc-start.sh.
#
# USAGE:
#   ./fetch-deployed-dna.sh [--force]
#
# OPTIONS:
#   --force          Re-download even when the cached copy matches the remote
#
# ENVIRONMENT VARIABLES:
#   DEPLOYED_HAPP_TAG     Harbor floating tag (default: dev-latest — alpha
#                         deploys from dev; main publishes 'latest')
#   DEPLOYED_HAPP_BRANCH  Jenkins branch for the artifact fallback (default: dev)
#   HARBOR_USER/HARBOR_PASS  Optional Harbor login (anonymous pull is allowed;
#                         credentials only needed if the project goes private)
#
# SOURCES (first one that works wins):
#   1. Harbor via oras CLI:  harbor.ethosengine.com/ethosengine/elohim-happ:{tag}
#      (canonical deployed source — the edge deploy consumes this artifact)
#   2. Jenkins artifact:     jenkins.ethosengine.com/job/elohim-holochain/job/{branch}/
#                            lastSuccessfulBuild/artifact/elohim.happ
#      (same pipeline archives the bundle it pushes to Harbor; anonymous read)
#
# OUTPUT:
#   elohim/holochain/local-dev/deployed-bundles/elohim.happ   (gitignored)
#   elohim/holochain/local-dev/deployed-bundles/.unpack/*.dna (unpacked, for inspection)
#
# Prints the per-DNA hashes (hc dna hash) after fetch so parity with a running
# conductor can be cross-checked manually.
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_DIR="$(dirname "$SCRIPT_DIR")"
HC_DIR="$APP_DIR/../../elohim/holochain"
DEST_DIR="$HC_DIR/local-dev/deployed-bundles"
DEST="$DEST_DIR/elohim.happ"
META="$DEST_DIR/elohim.happ.src"   # sidecar: which remote state the cached copy came from

: "${DEPLOYED_HAPP_TAG:=dev-latest}"
: "${DEPLOYED_HAPP_BRANCH:=dev}"

HARBOR_HOST="harbor.ethosengine.com"
HARBOR_REF="$HARBOR_HOST/ethosengine/elohim-happ:$DEPLOYED_HAPP_TAG"
JENKINS_ARTIFACT_URL="https://jenkins.ethosengine.com/job/elohim-holochain/job/$DEPLOYED_HAPP_BRANCH/lastSuccessfulBuild/artifact/elohim.happ"

FORCE=false
for arg in "$@"; do
    case "$arg" in
        --force) FORCE=true ;;
        -h|--help)
            sed -n '/^#/!q;s/^# \?//p' "$0" | tail -n +3
            exit 0
            ;;
        *)
            echo "Unknown option: $arg (supported: --force)"
            exit 1
            ;;
    esac
done

mkdir -p "$DEST_DIR"

# Trap: clean up temp files on interrupt or exit
trap 'rm -rf "$DEST_DIR"/.dl.* "$DEST_DIR"/.pull.* 2>/dev/null || true' EXIT

# ----------------------------------------------------------------------------
# Helpers
# ----------------------------------------------------------------------------

cached_matches() {
    # $1 = remote identity string; true iff cache exists and came from this state
    [ -f "$DEST" ] && [ -f "$META" ] && [ "$(cat "$META")" = "$1" ]
}

print_dna_hashes() {
    if ! command -v hc >/dev/null 2>&1; then
        echo "   ⚠️  hc CLI not found — skipping DNA hash printout"
        return 0
    fi
    local unpack_dir="$DEST_DIR/.unpack"
    rm -rf "$unpack_dir"
    if ! hc app unpack "$DEST" -o "$unpack_dir" >/dev/null; then
        echo "   ⚠️  hc app unpack failed — cannot print DNA hashes"
        return 0
    fi
    echo ""
    echo "   Deployed DNA hashes ($(basename "$DEST"), source: $1):"
    local dna
    for dna in "$unpack_dir"/*.dna; do
        [ -e "$dna" ] || continue
        printf "      %-22s %s\n" "$(basename "$dna")" "$(hc dna hash "$dna")"
    done
    echo ""
}

# ----------------------------------------------------------------------------
# Source 1: Harbor via oras (canonical deployed artifact)
# ----------------------------------------------------------------------------

fetch_via_harbor() {
    command -v oras >/dev/null 2>&1 || return 1
    curl -sI --max-time 5 "https://$HARBOR_HOST/v2/" >/dev/null 2>&1 || return 1

    if [ -n "$HARBOR_USER" ] && [ -n "$HARBOR_PASS" ]; then
        printf '%s' "$HARBOR_PASS" | oras login "$HARBOR_HOST" -u "$HARBOR_USER" --password-stdin >/dev/null 2>&1 || echo "⚠️  Harbor login failed — continuing with anonymous pull" >&2
    fi

    # Idempotence: compare the manifest digest with the cached copy's provenance.
    local digest remote_id
    digest=$(oras manifest fetch --descriptor "$HARBOR_REF" 2>/dev/null \
        | tr -d ' \n' | sed -n 's/.*"digest":"\(sha256:[a-f0-9]*\)".*/\1/p')
    remote_id="harbor $HARBOR_REF $digest"
    if [ "$FORCE" = false ] && [ -n "$digest" ] && cached_matches "$remote_id"; then
        echo "   ✅ Cached deployed bundle is current (Harbor digest unchanged): $DEST"
        echo "      (re-download with --force)"
        print_dna_hashes "harbor:$DEPLOYED_HAPP_TAG (cached)"
        return 0
    fi

    echo "   ⬇️  Pulling $HARBOR_REF via oras..."
    local tmp_dir
    tmp_dir=$(mktemp -d "$DEST_DIR/.pull.XXXXXX")
    if ! (cd "$tmp_dir" && oras pull "$HARBOR_REF" >/dev/null); then
        rm -rf "$tmp_dir"
        return 1
    fi
    if [ ! -f "$tmp_dir/elohim.happ" ]; then
        echo "   ⚠️  oras pull succeeded but no elohim.happ in the artifact"
        rm -rf "$tmp_dir"
        return 1
    fi
    mv "$tmp_dir/elohim.happ" "$DEST"
    rm -rf "$tmp_dir"
    echo "$remote_id" > "$META"
    echo "   ✅ Fetched from Harbor ($DEPLOYED_HAPP_TAG): $DEST ($(du -h "$DEST" | cut -f1))"
    print_dna_hashes "harbor:$DEPLOYED_HAPP_TAG"
    return 0
}

# ----------------------------------------------------------------------------
# Source 2: Jenkins artifact (anonymous read; same pipeline that feeds Harbor)
# ----------------------------------------------------------------------------

fetch_via_jenkins() {
    # Idempotence: Last-Modified + Content-Length from a HEAD request stand in
    # for a digest (Jenkins serves no ETag here).
    local head_out last_modified content_length remote_id curl_exit
    head_out=$(curl -fsI --max-time 15 "$JENKINS_ARTIFACT_URL" 2>/dev/null)
    curl_exit=$?
    if [ $curl_exit -ne 0 ]; then
        if [ -f "$DEST" ]; then
            if [ $curl_exit -eq 22 ]; then
                echo "   ⚠️  Jenkins returned an HTTP error (artifact missing?) — keeping existing cached bundle: $DEST"
            else
                echo "   ⚠️  Jenkins unreachable — keeping existing cached bundle: $DEST"
            fi
            echo "      ($(cat "$META" 2>/dev/null || echo 'provenance unknown'))"
            print_dna_hashes "cache (remote unreachable)"
            return 0
        fi
        return 1
    fi
    last_modified=$(printf '%s' "$head_out" | tr -d '\r' | sed -n 's/^[Ll]ast-[Mm]odified: //p')
    content_length=$(printf '%s' "$head_out" | tr -d '\r' | sed -n 's/^[Cc]ontent-[Ll]ength: //p')
    remote_id="jenkins $DEPLOYED_HAPP_BRANCH lastSuccessfulBuild $last_modified $content_length"

    if [ "$FORCE" = false ] && cached_matches "$remote_id"; then
        echo "   ✅ Cached deployed bundle is current (Jenkins artifact unchanged): $DEST"
        echo "      (re-download with --force)"
        print_dna_hashes "jenkins:$DEPLOYED_HAPP_BRANCH (cached)"
        return 0
    fi

    echo "   ⬇️  Downloading $JENKINS_ARTIFACT_URL..."
    local tmp_file
    tmp_file=$(mktemp "$DEST_DIR/.dl.XXXXXX")
    if ! curl -fsSL --max-time 300 -o "$tmp_file" "$JENKINS_ARTIFACT_URL"; then
        rm -f "$tmp_file"
        return 1
    fi
    mv "$tmp_file" "$DEST"
    echo "$remote_id" > "$META"
    echo "   ✅ Fetched from Jenkins ($DEPLOYED_HAPP_BRANCH/lastSuccessfulBuild): $DEST ($(du -h "$DEST" | cut -f1))"
    print_dna_hashes "jenkins:$DEPLOYED_HAPP_BRANCH"
    return 0
}

# ----------------------------------------------------------------------------
# Main: Harbor preferred (canonical), Jenkins fallback (anonymous)
# ----------------------------------------------------------------------------

echo "📡 Fetching deployed elohim.happ (DNA-hash parity with alpha)..."

if fetch_via_harbor; then
    exit 0
fi
if command -v oras >/dev/null 2>&1; then
    echo "   ⚠️  Harbor pull failed — falling back to Jenkins artifact"
else
    echo "   ℹ️  oras CLI not available — using Jenkins artifact fallback"
fi
if fetch_via_jenkins; then
    exit 0
fi

echo "   ❌ Could not fetch the deployed bundle from Harbor or Jenkins."
echo "      Tried: $HARBOR_REF (oras) and $JENKINS_ARTIFACT_URL"
exit 1
