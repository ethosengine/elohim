#!/usr/bin/env bash
# harbor-oracle.sh <project/repo> <sha256:closure-key>
# stdout: hit <digest> | miss <reason>. Exit 0 for either decision (including
# inability to judge); exit 2 for invalid invocation. No writes to the registry.
# HEAD resolves the closure tag; GET is pinned to that digest, never the tag.
# HARBOR_USER/HARBOR_PASS: optional robot credentials for the token exchange.
set +x
set -uo pipefail
umask 077
miss() { printf 'miss %s\n' "$*"; exit 0; }
[ "$#" -eq 2 ] || exit 2
repo="$1" key="$2"
[[ "$repo" =~ ^[a-z0-9._/-]+$ && "$key" =~ ^sha256:[a-f0-9]{64}$ ]] || exit 2
prefix=https://harbor.ethosengine.com/v2
accept='application/vnd.oci.image.index.v1+json, application/vnd.oci.image.manifest.v1+json, application/vnd.docker.distribution.manifest.list.v2+json, application/vnd.docker.distribution.manifest.v2+json'
tmp="$(mktemp -d)" || miss temporary-storage
trap 'rm -rf "$tmp"' EXIT
auth=()
hex="${key#sha256:}"
url="${prefix}/${repo}/manifests/closure-${hex:0:16}"
head_manifest() {
    curl -sS --connect-timeout 5 --max-time 15 -I -D "$tmp/headers" -o /dev/null \
        -w '%{http_code}' -H "Accept: $accept" "${auth[@]}" "$url" 2>/dev/null
}
code="$(head_manifest)" || miss registry-unreachable
if [ "$code" = 401 ]; then
    # curl config values are escaped, and credentials never become argv.
    config_value() {
        local value="$1"
        value="${value//\\/\\\\}"
        value="${value//\"/\\\"}"
        value="${value//$'\n'/\\n}"
        value="${value//$'\r'/\\r}"
        printf '%s' "$value"
    }
    : > "$tmp/credentials"
    creds=(--config "$tmp/credentials")
    if [ -n "${HARBOR_USER:-}" ] && [ -n "${HARBOR_PASS:-}" ]; then
        printf 'user = "%s"\n' "$(config_value "${HARBOR_USER}:${HARBOR_PASS}")" > "$tmp/credentials"
    fi
    curl -fsS --connect-timeout 5 --max-time 15 "${creds[@]}" \
        "https://harbor.ethosengine.com/service/token?service=harbor-registry&scope=repository:${repo}:pull" \
        -o "$tmp/token" 2>/dev/null || miss registry-auth
    token="$(node -e 'const j=JSON.parse(require("fs").readFileSync(process.argv[1])); if(typeof j.token!=="string" || !j.token) process.exit(1); process.stdout.write(j.token)' "$tmp/token" 2>/dev/null)" || miss registry-auth
    printf 'header = "%s"\n' "$(config_value "Authorization: Bearer $token")" > "$tmp/auth"
    auth=(--config "$tmp/auth")
    code="$(head_manifest)" || miss registry-unreachable
fi
[ "$code" = 200 ] || miss "http-$code"
digest="$(tr -d '\r' < "$tmp/headers" | awk 'tolower($1)=="docker-content-digest:" {print $2}')"
[[ "$digest" =~ ^sha256:[a-f0-9]{64}$ ]] || miss missing-digest
fetch_digest() {
    local kind="$1" path="$2" expected="$3"
    local code
    code="$(curl -sS --connect-timeout 5 --max-time 15 -o "$tmp/$kind" -w '%{http_code}' \
        -H "Accept: $accept" "${auth[@]}" "${prefix}/${repo}/${path}/${expected}" 2>/dev/null)" || miss "$kind-unreachable"
    [ "$code" = 200 ] || miss "$kind-http-$code"
    [ "sha256:$(sha256sum "$tmp/$kind" | cut -d ' ' -f1)" = "$expected" ] || miss "$kind-digest-mismatch"
}
fetch_digest manifest manifests "$digest"
# An index's first linux/amd64 descriptor identifies the image whose config we read.
child="$(node - "$tmp/manifest" <<'JS'
try {
  const m = JSON.parse(require('node:fs').readFileSync(process.argv[2]));
  if (m.mediaType === 'application/vnd.oci.image.index.v1+json' ||
      m.mediaType === 'application/vnd.docker.distribution.manifest.list.v2+json' ||
      'manifests' in m) {
    const d = m.manifests.find(x => x.platform?.os === 'linux' && x.platform?.architecture === 'amd64')?.digest;
    if (!/^sha256:[a-f0-9]{64}$/.test(d)) process.exit(1);
    process.stdout.write(d);
  }
} catch { process.exit(1); }
JS
)" || miss invalid-manifest-or-no-amd64
if [ -n "$child" ]; then fetch_digest manifest manifests "$child"; fi
config="$(node - "$tmp/manifest" <<'JS'
try {
  const d = JSON.parse(require('node:fs').readFileSync(process.argv[2])).config?.digest;
  if (!/^sha256:[a-f0-9]{64}$/.test(d)) process.exit(1);
  process.stdout.write(d);
} catch { process.exit(1); }
JS
)" || miss missing-config-digest
fetch_digest config blobs "$config"
revision="$(node - "$tmp/config" "$key" <<'JS'
try {
  const labels = JSON.parse(require('node:fs').readFileSync(process.argv[2])).config?.Labels;
  if (!labels || labels['elohim.host/closure'] !== process.argv[3] ||
      !/^[a-f0-9]{40}$/.test(labels['org.opencontainers.image.revision'])) process.exit(1);
  process.stdout.write(labels['org.opencontainers.image.revision']);
} catch { process.exit(1); }
JS
)" || miss missing-or-unequal-label
git merge-base --is-ancestor "$revision" origin/dev 2>/dev/null || miss revision-not-on-dev
printf 'hit %s\n' "$digest"
