#!/usr/bin/env bash
# doorway-seed-ensure.sh — self-provision + mint a doorway bearer JWT for CI seeding.
#
# Stage A of the jenkins-seed-bearer-gate plan: the genesis pipeline's substrate
# upload stage (substrate-verify.sh upload → PUT /admin/seed/blob) is bearer-gated
# on a non-dev doorway. This helper ENSURES the `jenkins-ci` seed-actor account
# exists (idempotent register), then logs it in (POST /auth/login) and prints ONLY
# the JWT to stdout — so the Jenkinsfile can capture it into SEED_DOORWAY_TOKEN and
# pass it to substrate-verify.sh via env, keeping secrets out of argv and the
# Jenkinsfile heredoc-free (64KB CPS limit).
#
# SELF-PROVISIONING (no new Jenkins credential, no manual operator curl):
#   The account is provisioned IN-PIPELINE from the EXISTING `doorway-admin-bootstrap-key`
#   credential (= doorway's API_KEY_ADMIN), surfaced here as ADMIN_KEY via withEnv.
#   That same key already promotes the registrant to Admin (auth_routes.rs register
#   handler: adminBootstrapKey == API_KEY_ADMIN ⇒ PermissionLevel::Admin), so a single
#   register call both creates AND grants the seed actor — no separate promotion step.
#
# WHY a DERIVED password (not a second stored secret):
#   The account password is derived deterministically from ADMIN_KEY (sha256). There is
#   NO password to store anywhere: only a holder of ADMIN_KEY can derive it, and ADMIN_KEY
#   is already a total-admin secret, so the seed actor adds ZERO marginal exposure. The
#   register-on-conflict semantics make this safe to re-run: register returns 409 USER_EXISTS
#   on an already-provisioned account (it does NOT reset the password), so the SAME derived
#   password keeps logging in across runs. ROTATION CAVEAT: doorway register never resets a
#   password on conflict, so if ADMIN_KEY is rotated the derived password changes but the
#   stored hash does not — DELETE the jenkins-ci account once after rotating ADMIN_KEY and
#   the next build re-provisions it under the new derivation.
#
# Why a JWT (not an X-API-Key): the bearer carries a login event + actor (identity +
# audit), which is the entire point of Stage A — see the plan's "Why this is the honest
# Stage A".
#
# Toolset: sha256sum / jq / curl ONLY (matches substrate-verify.sh — no openssl), so this
# runs in the same lean CI image. PW/ADMIN_KEY are NEVER echoed to stdout or stderr.
#
# Env:
#   ADMIN_KEY              the admin bootstrap key (= doorway API_KEY_ADMIN). REQUIRED for
#                         the authenticated path. Empty/unset ⇒ print nothing, exit 0
#                         (dev/local: no token, dev-mode doorway accepts).
#   DOORWAY_URL           base doorway URL (scheme + host), e.g. http://<internal-doorway>
#   SEED_ACTOR_IDENTIFIER seed-actor identifier (default jenkins-ci@alpha.elohim.host)
#   CURL_TIMEOUT          per-request timeout seconds (default 15)
#
# Contract:
#   - ADMIN_KEY empty/unset: print NOTHING to stdout, a notice to stderr, exit 0 —
#     the caller treats an empty token as "dev/unauthenticated" so the dev-mode doorway
#     path stays unbroken. (Non-dev alpha then 401s loudly downstream — the intended
#     fail-closed signal that the admin key was not wired.)
#   - register/login failure (ADMIN_KEY present but rejected/unreachable): stderr
#     diagnostic (distinguishing unreachable from HTTP-rejection), empty stdout, exit 1
#     — a hard CI failure.
#   - On success: prints the JWT (no trailing prose) to stdout, exit 0.
set -uo pipefail

CURL_TIMEOUT="${CURL_TIMEOUT:-15}"
IDENTIFIER="${SEED_ACTOR_IDENTIFIER:-jenkins-ci@alpha.elohim.host}"

