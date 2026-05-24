#!/usr/bin/env bash
# W3 — Per-DNA nextest binary filter harvester.
#
# Reads the orchestrator-supplied CHANGED_PATHS (newline-delimited) and the
# @dna-scope markers in src/tests/*.rs, and emits a nextest filter
# expression on stdout. The expression COMPOSES with the existing
# quarantine ALWAYS — never replaces it.
#
# Inputs (env):
#   CHANGED_PATHS — newline-delimited changed source paths; empty for
#                   manual / [build:X] tag / force dispatches → full
#                   suite fallback.
#   TESTS_DIR     — path to src/tests/ (default: relative to this script).
#
# Output:
#   stdout: a single nextest -E expression value (no quoting).
#   stderr: human-readable reasoning ("[harvester] ...") lines.
#
# Behavior:
#   - Empty CHANGED_PATHS                              → full suite
#   - Any changed path outside elohim/holochain/dna/<dna>/** → full suite
#   - Otherwise compose (binary(b1) | binary(b2) | ...) of every binary
#     whose @dna-scope marker overlaps the changed DNAs. Tests WITHOUT a
#     marker are ALWAYS included (safety default).
#   - The final expression appends `and not (existing quarantine)`.
#
# Quarantined tests (must stay excluded under every dispatch):
#   epr_2b_batch_a_full_loop, create_and_list_succeeds,
#   refresh_ttl_appends_timestamp, cross_agent_get_returns_none
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TESTS_DIR="${TESTS_DIR:-${SCRIPT_DIR}/../src/tests}"
CHANGED_PATHS_VAL="${CHANGED_PATHS:-}"

QUARANTINE='not (test(=epr_2b_batch_a_full_loop) | test(=create_and_list_succeeds) | test(=refresh_ttl_appends_timestamp) | test(=cross_agent_get_returns_none))'

emit_full_suite() {
    local reason="$1"
    echo "[harvester] $reason → full suite" >&2
    echo "$QUARANTINE"
}

if [ -z "$CHANGED_PATHS_VAL" ]; then
    emit_full_suite "CHANGED_PATHS empty (manual / tag / force dispatch)"
    exit 0
fi

# Any changed path outside elohim/holochain/dna/<dna>/ triggers full-suite
# fallback. This covers shared scaffolding: elohim-cache-core, holochain/rna,
# VERSION, the sweettest harness itself, root Cargo.*, the orchestrator
# Jenkinsfile, etc. Those changes can affect any test binary, so we run all.
if echo "$CHANGED_PATHS_VAL" | grep -qvE '^elohim/holochain/dna/[^/]+/'; then
    emit_full_suite "changeset includes paths outside dna/<dna>/**"
    exit 0
fi

# Extract the set of changed DNAs (one per line, deduplicated).
CHANGED_DNAS=$(echo "$CHANGED_PATHS_VAL" \
    | awk -F/ '/^elohim\/holochain\/dna\/[^/]+\// {print $4}' \
    | sort -u)

if [ -z "$CHANGED_DNAS" ]; then
    emit_full_suite "no DNA paths matched in changeset"
    exit 0
fi

echo "[harvester] changed DNAs: $(echo $CHANGED_DNAS | tr '\n' ' ')" >&2

# Walk every test file, decide include/exclude based on marker overlap.
BINARIES=""
for f in "$TESTS_DIR"/*.rs; do
    BIN=$(basename "$f" .rs)
    # Match marker on first line (sed insertion pattern from W3 prep).
    MARKER=$(awk '/^\/\/! @dna-scope:/ {sub(/^\/\/! @dna-scope: */, ""); print; exit}' "$f")

    INCLUDE=0
    REASON=""
    if [ -z "$MARKER" ]; then
        INCLUDE=1
        REASON="no marker (safety default)"
    else
        # Marker is comma-separated DNAs (possibly with spaces). Tokenise.
        # If any token overlaps CHANGED_DNAS, include.
        MARKER_DNAS=$(echo "$MARKER" | tr ',' '\n' | tr -d ' ')
        for dna in $CHANGED_DNAS; do
            if echo "$MARKER_DNAS" | grep -qFx "$dna"; then
                INCLUDE=1
                REASON="@dna-scope: $MARKER overlaps $dna"
                break
            fi
        done
        [ "$INCLUDE" = "0" ] && REASON="@dna-scope: $MARKER (no overlap with changed DNAs)"
    fi

    if [ "$INCLUDE" = "1" ]; then
        echo "[harvester]   include $BIN ($REASON)" >&2
        BINARIES="$BINARIES $BIN"
    else
        echo "[harvester]   skip    $BIN ($REASON)" >&2
    fi
done

# Trim, then compose the OR'd filter.
BINARIES_TRIMMED=$(echo "$BINARIES" | xargs || true)

if [ -z "$BINARIES_TRIMMED" ]; then
    # No binaries matched (CHANGED_DNAS hit no marker AND no unmarked tests).
    # Conservative: full suite. This shouldn't happen if every file is
    # marked, but is the safe fallback.
    emit_full_suite "no binaries matched (empty intersection; conservative fallback)"
    exit 0
fi

# Compose: each "binary(X)" joined by " | ".
FILTER_BODY=$(echo "$BINARIES_TRIMMED" | tr ' ' '\n' | sed 's/^/binary(/; s/$/)/' | paste -sd '|' - | sed 's/|/ | /g')

echo "[harvester] filter: ($FILTER_BODY) and (above quarantine)" >&2
echo "($FILTER_BODY) and $QUARANTINE"
