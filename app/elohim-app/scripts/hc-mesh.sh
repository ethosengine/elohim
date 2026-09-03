#!/bin/bash
#
# hc-mesh.sh - Local multi-peer Elohim mesh (Eclipse Che / single container)
#
# Brings up the ALPHA-SHAPED topology entirely on loopback, no k8s, no
# external services:
#
#   1 doorway   (dev mode, serves /bootstrap for the island DHT and
#                proxies all N storage peers)
#   N conductors (hc sandbox, holochain 0.7: --piped + -f pinned admin ports,
#                -n N native multi-sandbox; discovery via the LOCAL doorway)
#   N storages  (release binary, Track-2 backend selected independently as
#                libp2p, dual, or iroh; each bound to its own conductor)
#
# Verified bring-up 2026-08-16: mesh/upload/propagation/delivery/projection
# probes green via `substrate-verify.ts` against this fleet (same assertion
# set as CI's Dataplane Validation).
#
# USAGE:
#   ./hc-mesh.sh [start|stop|status|probe|prologue|join-peer|conductors-restart|coordswap]
#
#   `conductors-restart` restarts the N conductors IN PLACE against their
#   EXISTING sandboxes — no generate, so agent keys, chains and DHT databases
#   survive. Use it to pick up a new MESH_RUST_LOG (or after a conductor hang);
#   never to reset the mesh, and note that `start` on free ports would instead
#   REGENERATE the sandboxes and re-key every peer. Storage peers are left
#   alone and must reconnect to their conductors themselves.
#
#   `prologue` execs hc-mesh-prologue.sh — the Act I Prologue cast (named
#   CONDUCTOR_URLS seeding, SSR/landing/lamad-spa bundle staging, the full
#   substrate seed chain, and the household fixture manifest for a2o). It
#   never starts/stops/restarts mesh components; it seeds an ALREADY-running
#   mesh. This script can also be SOURCED (never executes start/stop/etc when
#   sourced — see the dispatch guard at the bottom) to reuse its env-shape
#   helpers (`conductor_csv`, `peer_csv`, `mesh_seed_env`) from another script
#   or an operator's shell.
#
#   `join-peer <name>` stages one NEW conductor + storage peer against an
#   already-running mesh. It never restarts or reconfigures an incumbent. The
#   peer name must be fresh; repeating the same name is refused before launch.
#
# ENVIRONMENT:
#   MESH_PEERS      Peer names, comma-separated (default: matthew,jessica,james)
#   MESH_DIR        Data root (default: /tmp/elohim-local-mesh)
#   MESH_TRANSPORT_BACKEND
#                   elohim-storage Track-2 backend: libp2p (default), dual, or
#                   iroh. This does NOT change the conductor's Holochain 0.7
#                   iroh transport. dual/iroh require STORAGE_BIN built with
#                   --features "p2p p2p-iroh"; start refuses otherwise.
#   MESH_PEER_TRANSPORTS
#                   Per-peer overrides, e.g. matthew=libp2p,jessica=iroh.
#                   Peers not named inherit MESH_TRANSPORT_BACKEND.
#   MESH_DOORWAYS    0 skips mongod and both doorways; storage peers launch
#                   without ELOHIM_DOORWAY_URL (default: 1).
#   DOORWAY_PORT    Doorway HTTP port (default: 8888)
#   MESH_PORTAL     0 skips the doorway sign-in portal (default: 1). The doorway proxies
#                   /threshold/* to THRESHOLD_URL as-is; without something listening there
#                   /threshold/login is a 502 and the chaperone portal cannot be exercised
#                   locally at all. Serving it is what lets the browser a2o lane validate a
#                   real login before anything is pushed.
#   THRESHOLD_PORT  Port the portal listens on (default: 8081 — the doorway's own
#                   THRESHOLD_URL default, so both doorways proxy to one portal).
#   DOORWAY_A_HEALTH_PORT / DOORWAY_B_HEALTH_PORT  health-watchdog listener ports (default 8079 / 8089;
#                   alpha runs 8079 — spawn_health_listener serves /health,/ready,/health/serving from
#                   its own OS-thread runtime; unset ⇒ liveness rides the MAIN listener and the watchdog
#                   a2o scenarios are unconstructible)
#   STORAGE_BIN     elohim-storage binary (default: pool release slot)
#   DOORWAY_BIN     doorway binary (default: pool debug slot)
#   MONGOD_BIN      mongod binary (default: first of $PATH mongod, ~/bin/mongod);
#                   empty/absent => the doorways run WITHOUT an archive (inert
#                   warm-shell store, memory-only projection) exactly as before
#   MONGO_PORT      loopback mongod port for the doorways' projection archive
#                   (default: 27017 — the doorway's own MONGODB_URI default)
#   MESH_API_KEY_ADMIN  Admin bootstrap key shared by both doorways AND the
#                   seed chain (default: mesh-admin-dev-key). A mesh-only
#                   preproduction credential — never a prod default; prod
#                   pulls API_KEY_ADMIN from a real secret. Printed in the
#                   `status` probe-env line and exported by `mesh_seed_env`.
#
#   MESH_CONDUCTOR_LAUNCH
#                   `hc` (default) launches via `hc sandbox run`; `direct` runs
#                   each conductor binary itself. Use `direct` with HOLOCHAIN_BIN
#                   whenever the conductor's config schema differs from the `hc`
#                   CLI's — the CLI rewrites the config in its own schema and a
#                   mismatched conductor then refuses to boot. `direct` also
#                   gives each conductor its own log file.
#                   `ark` runs each conductor as the CHILD of an `ark` process
#                   (elohim/ark — tevah in prose). Same argv as `direct`, so the
#                   spin detector and every `ps` grep are unchanged; what it adds
#                   is a parent that reaps the death itself and leaves a death
#                   witness in that peer's own spool ($LOCAL_DEV_DIR/<peer>/ark/).
#                   Each ark-launched storage peer pulls that spool through
#                   ELOHIM_ARK_SPOOL_PATH, defaults REPLICATION_INTERVAL_SECONDS
#                   to 10, and defaults CUSTODY_SWEEP_SECONDS and
#                   INVENTORY_BROADCAST_SECONDS to 15 (explicit values override).
#                   Declarations (manifest.json + berth.json) are rewritten per
#                   peer on every launch; the same data root is never run twice.
#
#   ARK_BIN         The `ark` binary. Default: the cargo-pool dev slot
#                   /projects/.cargo-target-pool/family/dev/elohim/dev/debug/ark,
#                   then whatever `ark` is on PATH. MESH_CONDUCTOR_LAUNCH=ark
#                   REFUSES to launch when neither is executable rather than
#                   falling back to a different launch mode; build it with
#                     cd elohim && CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim/dev \
#                       RUSTFLAGS="" cargo build -p elohim-ark
#
#   HOLOCHAIN_BIN   A conductor BINARY, or a DIRECTORY holding `holochain` +
#                   `hc`, to use INSTEAD of what is on PATH (see the block above
#                   stop_all). Unset = the auto-detected fork build, else stock.
#                   The matching `hc` goes on PATH for BOTH `generate` and
#                   `run`, and the start REFUSES a mismatched pair (both
#                   versions printed; MESH_ALLOW_TOOLCHAIN_SKEW=1 to override).
#                   Used for conductor-fork A/B runs; a binary swap alone cannot
#                   move a DNA hash, but verify via /health.
#
#   MESH_FORK_RELAY_URL
#                   Relay URL for Holochain 0.7's `network … quic <RELAY_URL>`
#                   generate grammar. Default = the LOCAL iroh-relay this script
#                   launches (http://localhost:$MESH_RELAY_PORT/). A 0.7
#                   conductor homes to its relay at boot and kitsune2 0.5 only
#                   dials peers whose advertised relay matches its own exactly,
#                   so with no reachable relay three LOOPBACK conductors sit at
#                   0 connections and never gossip (measured 2026-09-03 with
#                   relay_url pointed at the doorway, the pre-0.7 placeholder).
#
#   MESH_RELAY_BIN  The `iroh-relay` binary (1.0.3, `--features server`) to run
#                   on MESH_RELAY_PORT (default 3340, upstream's --dev port).
#                   Unset = the first `iroh-relay` on PATH. MESH_RELAY=0 skips
#                   the launch — you then own MESH_FORK_RELAY_URL.
#
#   MESH_RUST_LOG   Conductor log level. Default is targeted, not blanket:
#                   warn + INFO on exactly the three modules that diagnose a
#                   sys-validation spin (read-pool saturation, cascade
#                   NoPeersForLocation, sys-validation's missing-dependency
#                   counter). RUST_LOG unset means holochain logs ERROR only,
#                   which makes those lines unobservable rather than absent —
#                   see the block below the pacing profile.
#
#   Dev-tier pacing profile (see the block below the port-scheme helpers —
#   minutes-quiesce plan W3): MESH_RECONCILE_SECS, MESH_CONTEST_BACKOFF,
#   MESH_HEAL_MISSING_BACKOFF, MESH_EVIDENCE_ABSENT_BACKOFF,
#   MESH_HEAD_CORPUS_DIGEST override the storage peers' reconcile/backoff
#   cadence. Ark-launched storage peers additionally default
#   REPLICATION_INTERVAL_SECONDS to 10 seconds and CUSTODY_SWEEP_SECONDS and
#   INVENTORY_BROADCAST_SECONDS to 15 seconds; the
#   conductor sandboxes also get a fixed kitsune2 gossip
#   acceleration patch (k2Gossip initiate=1000ms) after generate, before run.
#
# LOAD-BEARING FACTS (verified against holochain 0.7.0 / hc 0.7.0):
#   - `hc sandbox --piped` reads the lair passphrase from stdin: the old
#     socat/PTY wrapper is obsolete.
#   - `-f p1,p2,..` pins admin ports; `-r=a1,a2,..` pins app ports; no more
#     log-scraping for dynamic ports.
#   - `hc sandbox generate` WITHOUT a network section can select public
#     discovery infrastructure. True isolation requires explicit
#     `network --bootstrap ... quic ...` arguments; the loopback mesh uses
#     its local doorway for bootstrap and the local iroh-relay as the relay
#     home (kitsune2 0.5: exact relay match, one relay per space — a
#     placeholder URL yields a booted conductor with 0 connections).
#   - Conductor admin/app interfaces bind loopback only; config exposes
#     `danger_bind_addr` when a cross-pod topology ever needs more. Inside
#     one container, loopback is correct.
#   - elohim-storage ignores the HTTP_PORT env var: pass --http-port.
#   - launches persist PID + process-start identity under $MESH_DIR/pids;
#     stop merges those records with listeners on this mesh's declared ports.
#     Service-name patterns are a warned, /proc-validated legacy fallback only.
#
set -u

MESH_PEERS="${MESH_PEERS:-matthew,jessica,james}"
MESH_DIR="${MESH_DIR:-/tmp/elohim-local-mesh}"
# Default flipped libp2p -> dual 2026-08-23: alpha has run dual since Wave-2 E3
# (campaign decision 2026-08-04); localdev now boots at fleet parity. Rollback: libp2p.
MESH_TRANSPORT_BACKEND="${MESH_TRANSPORT_BACKEND:-dual}"
case "$MESH_TRANSPORT_BACKEND" in
  libp2p|dual|iroh) ;;
  *)
    echo "invalid MESH_TRANSPORT_BACKEND='$MESH_TRANSPORT_BACKEND' (expected libp2p, dual, or iroh)" >&2
    if [ "${BASH_SOURCE[0]}" != "$0" ]; then return 2; else exit 2; fi ;;
esac
# `just test mesh` sources this file before invoking the report builder. Export
# the normalized/defaulted mode so that run evidence records the same backend
# the mesh launcher selects.
export MESH_TRANSPORT_BACKEND
MESH_TRANSPORT_BACKEND_EFFECTIVE="$MESH_TRANSPORT_BACKEND"
# Per-peer transport, the diversity axis of the two-peer recovery harness:
#   MESH_PEER_TRANSPORTS="matthew=libp2p,jessica=iroh"
# Any peer not named inherits MESH_TRANSPORT_BACKEND. The configurations are
# the library; the two slots cycle through it — a scenario never adds a peer.
MESH_PEER_TRANSPORTS="${MESH_PEER_TRANSPORTS:-}"
MESH_PEER_TRANSPORTS_EFFECTIVE="$MESH_PEER_TRANSPORTS"
peer_transport() { # <peer-name> -> libp2p|dual|iroh
  local kv
  IFS=',' read -ra _pt <<< "$MESH_PEER_TRANSPORTS_EFFECTIVE"
  for kv in "${_pt[@]}"; do
    [ "${kv%%=*}" = "$1" ] && { echo "${kv#*=}"; return 0; }
  done
  echo "$MESH_TRANSPORT_BACKEND_EFFECTIVE"
}
_validate_peer_transports() {
  local kv
  IFS=',' read -ra _pt <<< "$MESH_PEER_TRANSPORTS_EFFECTIVE"
  for kv in "${_pt[@]}"; do
    [ -z "$kv" ] && continue
    case "${kv#*=}" in libp2p|dual|iroh) ;; *)
      echo "invalid MESH_PEER_TRANSPORTS entry '$kv' (expected <peer>=libp2p|dual|iroh)" >&2
      return 2 ;;
    esac
  done
}
_validate_peer_transports || { if [ "${BASH_SOURCE[0]}" != "$0" ]; then return 2; else exit 2; fi; }
export MESH_PEER_TRANSPORTS
MESH_DOORWAYS="${MESH_DOORWAYS:-1}"
MESH_DOORWAYS_EFFECTIVE="$MESH_DOORWAYS"
DOORWAY_PORT="${DOORWAY_PORT:-8888}"
MESH_PORTAL="${MESH_PORTAL:-1}"
THRESHOLD_PORT="${THRESHOLD_PORT:-8081}"
DOORWAY_B_PORT="${DOORWAY_B_PORT:-8889}"
# The local iroh-relay every 0.7 conductor homes to (see MESH_FORK_RELAY_URL).
MESH_RELAY_PORT="${MESH_RELAY_PORT:-3340}"
MESH_RELAY_BIN="${MESH_RELAY_BIN:-$(command -v iroh-relay 2>/dev/null || true)}"
DOORWAY_A_HEALTH_PORT="${DOORWAY_A_HEALTH_PORT:-8079}"
DOORWAY_B_HEALTH_PORT="${DOORWAY_B_HEALTH_PORT:-8089}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
LOCAL_DEV_DIR="$REPO_ROOT/elohim/holochain/local-dev"
HAPP_WORKDIR="$REPO_ROOT/elohim/holochain/dna/elohim/workdir"
# MESH_HAPP_PATH overrides the bundle the sandboxes install. Default = the locally packed
# workdir bundle. Point it at elohim/holochain/local-dev/deployed-bundles/elohim.happ
# (app/elohim-app/scripts/fetch-deployed-dna.sh) for DNA parity with the fleet AND with a
# storage binary built from today's tree: measured 2026-08-28, a workdir bundle 12 days
# older than the storage binary turned every head-record zome call into a WasmError
# Deserialize (get_record_for_action input shape moved) that the probe below mislabelled
# as a stale token.
HAPP_PATH="${MESH_HAPP_PATH:-$HAPP_WORKDIR/elohim.happ}"
POOL="/projects/.cargo-target-pool/family/dev"
# Storage binary: the release pool slot when it exists, else the dev (debug) slot — the one
# `just gate elohim-storage` and the iroh build command below actually fill. Before 2026-08-28
# the default named only the release slot, which is usually absent, so every `just mesh start`
# needed STORAGE_BIN by hand and a dual run silently fell to whatever was passed.
_storage_release="$POOL/elohim__elohim-storage/release/release/elohim-storage"
_storage_debug="$POOL/elohim__elohim-storage/dev/debug/elohim-storage"
if [ -z "${STORAGE_BIN:-}" ] && [ ! -x "$_storage_release" ] && [ -x "$_storage_debug" ]; then
  STORAGE_BIN="$_storage_debug"
fi
STORAGE_BIN="${STORAGE_BIN:-$_storage_release}"
DOORWAY_BIN="${DOORWAY_BIN:-$POOL/doorway__doorway-service/dev/debug/doorway}"
# The ark launcher (elohim/ark/cli). Resolved here so `status` and the launch
# sites name the same binary, but NEVER required at source time: only
# MESH_CONDUCTOR_LAUNCH=ark needs it, and assert_ark_binary is what refuses.
# The elohim workspace has ONE pool slot for all its crates, hence dev/debug/ark.
ARK_BIN_POOL_SLOT="$POOL/elohim/dev/debug/ark"
if [ -z "${ARK_BIN:-}" ]; then
  if [ -x "$ARK_BIN_POOL_SLOT" ]; then
    ARK_BIN="$ARK_BIN_POOL_SLOT"
  else
    ARK_BIN="$(command -v ark 2>/dev/null || true)"
  fi
fi
# Absolutise whatever we ended up with — including an operator-supplied
# ARK_BIN, which skips the block above entirely. Every launch site runs inside
# a subshell that has `cd`'d into $LOCAL_DEV_DIR, so a relative path would name
# a DIFFERENT file there than the `-x` check answered about here (or nothing at
# all). Resolve once, at the only point where $PWD is still the caller's.
absolutise_ark_bin() {
  [ -n "${ARK_BIN:-}" ] || return 0
  local resolved
  resolved="$(readlink -f "$ARK_BIN" 2>/dev/null || true)"
  [ -n "$resolved" ] && ARK_BIN="$resolved"
  return 0
}
absolutise_ark_bin

# MESH_DOORWAY_GATEWAY_SCOPING — does a mesh doorway name its humans the way a
# FLEET doorway does? Every deployed doorway runs with DOORWAY_URL set, and
# doorway-service then re-qualifies an identifier's local part to
# `<local>@<gateway domain>` on register AND login (auth_routes.rs
# `gateway_domain` + `normalize_identifier`), so `/auth/me` answers
# `matthew.dowell@alpha.elohim.host`. With no DOORWAY_URL it stores identifiers
# verbatim -- which is what this mesh did until 2026-08-29, and why the fleet,
# not the mesh, was the thing that discovered a portal scenario asserting the
# bare name (genesis #1519; backlog portal-login-step-domain-scoped-identifier).
# A mesh that cannot express the fleet's identity convention cannot pre-empt a
# fleet red about it, so the default is ON. Set to 0 to run the verbatim shape.
MESH_DOORWAY_GATEWAY_SCOPING="${MESH_DOORWAY_GATEWAY_SCOPING:-1}"
# mongod backs the doorways' Mongo-side projection archive (app_file_cache /
# warm-shell ShellArchive / DoorwayResolver store). Without it every doorway
# constructs an INERT WarmShellStore and a memory-only projection — which is
# precisely the production shape 18a65fd0d found un-wired, so the archive leg
# of the boot-order self-heal family could never be proven on this mesh.
# Loopback-only, per-doorway database (doorway-a / doorway-b) so A and B do
# not share an archive and mask each other's boot-order gaps.
MONGOD_BIN="${MONGOD_BIN-$(command -v mongod 2>/dev/null || { [ -x "$HOME/bin/mongod" ] && echo "$HOME/bin/mongod"; })}"
MONGO_PORT="${MONGO_PORT:-27017}"
MONGO_DIR="$MESH_DIR/mongo"
LOGDIR="$MESH_DIR/logs"
PID_DIR="$MESH_DIR/pids"

# Port scheme per peer index i (0-based): admin 4444+10i, app 4445+10i,
# storage http 8090+i, libp2p 9701+i.
IFS=',' read -ra PEERS <<< "$MESH_PEERS"

admin_port() { echo $((4444 + 10 * $1)); }
app_port()   { echo $((4445 + 10 * $1)); }
http_port()  { echo $((8090 + $1)); }
p2p_port()   { echo $((9701 + $1)); }

process_start_ticks() { # <pid> — guards a persisted pid against PID reuse
  # stat field 2 (`comm`) may contain spaces; strip pid+comm through the final
  # ')' first. starttime is field 20 of the remaining field-3.. sequence.
  sed 's/^[^)]*) //' "/proc/$1/stat" 2>/dev/null | awk '{print $20}'
}

