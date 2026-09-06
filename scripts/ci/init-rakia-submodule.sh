#!/usr/bin/env bash
# init-rakia-submodule.sh — initialize the elohim/rakia submodule using the
# ee-bot-pat credential (rakia is a PRIVATE repo, not anonymously clonable
# from a CI agent — see elohim/holochain/Jenkinsfile Checkout stage comment).
#
# WHY THIS EXISTS
#   edge #1433 (commit 1008ca1e0) added an unauthenticated
#   `git submodule update --init --depth 1 -- elohim/rakia` on the belief
#   that rakia is public. It is not: an anonymous `ls-remote` only succeeded
#   because the devspace that ran it already carried ambient GitHub
#   credentials. On the CI agent the bare command dies with
#   "fatal: could not read Username for 'https://github.com': No such
#   device or address". This script authenticates the submodule fetch with
#   the same `ee-bot-pat` credential the main elohim checkout already uses.
#
# USAGE
#   Called from the Jenkinsfile inside withCredentials([usernamePassword(
#   credentialsId: 'ee-bot-pat', usernameVariable: 'RAKIA_GIT_USER',
#   passwordVariable: 'RAKIA_GIT_TOKEN')]) { sh 'bash ".../init-rakia-submodule.sh"' }
#
# ENV (required, provided by the withCredentials binding above)
#   RAKIA_GIT_USER   ee-bot-pat username
#   RAKIA_GIT_TOKEN  ee-bot-pat token/password
#
# CONTRACT — WARN-ONLY, NEVER BLOCKS THE DEPLOY PATH
#   elohim/rakia only feeds an ADVISORY schema mirror test (the Storage
#   quality gate's non-blocking release_adoption::verify) — it is not on the
#   critical path to a deploy. A submodule fetch failure here must never take
#   the edge deploy down the way build #1433 did. So:
#     success → elohim/rakia populated, schema present; prints
#               "rakia submodule at <sha>"; exit 0.
#     failure (fetch fails OR schema missing afterward) → prints a loud,
#               greppable "RAKIA-UNAVAILABLE" banner to stderr (so the
#               pipeline log still surfaces the problem for a human/agent to
#               fix) and exits 0 anyway — the schema test reads red in the
#               non-blocking gate, but the deploy continues.
#   The token is NEVER echoed: no `set -x`, and the credentialed git
#   invocation is the only line that touches $RAKIA_GIT_TOKEN.
set -euo pipefail
set +x

warn_unavailable() {
    echo "RAKIA-UNAVAILABLE: elohim/rakia could not be fetched — the release-manifest schema mirror test will read as red in the non-blocking Storage gate; deploy continues" >&2
}

: "${RAKIA_GIT_USER:?RAKIA_GIT_USER must be set (ee-bot-pat credential binding)}"
: "${RAKIA_GIT_TOKEN:?RAKIA_GIT_TOKEN must be set (ee-bot-pat credential binding)}"

git config --global --add safe.directory '*'

# Route ONLY the github.com HTTPS remote through the credentialed URL rewrite
# for the duration of this command — `-c` scopes the config to this git
# invocation only (process-local), so the token is never written to
# .git/config, .gitmodules, or any other persistent state.
if ! git -c "url.https://${RAKIA_GIT_USER}:${RAKIA_GIT_TOKEN}@github.com/.insteadOf=https://github.com/" \
    submodule update --init --depth 1 -- elohim/rakia; then
    warn_unavailable
    exit 0
fi

SCHEMA="elohim/rakia/schemas/v1/release-manifest.schema.json"
if [ ! -f "$SCHEMA" ]; then
    warn_unavailable
    exit 0
fi

echo "rakia submodule at $(git -C elohim/rakia rev-parse --short HEAD)"