if [ -z "${ADMIN_KEY:-}" ]; then
  echo "doorway-seed-ensure: ADMIN_KEY unset — emitting empty token (dev/unauthenticated seed path)" >&2
  exit 0
fi

if [ -z "${DOORWAY_URL:-}" ]; then
  echo "doorway-seed-ensure: DOORWAY_URL unset — cannot reach the doorway" >&2
  exit 1
fi

base="${DOORWAY_URL%/}"

# Derive a stable password from ADMIN_KEY (sha256, dependency-free — sha256sum, not
# openssl). Same ADMIN_KEY ⇒ same password every run; no secret stored anywhere. The
# ":jenkins-ci-seed-actor" salt domain-separates this from any other ADMIN_KEY derivation.
PW="$(printf '%s' "${ADMIN_KEY}:jenkins-ci-seed-actor" | sha256sum | cut -c1-48)"

# --- Ensure step: idempotent register (200/201 created, 409 already-provisioned) -----
# JSON body built with `jq -n` (escape-safe; never interpolate secrets into a format
# string). humanId/agentPubKey omitted — doorway-hosted, the doorway creates the identity.
reg_body="$(jq -n \
  --arg id "$IDENTIFIER" \
  --arg pw "$PW" \
  --arg k "$ADMIN_KEY" \
  '{
    identifier: $id,
    identifierType: "email",
    password: $pw,
    displayName: "Jenkins CI (seed actor)",
    bio: "Non-human CI seed actor — Stage A identity+audit for genesis substrate seeding.",
    profileReach: "private",
    adminBootstrapKey: $k
  }')"

reg_resp="$(curl -sS -m "$CURL_TIMEOUT" -w '\n%{http_code}' \
  -X POST "$base/auth/register" \
  -H 'Content-Type: application/json' \
  --data "$reg_body" 2>/dev/null)" || {
  echo "doorway-seed-ensure: POST $base/auth/register failed (network/timeout — doorway unreachable)" >&2
  exit 1
}
reg_status="${reg_resp##*$'\n'}"
reg_payload="${reg_resp%$'\n'*}"

case "$reg_status" in
  200|201)
    echo "doorway-seed-ensure: registered seed actor $IDENTIFIER (Admin via bootstrap key)" >&2
    ;;
  409)
    echo "doorway-seed-ensure: seed actor $IDENTIFIER already provisioned (409 USER_EXISTS) — verifying via login" >&2
    ;;
  *)
    echo "doorway-seed-ensure: register HTTP $reg_status for $IDENTIFIER — provisioning failed (check ADMIN_KEY == alpha doorway API_KEY_ADMIN)" >&2
    echo "doorway said: $(printf '%s' "$reg_payload" | head -c 300)" >&2
    exit 1
    ;;
esac

# --- Login step: mint the JWT ---------------------------------------------------------
login_body="$(jq -n --arg id "$IDENTIFIER" --arg pw "$PW" '{identifier:$id, password:$pw}')"

login_resp="$(curl -sS -m "$CURL_TIMEOUT" -w '\n%{http_code}' \
  -X POST "$base/auth/login" \
  -H 'Content-Type: application/json' \
  --data "$login_body" 2>/dev/null)" || {
  echo "doorway-seed-ensure: POST $base/auth/login failed (network/timeout — doorway unreachable)" >&2
  exit 1
}
login_status="${login_resp##*$'\n'}"
login_payload="${login_resp%$'\n'*}"

if [ "$login_status" != "200" ]; then
  echo "doorway-seed-ensure: login HTTP $login_status as $IDENTIFIER — derived password rejected (rotate ADMIN_KEY? delete the jenkins-ci account once to re-provision) or doorway not Admin" >&2
  echo "doorway said: $(printf '%s' "$login_payload" | head -c 300)" >&2
  exit 1
fi

token="$(printf '%s' "$login_payload" | jq -r '.token // empty')"
if [ -z "$token" ]; then
  echo "doorway-seed-ensure: login succeeded but /auth/login returned no token (unexpected response shape)" >&2
  exit 1
fi

printf '%s' "$token"