record_mesh_pid() { # <role> <name> <pid>
  local role="$1" name="$2" pid="$3" started
  [[ "$pid" =~ ^[0-9]+$ ]] || return 1
  started="$(process_start_ticks "$pid")"
  [ -n "$started" ] || return 1
  mkdir -p "$PID_DIR"
  printf '%s %s\n' "$pid" "$started" > "$PID_DIR/$role-$name"
}

# ---------------------------------------------------------------------------
# MESH_CONDUCTOR_LAUNCH=ark. Everything below this banner is additive: `hc` and
# `direct` never call any of it, and a mesh started in those modes writes no
# ark/ directory, so `status` and `stop` behave exactly as they always have.
#
# The shape: one `ark` process per peer, whose ONLY child is that peer's
# conductor with the argv `direct` would have used. The ark is the parent that
# reaps the death — so a SIGKILLed conductor leaves a death witness in
# $LOCAL_DEV_DIR/<peer>/ark/witnesses/ instead of a hole in a log.
#
# NOTE on the child's environment: ChildSpec.env_scrub defaults to true and only
# PATH and HOME survive it, so the conductor child does NOT inherit the
# RUST_LOG exported for the ark itself. Until the manifest carries an `env`
# block (the berth log_level dial, spec Task 10L), an ark-launched conductor
# logs at holochain's default level. Its stdout — including the
# "Conductor ready." line the readiness ladder needs — is unaffected.
# ---------------------------------------------------------------------------

# The recorded pid for one role/name, only when it is still the process this
# mesh launched. Validated against the persisted start-tick exactly as
# recorded_mesh_pids does: a bare `kill -0` would happily accept a recycled pid
# and let a second ark be refused (or launched) on the wrong evidence.
live_recorded_pid() { # <role> <name> -> prints pid, or returns 1
  local file="$PID_DIR/$1-$2" pid="" started="" current
  [ -f "$file" ] || return 1
  read -r pid started < "$file" || true
  [[ "$pid" =~ ^[0-9]+$ ]] || return 1
  [ -n "$started" ] || return 1
  current="$(process_start_ticks "$pid")"
  [ -n "$current" ] && [ "$current" = "$started" ] || return 1
  echo "$pid"
}

# The conductor child pid an ark is supervising, read from the passport the ark
# rewrites on every state change. This is the ONLY correct way to find that pid
# under ark: $PID_DIR/ark-<peer> names the ARK, and the child is replaced on
# every restart while the ark keeps its own pid.
mesh_conductor_pid() { # <peer-name> -> prints the conductor child pid, or returns 1
  local passport="$LOCAL_DEV_DIR/$1/ark/passport.json" pid
  [ -f "$passport" ] || return 1
  pid="$(jq -r '.processes[] | select(.name=="conductor") | .pid // empty' "$passport" 2>/dev/null)"
  [[ "$pid" =~ ^[0-9]+$ ]] || return 1
  echo "$pid"
}

assert_ark_binary() {
  # Re-absolutise first: ARK_BIN can be set (or re-set) after this file is
  # sourced, and the -x answer has to be about the same bytes the launch site
  # will exec from another directory.
  absolutise_ark_bin
  [ -n "${ARK_BIN:-}" ] && [ -x "$ARK_BIN" ] && return 0
  echo "MESH_CONDUCTOR_LAUNCH=ark: no executable ark binary (ARK_BIN='${ARK_BIN:-}')" >&2
  echo "  build it:" >&2
  echo "    cd '$REPO_ROOT/elohim' && CARGO_TARGET_DIR='$POOL/elohim/dev' RUSTFLAGS=\"\" cargo build -p elohim-ark" >&2
  echo "  or point ARK_BIN at one; refusing to fall back to another launch mode." >&2
  return 1
}

# What a launch mode must be able to do BEFORE anything is generated.
#
# `hc` (and `direct`, unchanged) demand toolchain parity because `hc sandbox`
# rewrites conductor-config.yaml in the CLI's own schema. `ark` never launches
# through the CLI — exactly as `direct` doesn't, which is why
# conductors_restart already skips the parity check for both — so that refusal
# is not the one that protects an ark run. What DOES protect it is the pair
# write_ark_declarations cannot work without: the ark binary (it hashes the
# conductor and mints the manifest CID) and jq (it writes both declarations).
# Both are asked for here, at the top, rather than after three sandboxes have
# been generated and the mesh is already half-built.
assert_launch_prerequisites() {
  if [ "${MESH_CONDUCTOR_LAUNCH:-hc}" = "ark" ]; then
    assert_ark_binary || return 1
    command -v jq >/dev/null 2>&1 || {
      echo "MESH_CONDUCTOR_LAUNCH=ark: jq is required to write each peer's manifest.json and berth.json" >&2
      return 1
    }
    return 0
  fi
  assert_toolchain_parity
}

# The conductor binary an ark will pin, hash and execute. Identical selection to
# the `direct` branches (HOLOCHAIN_BIN is already normalised to a binary path
# near detect_fork_bin), but absolutised: the manifest's claim is about BYTES at
# a path, so a PATH-relative name would let a later PATH change execute
# different bytes than the passport reports.
ark_conductor_bin() {
  local bin="${HOLOCHAIN_BIN:-}"
  [ -n "$bin" ] || bin="$(command -v holochain 2>/dev/null || true)"
  case "$bin" in
    ""|/*) ;;
    *) bin="$PWD/$bin" ;;
  esac
  [ -x "$bin" ] || {
    echo "ark: conductor binary is not executable: '${bin:-<none on PATH>}'" >&2
    return 1
  }
  echo "$bin"
}

# Writes <peer>/ark/manifest.json and <peer>/ark/berth.json. Rewritten on every
# launch, never hand-edited: the manifest pins the digest of the conductor
# binary about to run, and the berth names that manifest by CID — `ark run`
# exits 65 when the two disagree and 66 when the bytes on disk are not what was
# pinned, so both values are DERIVED here rather than remembered.
write_ark_declarations() { # <peer-name> <peer-index>
  local name="$1" index="$2"
  local data_root ark_dir manifest_path berth_path hc_bin sha cid incarnation
  command -v jq >/dev/null 2>&1 || {
    echo "ark: jq is required to write $name's declarations" >&2
    return 1
  }
  hc_bin="$(ark_conductor_bin)" || return 1
  data_root="$LOCAL_DEV_DIR/$name"
  ark_dir="$data_root/ark"
  manifest_path="$ark_dir/manifest.json"
  berth_path="$ark_dir/berth.json"
  [ -f "$data_root/conductor-config.yaml" ] || {
    echo "ark: $name has no conductor-config.yaml under $data_root — generate the sandbox first" >&2
    return 1
  }
  mkdir -p "$ark_dir" || return 1

  sha="$("$ARK_BIN" hash "$hc_bin")" || {
    echo "ark: could not hash the conductor binary $hc_bin" >&2
    return 1
  }

  # argv is the `direct` argv with {artifact} and {data_root} left as berth
  # templates, so the manifest stays content-addressable across peers: three
  # peers on one conductor binary share ONE manifest CID and differ only in
  # their berths. `ps` therefore still shows "holochain --piped", which is what
  # the spin detector and mesh_footprint grep for.
  jq -n --arg sha "$sha" '{
    schema: 1,
    kind: "runtime-manifest",
    reach: "trusted",
    processes: [{
      name: "conductor",
      kind: "native",
      artifact: { pinned: { sha256: $sha } },
      argv: ["{artifact}", "--piped", "--structured=Log", "--config-path", "{data_root}/conductor-config.yaml"],
      stdin: "passphrase",
      readiness: [
        { stdout_line: { contains: "Conductor ready.", patience_ms: 120000 } },
        { tcp_listen: { port_key: "admin_ws", patience_ms: 30000 } }
      ],
      policy: { shutdown: { signal: 2, grace_ms: 20000 } }
    }]
  }' > "$manifest_path" || return 1

  cid="$("$ARK_BIN" manifest cid --manifest "$manifest_path")" || {
    echo "ark: $manifest_path is not a valid runtime manifest" >&2
    return 1
  }

  # Incarnation is monotone across restarts and `ark run` bumps whatever it
  # finds, so a berth rewritten on every launch must carry the last passport's
  # value forward instead of resetting the count to zero.
  #
  # Fail CLOSED. A passport that exists but cannot be read for a nonnegative
  # integer incarnation is evidence about a data root that has already been
  # run — silently substituting 0 would rewrite the berth with a count LOWER
  # than the one the spool remembers, which is exactly the monotonicity this
  # value exists to keep. Refuse this peer's launch instead and say why.
  incarnation=0
  if [ -f "$ark_dir/passport.json" ]; then
    incarnation="$(jq -e -r '
      if (.incarnation | type) == "number"
         and .incarnation >= 0
         and (.incarnation | floor) == .incarnation
      then .incarnation else empty end
    ' "$ark_dir/passport.json" 2>/dev/null)" || {
      echo "ark: $name's passport.json carries no readable incarnation — refusing to reset it to 0" >&2
      echo "  read it yourself: $ark_dir/passport.json" >&2
      echo "  (remove the passport only if this data root is being deliberately re-seeded)" >&2
      return 1
    }
    [[ "$incarnation" =~ ^[0-9]+$ ]] || {
      echo "ark: $name's passport.json incarnation is not a nonnegative integer: '$incarnation'" >&2
      return 1
    }
  fi

  # "test" is the same passphrase the `hc` and `direct` branches pipe in, and
  # admin_ws is what the tcp_listen rung of the readiness ladder resolves.
  jq -n \
    --arg manifest "$cid" \
    --arg data_root "$data_root" \
    --arg conductor "$hc_bin" \
    --argjson admin_ws "$(admin_port "$index")" \
    --argjson incarnation "$incarnation" '{
    manifest: $manifest,
    data_root: $data_root,
    passphrase: { literal: "test" },
    ports: { admin_ws: $admin_ws },
    artifacts: { conductor: $conductor },
    incarnation: $incarnation
  }' > "$berth_path" || return 1
}

# One data root, one ark. Two arks over the same spool would interleave their
# intent logs, both claim the incarnation, and each reap a conductor the other
# spawned — so the launch sites ASK before launching rather than discovering it
# afterwards.
ark_peer_is_running() { # <peer-name> — 0 when an ark already owns this data root
  local pid
  if pid="$(live_recorded_pid ark "$1")"; then
    echo "  $1: an ark already supervises this data root (ark pid $pid) — not launching a second"
    return 0
  fi
  if pid="$(mesh_conductor_pid "$1")" && kill -0 "$pid" 2>/dev/null; then
    echo "  $1: the passport names a LIVE conductor child (pid $pid) — an ark still owns this data root"
    return 0
  fi
  return 1
}

launch_ark_conductor() { # <peer-name> <peer-index>
  local name="$1" index="$2"
  ark_peer_is_running "$name" && return 0
  write_ark_declarations "$name" "$index" || return 1
  # setsid, for the same reason the other launch sites use it: nohup only
  # ignores SIGHUP and does nothing about a SIGKILL to the process GROUP, which
  # is how a calling shell reaps its background children when it exits.
  (
    export RUST_LOG="$MESH_RUST_LOG"
    cd "$LOCAL_DEV_DIR" || exit 1
    setsid nohup "$ARK_BIN" run \
      --manifest "$LOCAL_DEV_DIR/$name/ark/manifest.json" \
      --berth "$LOCAL_DEV_DIR/$name/ark/berth.json" \
      >> "$LOCAL_DEV_DIR/.sandbox_run_log.$name" 2>&1 &
    record_mesh_pid ark "$name" "$!" || true
  )
  echo "  $name: ark launched (admin=$(admin_port "$index"), log $LOCAL_DEV_DIR/.sandbox_run_log.$name)"
}

launch_ark_conductors() { # one ark per configured peer; live peers are skipped
  local name i=0 failed=0
  assert_ark_binary || return 1
  echo "  launch mode: ark ($ARK_BIN — each conductor is the child of an ark that witnesses its death)"
  for name in "${PEERS[@]}"; do
    launch_ark_conductor "$name" "$i" || failed=1
    i=$((i + 1))
  done
  return "$failed"
}

# Ark rows for `status`. Prints NOTHING when no peer has a passport, so the
# status output of an `hc` or `direct` mesh is byte-identical to before.
ark_status_rows() {
  local name passport pid incarnation ready any=0
  for name in "${PEERS[@]}"; do
    passport="$LOCAL_DEV_DIR/$name/ark/passport.json"
    [ -f "$passport" ] || continue
    if [ "$any" -eq 0 ]; then echo "arks:"; any=1; fi
    pid="$(jq -r '.processes[] | select(.name=="conductor") | .pid // "-"' "$passport" 2>/dev/null)"
    incarnation="$(jq -r '.incarnation // "-"' "$passport" 2>/dev/null)"
    ready="$(jq -r '.processes[] | select(.name=="conductor") | .ready' "$passport" 2>/dev/null)"
    printf "  conductor(ark) %-8s pid=%s incarnation=%s ready=%s\n" \
      "$name" "${pid:--}" "${incarnation:--}" "${ready:-false}"
  done
}

listener_pids_for_ports() { # <port>... — TCP and UDP listeners, unique pids
  local port
  for port in "$@"; do
    {
      ss -H -ltnp "sport = :$port" 2>/dev/null
      ss -H -lunp "sport = :$port" 2>/dev/null
    } | grep -o 'pid=[0-9]*' | cut -d= -f2
  done | sort -un
}

record_listener_pid() { # <role> <name> <port>
  local role="$1" name="$2" port="$3" pid
  pid="$(listener_pids_for_ports "$port" | head -1)"
  [ -n "$pid" ] && record_mesh_pid "$role" "$name" "$pid"
}

mesh_owned_ports() {
  # Doorway/mongo ports remain owned even when the NEXT requested shape has
  # MESH_DOORWAYS=0; stop must still reap a previously doorway-backed shape.
  printf '%s\n' "$DOORWAY_PORT" "$DOORWAY_B_PORT" \
    "$DOORWAY_A_HEALTH_PORT" "$DOORWAY_B_HEALTH_PORT" "$MONGO_PORT" "$THRESHOLD_PORT" \
    "$MESH_RELAY_PORT"
  local i=0
  for _ in "${PEERS[@]}"; do
    printf '%s\n' "$(admin_port "$i")" "$(app_port "$i")" \
      "$(http_port "$i")" "$(p2p_port "$i")"
    i=$((i+1))
  done
}

refresh_mesh_pidfiles() {
  local i=0 name
  record_listener_pid doorway a "$DOORWAY_PORT" || true
  record_listener_pid doorway b "$DOORWAY_B_PORT" || true
  record_listener_pid mongod mesh "$MONGO_PORT" || true
  record_listener_pid portal mesh "$THRESHOLD_PORT" || true
  for name in "${PEERS[@]}"; do
    record_listener_pid conductor "$name" "$(admin_port "$i")" || true
    record_listener_pid storage "$name" "$(http_port "$i")" || true
    i=$((i+1))
  done
}

# Resolve the conductor that owns one peer's admin listener. The listener is
# the strongest evidence because it names the process actually serving the
# mesh port. The pgrep fallback covers platforms where `ss -p` cannot expose
# process ownership; candidates still have to be the holochain executable and
# carry this peer's exact config path.
conductor_pid_for_index() { # <peer-name> <peer-index>
  local name="$1" index="$2" pid raw
  pid="$(listener_pids_for_ports "$(admin_port "$index")" | head -1)"
  if [ -n "$pid" ]; then
    echo "$pid"
    return 0
  fi
  while IFS= read -r pid; do
    [ -n "$pid" ] || continue
    raw="$(tr '\0' ' ' < "/proc/$pid/cmdline" 2>/dev/null)"
    [[ "$raw" == *"$LOCAL_DEV_DIR/$name/conductor-config.yaml"* ]] || continue
    echo "$pid"
    return 0
  done < <(pgrep -x holochain 2>/dev/null)
  return 1
}

path_mode() { # <path> — diagnostic only; never changes permissions
  local path="$1" parent
  if stat -Lc 'mode=%A(%a) uid=%u gid=%g' "$path" 2>/dev/null; then
    return 0
  fi
  parent="$(dirname "$path")"
  printf 'mode=missing parent=%s parent-' "$parent"
  stat -Lc 'mode=%A(%a) uid=%u gid=%g' "$parent" 2>/dev/null || printf 'mode=unreadable'
}

# A live conductor is reusable only while its named sandbox still exists and
# none of its handles into that sandbox resolve to an unlinked inode. The
# second leg catches the subtler recreate-at-the-same-path case: `-d` sees the
# new directory while the old process continues writing the deleted database.
check_conductor_data_root() { # <peer-name> <pid> [sandbox]
  local name="$1" pid="$2" sandbox="${3:-$LOCAL_DEV_DIR/$1}" link raw evidence=""
  if [ ! -d "$sandbox" ]; then
    evidence="sandbox-path-missing"
  fi
  for link in "/proc/$pid"/fd/* "/proc/$pid/cwd"; do
    [ -L "$link" ] || continue
    raw="$(readlink "$link" 2>/dev/null)"
    case "$raw" in
      "$sandbox"*" (deleted)")
        evidence="${evidence:+$evidence; }handle=$link target=$raw $(path_mode "$link")"
        break ;;
    esac
  done
  [ -z "$evidence" ] && return 0

  echo "  $name: state=orphaned-data-root pid=$pid sandbox=$sandbox $(path_mode "$sandbox")" >&2
  echo "    evidence: $evidence" >&2
  return 1
}

report_conductor_data_roots() {
  local i=0 name pid failed=0
  for name in "${PEERS[@]}"; do
    pid="$(conductor_pid_for_index "$name" "$i" || true)"
    if [ -z "$pid" ]; then
      echo "  $name: not-running"
    elif check_conductor_data_root "$name" "$pid"; then
      echo "  $name: live pid=$pid sandbox=$LOCAL_DEV_DIR/$name"
    else
      failed=1
    fi
    i=$((i + 1))
  done
  return "$failed"
}

guard_conductor_data_roots() { # <verb>
  local verb="$1" i=0 name pid failed=0
  for name in "${PEERS[@]}"; do
    pid="$(conductor_pid_for_index "$name" "$i" || true)"
    if [ -n "$pid" ] && ! check_conductor_data_root "$name" "$pid"; then
      failed=1
    fi
    i=$((i + 1))
  done
  [ "$failed" -eq 0 ] && return 0
  echo "REFUSING $verb: state=orphaned-data-root" >&2
  echo "  remediation: ./hc-mesh.sh stop && ./hc-mesh.sh start" >&2
  echo "  stop kills the surviving conductor; start then regenerates its sandbox." >&2
  return 1
}

# Is ANY peer's data root still held by a live process? `start` decides whether
# to regenerate from ONE observation — peer 0's admin port being silent — and
# then removes every peer directory. That port is silent while an ark is
# between incarnations, and it says nothing at all about peers 1..n, so the
# regenerate branch can reach a `rm -rf` of a data root that a running ark or a
# running conductor is writing to (deleted-inode sandbox, orphaned ark, lost
# spool). Ask every peer directly instead, with the two pid facts this script
# already trusts: the recorded ark pid (validated against its start ticks) and
# the conductor pid that owns the peer's admin port.
assert_no_live_peer_processes() { # <verb> — 0 only when every peer is idle
  local verb="$1" i=0 name pid survivors=""
  for name in "${PEERS[@]}"; do
    if pid="$(live_recorded_pid ark "$name")"; then
      survivors+="  $name: ark pid $pid"$'\n'
    fi
    if pid="$(conductor_pid_for_index "$name" "$i")" && [ -n "$pid" ]; then
      survivors+="  $name: conductor pid $pid"$'\n'
    fi
    i=$((i + 1))
  done
  [ -z "$survivors" ] && return 0
  echo "REFUSING $verb: a live process still owns a peer data root under $LOCAL_DEV_DIR" >&2
  printf '%s' "$survivors" >&2
  echo "  regenerating would delete those sandboxes out from under running processes." >&2
  echo "  remediation: ./hc-mesh.sh stop, then start again." >&2
  return 1
}

# The storage CLI has no --print-features surface. The p2p-iroh build does,
# however, retain this exact tracing target literal from p2p_iroh/node.rs in
# both debug and release binaries. A source-path-only `p2p_iroh` grep is not
# sufficient: embedded migration comments contain that text even in the
# default-feature binary.
storage_has_iroh_feature() { # <binary>
  strings "$1" 2>/dev/null | grep -Fq 'elohim_storage::p2p_iroh'
}

print_iroh_build_command() { # <binary>
  local bin="$1" target_dir profile=""
  case "$bin" in
    */debug/elohim-storage) target_dir="${bin%/debug/elohim-storage}" ;;
    */release/elohim-storage)
      target_dir="${bin%/release/elohim-storage}"
      profile=" --release" ;;
    *) target_dir="$POOL/elohim__elohim-storage/dev" ;;
  esac
  echo "  cd '$REPO_ROOT/elohim/elohim-storage'"
  echo "  CARGO_TARGET_DIR='$target_dir' RUSTFLAGS='--cfg getrandom_backend=\"custom\"' cargo build$profile --features \"p2p p2p-iroh\" --bin elohim-storage"
}

