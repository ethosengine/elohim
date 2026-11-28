#!/usr/bin/env bash
# Hermetic contract test: check targets must run through BuildKit with the
# intended Dockerfile/context and no image exporter or nerdctl image load.
set -euo pipefail

REPO_ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
CI="${REPO_ROOT}/scripts/ci"
T="$(mktemp -d)"
trap 'rm -rf "${T}"' EXIT
BIN="${T}/bin"
mkdir -p "${BIN}"
CALLS="${T}/buildctl-calls"
export CALLS

cat > "${BIN}/buildctl" <<'STUB'
#!/usr/bin/env bash
printf '%s\n' "$*" >> "${CALLS}"
case " $* " in
  *' debug workers '*) exit 0 ;;
  *' build '*) ;;
  *) echo "unexpected buildctl invocation: $*" >&2; exit 97 ;;
esac
for arg in "$@"; do
  case "${arg}" in
    --output|-o|--output=*) echo "quality check must not export an image" >&2; exit 98 ;;
  esac
done
STUB

cat > "${BIN}/nerdctl" <<'STUB'
#!/usr/bin/env bash
echo "unexpected nerdctl invocation: $*" >&2
exit 99
STUB
chmod +x "${BIN}/buildctl" "${BIN}/nerdctl"
export PATH="${BIN}:${PATH}"

STORAGE_CARGO_BUILD_JOBS="$(node -e '
const fs = require("node:fs");
const manifest = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
process.stdout.write(manifest.gate.projects["elohim-storage"].run.cargo.env.CARGO_BUILD_JOBS);
' "${REPO_ROOT}/elohim/holochain/build-manifest.json")"

cd "${REPO_ROOT}"
bash "${CI}/doorway-quality-gate.sh"
bash "${CI}/storage-quality-gate.sh"

grep -Fqx -- '--addr unix:///run/buildkit/buildkitd.sock debug workers' "${CALLS}"
grep -Fqx -- '--addr unix:///run/buildkit/buildkitd.sock build --frontend dockerfile.v0 --local context=. --local dockerfile=. --opt filename=doorway/doorway-service/Dockerfile --opt target=check --progress plain' "${CALLS}"
grep -Fqx -- "--addr unix:///run/buildkit/buildkitd.sock build --frontend dockerfile.v0 --local context=. --local dockerfile=. --opt filename=elohim/elohim-storage/Dockerfile --opt target=check --opt build-arg:CARGO_BUILD_JOBS=${STORAGE_CARGO_BUILD_JOBS} --progress plain" "${CALLS}"
[ "$(wc -l < "${CALLS}")" -eq 4 ] || { echo 'unexpected buildctl call count' >&2; exit 1; }

echo 'ok - quality helpers solve check targets without exporting/loading images'
