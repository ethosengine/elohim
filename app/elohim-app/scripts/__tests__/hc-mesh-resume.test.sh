#!/usr/bin/env bash
# Exercise the resume argv with a stub hc; no conductor or mesh is launched.
set -u

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fail=0
t() { if eval "$2"; then echo "ok   $1"; else echo "FAIL $1"; fail=1; fi; }
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
export MESH_DIR="$tmp/mesh"
export MESH_PEERS=matthew,jessica,james
export MESH_CONDUCTOR_BASE_PORT=14444
export MESH_CONDUCTOR_LAUNCH=hc
source "$here/../hc-mesh.sh" >/dev/null

mkdir -p "$tmp/bin" "$LOCAL_DEV_DIR"
export RESUME_ARGS="$tmp/hc.args" RESUME_CWD="$tmp/hc.cwd"
cat > "$tmp/bin/hc" <<'HC'
#!/usr/bin/env bash
printf '%s\n' "$@" > "$RESUME_ARGS"
pwd > "$RESUME_CWD"
HC
chmod +x "$tmp/bin/hc"
export PATH="$tmp/bin:$PATH"
HOLOCHAIN_BIN=/bin/true
guard_conductor_data_roots() { return 0; }
assert_toolchain_parity() { return 0; }
conductor_restart_pids() { touch "$tmp/selection"; }
holochain_bin_export() { :; }
refresh_mesh_pidfiles() { :; }
setsid() { "$@"; }
# Join the short-lived stub before reading its argv, rather than racing the
# background launch. No actual service PID is recorded or discovered.
record_mesh_pid() { wait "$3"; }
curl() { return 1; }
ss() {
  local i
  for i in 0 1 2; do
    printf 'LISTEN 0 128 127.0.0.1:%s 0.0.0.0:*\n' "$(admin_port "$i")" "$(app_port "$i")"
  done
}

# Saved order differs from the configured roster, with late joiners interleaved.
# Relative paths and a final line without a newline still carry real indices.
printf '%s\n' "$LOCAL_DEV_DIR/ruth" "$LOCAL_DEV_DIR/james" './matthew' \
  "$LOCAL_DEV_DIR/naomi" > "$LOCAL_DEV_DIR/.hc"
printf '%s' "$LOCAL_DEV_DIR/jessica" >> "$LOCAL_DEV_DIR/.hc"
expected='sandbox --piped -f 14444,14454,14464 run 2 4 1'
for attach in 0 1; do
  MESH_ATTACH_APP_PORTS="$attach"
  out="$(restart_conductors 2>&1)"; rc=$?
  actual="$(paste -sd ' ' "$RESUME_ARGS")"
  want="$expected"
  [ "$attach" = 1 ] && want+=' -p=14445,14455,14465'
  t "resume selects only configured indices with aligned ports (attach=$attach, rc=$rc)" \
    '[ "$rc" -eq 0 ] && [ "$actual" = "$want" ] && [ "$(cat "$RESUME_CWD")" = "$LOCAL_DEV_DIR" ]'
done

# A missing entry must refuse before PID selection, config rewriting or launch.
for missing in peer registry; do
  rm -f "$RESUME_ARGS" "$tmp/selection"
  if [ "$missing" = peer ]; then
    printf '%s\n' "$LOCAL_DEV_DIR/matthew" "$LOCAL_DEV_DIR/james" > "$LOCAL_DEV_DIR/.hc"
    missing_name=jessica
  else
    rm "$LOCAL_DEV_DIR/.hc"
    missing_name=matthew
  fi
  out="$(restart_conductors 2>&1)"; rc=$?
  t "missing $missing refuses before restart and names the cure (rc=$rc)" \
    '[ "$rc" -ne 0 ] && [ ! -e "$RESUME_ARGS" ] && [ ! -e "$tmp/selection" ] && [[ "$out" == *"REFUSED conductor resume: $missing_name has no entry"* ]] && [[ "$out" == *"restore its saved sandbox entry"* ]]'
done

exit "$fail"