assert_storage_transport_capability() { # <binary> <mode>
  local bin="$1" mode="$2"
  case "$mode" in libp2p) return 0 ;; dual|iroh) ;; *)
    echo "invalid storage transport '$mode'" >&2; return 1 ;; esac
  if storage_has_iroh_feature "$bin"; then return 0; fi
  echo "REFUSING TO START storage transport=$mode — binary lacks the compiled p2p-iroh marker:" >&2
  echo "  $bin" >&2
  echo "Build that pool slot with:" >&2
  print_iroh_build_command "$bin" >&2
  return 1
}

storage_pid_for_port() { # <http-port>
  ps -eo pid=,args= | awk -v me="$$" -v pat="--http-port $1" \
    '$1 != me && index($0, "elohim-storage") && index($0, pat) { print $1; exit }'
}

transport_from_environ() { # <nul-delimited environ file>
  [ -r "$1" ] || return 0
  tr '\0' '\n' < "$1" 2>/dev/null | sed -n 's/^ELOHIM_TRANSPORT_BACKEND=//p' | head -1
}

storage_transport_for() { # <peer-name> <http-port>
  local name="$1" port="$2" pid mode=""
  pid="$(storage_pid_for_port "$port")"
  if [ -n "$pid" ] && [ -r "/proc/$pid/environ" ]; then
    mode="$(transport_from_environ "/proc/$pid/environ")"
  fi
  if [ -z "$mode" ]; then
    mode="$(transport_from_environ "$MESH_DIR/storage-restart/$name.environ")"
  fi
  # A peer launched before this knob landed has no captured declaration; the
  # daemon's own default is libp2p, so name that fact instead of saying unknown.
  echo "${mode:-libp2p(default)}"
}

storage_spool_path_for() { # <peer-name> <http-port>
  local name="$1" port="$2" pid path=""
  pid="$(storage_pid_for_port "$port")"
  if [ -n "$pid" ]; then
    if [ -r "/proc/$pid/environ" ]; then
      path="$(tr '\0' '\n' < "/proc/$pid/environ" 2>/dev/null \
        | grep '^ELOHIM_ARK_SPOOL_PATH=' | head -1)"
      if [ -n "$path" ]; then
        echo "${path#ELOHIM_ARK_SPOOL_PATH=}"
      else
        echo off
      fi
    else
      echo unreadable
    fi
  elif [ "${MESH_CONDUCTOR_LAUNCH:-hc}" = "ark" ] \
    || [ -f "$LOCAL_DEV_DIR/$name/ark/passport.json" ]; then
    echo "next-launch:$LOCAL_DEV_DIR/$name/ark"
  else
    echo off
  fi
}

storage_replication_interval_for() { # <peer-name> <http-port>
  local name="$1" port="$2" pid value=""
  pid="$(storage_pid_for_port "$port")"
  if [ -n "$pid" ]; then
    if [ -r "/proc/$pid/environ" ]; then
      value="$(tr '\0' '\n' < "/proc/$pid/environ" 2>/dev/null \
        | grep '^REPLICATION_INTERVAL_SECONDS=' | head -1)"
      if [ -n "$value" ]; then
        echo "${value#REPLICATION_INTERVAL_SECONDS=}s"
      else
        echo "60s(default)"
      fi
    else
      echo unreadable
    fi
  elif [ "${MESH_CONDUCTOR_LAUNCH:-hc}" = "ark" ] \
    || [ -f "$LOCAL_DEV_DIR/$name/ark/passport.json" ]; then
    echo "next-launch:${REPLICATION_INTERVAL_SECONDS:-10}s"
  else
    echo "60s(default)"
  fi
}

# Transport stamp for evidence produced by `just test mesh`. The launcher knob
# says what a future start/restart requests; it is not proof of what the running
# peers mounted. `/p2p/status.irohNodeId` is emitted only by a peer whose iroh
# plane is actually co-resident with libp2p; an iroh-only peer instead reports
# its 64-hex NodeId as `peerId`. Read EVERY peer so mixed or partially restarted
# meshes are named peer-by-peer; unreadable peers still make the stamp unknown.
mesh_transport_backend_from_status() {
  local i=0 status dual=0 iroh=0 libp2p=0 unreadable=0
  for _name in "${PEERS[@]}"; do
    if status="$(curl -fsS -m 3 "http://localhost:$(http_port "$i")/p2p/status" 2>/dev/null)"; then
      if jq -e '.irohNodeId | type == "string" and length > 0' >/dev/null 2>&1 <<<"$status"; then
        dual=$((dual + 1))
      elif jq -e '.peerId | type == "string" and test("^[0-9a-fA-F]{64}$")' >/dev/null 2>&1 <<<"$status"; then
        iroh=$((iroh + 1))
      else
        libp2p=$((libp2p + 1))
      fi
    else
      unreadable=$((unreadable + 1))
    fi
    i=$((i + 1))
  done
  if [ "$unreadable" -gt 0 ]; then
    echo unknown
  elif [ "$dual" -eq "${#PEERS[@]}" ]; then
    echo dual
  elif [ "$iroh" -eq "${#PEERS[@]}" ]; then
    echo iroh
  elif [ "$libp2p" -eq "${#PEERS[@]}" ]; then
    echo libp2p
  else
    # Mixed by design (MESH_PEER_TRANSPORTS): name each peer's live plane.
    local j=0 out="" st
    for _name in "${PEERS[@]}"; do
      st="$(curl -fsS -m 3 "http://localhost:$(http_port "$j")/p2p/status" 2>/dev/null)"
      if jq -e '.irohNodeId | type == "string" and length > 0' >/dev/null 2>&1 <<<"$st"; then out+="${out:+,}$_name=dual"
      elif jq -e '.peerId | type == "string" and test("^[0-9a-fA-F]{64}$")' >/dev/null 2>&1 <<<"$st"; then out+="${out:+,}$_name=iroh"
      else out+="${out:+,}$_name=libp2p"; fi
      j=$((j + 1))
    done
    echo "$out"
  fi
}

# ---------------------------------------------------------------------------
# dev-tier pacing profile (declared preproduction stakes — see minutes-quiesce
# plan W3: genesis/docs/superpowers/plans/2026-08-16-minutes-quiesce-fixture-
# trust-swarm-plan.md §3). Same reconcile/backoff machinery elohim-storage
# runs everywhere — never a parallel dev path — just tuned to a declared
# preproduction cadence so the 3-peer local mesh converges in minutes
# instead of the ~90min baseline (.claude/shifts/2026-08-16T04-15-local-
# mesh-saga-delivery.journal.md). Every knob overrides via its own MESH_*
# var; defaults below are the dev-tier stakes, never applied silently in
# prod (this file is dev-only). Names verified against elohim-storage/src/
# {main.rs,config.rs} 2026-08-16 — these are the exact env vars the storage
# binary reads, not aliases:
#   PROJECTION_RECONCILE_SECS            reconcile sweep cadence (prod default 300s)
#   ACQUISITION_RECONCILE_SECS           acquisition/provide pin reconcile tick (prod default 60s)
#   CONTEST_BACKOFF_SECONDS               contest-backoff ladder rung (prod default 3600s)
#   HEAL_MISSING_BACKOFF_SECONDS          heal-missing backoff rung (prod default 600s)
#   ELOHIM_EVIDENCE_ABSENT_BACKOFF_SECS   evidence-absent backoff rung (prod default 86400s)
#   ELOHIM_HEAD_CORPUS_DIGEST             T5 digest-requester flip (prod default off/0)
#   ELOHIM_NETWORK_STAKES                 T10 declared-stakes operator-config leg (prod:
#                                          unset -> Bootstrap fail-closed default). The
#                                          explicit Simulacra declaration for THIS local
#                                          preproduction mesh only — Simulacra is never a
#                                          default and never derived from DEV_MODE; it is
#                                          reached only by this positive declaration
#                                          (trust::manifest_resolver::ManifestStakesResolver,
#                                          T10 runtime half, §3 W2 task Q6).
#
# Two more storage-peer levers below (not part of the pacing profile — always
# on, unconditionally, for every peer): ALLOW_SEED_NETWORK_STAKES=1 and
# ALLOW_SEED_DELEGATES_COMPUTE=1 (elohim-storage/src/api/seed_network_stakes.rs
# / seed-delegates-compute route). Both gate a seed-only HTTP lever behind a
# 403 by default; the Act I Prologue's stage-manifest leg and
# seed-delegates-compute.ts honest-fail without them. Mesh-only preproduction
# levers, same as ELOHIM_NETWORK_STAKES=simulacra above — never a prod default.
# ALLOW_SEED_SHARD_MANIFEST=1 is the third (services/seed_shard_manifest.rs; alpha
# sets it per env in elohim/holochain/Jenkinsfile) — grandma-photos's four
# scenarios pend on a 403 without it.
# ---------------------------------------------------------------------------
PROJECTION_RECONCILE_SECS="${MESH_RECONCILE_SECS:-30}"
# Acquisition/provide pin reconcile tick. Chapter 11's exhaustion scenario can
# only WAIT for this loop (retry budget = max(3, peers) probes, one per tick,
# then retire on the next): at the prod 60s that is ~5 min of a 5-min saga.
ACQUISITION_RECONCILE_SECS="${MESH_ACQUISITION_RECONCILE_SECS:-10}"
CONTEST_BACKOFF_SECONDS="${MESH_CONTEST_BACKOFF:-120}"
HEAL_MISSING_BACKOFF_SECONDS="${MESH_HEAL_MISSING_BACKOFF:-60}"
ELOHIM_EVIDENCE_ABSENT_BACKOFF_SECS="${MESH_EVIDENCE_ABSENT_BACKOFF:-600}"
ELOHIM_HEAD_CORPUS_DIGEST="${MESH_HEAD_CORPUS_DIGEST:-1}"
# Adopt-before-author pre-flight ON for the mesh: without it cross-peer head
# divergence has no adopt discharge and accumulates into contest grind (the
# 2026-08-16 measure's 23→81 actionable plateau); the overnight converged run
# had it live (adopt_peer canonical links). Prod default stays off.
ELOHIM_ADOPT_BEFORE_AUTHOR="${MESH_ADOPT_BEFORE_AUTHOR:-1}"
# Carry-the-election (2026-08-31): let the obey arm supply a missing election
# from the advertising peer, re-derived in wasm by the OWN conductor. Mesh
# default ON (the mesh is where the capability is proven); prod ships dormant
# behind ELOHIM_OBEY_CARRIED_ELECTION.
ELOHIM_OBEY_CARRIED_ELECTION="${MESH_OBEY_CARRIED_ELECTION:-1}"
# Rung-1 coordinator hot-swap vehicle (2026-08-31): allow the conductor's
# update_coordinators hot-swap on the mesh by default — the mesh is where
# coordinator rollouts are proven before the fleet. Prod gates this per-env.
ALLOW_COORDINATOR_UPDATE="${MESH_ALLOW_COORDINATOR_UPDATE:-true}"
# Serialize adopt/contest declares on the mesh: concurrent declares race the
# conductor source-chain head ("bundle head has moved", 2026-08-16 measure) and
# every collision costs a fallback + next-sweep retry — fanout 1 lands first-try.
# Declared profile difference vs alpha (6); attributed in the transfer ratio.
ADOPT_CONTEST_FANOUT="${MESH_ADOPT_FANOUT:-1}"
# Explicit preproduction Simulacra declaration for the local mesh's storage peers
# (never a default — see the comment block above).
ELOHIM_NETWORK_STAKES="${MESH_NETWORK_STAKES:-simulacra}"

# ---------------------------------------------------------------------------
# Conductor log level (MESH_RUST_LOG). Holochain defaults to ERROR when RUST_LOG
# is unset, and the three lines that diagnose a sys-validation spin are all
# INFO: the DHT read-pool saturation line (holochain_sqlite::db::access), the
# cascade's NoPeersForLocation (holochain_cascade), and sys-validation's own
# "N fetched of M missing dependencies" counter. Without this, those lines are
# not merely absent from .sandbox_run_log — they are UNOBSERVABLE, and any tool
# reading that file reports a confident zero that means nothing. Measured
# 2026-08-21 on this mesh before the fix: 86 ERROR lines, 0 INFO.
#
# Deliberately TARGETED rather than a blanket `info`: a blanket level buries the
# shared run log (all N conductors multiplex into one prefix-less file) under
# gossip chatter and makes it useless to read by eye. Everything else stays at
# warn; kitsune2_gossip is pinned to warn explicitly because it is the loudest
# module at info and says nothing about this class.
#
# Diagnosing something else? Override the whole string:
#   MESH_RUST_LOG="warn,holochain_p2p=debug" ./hc-mesh.sh conductors-restart
# ---------------------------------------------------------------------------
MESH_RUST_LOG="${MESH_RUST_LOG:-warn,holochain_sqlite::db::access=info,holochain::core::workflow::sys_validation_workflow=info,holochain_cascade=info,kitsune2_gossip=warn}"

# Alpha identity model (edgenode template): each storage node self-heals its
# own human's agent_pub_key from its conductor cell key, NULL-only. Without
# this env the saga ch02 finish line (non-null agentPubKey) can never light.
human_id() {
  case "$1" in
    matthew) echo human-matthew-manager ;;
    jessica) echo human-jessica-spouse ;;
    james)   echo human-james-son ;;
    *)       echo "human-$1" ;;
  esac
}

peer_csv() { # name=host:port CSV for substrate-verify / PEER_STORAGE_URLS
  local out="" i=0
  for name in "${PEERS[@]}"; do
    out+="${out:+,}$name=localhost:$(http_port $i)"; i=$((i+1))
  done
  echo "$out"
}

conductor_csv() { # name=ws://localhost:PORT CSV for named CONDUCTOR_URLS —
  # the cast fix (backlog mesh-prologue-cast-and-env-gaps.md): an unnamed
  # loopback conductor URL resolves by first-reachable-wins, which cast
  # Adam onto james's conductor on 2026-08-21. Named entries let
  # seed-conductor-identities.ts / seed-agent-bindings.ts /
  # seed-household-formation.ts bind each human to ITS OWN conductor.
  local out="" i=0
  for name in "${PEERS[@]}"; do
    out+="${out:+,}$name=ws://localhost:$(app_port $i)"; i=$((i+1))
  done
  echo "$out"
}

peer_url_csv() { # http://localhost:PORT CSV, no names — SEEDER_TARGET_PEERS shape
  local out="" i=0
  for _ in "${PEERS[@]}"; do
    out+="${out:+,}http://localhost:$(http_port $i)"; i=$((i+1))
  done
  echo "$out"
}

mesh_seed_env() { # ONE source of truth for the seed-chain-facing env block.
  # Exports (not echoes) so both hc-mesh-prologue.sh and an operator's shell
  # (`source hc-mesh.sh && mesh_seed_env`) populate identically — the named
  # CONDUCTOR_URLS cast fix plus the admin credential the seed chain needs to
  # clear /admin/* 403s (see the doorway launch comments below).
  export CONDUCTOR_URLS="$(conductor_csv)"
  export HOLOCHAIN_ADMIN_URL="ws://localhost:$(admin_port 0)"
  export STORAGE_URL="http://localhost:$(http_port 0)"
  export DOORWAY_URL="http://localhost:$DOORWAY_PORT"
  export PEER_STORAGE_URLS="$(peer_csv)"
  export SEEDER_TARGET_PEERS="$(peer_url_csv)"
  export API_KEY_ADMIN="${MESH_API_KEY_ADMIN:-mesh-admin-dev-key}"
  export DOORWAY_API_KEY="$API_KEY_ADMIN"
  export STORAGE_API_KEY_ADMIN="$API_KEY_ADMIN"
}

# ---------------------------------------------------------------------------
# HOLOCHAIN_BIN — which conductor binary the sandboxes run.
#
# Empty (default) = whatever `holochain` is on PATH, i.e. the stock build. Set
# it to an absolute path to run a fork instead, for an A/B where the ONLY thing
# that changes is the conductor:
#
#   HOLOCHAIN_BIN=/path/to/fork/holochain ./hc-mesh.sh conductors-restart
#
# `hc sandbox` takes the binary via HC_HOLOCHAIN_PATH (equivalently -H), which
# is the mechanism this uses; the binary's directory is also prepended to PATH
# so anything else the run step shells out to resolves consistently.
#
# A binary swap alone CANNOT move a DNA hash: the hash covers integrity zomes +
# modifiers, both of which live in the already-installed sandboxes that
# `conductors-restart` reuses. Verify rather than assume — compare
# `dhtParticipation.dnaHashes` from any storage peer's /health before and after.
# ---------------------------------------------------------------------------
# Default: the fork build, when one is present.
#
# PARITY IS THE POINT. Alpha runs the conductor FORK; for months the local mesh
# ran whatever stock `holochain` happened to be on PATH, which means the proving
# ground was not running the fleet's conductor. A local QUIET then says nothing
# about alpha, and a local reproduction attempt is testing a different program.
# So if a fork build is present, it is used unless HOLOCHAIN_BIN says otherwise.
#
# MESH_FORK_BIN_DIRS is searched in order for a directory containing BOTH
# `holochain` and `hc` — both, because the CLI writes conductor-config.yaml in
# ITS schema and a mismatched pair simply will not boot (for example, a 0.6 CLI
# writes transport keys that a 0.7 conductor rejects). A directory with only one
# of them is skipped, loudly, rather than
# half-adopted.
# NOTE the cargo-pool slot where a local fork build lands
# (/projects/.cargo-target-pool/family/dev/crates/dev/release) is deliberately
# NOT in this list. Auto-detect must mean "someone put a conductor here on
# purpose", and a build-output directory appears merely because someone ran
# cargo — swapping the mesh's conductor on that evidence is the silent switch
# this file already paid for once. Point HOLOCHAIN_BIN at it explicitly:
#   HOLOCHAIN_BIN=/projects/.cargo-target-pool/family/dev/crates/dev/release just mesh start
MESH_FORK_BIN_DIRS="${MESH_FORK_BIN_DIRS:-$MESH_DIR/fork-bin:$REPO_ROOT/.fork-bin:/opt/elohim/fork-bin}"

detect_fork_bin() { # -> prints the fork dir, or nothing
  local d
  IFS=':' read -ra _dirs <<< "$MESH_FORK_BIN_DIRS"
  for d in "${_dirs[@]}"; do
    [ -n "$d" ] || continue
    if [ -x "$d/holochain" ] && [ -x "$d/hc" ]; then echo "$d"; return 0; fi
    if [ -x "$d/holochain" ] && [ ! -x "$d/hc" ]; then
      echo "WARN: $d has holochain but no matching hc — skipping (the CLI must match the conductor's config schema)" >&2
    fi
  done
  return 1
}

