#!/usr/bin/env bash
# Source-mode coverage for modifier-aware hApp identity and the readiness record.
# Stub hc/curl fixtures never launch a conductor, storage peer, or network service.
set -u

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fail=0
t() { if eval "$2"; then echo "ok   $1"; else echo "FAIL $1"; fail=1; fi; }

export MESH_DIR="$tmp/mesh" MESH_PEERS=unit TMPDIR="$tmp/scratch"
mkdir -p "$TMPDIR" "$tmp/bin" "$tmp/unpacked"
source "$here/../hc-mesh.sh" >/dev/null
export PATH="$tmp/bin:$PATH"
export HC_FIXTURE_APP="$tmp/unpacked" HC_LOG="$tmp/hc.log" HC_FAIL_IF_CALLED=0
HAPP_PATH="$tmp/candidate.happ"
printf 'content-addressed fixture bundle\n' > "$HAPP_PATH"

cat > "$HC_FIXTURE_APP/happ.yaml" <<'YAML'
manifest_version: "0"
name: fixture
roles:
  - name: lamad
    dna:
      path: lamad.dna
      modifiers:
        network_seed: seed-lamad
        properties:
          progenitor_pubkey: null
  - name: infrastructure
    dna:
      path: infrastructure.dna
      modifiers:
        network_seed: seed-infrastructure
        properties: null
  - name: imagodei
    dna:
      path: imagodei.dna
      modifiers:
        network_seed: seed-imagodei
        properties:
          progenitor_pubkey: null
YAML
touch "$HC_FIXTURE_APP/lamad.dna" "$HC_FIXTURE_APP/infrastructure.dna" \
  "$HC_FIXTURE_APP/imagodei.dna"

cat > "$tmp/bin/hc" <<'HC'
#!/usr/bin/env bash
set -eu
[ "${HC_FAIL_IF_CALLED:-0}" = 0 ] || exit 97
printf '%s\n' "$*" >> "$HC_LOG"
case "$1 $2" in
  'app unpack')
    [ "$3" = -o ]
    mkdir -p "$4"
    cp -R "$HC_FIXTURE_APP/." "$4/"
    ;;
  'dna unpack')
    [ "$3" = -o ]
    mkdir -p "$4"
    role="$(basename "$5" .dna)"
    printf '%s\n' "$role" > "$4/role"
    cat > "$4/dna.yaml" <<YAML
manifest_version: "0"
name: $role
integrity:
  network_seed: base-seed
  properties:
    base: keep
  zomes: []
coordinator:
  zomes: []
YAML
    ;;
  'dna pack')
    [ "$3" = -o ]
    role="$(cat "$5/role")"
    python3 - "$5/dna.yaml" "$role" <<'PY'
import sys, yaml
manifest = yaml.safe_load(open(sys.argv[1]))
role = sys.argv[2]
integrity = manifest["integrity"]
assert integrity["network_seed"] == "seed-" + role
if role == "infrastructure":
    assert integrity["properties"] == {"base": "keep"}
else:
    assert integrity["properties"] == {"progenitor_pubkey": None}
PY
    printf '%s\n' "$role" > "$4"
    ;;
  'dna hash')
    case "$(cat "$3")" in
      lamad)          echo uhC0kLAMAD1234567890fixturehash ;;
      infrastructure) echo uhC0kINFRA1234567890fixturehash ;;
      imagodei)       echo uhC0kIMAGO1234567890fixturehash ;;
      *) exit 98 ;;
    esac
    ;;
  *) exit 99 ;;
esac
HC
chmod +x "$tmp/bin/hc"

expected='{"imagodei":"uhC0kIMAGO1234567890fixturehash","infrastructure":"uhC0kINFRA1234567890fixturehash","lamad":"uhC0kLAMAD1234567890fixturehash"}'
actual="$(bundle_dna_hashes "$HAPP_PATH")"; rc=$?
t "bundle hashes apply each happ role's modifiers before hc dna hash (rc=$rc)" \
  '[ "$rc" -eq 0 ] && [ "$actual" = "$expected" ]'
first_call="$(head -1 "$HC_LOG")"
t "hc uses documented app-unpack and dna-unpack/pack/hash forms" \
  '[[ "$first_call" == "app unpack -o $TMPDIR/hc-mesh-bundle-identity."*"/app $HAPP_PATH" ]] && [ "$(grep -c "^dna unpack -o " "$HC_LOG")" -eq 3 ] && [ "$(grep -c "^dna pack -o " "$HC_LOG")" -eq 3 ] && [ "$(grep -c "^dna hash " "$HC_LOG")" -eq 3 ]'

calls_before="$(wc -l < "$HC_LOG")"
export HC_FAIL_IF_CALLED=1
cached="$(bundle_dna_hashes "$HAPP_PATH")"; rc=$?
t "bundle SHA256 cache avoids repeated hc work (rc=$rc)" \
  '[ "$rc" -eq 0 ] && [ "$cached" = "$expected" ] && [ "$(wc -l < "$HC_LOG")" -eq "$calls_before" ]'
