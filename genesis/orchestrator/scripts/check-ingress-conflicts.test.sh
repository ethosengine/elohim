#!/usr/bin/env bash
# Regression coverage for check-ingress-conflicts.sh's (host, ingressClassName)
# conflict key — the apex multi-A precondition (2026-09-12 operator ruling,
# doorway/doorway-service/.epr-meta/doorway-failover.habit.md): two ingresses
# may legitimately claim the same host on DIFFERENT per-premise classes
# (public-ethosengine / public-shem), but same host + same class in one
# namespace is still refused. `kubectl` is faked; no live cluster is used.

set -euo pipefail

REPO_ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
SCRIPT="${REPO_ROOT}/genesis/orchestrator/scripts/check-ingress-conflicts.sh"
TEST_ROOT="$(mktemp -d)"

cleanup() {
  rm -rf "${TEST_ROOT}"
}
trap cleanup EXIT

MANIFEST_WITH_CLASS="${TEST_ROOT}/manifest-with-class.yaml"
cat > "${MANIFEST_WITH_CLASS}" <<'EOF'
apiVersion: v1
kind: Secret
metadata:
  name: some-secret
  namespace: ns
type: Opaque
stringData:
  name: irrelevant-name-field-before-the-ingress-doc
---
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: doorway-a
  namespace: ns
  labels:
    app.kubernetes.io/name: doorway
spec:
  ingressClassName: public-ethosengine
  tls:
    - hosts:
        - elohim.host
        - doorway-alpha.elohim.host
      secretName: tls-a
  rules:
    - host: doorway-alpha.elohim.host
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: svc
                port:
                  number: 8080
    - host: elohim.host
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: svc
                port:
                  number: 8080
---
EOF

MANIFEST_LEGACY="${TEST_ROOT}/manifest-legacy.yaml"
cat > "${MANIFEST_LEGACY}" <<'EOF'
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: doorway-a
  namespace: ns
spec:
  rules:
    - host: doorway-alpha.elohim.host
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: svc
                port:
                  number: 8080
---
EOF

# Fake kubectl: reads FAKE_EXISTING_INGRESS ("" = no existing ingresses) and
# renders it as the jsonpath the script actually queries with.
kubectl() {
  if [ -z "${FAKE_EXISTING_INGRESS:-}" ]; then
    return 0
  fi
  printf '%s\n' "${FAKE_EXISTING_INGRESS}"
}
export -f kubectl

run() {
  local manifest="$1" existing="$2" output="${TEST_ROOT}/out.log"
  set +e
  FAKE_EXISTING_INGRESS="$existing" bash "$SCRIPT" "$manifest" ns > "$output" 2>&1
  local status=$?
  set -e
  printf '%s\t%s\n' "$status" "$output"
}

assert_passes() {
  local label="$1" manifest="$2" existing="$3" result status output
  result="$(run "$manifest" "$existing")"
  status="${result%%$'\t'*}"
  output="${result#*$'\t'}"
  if [ "$status" -ne 0 ]; then
    echo "Expected '${label}' to PASS; got exit ${status}" >&2
    cat "$output" >&2
    exit 1
  fi
  if ! grep -q 'PASSED' "$output"; then
    echo "'${label}' exited 0 but did not print PASSED" >&2
    cat "$output" >&2
    exit 1
  fi
}

assert_refuses() {
  local label="$1" manifest="$2" existing="$3" result status output
  result="$(run "$manifest" "$existing")"
  status="${result%%$'\t'*}"
  output="${result#*$'\t'}"
  if [ "$status" -eq 0 ]; then
    echo "Expected '${label}' to REFUSE; got exit 0" >&2
    cat "$output" >&2
    exit 1
  fi
  if ! grep -q 'CONFLICT' "$output"; then
    echo "'${label}' refused but did not print CONFLICT" >&2
    cat "$output" >&2
    exit 1
  fi
}

# 1. Same host, DIFFERENT ingressClassName (public-ethosengine vs
#    public-shem) — the apex multi-A shape — must PASS.
assert_passes \
  "same host, different classes" \
  "$MANIFEST_WITH_CLASS" \
  'doorway-b|public-shem|elohim.host,;'

# 2. Same host, SAME ingressClassName, different ingress name — the
#    unresolved-owner hazard — must REFUSE.
assert_refuses \
  "same host, same class" \
  "$MANIFEST_WITH_CLASS" \
  'doorway-b|public-ethosengine|elohim.host,;'

# 3. Same-named ingress (an update to itself), same host+class — never a
#    conflict regardless of class match.
assert_passes \
  "self-update, same name" \
  "$MANIFEST_WITH_CLASS" \
  'doorway-a|public-ethosengine|elohim.host,doorway-alpha.elohim.host,;'

# 4. Legacy manifest (no ingressClassName field) vs. an existing ingress
#    explicitly on class "public" — both must resolve to the SAME default
#    class and REFUSE, proving a classless manifest is treated as "public".
assert_refuses \
  "legacy manifest vs explicit public existing" \
  "$MANIFEST_LEGACY" \
  'doorway-old|public|doorway-alpha.elohim.host,;'

# 5. Legacy manifest (no class) vs. a legacy EXISTING ingress (no class in
#    the cluster jsonpath either, i.e. empty class segment) — both default
#    to "public" and must still REFUSE.
assert_refuses \
  "legacy manifest vs legacy existing" \
  "$MANIFEST_LEGACY" \
  'doorway-old||doorway-alpha.elohim.host,;'

# 6. Legacy manifest (defaults to "public") vs. an existing ingress on a
#    DIFFERENT explicit class — must PASS, same multi-A shape as case 1 but
#    exercised from the legacy/default side.
assert_passes \
  "legacy manifest vs explicit different class existing" \
  "$MANIFEST_LEGACY" \
  'doorway-b|public-shem|doorway-alpha.elohim.host,;'

# 7. No existing ingresses at all — must PASS.
assert_passes \
  "no existing ingresses in namespace" \
  "$MANIFEST_WITH_CLASS" \
  ''

echo "check-ingress-conflicts: (host, ingressClassName) regression tests passed"