# ---------------------------------------------------------------------------
# HOLOCHAIN_BIN accepts a BINARY or a DIRECTORY holding `holochain` + `hc`.
#
# A directory used to be rejected by the `[ -x ]` guard, and the start SILENTLY
# fell through to the auto-detected fork conductor while `hc` stayed stock
# (2026-08-21 19:14, parity attempt 2): `generate` then wrote 0.6.0-schema
# configs and the stock `hc sandbox run` panicked at hc_sandbox/src/run.rs:176
# on all three conductors, storage and doorways came up, and the Prologue ran
# against dead conductors. A directory is the shape a person naturally has (a
# build output dir), so accept it — and take the matching `hc` with it.
# ---------------------------------------------------------------------------
if [ -n "${HOLOCHAIN_BIN:-}" ] && [ -d "$HOLOCHAIN_BIN" ]; then
  if [ -x "$HOLOCHAIN_BIN/holochain" ] && [ -x "$HOLOCHAIN_BIN/hc" ]; then
    HC_BIN_DIR="${HOLOCHAIN_BIN%/}"
    HOLOCHAIN_BIN="$HC_BIN_DIR/holochain"
  else
    echo "HOLOCHAIN_BIN is a directory but does not hold BOTH holochain and hc: $HOLOCHAIN_BIN" >&2
    echo "  (the CLI writes conductor-config.yaml in ITS schema — a half-adopted pair cannot boot)" >&2
    exit 1
  fi
fi

FORK_BIN_DIR="$(detect_fork_bin || true)"
if [ -z "${HOLOCHAIN_BIN:-}" ] && [ -n "$FORK_BIN_DIR" ]; then
  HOLOCHAIN_BIN="$FORK_BIN_DIR/holochain"
fi
HOLOCHAIN_BIN="${HOLOCHAIN_BIN:-}"

# The matching CLI has to win PATH for **generate AND run**, not just run.
# `hc sandbox` REWRITES conductor-config.yaml (that is how `-f` pins admin
# ports), so the CLI decides the schema the conductor is then handed. The old
# code prepended PATH only inside holochain_bin_export(), which the `run` sites
# call and the `generate` site does not — that asymmetry is exactly how a fork
# conductor got 0.6.0-schema configs. Do it ONCE, here, at script scope.
if [ -n "$HOLOCHAIN_BIN" ]; then
  HC_BIN_DIR="${HC_BIN_DIR:-$(dirname "$HOLOCHAIN_BIN")}"
  [ -x "$HC_BIN_DIR/hc" ] && export PATH="$HC_BIN_DIR:$PATH"
fi

# Export the conductor-binary selection into the CURRENT shell. Call it inside a
# subshell right before launching `hc sandbox`.
#
# It is a function that exports rather than one that prints an env prefix,
# because `VAR=x $(prefix_fn) prog` does NOT do what it looks like: the command
# substitution expands into the COMMAND WORD position and bash tries to execute
# "HC_HOLOCHAIN_PATH=..." as a program. That failed silently into the conductor
# log ("No such file or directory") while every conductor stayed down.
holochain_bin_export() {
  [ -n "$HOLOCHAIN_BIN" ] || return 0
  [ -x "$HOLOCHAIN_BIN" ] || { echo "HOLOCHAIN_BIN is not executable: $HOLOCHAIN_BIN" >&2; exit 1; }
  export HC_HOLOCHAIN_PATH="$HOLOCHAIN_BIN"
  export PATH="$(dirname "$HOLOCHAIN_BIN"):$PATH"
}

hc_version()        { hc --version 2>&1 | head -1 | awk '{print $NF}'; }
conductor_version() {
  if [ -n "$HOLOCHAIN_BIN" ]; then "$HOLOCHAIN_BIN" --version 2>&1 | head -1 | awk '{print $NF}'
  else holochain --version 2>&1 | head -1 | awk '{print $NF}'; fi
}

# ---------------------------------------------------------------------------
# REFUSE TO START ON A MISMATCHED PAIR.
#
# The `hc` CLI writes the conductor's config; the conductor parses it. When they
# are different builds the config is written in one schema and read in another,
# and the failure surfaces as a config PARSE error at boot with the reason only
# in the conductor log. At 0.7 the removed transport fields are unknown keys,
# while older CLIs also omit relay_url; either skew fails before boot.
#
# The gate compares the FULL version, not the major.minor "minor line": patch
# releases have previously carried incompatible config schemas, so a line check
# would have passed the exact pair that cost the evening. Both versions are
# printed either way so the mismatch is readable, not inferred.
#
# MESH_ALLOW_TOOLCHAIN_SKEW=1 downgrades the refusal to a warning for the one
# case where it is deliberate (proving what a skewed pair does).
# ---------------------------------------------------------------------------
assert_toolchain_parity() {
  local hv cv
  hv="$(hc_version)"; cv="$(conductor_version)"
  if [ "$hv" = "$cv" ]; then
    echo "toolchain: holochain $cv + hc $hv  ($(command -v hc))"
    return 0
  fi
  echo "" >&2
  echo "REFUSING TO START — the conductor and the hc CLI are different builds:" >&2
  echo "  holochain: $(if [ -n "$HOLOCHAIN_BIN" ]; then echo "$HOLOCHAIN_BIN"; else command -v holochain; fi)  -> $cv" >&2
  echo "  hc:        $(command -v hc)  -> $hv" >&2
  echo "" >&2
  echo "  \`hc sandbox\` REWRITES conductor-config.yaml in its own schema, so the" >&2
  echo "  conductor is handed a file it may refuse to parse (for example, a 0.6" >&2
  echo "  hc paired with a 0.7 conductor). Point HOLOCHAIN_BIN at a DIRECTORY" >&2
  echo "  holding both matching binaries:" >&2
  echo "     HOLOCHAIN_BIN=/path/to/fork-bin ./hc-mesh.sh start" >&2
  echo "  (MESH_ALLOW_TOOLCHAIN_SKEW=1 to proceed anyway — deliberate skew only.)" >&2
  echo "" >&2
  [ "${MESH_ALLOW_TOOLCHAIN_SKEW:-0}" = "1" ] || exit 1
  echo "WARN: proceeding on a skewed pair because MESH_ALLOW_TOOLCHAIN_SKEW=1" >&2
}

# ---------------------------------------------------------------------------
# Holochain 0.7 has one iroh transport; the matching hc CLI accepts the `quic`
# network tail and writes relay_url in the conductor's 0.7 schema.
#
# The relay is NOT a NAT-traversal nicety on a loopback mesh. A 0.7 conductor
# homes to relay_url at boot, advertises it in its agent info, and kitsune2 0.5
# dials only peers whose relay matches its own exactly (one relay per space).
# With relay_url pointed at the doorway — the pre-0.7 "parseable placeholder" —
# three loopback conductors booted clean and reported 0 connections for the
# whole prologue (2026-09-03). So the mesh launches a real iroh-relay
# (start_local_relay) and every conductor is generated against it. It must be
# a URL either way — `relay_url: null` is rejected with
# `relative URL without a base: "null"`.
# ---------------------------------------------------------------------------
mesh_relay_url() { echo "${MESH_FORK_RELAY_URL:-http://localhost:$MESH_RELAY_PORT/}"; }

mesh_network_args() { # -> the `network …` tail for `hc sandbox generate`
  echo "network --bootstrap http://localhost:$DOORWAY_PORT/bootstrap quic $(mesh_relay_url)"
}

# Launch the local iroh-relay unless the operator pointed MESH_FORK_RELAY_URL
# elsewhere or set MESH_RELAY=0. Plain HTTP (`--dev`), loopback-only; the
# conductors are generated with relayAllowPlainText so an http:// relay is
# accepted. The config file is what proves ownership to fallback_pattern_pids.
start_local_relay() {
  local url; url="$(mesh_relay_url)"
  if [ "${MESH_RELAY:-1}" != "1" ]; then
    echo "relay: not launched (MESH_RELAY=0); conductors home to $url"; return 0
  fi
  case "$url" in
    "http://localhost:$MESH_RELAY_PORT/"|"http://127.0.0.1:$MESH_RELAY_PORT/") ;;
    *) echo "relay: external ($url); not launching a local iroh-relay"; return 0 ;;
  esac
  if curl -s -m 2 -o /dev/null "http://localhost:$MESH_RELAY_PORT/"; then
    record_listener_pid relay a "$MESH_RELAY_PORT" || true
    echo "iroh-relay already up on :$MESH_RELAY_PORT"; return 0
  fi
  if [ -z "$MESH_RELAY_BIN" ] || [ ! -x "$MESH_RELAY_BIN" ]; then
    echo "ERROR: no iroh-relay binary (MESH_RELAY_BIN unset and none on PATH)." >&2
    echo "       A 0.7 conductor needs a reachable relay or it never connects. Build one:" >&2
    echo "         RUSTFLAGS=\"\" cargo install iroh-relay --version 1.0.3 --locked --features server --root <dir>" >&2
    echo "       then MESH_RELAY_BIN=<dir>/bin/iroh-relay — or MESH_RELAY=0 MESH_FORK_RELAY_URL=<reachable relay>." >&2
    return 1
  fi
  cat > "$MESH_DIR/iroh-relay.toml" <<EOF
# Written by hc-mesh.sh — the local relay every 0.7 conductor on this mesh
# homes to. Keys mirror genesis/orchestrator/manifests/doorway/alpha.yaml's
# relay.toml minus TLS/metrics (--dev ignores TLS anyway).
enable_relay = true
http_bind_addr = "127.0.0.1:$MESH_RELAY_PORT"
enable_quic_addr_discovery = false
key_cache_capacity = 8192
access = "everyone"
enable_metrics = false
EOF
  RUST_LOG="${MESH_RELAY_RUST_LOG:-warn}" \
    nohup "$MESH_RELAY_BIN" --dev -c "$MESH_DIR/iroh-relay.toml" > "$LOGDIR/iroh-relay.log" 2>&1 &
  record_mesh_pid relay a "$!" || true
  for _ in $(seq 1 20); do
    curl -s -m 2 -o /dev/null "http://localhost:$MESH_RELAY_PORT/" && break; sleep 1
  done
  if ! curl -s -m 2 -o /dev/null "http://localhost:$MESH_RELAY_PORT/"; then
    echo "ERROR: iroh-relay did not come up on :$MESH_RELAY_PORT — see $LOGDIR/iroh-relay.log" >&2
    return 1
  fi
  echo "iroh-relay up on :$MESH_RELAY_PORT ($("$MESH_RELAY_BIN" --version 2>/dev/null || echo iroh-relay))"
}

patch_mesh_gossip_config() { # <conductor-config.yaml>
  python3 - "$1" <<'PYEOF'
import sys
import yaml

path = sys.argv[1]
with open(path) as f:
    cfg = yaml.safe_load(f)

network = cfg.setdefault("network", {})
advanced = network.get("advanced") or {}
k2gossip = advanced.get("k2Gossip") or {}
k2gossip["initiateIntervalMs"] = 1000
k2gossip["minInitiateIntervalMs"] = 1000
k2gossip["initialInitiateIntervalMs"] = 1000
advanced["k2Gossip"] = k2gossip

network["advanced"] = advanced

with open(path, "w") as f:
    yaml.safe_dump(cfg, f, default_flow_style=False, sort_keys=False)
PYEOF
}

recorded_mesh_pids() {
  local file pid started current
  [ -d "$PID_DIR" ] || return 0
  while IFS= read -r file; do
    pid=""; started=""
    read -r pid started < "$file" || true
    if [[ "$pid" =~ ^[0-9]+$ ]] && [ -n "${started:-}" ]; then
      current="$(process_start_ticks "$pid")"
      if [ -n "$current" ] && [ "$current" = "$started" ]; then
        echo "$pid"
        continue
      fi
    fi
    # Dead or reused: it is no longer the process this mesh launched.
    rm -f "$file"
  done < <(find "$PID_DIR" -maxdepth 1 -type f -print 2>/dev/null)
}

terminate_mesh_pids() { # <pid>...
  local unique=() pid alive deadline
  for pid in "$@"; do
    [[ "$pid" =~ ^[0-9]+$ ]] || continue
    [ "$pid" = "$$" ] && continue
    case " ${unique[*]} " in *" $pid "*) ;; *) unique+=("$pid") ;; esac
  done
  [ ${#unique[@]} -gt 0 ] || return 0

  echo "stopping ${#unique[@]} mesh process(es) by recorded pid / owned port"
  kill "${unique[@]}" 2>/dev/null || true
  deadline=$((SECONDS + 10))
  while [ "$SECONDS" -lt "$deadline" ]; do
    alive=0
    for pid in "${unique[@]}"; do kill -0 "$pid" 2>/dev/null && alive=$((alive+1)); done
    [ "$alive" -eq 0 ] && return 0
    sleep 1
  done
  for pid in "${unique[@]}"; do
    kill -0 "$pid" 2>/dev/null && kill -9 "$pid" 2>/dev/null || true
  done
}

stop_recorded_arks() {
  local ark_names=() ark_pids=() file name pid deadline alive i failed=0
  [ -d "$PID_DIR" ] || return 0

  for file in "$PID_DIR"/ark-*; do
    [ -f "$file" ] || continue
    name="${file##*/ark-}"
    if pid="$(live_recorded_pid ark "$name")"; then
      ark_names+=("$name")
      ark_pids+=("$pid")
    fi
  done
  [ ${#ark_pids[@]} -gt 0 ] || return 0

  # Signal every supervisor before waiting so no conductor child is included in
  # the ordinary recorded-pid pass while its restart-owning ark is still alive.
  kill -TERM "${ark_pids[@]}" 2>/dev/null || true
  deadline=$((SECONDS + 25))
  while [ "$SECONDS" -lt "$deadline" ]; do
    alive=0
    for i in "${!ark_pids[@]}"; do
      if [ "$(live_recorded_pid ark "${ark_names[$i]}" 2>/dev/null || true)" = "${ark_pids[$i]}" ]; then
        alive=1
        break
      fi
    done
    [ "$alive" -eq 0 ] && break
    sleep 1
  done

  for i in "${!ark_pids[@]}"; do
    name="${ark_names[$i]}"
    pid="${ark_pids[$i]}"
    if [ "$(live_recorded_pid ark "$name" 2>/dev/null || true)" = "$pid" ]; then
      echo "stopping ark $name pid=$pid … still alive after 25s" >&2
      failed=1
    else
      echo "stopping ark $name pid=$pid … exited"
    fi
  done
  return "$failed"
}

fallback_pattern_pids() {
  # Compatibility for meshes launched before PID files existed, or platforms
  # where ss can see a listener but not its owner. The pgrep patterns only
  # NOMINATE candidates; /proc/exe must prove each is an actual service binary,
  # so a shell whose argv merely contains a binary path is never killed.
  local pid exe args cwd port owned
  while IFS= read -r pid; do
    [ -n "$pid" ] || continue
    exe="$(readlink "/proc/$pid/exe" 2>/dev/null)"
    exe="${exe% (deleted)}"; exe="${exe##*/}"
    args="$(tr '\0' ' ' < "/proc/$pid/cmdline" 2>/dev/null)"
    cwd="$(readlink "/proc/$pid/cwd" 2>/dev/null)"
    owned=0
    case "$exe" in
      holochain) [[ "$args" == *"$LOCAL_DEV_DIR/"*"/conductor-config.yaml"* ]] && owned=1 ;;
      hc) [ "$cwd" = "$LOCAL_DEV_DIR" ] && [[ "$args" == *" sandbox "*" run"* ]] && owned=1 ;;
      elohim-storage)
        while IFS= read -r port; do
          [[ "$args" == *"--http-port $port"* ]] && { owned=1; break; }
        done < <(mesh_owned_ports) ;;
      doorway)
        [[ "$args" == *"--listen 0.0.0.0:$DOORWAY_PORT"* || \
           "$args" == *"--listen 0.0.0.0:$DOORWAY_B_PORT"* ]] && owned=1 ;;
      mongod) [[ "$args" == *"--dbpath $MONGO_DIR"* ]] && owned=1 ;;
      iroh-relay) [[ "$args" == *"$MESH_DIR/iroh-relay.toml"* ]] && owned=1 ;;
    esac
    [ "$owned" -eq 1 ] && echo "$pid"
  done < <({
    pgrep -x holochain 2>/dev/null
    pgrep -f "[h]c sandbox" 2>/dev/null
    pgrep -f "elohim-storag[e]" 2>/dev/null
    pgrep -f "(debug|release)/doorwa[y]" 2>/dev/null
    pgrep -f "mongod --dbpath $MESH_DIR/mong[o]" 2>/dev/null
    pgrep -x iroh-relay 2>/dev/null
  } | sort -un)
}

mesh_ports_busy() {
  local port
  while IFS= read -r port; do
    ss -H -ltn "sport = :$port" 2>/dev/null | grep -q . && return 0
    ss -H -lun "sport = :$port" 2>/dev/null | grep -q . && return 0
  done < <(mesh_owned_ports)
  return 1
}

clear_mesh_pidfiles() {
  local file
  [ -d "$PID_DIR" ] || return 0
  while IFS= read -r file; do rm -f "$file"; done \
    < <(find "$PID_DIR" -maxdepth 1 -type f -print 2>/dev/null)
  rmdir "$PID_DIR" 2>/dev/null || true
}

stop_all() {
  local pids=() fallback=() pid
  stop_recorded_arks || return 1
  while IFS= read -r pid; do pids+=("$pid"); done < <(recorded_mesh_pids)
  while IFS= read -r pid; do pids+=("$pid"); done \
    < <(listener_pids_for_ports $(mesh_owned_ports))
  terminate_mesh_pids "${pids[@]}"

  # Pattern matching is deliberately last. It runs only when the exact
  # ownership paths found nothing or a declared mesh port remains occupied.
  if [ ${#pids[@]} -eq 0 ] || mesh_ports_busy; then
    while IFS= read -r pid; do fallback+=("$pid"); done < <(fallback_pattern_pids)
    if [ ${#fallback[@]} -gt 0 ]; then
      echo "WARN: PID/port shutdown was incomplete; using validated process-name fallback" >&2
      terminate_mesh_pids "${fallback[@]}"
    fi
  fi

  clear_mesh_pidfiles
  if mesh_ports_busy; then
    echo "WARN: mesh stopped, but one or more declared mesh ports remain occupied" >&2
    return 1
  fi
  echo "mesh stopped"
}

# What the mesh COSTS, read from ps — so "can we afford this locally" is a
# number in the same command that reports health. Conductors are ~2 GB RSS
# each (measured 2026-08-24: 3 conductors = 6.0 of a 6.9 GB mesh); that is why
# the recovery harness runs two peers, not three.
mesh_footprint() {
  local total=0
  while IFS= read -r line; do
    set -- $line; local pid="$1" rss="$2" cpu="$3"; shift 3
    local role="other" name="-"
    case "$*" in
      *holochain*--config-path*) role=conductor; name="$(sed -n 's#.*/local-dev/\([^/]*\)/.*#\1#p' <<<"$*")" ;;
      *elohim-storage*--http-port*) role=storage; name="$(sed -n 's/.*--http-port \([0-9]*\).*/\1/p' <<<"$*")" ;;
      *"/doorway "*--listen*) role=doorway; name="$(sed -n 's/.*--listen [^:]*:\([0-9]*\).*/\1/p' <<<"$*")" ;;
      *mongod*"$MESH_DIR"*) role=mongod ;;
      *iroh-relay*"$MESH_DIR/iroh-relay.toml"*) role=relay; name="$MESH_RELAY_PORT" ;;
      *) continue ;;
    esac
    printf 'footprint %-9s %-8s rss=%dMB cpu=%s%%\n' "$role" "$name" $((rss / 1024)) "$cpu"
    total=$((total + rss / 1024))
  done < <(ps -eo pid=,rss=,pcpu=,args= | grep -E "holochain --piped|elohim-storage.*--http-port|/doorway .*--listen|mongod --dbpath|iroh-relay --dev" | grep -v grep)
  echo "footprint total rss=${total}MB"
}