digest="$(sha256sum "$HAPP_PATH" | cut -d' ' -f1)"
t "cache key is the bundle SHA256 under MESH_DIR" \
  '[ "$(jq -cS . "$BUNDLE_IDENTITY_CACHE_DIR/$digest.json")" = "$expected" ]'

mkdir -p "$MESH_DIR"
printf '%s\n' "$expected" > "$INSTALLED_CELLS_FILE"
out="$(happ_bundle_identity resume 2>&1)"; rc=$?
t "resume accepts a bundle reproducing recorded cells (rc=$rc)" \
  '[ "$rc" -eq 0 ] && [[ "$out" == "ok hApp bundle identity:"* ]]'

mismatch='{"imagodei":"uhC0kIMAGO1234567890fixturehash","infrastructure":"uhC0kINFRA1234567890fixturehash","lamad":"uhC0kOLDXX1234567890fixturehash"}'
printf '%s\n' "$mismatch" > "$INSTALLED_CELLS_FILE"
out="$(happ_bundle_identity resume 2>&1)"; rc=$?
t "resume refuses skew with role, both prefixes, and recast cure (rc=$rc)" \
  '[ "$rc" -ne 0 ] && [[ "$out" == *"lamad installed=uhC0kOLDXX"* ]] && [[ "$out" == *"bundle=uhC0kLAMAD"* ]] && [[ "$out" == *"point MESH_HAPP_PATH"* ]] && [[ "$out" == *"MESH_RESET=1"* ]]'

# Drive the real preflight with every unrelated external edge reduced to a
# source-mode stub. This proves the identity mismatch contributes to its final
# refusal before any launch path is reachable.
load_canonical_household_binding() {
  CANONICAL_HOUSEHOLD_ID=fixture-household
  DEFAULT_HOUSEHOLD_PEERS=unit
  return 0
}
household_start_mode() { echo resume; }
assert_binary_newer_than_source() { return 0; }
storage_bin_in_gate_slot() { return 1; }
happ_bundle_freshness() { echo 'hApp bundle: fixture'; }
assert_storage_transport_capability() { return 0; }
mesh_owned_ports() { :; }
assert_launch_prerequisites() { return 0; }
MESH_HOLOCHAIN_BIN_EXPLICIT=1
HOLOCHAIN_BIN=/bin/true
STORAGE_BIN=/bin/true
STORAGE_BIN_EXPLICIT=1
MESH_DOORWAYS_EFFECTIVE=0
MESH_MEMBERSHIP=0
MESH_RELAY=0
out="$(preflight 2>&1)"; rc=$?
t "preflight refuses bundle skew before launch (rc=$rc)" \
  '[ "$rc" -ne 0 ] && [[ "$out" == *"REFUSED hApp bundle identity: lamad"* ]] && [[ "$out" == *"preflight: REFUSED"* ]]'

MESH_ALLOW_BUNDLE_SKEW=1
out="$(happ_bundle_identity resume 2>&1)"; rc=$?
t "explicit skew override is loud and proceeds (rc=$rc)" \
  '[ "$rc" -eq 0 ] && [[ "$out" == WARN* ]] && [[ "$out" == *"MESH_ALLOW_BUNDLE_SKEW=1"* ]]'
unset MESH_ALLOW_BUNDLE_SKEW
out="$(happ_bundle_identity reset 2>&1)"; rc=$?
t "deliberate reset bypasses the retired identity (rc=$rc)" \
  '[ "$rc" -eq 0 ] && [[ "$out" == *"deliberate recast"* ]]'
rm "$INSTALLED_CELLS_FILE"
out="$(happ_bundle_identity resume 2>&1)"; rc=$?
t "pre-fix household is admitted once as unrecorded (rc=$rc)" \
  '[ "$rc" -eq 0 ] && [[ "$out" == *"unrecorded"* ]]'

health_hashes="$expected"
curl() {
  case "$*" in
    *'/health'*) printf '{"dhtParticipation":{"dnaHashes":%s}}\n' "$health_hashes" ;;
    *) : ;;
  esac
}
http_port() { echo 18090; }
PEERS=(unit)
out="$(record_installed_cells 2>&1)"; rc=$?
t "start readiness records exact health dnaHashes (rc=$rc)" \
  '[ "$rc" -eq 0 ] && [ "$(jq -cS . "$INSTALLED_CELLS_FILE")" = "$expected" ] && [[ "$out" == *"installed cells: recorded"* ]]'

health_hashes="$mismatch"
out="$(record_installed_cells 2>&1)"; rc=$?
t "differing live cells never overwrite the durable record (rc=$rc)" \
  '[ "$rc" -ne 0 ] && [ "$(jq -cS . "$INSTALLED_CELLS_FILE")" = "$expected" ] && [[ "$out" == *"never overwriting"* ]]'

assert_mesh_stopped_for_reset() { return 0; }
reset_household_state >/dev/null
t "explicit recast removes the retired installed-cell identity" '[ ! -e "$INSTALLED_CELLS_FILE" ]'

exit "$fail"