status_all() {
  echo "conductors:"; ss -tln 2>/dev/null | grep -E "127.0.0.1:44[0-9]{2} " || echo "  (none)"
  local i=0 port transport spool_path replication_interval data_root_status=0
  echo "conductor data roots:"
  report_conductor_data_roots || data_root_status=1
  for name in "${PEERS[@]}"; do
    port="$(http_port $i)"
    transport="$(storage_transport_for "$name" "$port")"
    spool_path="$(storage_spool_path_for "$name" "$port")"
    replication_interval="$(storage_replication_interval_for "$name" "$port")"
    printf "  %-8s admin=%s app=%s  storage=" "$name" "$(admin_port $i)" "$(app_port $i)"
    if curl -s -m 2 "http://localhost:$port/health" >/dev/null; then
      echo "UP :$port transport=$transport spool=$spool_path replication_interval=$replication_interval"
    else
      echo "down transport=$transport spool=$spool_path replication_interval=$replication_interval"
    fi
    i=$((i+1))
  done
  ark_status_rows
  if [ "$MESH_DOORWAYS_EFFECTIVE" = "1" ]; then
    printf "doorway  :%s " "$DOORWAY_PORT"
    curl -s -m 2 "http://localhost:$DOORWAY_PORT/health" >/dev/null && echo UP || echo down
    printf "doorwayB :%s " "${DOORWAY_B_PORT:-8889}"
    curl -s -m 2 "http://localhost:${DOORWAY_B_PORT:-8889}/health" >/dev/null && echo UP || echo down
    printf "portal   :%s " "$THRESHOLD_PORT"
    if curl -s -m 2 -o /dev/null "http://localhost:$DOORWAY_PORT/threshold/login"; then
      case "$(curl -s -m 2 -o /dev/null -w '%{http_code}' "http://localhost:$DOORWAY_PORT/threshold/login")" in
        200) echo "UP (/threshold/login serves through the doorway)" ;;
        502) echo "down (doorway proxies /threshold/* here; nothing listening)" ;;
        *)   echo "down" ;;
      esac
    else
      echo "down"
    fi
  else
    echo "doorways: disabled (MESH_DOORWAYS=0)"
  fi
  printf "relay    :%s " "$MESH_RELAY_PORT"
  if curl -s -m 2 -o /dev/null "http://localhost:$MESH_RELAY_PORT/"; then echo "UP (conductors home to $(mesh_relay_url))"; else echo "down (0.7 conductors report 0 connections without a reachable relay)"; fi
  printf "mongod   :%s " "$MONGO_PORT"
  if (exec 3<>"/dev/tcp/127.0.0.1/$MONGO_PORT") 2>/dev/null; then echo "UP (archive-backed doorways)"; else echo "down (doorways run archive-less: inert warm shell)"; fi
  mesh_footprint
  echo
  # What is ACTUALLY RUNNING comes first, read from /proc — not what the next
  # launch would choose. Those differ whenever someone overrides HOLOCHAIN_BIN
  # for one restart, and a status line that reports the intention instead of the
  # fact is exactly the kind of confident-wrong measure this mesh keeps teaching
  # us to distrust.
  local _running_bin _running_desc
  _running_bin="$(for pid in $(ps -eo pid=,args= | awk -v me="$$" '$1 != me && index($0, "--config-path") && index($0, "/local-dev/") { print $1 }'); do
      readlink "/proc/$pid/exe" 2>/dev/null | sed 's/ (deleted)$//'; done | sort -u | head -1)"
  if [ -n "$_running_bin" ]; then
    _running_desc="$_running_bin ($("$_running_bin" --version 2>&1 | head -1))"
    case "$_running_bin" in
      *fork-bin*) _running_desc="$_running_desc [FORK]" ;;
      *) _running_desc="$_running_desc [STOCK — alpha runs the fork, so this mesh is NOT at parity]" ;;
    esac
  else
    _running_desc="(no conductor running)"
  fi
  echo "conductor RUNNING: $_running_desc"

  local _hc_desc
  if [ -n "$HOLOCHAIN_BIN" ]; then
    _hc_desc="$HOLOCHAIN_BIN ($("$HOLOCHAIN_BIN" --version 2>&1 | head -1))$([ -n "$FORK_BIN_DIR" ] && printf ' [FORK, auto-detected]' || printf ' [explicit HOLOCHAIN_BIN]')"
  else
    _hc_desc="$(command -v holochain) ($(holochain --version 2>&1 | head -1)) [STOCK — alpha runs the fork, so this mesh is NOT at parity]"
  fi
  echo "conductor NEXT LAUNCH: $_hc_desc"
  # The CLI is half the boot: it writes the config the conductor must parse, so
  # a version skew here is a start failure waiting to happen, not a detail.
  echo "hc CLI:            $(command -v hc) ($(hc --version 2>&1 | head -1))$([ "$(hc_version)" = "$(conductor_version)" ] && printf ' [matches the conductor]' || printf ' [MISMATCH — `start` will refuse; see HOLOCHAIN_BIN]')"
  echo "probe env:  PEER_STORAGE_URLS=\"$(peer_csv)\" CONDUCTOR_URLS=\"$(conductor_csv)\" INTERNAL_DOORWAY_URL=\"localhost:$DOORWAY_PORT\" E2E_DOORWAY_B=\"http://localhost:${DOORWAY_B_PORT:-8889}\" API_KEY_ADMIN=\"${MESH_API_KEY_ADMIN:-mesh-admin-dev-key}\""
  return "$data_root_status"
}

probe_all() {
  guard_conductor_data_roots probe || return 1
  cd "$REPO_ROOT/genesis/a2o" || exit 2
  mkdir -p "$MESH_DIR/reports"
  local failures=0
  for cmd in mesh upload propagation delivery projection federation resilience; do
    PEER_STORAGE_URLS="$(peer_csv)" \
    INTERNAL_DOORWAY_URL="localhost:$DOORWAY_PORT" \
    REPORT_DIR="$MESH_DIR/reports" \
    STATE_DIR="$MESH_DIR/state" \
    CONTENT_PATH="$REPO_ROOT/genesis/docs/content/elohim-protocol/manifesto.md" \
    BUILD_TAG="local-mesh" \
    pnpm exec tsx scripts/substrate-verify.ts "$cmd" || failures=$((failures+1))
  done
  echo "probe complete: $failures subcommand(s) with failures (reports: $MESH_DIR/reports)"
}

# /proc/<pid>/exe resolves to "<path> (deleted)" when the binary has been
# REPLACED under a running process — which happens constantly here, because the
# storage binary is rebuilt into a shared cargo pool slot while the mesh runs.
# Handing that literal string to exec gives FileNotFoundError on a path that
# looks perfectly real in the error message. Strip the suffix, and fall back to
# the configured binary if the stripped path is gone too (the old inode is still
# executable via /proc/<pid>/exe, but only while that process lives).
resolve_exe() { # <pid> [fallback]
  local raw stripped
  raw="$(readlink "/proc/$1/exe" 2>/dev/null)"
  stripped="${raw% (deleted)}"
  if [ -x "$stripped" ]; then echo "$stripped"; return 0; fi
  if [ -n "${2:-}" ] && [ -x "$2" ]; then echo "$2"; return 0; fi
  echo "$stripped"
  return 1
}

release_adoption_slot_for() { # <peer-name>
  echo "$MESH_DIR/$1/release-adoption/slot/elohim-storage.next"
}

# Disarm one attempted release slot. A successful candidate is retained at an
# immutable applied path so `/proc/<pid>/exe` and the dead-peer exe record keep
# resolving. A failed candidate is retained with a failed suffix for diagnosis,
# while the exe record returns to the previous known-good binary. Both outcomes
# remove `.next`, so an operator retry cannot loop the same failed boot.
archive_release_adoption_slot() { # <peer> <slot> <applied|failed> <exe-record> [previous-exe]
  local name="$1" slot="$2" outcome="$3" exefile="$4" previous="${5:-}"
  local stamp destination receipt receipt_destination
  stamp="$(date -u +%Y%m%dT%H%M%SZ)"
  destination="$slot.$outcome-$stamp"
  receipt="$slot.json"
  receipt_destination="$receipt.$outcome-$stamp"
  if ! mv -- "$slot" "$destination"; then
    echo "  $name: could not disarm release slot $slot after $outcome boot" >&2
    return 1
  fi
  [ ! -e "$receipt" ] || mv -- "$receipt" "$receipt_destination" || {
    echo "  $name: WARN could not archive release receipt $receipt" >&2
  }
  if [ "$outcome" = "applied" ]; then
    printf '%s\n' "$destination" > "$exefile"
  elif [ -n "$previous" ] && [ -x "$previous" ]; then
    printf '%s\n' "$previous" > "$exefile"
  else
    rm -f "$exefile"
  fi
  echo "  $name: release slot $outcome; exe record=$(head -1 "$exefile" 2>/dev/null || echo absent) receipt=${receipt_destination}"
}

restart_storage() {
  # Restart storage peers in place, each with the EXACT environment it is already
  # running with — recovered from /proc/<pid>/environ, never rebuilt from this
  # script. Rebuilding would drift the moment start_all's env block changes, and
  # would clobber AGENT_PUBKEY for any peer re-keyed since boot (after a chaos
  # re-key, james runs on a key this script has no way to know).
  #
  # Why this exists: a conductor restart invalidates the app-interface auth token
  # each peer minted at ITS startup, and the peer does not re-mint it. It keeps
  # answering /health with 200 and keeps accepting writes while every zome call
  # fails and nothing can be anchored. Restarting the peer is the only way back.
  # Backlog: storage-stale-app-interface-token-after-conductor-restart.
  #
  # Optional arguments: peer names (default: all).
  local targets=("$@")
  [ ${#targets[@]} -eq 0 ] && targets=("${PEERS[@]}")
  local workdir="${MESH_DIR}/storage-restart"
  mkdir -p "$workdir"

  local name port pid envfile exefile cwd bin previous_bin slot receipt key k failed=""
  declare -A staged_slots=()
  declare -A previous_bins=()
  for name in "${targets[@]}"; do
    port=""; k=0
    for n2 in "${PEERS[@]}"; do [ "$n2" = "$name" ] && port="$(http_port $k)"; k=$((k+1)); done
    [ -n "$port" ] || { echo "  $name: not a mesh peer ($MESH_PEERS)" >&2; continue; }
    envfile="$workdir/$name.environ"
    exefile="$workdir/$name.exe"
    slot="$(release_adoption_slot_for "$name")"
    receipt="$slot.json"
    if [ -e "$slot" ]; then
      if [ ! -x "$slot" ]; then
        echo "  $name: staged release slot is not executable: $slot" >&2
        failed+=" $name"; continue
      fi
      staged_slots["$name"]="$slot"
      echo "  $name: staged release candidate=$slot receipt=$([ -s "$receipt" ] && echo "$receipt" || echo absent)"
    fi

    # Exact pid: the storage binary's argv carries --http-port <port>, which no
    # other process has, and which never appears in this script's own argv.
    pid="$(ps -eo pid=,args= | awk -v me="$$" -v pat="--http-port $port" \
      '$1 != me && index($0, "elohim-storage") && index($0, pat) { print $1; exit }')"

    # Capture the live environment when we can. A pid that exists but whose
    # /proc is unreadable (a process already exiting) is treated exactly like no
    # pid at all — fall through to the last good capture rather than give up,
    # which is the difference between a recoverable peer and a dead one.
    if [ -n "$pid" ] && python3 - "$pid" "$envfile" <<'PY' 2>/dev/null
import sys
pid, destination = sys.argv[1:]
with open(f"/proc/{pid}/environ", "rb") as source:
    raw = source.read()
with open(destination, "wb") as target:
    target.write(raw)
PY
    then
      cwd="$(readlink "/proc/$pid/cwd")"
      if ! previous_bin="$(resolve_exe "$pid" "$STORAGE_BIN")"; then
        echo "  $name: binary is gone (was '$previous_bin'); rebuild it or set STORAGE_BIN" >&2
        failed+=" $name"; continue
      fi
      previous_bins["$name"]="$previous_bin"
      bin="${staged_slots[$name]:-$previous_bin}"
      if ! assert_storage_transport_capability "$bin" "$(peer_transport "$name")"; then
        failed+=" $name"; continue
      fi
      # Record the binary beside the environ: a later dead-peer restore must not
      # depend on STORAGE_BIN's default path existing (the mesh usually runs the
      # doorway-family DEBUG slot, not the release default).
      printf '%s\n' "$bin" > "$exefile"
      key="$(tr '\0' '\n' < "$envfile" | sed -n 's/^AGENT_PUBKEY=//p' | head -1)"
      echo "  $name :$port pid=$pid agent=${key:0:20}… bin=$bin (env captured live)"
      kill "$pid" 2>/dev/null
      local t=15
      while [ "$t" -gt 0 ] && kill -0 "$pid" 2>/dev/null; do sleep 1; t=$((t-1)); done
      kill -0 "$pid" 2>/dev/null && { kill -9 "$pid" 2>/dev/null; sleep 2; }
    elif [ -s "$envfile" ]; then
      cwd="$LOCAL_DEV_DIR"
      # A staged release beats every restore fallback. Without one, use the
      # capture's exe record, a live sibling's exe, then STORAGE_BIN.
      previous_bin="$(restore_binary_for "$exefile")"
      [ -n "$previous_bin" ] && previous_bins["$name"]="$previous_bin"
      bin="${staged_slots[$name]:-$previous_bin}"
      if [ -z "$bin" ] || [ ! -x "$bin" ]; then
        echo "  $name: no executable storage binary — $exefile absent, no sibling peer running, STORAGE_BIN=$STORAGE_BIN not executable" >&2
        failed+=" $name"; continue
      fi
      if ! assert_storage_transport_capability "$bin" "$(peer_transport "$name")"; then
        failed+=" $name"; continue
      fi
      key="$(tr '\0' '\n' < "$envfile" | sed -n 's/^AGENT_PUBKEY=//p' | head -1)"
      echo "  $name :$port not readable/not running — restoring the capture from $(date -r "$envfile" -u +%H:%M:%SZ), agent=${key:0:20}… bin=$bin"
      [ -n "$pid" ] && { kill -9 "$pid" 2>/dev/null; sleep 1; }
    else
      # An EMPTY envfile lands here too (-s): a capture taken with fs.copyFile on
      # procfs is 0 bytes (2026-08-22). Say so — silently continuing is how a
      # dead peer turned into 21 ECONNREFUSED reds downstream.
      if [ -e "$envfile" ] && [ ! -s "$envfile" ]; then
        echo "  $name :$port not running and its captured environment is EMPTY ($envfile) — use ./hc-mesh.sh start" >&2
      else
        echo "  $name :$port not running and no captured environment — use ./hc-mesh.sh start" >&2
      fi
      failed+=" $name"; continue
    fi

    # setsid: nohup ignores SIGHUP but not a SIGKILL to the process group, which
    # is how a calling shell reaps its background children when it exits.
    # The captured environment is authoritative (AGENT_PUBKEY etc.); an overlay
    # (restart_env_overlay) adds/replaces ONLY the named keys.
    setsid nohup python3 -c '
import os, sys
envfile, binpath, port, cwd, overlay = sys.argv[1:6]
with open(envfile, "rb") as f:
    raw = f.read().decode("utf-8", "replace")
env = dict(p.split("=", 1) for p in raw.split("\0") if "=" in p)
for item in overlay.split("\n"):
    if "=" in item:
        k, v = item.split("=", 1); env[k] = v
os.chdir(cwd)
# Drop every inherited descriptor above stderr before exec. A caller that runs
# this script under `flock` (the a2o mesh lock) otherwise hands its lock fd to
# the long-lived peer, which then holds the lock forever (2026-08-22 deadlock:
# three storage peers owned a2o.lock; every waiter stalled for 40 min).
os.closerange(3, 65536)
os.execve(binpath, [binpath, "--http-port", port], env)
' "$envfile" "$bin" "$port" "$cwd" "$(restart_env_overlay "$envfile" "$name")" >> "$LOGDIR/$name.log" 2>&1 &
    record_mesh_pid storage "$name" "$!" || true
    disown 2>/dev/null || true
  done

  echo -n "waiting for storage peers to serve"
  local dl=$((SECONDS + 180))
  while [ "$SECONDS" -lt "$dl" ]; do
    local up=0 k2=0
    for n2 in "${PEERS[@]}"; do
      curl -s -m 2 "http://localhost:$(http_port $k2)/health" >/dev/null && up=$((up+1)); k2=$((k2+1))
    done
    [ "$up" -ge ${#PEERS[@]} ] && break
    printf "."; sleep 3
  done
  echo
  # Every requested peer must answer, by PORT — a restart that leaves a target
  # down is a failure the caller (a chaos drill, an operator) has to see.
  k=0
  for n2 in "${PEERS[@]}"; do
    port="$(http_port $k)"; k=$((k+1))
    case " ${targets[*]} " in *" $n2 "*) ;; *) continue ;; esac
    case "$failed" in *" $n2"*) continue ;; esac
    if curl -s -m 2 "http://localhost:$port/health" >/dev/null; then
      if [ -n "${staged_slots[$n2]:-}" ]; then
        archive_release_adoption_slot "$n2" "${staged_slots[$n2]}" applied \
          "$workdir/$n2.exe" "${previous_bins[$n2]:-}" || failed+=" $n2"
      fi
    else
      echo "  $n2 :$port did not come back within the wait" >&2
      if [ -n "${staged_slots[$n2]:-}" ]; then
        archive_release_adoption_slot "$n2" "${staged_slots[$n2]}" failed \
          "$workdir/$n2.exe" "${previous_bins[$n2]:-}" || true
      fi
      failed+=" $n2"
    fi
  done
  refresh_fixture_pids
  # /health is not the question. "Serving" and "able to anchor" are different
  # claims and only one of them matters.
  probe_zome_paths
  if [ -n "$failed" ]; then
    echo "storage-restart FAILED for:$failed" >&2
    return 1
  fi
}

storage_ark_env() { # <peer-name>
  [ "${MESH_CONDUCTOR_LAUNCH:-hc}" = "ark" ] || return 0
  printf '%s\n' \
    "ELOHIM_ARK_SPOOL_PATH=$LOCAL_DEV_DIR/$1/ark" \
    "REPLICATION_INTERVAL_SECONDS=${REPLICATION_INTERVAL_SECONDS:-10}" \
    "CUSTODY_SWEEP_SECONDS=${CUSTODY_SWEEP_SECONDS:-5}" \
    "INVENTORY_BROADCAST_SECONDS=${INVENTORY_BROADCAST_SECONDS:-15}"
}

# Env keys layered over a restarted peer's captured environment, one K=V per
# line. Empty by default — the capture is the truth except for the explicit
# per-peer transport launch knob. Two further opt-ins:
#   MESH_RESTART_APPLY_PROFILE=1   re-apply THIS script's dev-tier pacing profile
#                                  (the same knobs `start` exports), so a knob
#                                  added after boot reaches a running mesh
#                                  without regenerating it. Never touches
#                                  AGENT_PUBKEY or any non-profile key.
#   MESH_RESTART_ENV_OVERLAY="K=V K=V"   ad-hoc keys for one experiment.
# The per-peer runtime-config file (rung 4: a flag flip or a release-channel
# follow lands on the RUNNING peer within one poll, no restart). elohim-storage's
# watcher is OFF unless ELOHIM_RUNTIME_CONFIG_PATH names a file, and the a2o
# release ceremony writes `ELOHIM_RELEASE_CHANNELS = "…"` into exactly this path
# (steps/delivery/runtime-upgrade-propagation.steps.ts `runtimeConfigPath`) then
# POSTs /admin/runtime-config/reload — a peer started without it answers
# `/admin/adoption` with `sweeps: 0, channels: []` forever and station 1 times
# out on "waiting on <peer>'s /admin/adoption" (2026-09-03). Created empty so the
# watcher is active from boot; a restart keeps it through restart_env_overlay.
runtime_config_path_for() { # <peer-name> -> prints the path (file guaranteed to exist)
  local f="$MESH_DIR/$1/runtime-config.toml"
  mkdir -p "$MESH_DIR/$1"; [ -f "$f" ] || : > "$f"
  echo "$f"
}

restart_env_overlay() { # <captured-environ> <peer-name>
  # The caller's PER-PEER transport selection deliberately beats the captured
  # daemon environment. This is how one slot is cycled into a new transport.
  # MESH_RESTART_ENV_OVERLAY remains last for one-off experiments.
  printf '%s\n' "ELOHIM_TRANSPORT_BACKEND=$(peer_transport "$2")"
  printf '%s\n' "ELOHIM_RUNTIME_CONFIG_PATH=$(runtime_config_path_for "$2")"
  # T0' pure-iroh bootstrap: storage announces to / seeds its peer book from
  # the doorway's /p2p/manifests projection; localdev doorway is :$DOORWAY_PORT.
  if [ "$MESH_DOORWAYS_EFFECTIVE" = "1" ]; then
    printf '%s\n' "ELOHIM_DOORWAY_URL=http://localhost:$DOORWAY_PORT"
  fi
  storage_ark_env "$2"
  if [ "${MESH_RESTART_APPLY_PROFILE:-0}" = "1" ]; then
    printf '%s\n' \
      "PROJECTION_RECONCILE_SECS=$PROJECTION_RECONCILE_SECS" \
      "ACQUISITION_RECONCILE_SECS=$ACQUISITION_RECONCILE_SECS" \
      "CONTEST_BACKOFF_SECONDS=$CONTEST_BACKOFF_SECONDS" \
      "HEAL_MISSING_BACKOFF_SECONDS=$HEAL_MISSING_BACKOFF_SECONDS" \
      "ELOHIM_EVIDENCE_ABSENT_BACKOFF_SECS=$ELOHIM_EVIDENCE_ABSENT_BACKOFF_SECS" \
      "ELOHIM_HEAD_CORPUS_DIGEST=$ELOHIM_HEAD_CORPUS_DIGEST" \
      "ELOHIM_ADOPT_BEFORE_AUTHOR=$ELOHIM_ADOPT_BEFORE_AUTHOR" \
      "ELOHIM_OBEY_CARRIED_ELECTION=$ELOHIM_OBEY_CARRIED_ELECTION" \
      "ALLOW_COORDINATOR_UPDATE=$ALLOW_COORDINATOR_UPDATE" \
      "ADOPT_CONTEST_FANOUT=$ADOPT_CONTEST_FANOUT" \
      "ELOHIM_NETWORK_STAKES=$ELOHIM_NETWORK_STAKES" \
      "ALLOW_SEED_NETWORK_STAKES=1" \
      "ALLOW_SEED_DELEGATES_COMPUTE=1" \
      "ALLOW_SEED_SHARD_MANIFEST=1"
  fi
  local kv
  for kv in ${MESH_RESTART_ENV_OVERLAY:-}; do printf '%s\n' "$kv"; done
}

# The binary for a dead peer whose /proc is gone: its recorded exe, a running
# sibling's exe (same build), then STORAGE_BIN. Prints nothing if none is executable.
restore_binary_for() {
  local exefile="$1" cand
  if [ -s "$exefile" ]; then
    cand="$(head -1 "$exefile")"
    [ -x "$cand" ] && { echo "$cand"; return 0; }
  fi
  local spid
  for spid in $(ps -eo pid=,args= | awk -v me="$$" 'index($0, "elohim-storage") && index($0, "--http-port") && $1 != me { print $1 }'); do
    cand="$(readlink "/proc/$spid/exe" 2>/dev/null)"; cand="${cand% (deleted)}"
    [ -n "$cand" ] && [ -x "$cand" ] && { echo "$cand"; return 0; }
  done
  [ -x "$STORAGE_BIN" ] && { echo "$STORAGE_BIN"; return 0; }
  return 1
}

# The household fixture (written once by hc-mesh-prologue.sh) names each storage
# peer's pid, and the a2o chaos drills kill/verify peers BY THAT PID. An in-place
# restart mints new pids, so a fixture left behind turns every later drill into
# `kill ESRCH` / "/proc/<pid> is gone" (2026-08-22: 3 chaos-peer-churn reds from
# one storage-restart). Re-resolve every peer's pid from its listening port,
# and stamp its AGENT_PUBKEY beside it.
refresh_fixture_pids() {
  local fixture="$MESH_DIR/household-fixture.json"
  [ -s "$fixture" ] || return 0
  python3 - "$fixture" "$MESH_PEERS" <<'PY'
import json, subprocess, sys
fixture, peers = sys.argv[1], sys.argv[2].split(',')
d = json.load(open(fixture))
sp = d.setdefault('storagePeers', {})
changed = []
for i, name in enumerate(peers):
    port = 8090 + i
    out = subprocess.run(['bash', '-c', f"ss -ltnp | grep ':{port} ' | sed -n 's/.*pid=\\([0-9]*\\).*/\\1/p' | head -1"],
                         capture_output=True, text=True).stdout.strip()
    if not out or name not in sp:
        continue
    if sp[name].get('pid') != int(out):
        changed.append(f"{name}:{sp[name].get('pid')}->{out}")
        sp[name]['pid'] = int(out)
    # The peer's agent key, in the namespace custody commitments name providers
    # in (a2o drills match either this or the libp2p peerId).
    try:
        env = open(f'/proc/{out}/environ', 'rb').read().split(b'\0')
        key = next((e.split(b'=', 1)[1].decode() for e in env if e.startswith(b'AGENT_PUBKEY=')), '')
    except OSError:
        key = ''
    if key and sp[name].get('agentPubKey') != key:
        changed.append(f"{name}:agentPubKey={key[:12]}…")
        sp[name]['agentPubKey'] = key
if changed:
    json.dump(d, open(fixture, 'w'), indent=2)
    print('fixture pids refreshed: ' + ' '.join(changed))
PY
}

# Report, per peer, whether its ZOME path is alive — i.e. whether it can still
# reach its conductor. Shared by conductors-restart and storage-restart.
probe_zome_paths() {
  echo "storage peers (zome path, not just /health):"
  local j=0 name port body probe_id needs_restart=""
  for name in "${PEERS[@]}"; do
    port="$(http_port $j)"
    printf "  %-8s :%s " "$name" "$port"
    if ! curl -s -m 3 "http://localhost:$port/health" >/dev/null; then
      echo "DOWN (not serving at all)"
    else
      probe_id="$(curl -s -m 8 "http://localhost:$port/db/content?limit=25" 2>/dev/null \
        | python3 -c '
import json, sys
try:
    items = json.load(sys.stdin).get("items", [])
except Exception:
    items = []
# An anchor is an ACTION hash (uhCkk…); a2o fixture rows carry blob hashes there
# (sha256-…) and would make the zome probe read as a shape mismatch (2026-08-29).
print(next((i["id"] for i in items if str(i.get("dhtAnchorHash") or "").startswith("uhCkk")), ""))
' 2>/dev/null)"
      if [ -z "$probe_id" ]; then
        echo "serving, but no ANCHORED content to probe the zome path with (inconclusive)"
      else
        body="$(curl -s -m 15 -H "Authorization: Bearer ${MESH_API_KEY_ADMIN:-mesh-admin-dev-key}" \
          "http://localhost:$port/db/content/$probe_id/head-record" 2>/dev/null)"
        case "$body" in
          *"WasmError"*|*"Deserialize"*)
            echo "serving, but ZOME INPUT SHAPE MISMATCH (storage binary vs installed DNA) <-- rebuild the DNA or MESH_HAPP_PATH=<deployed bundle>, then mesh stop/start"
            needs_restart+="${needs_restart:+ }$name" ;;
          *"Zome call failed"*|*"Websocket closed"*|*"No connection"*)
            echo "serving, but ZOME CALLS ARE DEAD (stale app-interface token) <-- restart this peer"
            needs_restart+="${needs_restart:+ }$name" ;;
          "") echo "serving, zome probe timed out <-- check this peer"
            needs_restart+="${needs_restart:+ }$name" ;;
          *) echo "UP (zome path alive)" ;;
        esac
      fi
    fi
    j=$((j+1))
  done
  if [ -n "$needs_restart" ]; then
    echo
    echo "  Stale conductor token — restart deliberately: ./hc-mesh.sh storage-restart $needs_restart"
    echo "  Until then these peers accept writes that can never be anchored."
    return 1
  fi
  return 0
}

restart_conductors() {
  # Restart the conductors IN PLACE, against the sandboxes that already exist.
  #
  # Why this is a separate action and not "stop && start": `start` regenerates
  # sandboxes when the admin ports are free (rm -rf + `hc sandbox generate`),
  # which mints NEW agent keys for every peer and throws away their chains. That
  # is a re-key of the whole household, not a restart. It is also load-sensitive
  # — the cold wasm install inside generate races the conductor's 60s admin
  # request timeout, measured 65s and failing at load average 79 on 2026-08-21.
  #
  # This action changes exactly one thing: the process, and whatever env it is
  # launched with (MESH_RUST_LOG). Keys, chains, DHT databases, wasm caches and
  # conductor-config.yaml are all untouched. Reach for it when you need the
  # conductors running under different logging or after a hang — never to
  # "reset" the mesh.
  #
  # Storage peers are NOT touched. Each one holds a websocket to its conductor's
  # admin+app interfaces; watch for its "Connected ... to app interface" line
  # after this returns, and check /health. If a peer does not reconnect on its
  # own, restart THAT peer deliberately — this action will not do it for you,
  # because a storage restart is a different blast radius.
  guard_conductor_data_roots conductors-restart || return 1
  cd "$LOCAL_DEV_DIR" || exit 2

  local fports="" aports="" i=0
  for _ in "${PEERS[@]}"; do
    fports+="${fports:+,}$(admin_port $i)"; aports+="${aports:+,}$(app_port $i)"; i=$((i+1))
  done

  # A restart re-runs `hc sandbox run`, which rewrites conductor-config.yaml in
  # the CLI's schema before the conductor reads it — so the pair must agree here
  # too, not only at generate time.
  # `ark` skips the parity check for the same reason `direct` does: neither one
  # runs the `hc` CLI, so neither can rewrite a config in the CLI's schema.
  [ "${MESH_CONDUCTOR_LAUNCH:-hc}" = "direct" ] || \
    [ "${MESH_CONDUCTOR_LAUNCH:-hc}" = "ark" ] || assert_toolchain_parity

  # Exact pids only. Every lookup below matches an argv substring unique to the
  # target process AND excludes this shell — `pkill -f holochain` would match
  # the caller's own command line, which is how shells have been SIGTERM'd here
  # before. Nothing in this function's own argv contains these patterns.
  local pids=() pid
  for name in "${PEERS[@]}"; do
    pid="$(ps -eo pid=,args= | awk -v me="$$" -v pat="$LOCAL_DEV_DIR/$name/conductor-config.yaml" \
      '$1 != me && index($0, "--config-path") && index($0, pat) { print $1; exit }')"
    [ -n "$pid" ] && { pids+=("$pid"); echo "  conductor $name: pid $pid"; } \
                  || echo "  conductor $name: not running"
  done
  # The `hc sandbox ... run` supervisor and the sh -c that launched it: they
  # respawn nothing, but leaving them behind orphans the next run's port pins.
  while read -r pid; do
    [ -n "$pid" ] && { pids+=("$pid"); echo "  hc sandbox run supervisor: pid $pid"; }
  done < <(ps -eo pid=,args= | awk -v me="$$" \
    '$1 != me && index($0, "hc sandbox") && index($0, " run ") { print $1 }')

  if [ ${#pids[@]} -eq 0 ]; then
    echo "no conductors running — use ./hc-mesh.sh start"
  else
    echo "stopping ${#pids[@]} process(es) by exact pid"
    kill "${pids[@]}" 2>/dev/null
    for _ in $(seq 1 20); do
      local alive=0
      for pid in "${pids[@]}"; do kill -0 "$pid" 2>/dev/null && alive=$((alive+1)); done
      [ "$alive" -eq 0 ] && break
      sleep 1
    done
    for pid in "${pids[@]}"; do kill -0 "$pid" 2>/dev/null && kill -9 "$pid" 2>/dev/null; done
    sleep 1
  fi

  # Append rather than truncate: the previous run's log is evidence, and the
  # spin detector tracks byte offsets and handles growth cleanly.
  echo "restarting ${#PEERS[@]} conductors from EXISTING sandboxes (no generate, keys kept)"
  echo "  RUST_LOG=$MESH_RUST_LOG"
  # setsid, not just nohup. nohup only ignores SIGHUP; it does nothing about a
  # SIGKILL delivered to the whole process GROUP, which is how a calling shell's
  # cleanup reaps its background children. Restarting the conductors from an
  # agent tool-call or a CI step and having them vanish the moment that step
  # returns is the failure this prevents (observed 2026-08-21: conductors up at
  # 17:16:51, gone by 17:20, no crash in the log because there was none).
  # CONFIG-SCHEMA COMPAT (0.6.0 -> 0.6.3). A conductor-config.yaml written by
  # hc 0.6.0 carries `network.base64_auth_material`, which 0.6.3 REMOVED in
  # favour of base64_auth_material_bootstrap / _relay. A 0.6.3 conductor refuses
  # to parse it outright:
  #
  #   network: unknown field `base64_auth_material`, expected one of
  #   `base64_auth_material_bootstrap`, `base64_auth_material_relay`, ...
  #
  # so swapping the binary against existing sandboxes fails before boot, with
  # every conductor down and the reason buried in the run log. On this mesh the
  # field is null, i.e. it carries no information, so dropping it makes the same
  # file readable by BOTH versions — which is also what an A/B wants: one config,
  # two binaries, no other difference. A NON-null value is a real credential and
  # is left alone, loudly, because migrating it is a decision this script must
  # not make silently.
  #
  # This is not only a local-mesh concern: rolling a 0.6.3-line conductor onto
  # any fleet whose conductor data dir was written by 0.6.0 hits exactly this,
  # and the conductor PVC is persistent.
  local cfg
  for name in "${PEERS[@]}"; do
    cfg="$LOCAL_DEV_DIR/$name/conductor-config.yaml"
    [ -f "$cfg" ] || continue
    python3 - "$cfg" "$name" <<'CFGEOF'
import sys, yaml
path, name = sys.argv[1], sys.argv[2]
with open(path) as f:
    cfg = yaml.safe_load(f) or {}
net = cfg.get("network") or {}
if "base64_auth_material" in net:
    if net["base64_auth_material"] is None:
        del net["base64_auth_material"]
        cfg["network"] = net
        with open(path, "w") as f:
            yaml.safe_dump(cfg, f, default_flow_style=False, sort_keys=False)
        print(f"  {name}: dropped null network.base64_auth_material (0.6.3 rejects the key)")
    else:
        print(f"  {name}: WARN network.base64_auth_material is SET — a 0.6.3 conductor will refuse this config; migrate it deliberately", file=sys.stderr)
CFGEOF
  done

  # NO -p ON A RESTART. `hc sandbox run --help` says it outright: "Interfaces
  # are persistent. If you add an interface it will be there next time you run
  # the conductor." The app interfaces were attached by the original generate
  # and are still in each conductor-config.yaml, so re-attaching them makes
  # `hc sandbox run` fail its post-boot connect step and EXIT 0 — silently,
  # with every conductor already reporting "Conductor ready" — which then tears
  # the conductors down with it. Measured 2026-08-21: with -p, all three booted
  # and the supervisor exited within a second, no error in the log, and the
  # "Conductor launched #!N" lines a healthy run prints were simply absent;
  # without -p, the same command stays up and the app ports come back by
  # themselves. `start` may pass -p because it has just generated the sandboxes
  # and no interface exists yet; a restart must not.
  #
  # setsid, not just nohup: nohup only ignores SIGHUP and does nothing about a
  # SIGKILL to the process GROUP, which is how a calling shell reaps background
  # children when it exits.
  if [ -n "$HOLOCHAIN_BIN" ]; then
    echo "  HOLOCHAIN_BIN=$HOLOCHAIN_BIN ($("$HOLOCHAIN_BIN" --version 2>&1 | head -1))"
  else
    echo "  holochain: $(command -v holochain) ($(holochain --version 2>&1 | head -1)) [stock, PATH]"
  fi
  # LAUNCH MODE. `hc sandbox run` is the normal path, but it cannot be used for a
  # conductor A/B across a config-schema change: the `hc` CLI REWRITES each
  # conductor-config.yaml (that is how -f pins admin ports) in ITS OWN version's
  # schema. With hc 0.6.0 on PATH and a 0.6.3 conductor binary, hc puts
  # `base64_auth_material` back immediately after the compat shim above removes
  # it, and the 0.6.3 conductor then refuses to parse the file. The CLI and the
  # conductor must agree on the schema, and only the conductor was rebuilt.
  #
  # `direct` launches each conductor itself — exactly the argv hc would have used
  # — with the passphrase piped the same way. No CLI, no rewrite, and as a bonus
  # each conductor gets its OWN log file, so the spin detector can attribute log
  # rates per conductor instead of reading one multiplexed prefix-less stream.
  #
  # Ports come from each conductor-config.yaml, which already carries the pinned
  # admin port, so nothing needs -f.
  if [ "${MESH_CONDUCTOR_LAUNCH:-hc}" = "ark" ]; then
    # Nothing to relaunch for a peer whose ark is alive: killing the child above
    # IS the restart request, and its ark reaps the death, writes the witness and
    # spawns the replacement itself. launch_ark_conductors covers the other case
    # — an ark that is gone — and skips the peers it must not double-run.
    launch_ark_conductors || return 1
  elif [ "${MESH_CONDUCTOR_LAUNCH:-hc}" = "direct" ]; then
    local hc_bin="${HOLOCHAIN_BIN:-$(command -v holochain)}"
    [ -x "$hc_bin" ] || { echo "no conductor binary: $hc_bin" >&2; return 1; }
    echo "  launch mode: direct (per-conductor logs, no hc CLI rewrite)"
    for name in "${PEERS[@]}"; do
      (
        export RUST_LOG="$MESH_RUST_LOG"
        cd "$LOCAL_DEV_DIR" || exit 1
        setsid nohup sh -c "echo test | '$hc_bin' --piped --structured=Log --config-path '$LOCAL_DEV_DIR/$name/conductor-config.yaml'" \
          >> "$LOCAL_DEV_DIR/.sandbox_run_log.$name" 2>&1 &
        record_mesh_pid conductor "$name" "$!" || true
      )
    done
  else
    (
      export RUST_LOG="$MESH_RUST_LOG"
      holochain_bin_export
      # MESH_ATTACH_APP_PORTS=1 re-attaches the app interfaces. Normally OFF,
      # because interfaces are persistent and re-attaching them makes the
      # supervisor exit 0 and take the conductors with it. It is needed exactly
      # once after a config-schema migration: the fork's hc rewrites
      # conductor-config.yaml in the 0.6.3 schema and the persistent app
      # interfaces do NOT survive that rewrite (`app_ports:[]` on first boot),
      # so without this the conductors come up with no app interface and every
      # storage peer is left with nothing to talk to.
      local _pflag=""
      [ "${MESH_ATTACH_APP_PORTS:-0}" = "1" ] && _pflag=" -p=$aports"
      setsid nohup sh -c "echo test | hc sandbox --piped -f $fports run -a$_pflag" >> .sandbox_run_log 2>&1 &
      record_mesh_pid conductor-supervisor mesh "$!" || true
    )
  fi

  echo -n "waiting for ${#PEERS[@]} conductors to boot"
  for _ in $(seq 1 60); do
    [ "$(ss -tln | grep -cE "127.0.0.1:($(echo "$fports" | tr ',' '|')) ")" -ge ${#PEERS[@]} ] && break
    printf "."; sleep 3
  done
  echo
  if [ "$(ss -tln | grep -cE "127.0.0.1:($(echo "$fports" | tr ',' '|')) ")" -lt ${#PEERS[@]} ]; then
    echo "NOT all conductors came back — see $LOCAL_DEV_DIR/.sandbox_run_log" >&2
    return 1
  fi
  echo "conductors up on $fports"
  refresh_mesh_pidfiles
  # The app interfaces are persistent, so they should return WITHOUT -p. If they
  # do not, the storage peers have nothing to talk to and the mesh looks alive
  # while being useless — worth saying out loud rather than leaving to discovery.
  local app_up
  app_up="$(ss -tln | grep -cE "127.0.0.1:($(echo "$aports" | tr ',' '|')) ")"
  echo "app interfaces up on $aports: $app_up/${#PEERS[@]}"
  [ "$app_up" -lt "${#PEERS[@]}" ] && echo "  WARN: an app interface did not return — storage peers cannot make zome calls" >&2
  echo
  # Storage peers are NOT restarted here, and /health is NOT the question to ask
  # them. Each peer authenticates a websocket to its conductor's APP interface
  # with a token minted at ITS startup; a conductor restart invalidates that
  # token ("Authentication failed with reason: Invalid token" in the conductor
  # log), and the peer does not re-mint it. The result is a peer that answers
  # /health with a cheerful 200 while every zome call fails with "Websocket
  # closed: No connection" — writes still land in its database but nothing can
  # be anchored, and reanchor_backfill quietly stops being able to do its job.
  # Measured 2026-08-21: still broken 90s after the restart on all three peers,
  # with newly written rows stuck at trust:published and a NULL anchor.
  #
  # So probe the ZOME PATH, not the HTTP path, and say plainly which peers need
  # a deliberate restart. This action will not restart them: a storage restart
  # is a different blast radius and may collide with whoever else is working.
  echo "storage peers (NOT restarted — probing the ZOME path, not just /health):"
  local j=0 needs_restart=""
  for name in "${PEERS[@]}"; do
    local port body
    port="$(http_port $j)"
    printf "  %-8s :%s " "$name" "$port"
    if ! curl -s -m 3 "http://localhost:$port/health" >/dev/null; then
      echo "DOWN (not serving at all)"
    else
      # Probe an endpoint that MUST make a zome call. /health, /db/humans and
      # even /p2p/status are answered from the local database and stay cheerful
      # while the conductor link is dead; head-record has to ask the conductor.
      #
      # Two traps, both hit while building this:
      #   - /db/p2p/conductor-diagnostics answers "no embedded conductor admin
      #     connection" for every EXTERNAL-conductor topology, which is this
      #     mesh's normal shape. It cannot tell healthy from broken.
      #   - head-record on a NULL-ANCHORED row short-circuits with "no notarized
      #     head declared" BEFORE reaching the conductor, so an unanchored probe
      #     row reports a DEAD peer as alive. Pick an ANCHORED row.
      local probe_id
      probe_id="$(curl -s -m 8 "http://localhost:$port/db/content?limit=25" 2>/dev/null \
        | python3 -c '
import json, sys
try:
    items = json.load(sys.stdin).get("items", [])
except Exception:
    items = []
# An anchor is an ACTION hash (uhCkk…); a2o fixture rows carry blob hashes there
# (sha256-…) and would make the zome probe read as a shape mismatch (2026-08-29).
print(next((i["id"] for i in items if str(i.get("dhtAnchorHash") or "").startswith("uhCkk")), ""))
' 2>/dev/null)"
      if [ -z "$probe_id" ]; then
        echo "serving, but no ANCHORED content to probe the zome path with (inconclusive)"
      else
        body="$(curl -s -m 15 -H "Authorization: Bearer ${MESH_API_KEY_ADMIN:-mesh-admin-dev-key}" \
          "http://localhost:$port/db/content/$probe_id/head-record" 2>/dev/null)"
        case "$body" in
          *"WasmError"*|*"Deserialize"*)
            echo "serving, but ZOME INPUT SHAPE MISMATCH (storage binary vs installed DNA) <-- rebuild the DNA or MESH_HAPP_PATH=<deployed bundle>, then mesh stop/start"
            needs_restart+="${needs_restart:+ }$name" ;;
          *"Zome call failed"*|*"Websocket closed"*|*"No connection"*)
            echo "serving, but ZOME CALLS ARE DEAD (stale app-interface token) <-- restart this peer"
            needs_restart+="${needs_restart:+ }$name" ;;
          "") echo "serving, zome probe timed out <-- check this peer"
            needs_restart+="${needs_restart:+ }$name" ;;
          *) echo "UP (zome path alive)" ;;
        esac
      fi
    fi
    j=$((j+1))
  done
  if [ -n "$needs_restart" ]; then
    echo
    echo "  These peers hold a stale conductor token and must be restarted deliberately:"
    echo "    $needs_restart"
    echo "  Until then they accept writes that can never be anchored."
  fi
}

start_storage_peer() { # <peer-name> <peer-index>
  local name="$1" i="$2"
  local doorway_env=() ark_env=()
  if [ "$MESH_DOORWAYS_EFFECTIVE" = "1" ]; then
    doorway_env=("ELOHIM_DOORWAY_URL=http://localhost:$DOORWAY_PORT")
  fi
  mapfile -t ark_env < <(storage_ark_env "$name")
  if ! curl -s -m 2 "http://localhost:$(http_port "$i")/health" >/dev/null; then
    local agent
    agent=$(hc sandbox call --running "$(admin_port "$i")" list-apps 2>/dev/null \
      | grep -o '"agent_pub_key":"[^"]*"' | head -1 | cut -d'"' -f4)
    mkdir -p "$MESH_DIR/$name"
    env -u ELOHIM_ARK_SPOOL_PATH \
    -u REPLICATION_INTERVAL_SECONDS \
    -u CUSTODY_SWEEP_SECONDS \
    -u INVENTORY_BROADCAST_SECONDS \
    "${doorway_env[@]}" "${ark_env[@]}" \
    HOLOCHAIN_ADMIN_URL="ws://localhost:$(admin_port "$i")" \
    HOLOCHAIN_APP_URL="ws://localhost:$(app_port "$i")" \
    STORAGE_DIR="$MESH_DIR/$name" \
    ENABLE_CONTENT_DB=true ENABLE_IMPORT_API=true \
    ENABLE_P2P=true P2P_PORT="$(p2p_port "$i")" \
    ELOHIM_TRANSPORT_BACKEND="$(peer_transport "$name")" \
    AGENT_PUBKEY="$agent" RELAY_MODE=server \
    GENESIS_SELF_HEAL_IDENTITY=1 SELF_HUMAN_ID="$(human_id "$name")" \
    HOUSEHOLD_ID=household-dowell \
    DEVICE_ARCHETYPE=device-family-node-base \
    ELOHIM_STORAGE_PEER_POLICY_PATH="$MESH_DIR/peer-policy.toml" \
    ELOHIM_RUNTIME_CONFIG_PATH="$(runtime_config_path_for "$name")" \
    PROJECTION_RECONCILE_SECS="$PROJECTION_RECONCILE_SECS" \
    ACQUISITION_RECONCILE_SECS="$ACQUISITION_RECONCILE_SECS" \
    CONTEST_BACKOFF_SECONDS="$CONTEST_BACKOFF_SECONDS" \
    HEAL_MISSING_BACKOFF_SECONDS="$HEAL_MISSING_BACKOFF_SECONDS" \
    ELOHIM_EVIDENCE_ABSENT_BACKOFF_SECS="$ELOHIM_EVIDENCE_ABSENT_BACKOFF_SECS" \
    ELOHIM_HEAD_CORPUS_DIGEST="$ELOHIM_HEAD_CORPUS_DIGEST" \
    ELOHIM_ADOPT_BEFORE_AUTHOR="$ELOHIM_ADOPT_BEFORE_AUTHOR" \
    ELOHIM_OBEY_CARRIED_ELECTION="$ELOHIM_OBEY_CARRIED_ELECTION" \
    ALLOW_COORDINATOR_UPDATE="$ALLOW_COORDINATOR_UPDATE" \
    ADOPT_CONTEST_FANOUT="$ADOPT_CONTEST_FANOUT" \
    ELOHIM_NETWORK_STAKES="$ELOHIM_NETWORK_STAKES" \
    ALLOW_SEED_NETWORK_STAKES=1 \
    ALLOW_SEED_DELEGATES_COMPUTE=1 \
    ALLOW_SEED_SHARD_MANIFEST=1 \
    nohup "$STORAGE_BIN" --http-port "$(http_port "$i")" > "$LOGDIR/$name.log" 2>&1 &
    record_mesh_pid storage "$name" "$!" || true
    echo "storage $name: http=$(http_port "$i") p2p=$(p2p_port "$i") transport=$(peer_transport "$name") agent=${agent:0:16}..."
  else
    record_listener_pid storage "$name" "$(http_port "$i")" || true
    echo "storage $name already up on :$(http_port "$i")"
  fi
}

join_peer() { # <fresh-peer-name>
  if [ "$#" -ne 1 ]; then
    echo "usage: hc-mesh.sh join-peer <fresh-peer-name>" >&2
    return 2
  fi
  local name="$1"
  if [[ ! "$name" =~ ^[A-Za-z0-9][A-Za-z0-9_-]{0,47}$ ]]; then
    echo "join-peer: invalid peer name '$name' (use 1-48 letters, digits, '_' or '-', starting alphanumeric)" >&2
    return 2
  fi
  guard_conductor_data_roots join-peer || return 1

  # This verb is deliberately an append to a LIVE mesh. A cold or partial
  # roster is not a late-join regime, and starting around it would make the
  # receipt attribute recovery/restart effects to membership refresh.
  [ -s "$LOCAL_DEV_DIR/.hc" ] || {
    echo "join-peer: no running mesh roster at $LOCAL_DEV_DIR/.hc; run ./hc-mesh.sh start first" >&2
    return 2
  }
  [ -f "$MESH_DIR/peer-policy.toml" ] || {
    echo "join-peer: mesh peer policy is absent; run ./hc-mesh.sh start first" >&2
    return 2
  }
  if [ "$MESH_DOORWAYS_EFFECTIVE" != "1" ] || \
     ! curl -s -m 2 "http://localhost:$DOORWAY_PORT/health" >/dev/null; then
    echo "join-peer: the running mesh doorway/manifest board is unavailable on :$DOORWAY_PORT" >&2
    return 2
  fi
  local base_i=0 base_name
  for base_name in "${PEERS[@]}"; do
    if ! curl -s -m 2 "http://localhost:$(http_port "$base_i")/health" >/dev/null || \
       ! ss -H -ltn "sport = :$(admin_port "$base_i")" 2>/dev/null | grep -q . || \
       ! ss -H -ltn "sport = :$(app_port "$base_i")" 2>/dev/null | grep -q .; then
      echo "join-peer: incumbent $base_name is not fully running; refusing to reshape a partial mesh" >&2
      return 2
    fi
    base_i=$((base_i + 1))
  done

  local sandbox="$LOCAL_DEV_DIR/$name"
  if [ -e "$sandbox" ] || grep -Fxq "$sandbox" "$LOCAL_DEV_DIR/.hc" 2>/dev/null || \
     [ -e "$PID_DIR/storage-$name" ] || [ -e "$PID_DIR/conductor-$name" ] || \
     [ -e "$PID_DIR/conductor-supervisor-$name" ] || [ -e "$PID_DIR/ark-$name" ]; then
    echo "join-peer: peer '$name' was already staged; refusing a duplicate" >&2
    return 2
  fi

  # `hc sandbox generate` appends one path to .hc. Its pre-append line count is
  # therefore the durable sandbox index and keeps fresh names from colliding
  # across consecutive late-join receipts.
  local index
  index="$(awk 'NF { n += 1 } END { print n + 0 }' "$LOCAL_DEV_DIR/.hc")"
  local port
  for port in "$(admin_port "$index")" "$(app_port "$index")" \
              "$(http_port "$index")" "$(p2p_port "$index")"; do
    if listener_pids_for_ports "$port" | grep -q .; then
      echo "join-peer: derived port :$port for index $index is already in use; refusing before launch" >&2
      return 2
    fi
  done

  # Whatever this launch mode needs, asked for before the sandbox is generated:
  # toolchain parity under `hc`/`direct`, the ark binary and jq under `ark`.
  assert_launch_prerequisites || return 1
  [ -x "$STORAGE_BIN" ] || {
    echo "join-peer: storage binary is not executable: $STORAGE_BIN" >&2
    return 1
  }
  assert_storage_transport_capability "$STORAGE_BIN" "$(peer_transport "$name")" || return 1
  [ -f "$HAPP_PATH" ] || {
    echo "join-peer: hApp bundle is absent: $HAPP_PATH" >&2
    return 1
  }
  mkdir -p "$MESH_DIR" "$LOGDIR" "$PID_DIR"

  local netargs generate_log="$LOGDIR/$name.generate.log"
  netargs="$(mesh_network_args)"
  echo -n "generating late joiner $name at index $index ($netargs)"
  (
    cd "$LOCAL_DEV_DIR" || exit 2
    timeout 300 sh -c "echo test | hc sandbox --piped -f $(admin_port "$index") generate -n 1 \
      --app-id elohim --in-process-lair --root \"\$PWD\" -d $name \
      \"$HAPP_PATH\" $netargs"
  ) > "$generate_log" 2>&1
  local generate_status=$?
  echo " done"
  if [ "$generate_status" -ne 0 ] || grep -qa "Payload: Could not" "$generate_log"; then
    echo "join-peer: conductor generate failed (exit=$generate_status) — see $generate_log" >&2
    return 1
  fi

  local recorded_index
  recorded_index="$(awk -v want="$sandbox" '$0 == want { print NR - 1; exit }' "$LOCAL_DEV_DIR/.hc")"
  if [ -z "$recorded_index" ] || [ "$recorded_index" -ne "$index" ]; then
    echo "join-peer: generated sandbox index is '${recorded_index:-absent}', expected $index; refusing to guess" >&2
    return 1
  fi
  patch_mesh_gossip_config "$sandbox/conductor-config.yaml" || {
    echo "join-peer: gossip-config patch failed for $name" >&2
    return 1
  }

  local conductor_log="$LOCAL_DEV_DIR/.sandbox_run_log.$name"
  echo "starting late-join conductor $name: admin=$(admin_port "$index") app=$(app_port "$index")"
  if [ "${MESH_CONDUCTOR_LAUNCH:-hc}" = "ark" ]; then
    # assert_ark_binary already ran in assert_launch_prerequisites above, before
    # this peer's sandbox was generated — asking again here would only be able
    # to refuse a mesh that has already grown a data root.
    launch_ark_conductor "$name" "$index" || return 1
  elif [ "${MESH_CONDUCTOR_LAUNCH:-hc}" = "direct" ]; then
    local conductor_bin="${HOLOCHAIN_BIN:-$(command -v holochain)}"
    [ -x "$conductor_bin" ] || {
      echo "join-peer: conductor binary is not executable: $conductor_bin" >&2
      return 1
    }
    (
      export RUST_LOG="$MESH_RUST_LOG"
      cd "$LOCAL_DEV_DIR" || exit 2
      setsid nohup sh -c "echo test | '$conductor_bin' --piped --structured=Log --config-path '$sandbox/conductor-config.yaml'" \
        > "$conductor_log" 2>&1 &
      record_mesh_pid conductor "$name" "$!" || true
    )
  else
    (
      export RUST_LOG="$MESH_RUST_LOG"
      holochain_bin_export
      cd "$LOCAL_DEV_DIR" || exit 2
      setsid nohup sh -c "echo test | hc sandbox --piped -f $(admin_port "$index") run $index -p=$(app_port "$index")" \
        > "$conductor_log" 2>&1 &
      record_mesh_pid conductor-supervisor "$name" "$!" || true
    )
  fi

  local deadline=$((SECONDS + 180))
  while [ "$SECONDS" -lt "$deadline" ]; do
    if ss -H -ltn "sport = :$(admin_port "$index")" 2>/dev/null | grep -q . && \
       ss -H -ltn "sport = :$(app_port "$index")" 2>/dev/null | grep -q .; then
      break
    fi
    sleep 3
  done
  if ! ss -H -ltn "sport = :$(admin_port "$index")" 2>/dev/null | grep -q . || \
     ! ss -H -ltn "sport = :$(app_port "$index")" 2>/dev/null | grep -q .; then
    echo "join-peer: conductor $name did not expose both interfaces — see $conductor_log" >&2
    return 1
  fi
  record_listener_pid conductor "$name" "$(admin_port "$index")" || true

  # Reuse the boot roster's ONE storage launch path. Appending to PEERS is
  # invocation-local: status/ownership helpers below see the joiner, while a
  # later cold `start` retains the byte-identical configured roster flow.
  PEERS+=("$name")
  start_storage_peer "$name" "$index"
  deadline=$((SECONDS + 180))
  while [ "$SECONDS" -lt "$deadline" ]; do
    curl -s -m 2 "http://localhost:$(http_port "$index")/health" >/dev/null && break
    sleep 3
  done
  if ! curl -s -m 2 "http://localhost:$(http_port "$index")/health" >/dev/null; then
    echo "join-peer: storage $name did not serve — see $LOGDIR/$name.log" >&2
    return 1
  fi
  refresh_mesh_pidfiles

  local status node_id
  status="$(curl -fsS -m 5 "http://localhost:$(http_port "$index")/p2p/status" 2>/dev/null)"
  node_id="$(jq -r '
    if (.irohNodeId | type) == "string" and (.irohNodeId | length) > 0 then .irohNodeId
    elif (.peerId | type) == "string" and (.peerId | test("^[0-9a-fA-F]{64}$")) then .peerId
    else empty end
  ' <<<"$status" 2>/dev/null)"
  if [ -z "$node_id" ]; then
    echo "join-peer: $name serves but exposes no iroh NodeId; the late-join receipt requires dual/iroh" >&2
    return 1
  fi
  echo "JOINED_PEER name=$name index=$index http=http://localhost:$(http_port "$index") irohNodeId=$node_id"
}

start_all() {
  guard_conductor_data_roots start || return 1
  # The CLI writes the config the conductor must parse — refuse a mismatched
  # pair before anything is generated, not after three conductors panic. Under
  # `ark` the refusal that matters is a different one (the ark binary and jq),
  # and it is made in the same place, for the same reason.
  assert_launch_prerequisites || return 1
  mkdir -p "$MESH_DIR" "$LOGDIR" "$LOCAL_DEV_DIR" "$PID_DIR"

  # Peer policy: the storage binary loads ./config/peer-policy.toml relative to
  # ITS CWD; a missing file silently disables the whole heartbeat + signal-
  # subscriber + genesis-self-heal block ("PeerStatus heartbeat disabled" —
  # found 2026-08-16 as the root of NULL agent keys + zero peer-status rows on
  # the mesh). Generate a minimal local policy and pass it explicitly.
  POLICY_FILE="$MESH_DIR/peer-policy.toml"
  if [ ! -f "$POLICY_FILE" ]; then
    cat > "$POLICY_FILE" <<'EOF'
[pool]
accept_general_traffic = "auto"
min_free_storage_pct = 5
require_conductor_healthy = true

[stewardship]
accept_new_reserves = "auto"
max_storage_pct = 80

[network]
# All peers share one netns locally — conductor stays loopback-only, no
# forwarders (they would EADDRINUSE across the three peers). The bind/port
# fields are required by the TOML schema even when the switch is off.
expose_conductor_externally = false
conductor_external_bind = "0.0.0.0:8445"
conductor_internal_port = 4445
conductor_admin_external_bind = "0.0.0.0:8444"
conductor_admin_internal_port = 4444
EOF
  fi

  local required_bins=("$STORAGE_BIN")
  if [ "$MESH_DOORWAYS_EFFECTIVE" = "1" ]; then required_bins+=("$DOORWAY_BIN"); fi
  for bin in "${required_bins[@]}"; do
    [ -x "$bin" ] || { echo "missing binary: $bin (build it first — see CLAUDE.md pool-slot paths)"; exit 1; }
  done
  for name in "${PEERS[@]}"; do
    assert_storage_transport_capability "$STORAGE_BIN" "$(peer_transport "$name")" || exit 1
  done

  # Repack the happ when any DNA is newer than the bundle (stale-bundle trap:
  # elohim.happ predated lamad.dna by 3 months on 2026-08-16).
  if [ ! -f "$HAPP_PATH" ] || [ -n "$(find "$HAPP_WORKDIR" -name '*.dna' -newer "$HAPP_PATH" 2>/dev/null)" ]; then
    echo "repacking elohim.happ (DNA newer than bundle)"
    (cd "$HAPP_WORKDIR" && hc app pack . -o elohim.happ) || exit 1
  fi

  if [ "$MESH_DOORWAYS_EFFECTIVE" = "1" ]; then
  # 0. mongod — the doorways' projection archive. Must be listening BEFORE a
  #    doorway boots: AppState::init_projection binds the archive at startup
  #    (bind_warm_shell_to_archive), and a doorway that boots archive-less
  #    stays inert for its whole life. Optional: no binary => skip, and the
  #    doorways degrade to today's memory-only shape (status says so).
  if [ -n "$MONGOD_BIN" ] && [ -x "$MONGOD_BIN" ]; then
    if ! (exec 3<>"/dev/tcp/127.0.0.1/$MONGO_PORT") 2>/dev/null; then
      mkdir -p "$MONGO_DIR"
      "$MONGOD_BIN" --dbpath "$MONGO_DIR" --bind_ip 127.0.0.1 --port "$MONGO_PORT" \
        --fork --logpath "$LOGDIR/mongod.log" >/dev/null 2>&1 \
        || echo "WARN: mongod failed to start (see $LOGDIR/mongod.log) — doorways will run archive-less" >&2
      for _ in $(seq 1 20); do
        (exec 3<>"/dev/tcp/127.0.0.1/$MONGO_PORT") 2>/dev/null && break; sleep 1
      done
      record_listener_pid mongod mesh "$MONGO_PORT" || true
      echo "mongod up on :$MONGO_PORT (dbpath $MONGO_DIR)"
    else
      record_listener_pid mongod mesh "$MONGO_PORT" || true
      echo "mongod already up on :$MONGO_PORT"
    fi
  else
    echo "mongod not found (MONGOD_BIN unset/absent) — doorways will run archive-less (inert warm shell)"
  fi

  # 0. Relay first: every 0.7 conductor homes to it at boot (see
  # MESH_FORK_RELAY_URL) — a conductor generated before the relay exists is
  # fine, one that never finds it reports 0 connections forever.
  start_local_relay || return 1

  # 1. Doorway first: it is the island DHT's bootstrap + signal home.
  if ! curl -s -m 2 "http://localhost:$DOORWAY_PORT/health" >/dev/null; then
    local i=0 primary="" extras=""
    for name in "${PEERS[@]}"; do
      if [ $i -eq 0 ]; then primary="http://127.0.0.1:$(http_port $i)"
      else extras+="${extras:+,}http://127.0.0.1:$(http_port $i)"; fi
      i=$((i+1))
    done
    # DOORWAY_ID models alpha's alpha-elohim-host: the EPR router filters
    # projections by doorway_id, so an unset id matches ZERO seeded rows.
    # SSR env mirrors genesis/orchestrator/manifests/doorway/alpha.yaml: the
    # RendererRegistry materializes each slug's server bundle from the
    # substrate (serverBlobHash staged via scripts/ci/stage-spa-blob.sh); an
    # unstaged slug degrades to CSR with x-ssr-skipped. The landing browser
    # bundle carries only index.csr.html, so WITHOUT SSR the / mount 404s.
    # Membrane thresholds (per client IP per 60s window; doorway membrane.rs defaults
    # 300/600/1200, ban 900s). Every a2o request on this mesh arrives from ONE loopback
    # address, and the destructive/load scenarios the owned lane runs (warm-up budgets,
    # reconnect churn, coalescing cold reads) exceed 1200/min — 2026-08-22 a re-exec'd
    # doorway A answered 403 {"error":"Forbidden"} x-membrane:deny to the rest of the lane.
    # Override per run; the fleet keeps the binary defaults.
    #
    # NO COMMENT LINES INSIDE THE ASSIGNMENT LIST BELOW: a comment ends the backslash
    # continuation, so everything above it becomes a plain shell assignment that the
    # nohup command never sees. 2026-08-22 (123cea498) that severed DOORWAY_ID and
    # DOORWAY_HEALTH_PORT — doorway A booted with a random doorway_id, matched ZERO
    # project-epr rows, and served / as 503 and /lamad as 404 for a whole lane.
    local gw_a=()
    [ "$MESH_DOORWAY_GATEWAY_SCOPING" = "1" ] && gw_a=("DOORWAY_URL=http://localhost:$DOORWAY_PORT")
    env "${gw_a[@]}" \
    DOORWAY_ID="${DOORWAY_ID:-alpha-elohim-host}" \
    DOORWAY_HEALTH_PORT="$DOORWAY_A_HEALTH_PORT" \
    MONGODB_URI="mongodb://127.0.0.1:$MONGO_PORT" MONGODB_DB="doorway-a" \
    ELOHIM_NETWORK_STAKES="$ELOHIM_NETWORK_STAKES" \
    API_KEY_ADMIN="${MESH_API_KEY_ADMIN:-mesh-admin-dev-key}" \
    DOORWAY_MEMBRANE_SHAPE_THRESHOLD="${DOORWAY_MEMBRANE_SHAPE_THRESHOLD:-100000}" \
    DOORWAY_MEMBRANE_CHALLENGE_THRESHOLD="${DOORWAY_MEMBRANE_CHALLENGE_THRESHOLD:-200000}" \
    DOORWAY_MEMBRANE_BAN_THRESHOLD="${DOORWAY_MEMBRANE_BAN_THRESHOLD:-400000}" \
    SSR_BUNDLE_PATH="${SSR_BUNDLE_PATH:-$REPO_ROOT/app/elohim-app/dist/elohim-app/server/main.server.mjs}" \
    SSR_BUNDLE_SLUG="${SSR_BUNDLE_SLUG:-elohim-host-landing}" \
    SSR_BUNDLE_SLUGS="${SSR_BUNDLE_SLUGS:-elohim-host-landing,lamad-spa}" \
    DOORWAY_MANIFEST_BOARD_ENABLED="${DOORWAY_MANIFEST_BOARD_ENABLED:-true}" \
    nohup "$DOORWAY_BIN" --dev-mode --dev-signal-subscriber --listen "0.0.0.0:$DOORWAY_PORT" \
      --conductor-url "ws://localhost:$(admin_port 0)" \
      --app-port-min "$(app_port 0)" \
      --storage-url "$primary" ${extras:+--storage-urls "$extras"} \
      --bootstrap-enabled --signal-enabled > "$LOGDIR/doorway.log" 2>&1 &
    record_mesh_pid doorway a "$!" || true
    for _ in $(seq 1 20); do
      curl -s -m 2 "http://localhost:$DOORWAY_PORT/health" >/dev/null && break; sleep 1
    done
    echo "doorway up on :$DOORWAY_PORT (bootstrap+signal enabled)"
  else
    record_listener_pid doorway a "$DOORWAY_PORT" || true
    echo "doorway already up on :$DOORWAY_PORT"
  fi

  # 1b. Doorway B (apex/elohim.host stand-in): jessica-primary, NO bootstrap/
  # signal (A owns discovery — two mem-bootstrap doorways would partition the
  # island DHT). Gives the saga's cross-doorway legs a LOCAL target instead of
  # bleeding to the live production doorway (E2E_DOORWAY_B).
  if ! curl -s -m 2 "http://localhost:$DOORWAY_B_PORT/health" >/dev/null; then
    local gw_b=()
    [ "$MESH_DOORWAY_GATEWAY_SCOPING" = "1" ] && gw_b=("DOORWAY_URL=http://localhost:$DOORWAY_B_PORT")
    env "${gw_b[@]}" \
    DOORWAY_ID="${DOORWAY_B_ID:-apex-elohim-host}" \
    DOORWAY_HEALTH_PORT="$DOORWAY_B_HEALTH_PORT" \
    MONGODB_URI="mongodb://127.0.0.1:$MONGO_PORT" MONGODB_DB="doorway-b" \
    ELOHIM_NETWORK_STAKES="$ELOHIM_NETWORK_STAKES" \
    API_KEY_ADMIN="${MESH_API_KEY_ADMIN:-mesh-admin-dev-key}" \
    DOORWAY_MEMBRANE_SHAPE_THRESHOLD="${DOORWAY_MEMBRANE_SHAPE_THRESHOLD:-100000}" \
    DOORWAY_MEMBRANE_CHALLENGE_THRESHOLD="${DOORWAY_MEMBRANE_CHALLENGE_THRESHOLD:-200000}" \
    DOORWAY_MEMBRANE_BAN_THRESHOLD="${DOORWAY_MEMBRANE_BAN_THRESHOLD:-400000}" \
    SSR_BUNDLE_PATH="${SSR_BUNDLE_PATH:-$REPO_ROOT/app/elohim-app/dist/elohim-app/server/main.server.mjs}" \
    SSR_BUNDLE_SLUG="${SSR_BUNDLE_SLUG:-elohim-host-landing}" \
    SSR_BUNDLE_SLUGS="${SSR_BUNDLE_SLUGS:-elohim-host-landing,lamad-spa}" \
    DOORWAY_MANIFEST_BOARD_ENABLED="${DOORWAY_MANIFEST_BOARD_ENABLED:-true}" \
    nohup "$DOORWAY_BIN" --dev-mode --dev-signal-subscriber --listen "0.0.0.0:$DOORWAY_B_PORT" \
      --conductor-url "ws://localhost:$(admin_port 1)" \
      --app-port-min "$(app_port 1)" \
      --storage-url "http://127.0.0.1:$(http_port 1)" \
      --storage-urls "http://127.0.0.1:$(http_port 0),http://127.0.0.1:$(http_port 2)" \
      > "$LOGDIR/doorway-b.log" 2>&1 &
    record_mesh_pid doorway b "$!" || true
    for _ in $(seq 1 20); do
      curl -s -m 2 "http://localhost:$DOORWAY_B_PORT/health" >/dev/null && break; sleep 1
    done
    echo "doorway B up on :$DOORWAY_B_PORT (apex stand-in, jessica-primary)"
  else
    record_listener_pid doorway b "$DOORWAY_B_PORT" || true
    echo "doorway B already up on :$DOORWAY_B_PORT"
  fi

  # 1c. The doorway sign-in portal (doorway-app). The doorway forwards /threshold/*
  # to THRESHOLD_URL with the path INTACT (doorway-service/src/routes/threshold.rs),
  # and its default is http://localhost:8081 — so serving doorway-app there under
  # /threshold makes the chaperone portal reachable at
  # http://localhost:$DOORWAY_PORT/threshold/login exactly as the deployed sidecar
  # serves it. Without this the path is a 502 and no browser scenario can exercise
  # a real login locally.
  #
  # The SPA calls the doorway SAME-ORIGIN (doorway-app environment.doorwayUrl is
  # ''), so it must be driven through the doorway, never against this port directly
  # -- there its API calls would hit the dev server and 404.
  #
  # Started detached and NOT waited on: the dev server takes ~40s to become ready
  # and nothing else in the mesh depends on it, so blocking here would tax every
  # `mesh start` for a surface most runs never touch.
  if [ "$MESH_PORTAL" = "1" ]; then
    if curl -s -m 2 -o /dev/null "http://127.0.0.1:$THRESHOLD_PORT/threshold/"; then
      record_listener_pid portal mesh "$THRESHOLD_PORT" || true
      echo "portal already up on :$THRESHOLD_PORT"
    elif [ -d "$REPO_ROOT/doorway/doorway-app/node_modules" ] || [ -d "$REPO_ROOT/node_modules" ]; then
      ( cd "$REPO_ROOT/doorway/doorway-app" && \
        # --live-reload=false: the doorway PROXIES /threshold/* to this server, and
        # a hot-reload WebSocket cannot traverse that proxy — it fails the
        # handshake against the proxied 200 and emits console errors on every
        # page. Those errors are indistinguishable from product errors to the
        # browser a2o lane, which asserts a clean console after login. A portal
        # that is only ever reached through the proxy has no use for HMR anyway.
        nohup pnpm exec ng serve --port "$THRESHOLD_PORT" --serve-path /threshold \
          --live-reload false \
          --host 127.0.0.1 > "$LOGDIR/portal.log" 2>&1 & \
        record_mesh_pid portal mesh "$!" || true )
      echo "portal starting on :$THRESHOLD_PORT (doorway-app; ~40s to first paint, log: $LOGDIR/portal.log)"
    else
      echo "portal SKIPPED: no node_modules for doorway-app — run pnpm install (set MESH_PORTAL=0 to silence)" >&2
    fi
  fi
  fi

  # 2. Conductors: hc sandbox generate (installs the happ + writes each
  #    sandbox's conductor-config.yaml, but does NOT launch the conductor —
  #    only `-r`/`run` does that per `hc sandbox generate --help`), THEN
  #    patch the dev-tier gossip config into each written config, THEN run.
  #    Splitting generate from run (the old code combined them via
  #    `generate -r=$rports`) creates the window the pacing profile needs:
  #    the conductor must not boot until the patch has landed, or its first
  #    kitsune2 gossip round starts at prod cadence.
  if [ "$(ss -tln | grep -cE "127.0.0.1:$(admin_port 0) ")" -eq 0 ]; then
    # Peer 0's silent admin port is not evidence that peers 1..n are idle, nor
    # that an ark is not alive between incarnations — and the next line removes
    # every peer's data root. Ask each peer before destroying anything.
    assert_no_live_peer_processes start || return 1
    cd "$LOCAL_DEV_DIR" || exit 2
    rm -rf .hc .sandbox_log .sandbox_run_log "${PEERS[@]}"
    local fports="" rports=""
    local i=0
    for _ in "${PEERS[@]}"; do
      fports+="${fports:+,}$(admin_port $i)"; rports+="${rports:+,}$(app_port $i)"; i=$((i+1))
    done

    local netargs; netargs="$(mesh_network_args)"
    echo -n "generating ${#PEERS[@]} conductor sandboxes ($netargs; cold install can take ~2-4 min)"
    timeout 300 sh -c "echo test | hc sandbox --piped -f $fports generate -n ${#PEERS[@]} \
      --app-id elohim --in-process-lair --root \"\$PWD\" -d $MESH_PEERS \
      \"$HAPP_PATH\" $netargs" > .sandbox_log 2>&1
    gen_status=$?
    echo " done"
    if [ "$gen_status" -ne 0 ] || grep -qa "Payload: Could not" .sandbox_log; then
      echo "conductor generate failed (exit=$gen_status) — see $LOCAL_DEV_DIR/.sandbox_log"
      exit 1
    fi

    # -------------------------------------------------------------------
    # dev-tier gossip acceleration (declared preproduction stakes — see
    # minutes-quiesce plan W3). Holochain 0.6's ConductorConfig.network.
    # advanced JSON passes straight through to kitsune2
    # (holochain-conductor crates/holochain_conductor_api/src/config/
    # conductor.rs with_gossip_*_interval_ms; the k2Gossip module name and
    # camelCase keys are its wire shape). initiateIntervalMs /
    # minInitiateIntervalMs at 1000ms match what SweetConductorConfig::
    # standard() sets for upstream tests (sweettest/sweet_conductor_
    # config.rs:82-84) — vs prod defaults of 120_000ms / 300_000ms
    # (elohim/kitsune2 crates/gossip/src/config.rs). initialInitiateIntervalMs
    # (first-round-only interval; same struct, same file — confirmed present
    # under `#[serde(rename_all = "camelCase")]` before wiring it here) is
    # set explicitly for clarity — its default is already 1000ms.
    # Boot-verified 2026-08-16: a scratch single-sandbox conductor patched
    # with this exact block came up clean (admin port live, `list-apps`
    # returned the installed happ, no config-parse errors) before this loop
    # was wired to run for real.
    # -------------------------------------------------------------------
    for name in "${PEERS[@]}"; do
      patch_mesh_gossip_config "$LOCAL_DEV_DIR/$name/conductor-config.yaml"
      if [ $? -ne 0 ]; then
        echo "gossip-config patch failed for $name — see $LOCAL_DEV_DIR/$name/conductor-config.yaml"
        exit 1
      fi
    done
    echo "dev-tier gossip config patched into ${#PEERS[@]} conductor-config.yaml (k2Gossip initiate=1000ms)"

    if [ "${MESH_CONDUCTOR_LAUNCH:-hc}" = "ark" ]; then
      # No -p here, and none is needed: elohim-storage attaches its own app
      # interface over the admin websocket at startup (hc_client.rs /
      # signing.rs attach_app_interface), so the app ports come up with the
      # storage peers rather than with the conductor launcher.
      launch_ark_conductors || exit 1
    else
      (
        export RUST_LOG="$MESH_RUST_LOG"
        holochain_bin_export
        setsid nohup sh -c "echo test | hc sandbox --piped -f $fports run -a -p=$rports" > .sandbox_run_log 2>&1 &
        record_mesh_pid conductor-supervisor mesh "$!" || true
      )
    fi
    echo -n "waiting for ${#PEERS[@]} conductors to boot"
    for _ in $(seq 1 90); do
      [ "$(ss -tln | grep -cE "127.0.0.1:($(echo "$fports" | tr ',' '|')) ")" -ge ${#PEERS[@]} ] && break
      # 2>/dev/null: ark mode writes per-peer logs and never creates the
      # multiplexed .sandbox_run_log, so this probe must be silent about its
      # absence rather than printing a grep error every three seconds. In `hc`
      # mode the file exists and the behaviour is unchanged.
      grep -qa "Payload: Could not" .sandbox_run_log 2>/dev/null && { echo; echo "conductor run failed — see $LOCAL_DEV_DIR/.sandbox_run_log"; exit 1; }
      printf "."; sleep 3
    done
    echo " up"
    refresh_mesh_pidfiles
  else
    refresh_mesh_pidfiles
    echo "conductors already up"
  fi

  # 3. Storage peers: one per conductor, agent key read from its conductor.
  local i=0
  for name in "${PEERS[@]}"; do
    start_storage_peer "$name" "$i"
    i=$((i+1))
  done

  for _ in $(seq 1 30); do
    local ok=0 j=0
    for _n in "${PEERS[@]}"; do
      curl -s -m 2 "http://localhost:$(http_port $j)/health" >/dev/null && ok=$((ok+1)); j=$((j+1))
    done
    [ "$ok" -ge ${#PEERS[@]} ] && break; sleep 2
  done

  refresh_mesh_pidfiles

  echo
  status_all
  echo
  echo "next: ./hc-mesh.sh probe   # run the CI Dataplane Validation probes here"
}

mesh_coordswap() {
  guard_conductor_data_roots coordswap || return 1
  "$REPO_ROOT/scripts/ci/fleet-coordswap.sh" "$@"
}

# Dispatch guard: only run the action switch when this file is EXECUTED, not
# when it is SOURCED. hc-mesh-prologue.sh (and an operator's shell) source
# this script to reuse conductor_csv/peer_csv/mesh_seed_env without risking an
# accidental start/stop of a live mesh (a storage agent may be using it).
if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
  case "${1:-start}" in
    start)    start_all ;;
    stop)     stop_all ;;
    status)   status_all ;;
    probe)    probe_all ;;
    join-peer) shift; join_peer "$@" ;;
    conductors-restart) restart_conductors ;;
    coordswap) shift; mesh_coordswap "$@" ;;
    storage-restart) shift; restart_storage "$@" ;;
    zome-probe) probe_zome_paths ;;
    fixture-refresh) refresh_fixture_pids ;;
    prologue) shift; exec bash "$SCRIPT_DIR/hc-mesh-prologue.sh" "$@" ;;
    *) echo "usage: hc-mesh.sh [start|stop|status|probe|prologue|join-peer <fresh-name>|conductors-restart|coordswap <fleet-coordswap args...>|storage-restart [peer...]|zome-probe|fixture-refresh]"; exit 2 ;;
  esac
fi
