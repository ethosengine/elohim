#!/bin/bash
#
# hc-mesh.sh - Local multi-peer Elohim mesh (Eclipse Che / single container)
#
# Brings up the ALPHA-SHAPED topology entirely on loopback, no k8s, no
# external services:
#
#   1 doorway   (dev mode, serves /bootstrap + /signal for the island DHT,
#                proxies all N storage peers)
#   N conductors (hc sandbox, holochain 0.6: --piped + -f pinned admin ports,
#                -n N native multi-sandbox; discovery via the LOCAL doorway)
#   N storages  (release binary, libp2p mesh via mDNS inside the container,
#                each bound to its own conductor)
#
# Verified bring-up 2026-08-16: mesh/upload/propagation/delivery/projection
# probes green via `substrate-verify.ts` against this fleet (same assertion
# set as CI's Dataplane Validation).
#
# USAGE:
#   ./hc-mesh.sh [start|stop|status|probe|prologue|conductors-restart]
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
# ENVIRONMENT:
#   MESH_PEERS      Peer names, comma-separated (default: matthew,jessica,james)
#   MESH_DIR        Data root (default: /tmp/elohim-local-mesh)
#   DOORWAY_PORT    Doorway HTTP port (default: 8888)
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
#                   Relay URL for the fork's `network … quic <RELAY_URL>`
#                   generate grammar (stock 0.6.0 takes `webrtc <SIGNAL_URL>`
#                   instead; the grammar is read from the CLI, not guessed).
#                   Defaults to the doorway — a placeholder that must parse as a
#                   URL, since loopback peers never need a relay.
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
#   cadence; the conductor sandboxes also get a fixed kitsune2 gossip
#   acceleration patch (k2Gossip initiate=1000ms) after generate, before run.
#
# LOAD-BEARING FACTS (verified against holochain 0.6.0 / hc 0.6.0):
#   - `hc sandbox --piped` reads the lair passphrase from stdin: the old
#     socat/PTY wrapper is obsolete.
#   - `-f p1,p2,..` pins admin ports; `-r=a1,a2,..` pins app ports; no more
#     log-scraping for dynamic ports.
#   - tx5 REJECTS a signal URL with a path ("parsing tx5 sig url ...
#     InvalidLastSymbol"): the URL must be pathless. The doorway detects a
#     signal request by Host header prefix `signal.`, and `signal.localhost`
#     resolves to loopback out of the box -> ws://signal.localhost:PORT.
#   - `hc sandbox generate` WITHOUT a network section points at the PUBLIC
#     dev-test-bootstrap2.holochain.org. True isolation requires the explicit
#     `network --bootstrap ... webrtc ...` pointing at the local doorway.
#   - Conductor admin/app interfaces bind loopback only; config exposes
#     `danger_bind_addr` when a cross-pod topology ever needs more. Inside
#     one container, loopback is correct.
#   - elohim-storage ignores the HTTP_PORT env var: pass --http-port.
#
set -u

MESH_PEERS="${MESH_PEERS:-matthew,jessica,james}"
MESH_DIR="${MESH_DIR:-/tmp/elohim-local-mesh}"
DOORWAY_PORT="${DOORWAY_PORT:-8888}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
LOCAL_DEV_DIR="$REPO_ROOT/elohim/holochain/local-dev"
HAPP_WORKDIR="$REPO_ROOT/elohim/holochain/dna/elohim/workdir"
HAPP_PATH="$HAPP_WORKDIR/elohim.happ"
POOL="/projects/.cargo-target-pool/family/dev"
STORAGE_BIN="${STORAGE_BIN:-$POOL/elohim__elohim-storage/release/release/elohim-storage}"
DOORWAY_BIN="${DOORWAY_BIN:-$POOL/doorway__doorway-service/dev/debug/doorway}"
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

# Port scheme per peer index i (0-based): admin 4444+10i, app 4445+10i,
# storage http 8090+i, libp2p 9701+i.
IFS=',' read -ra PEERS <<< "$MESH_PEERS"

admin_port() { echo $((4444 + 10 * $1)); }
app_port()   { echo $((4445 + 10 * $1)); }
http_port()  { echo $((8090 + $1)); }
p2p_port()   { echo $((9701 + $1)); }

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
# ITS schema and a mismatched pair simply will not boot (0.6.0 hc + 0.6.3
# conductor: "unknown field base64_auth_material", then "missing field
# relay_url"). A directory with only one of them is skipped, loudly, rather than
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
# in the conductor log — measured both directions on 2026-08-21: 0.6.3 refuses
# `network.base64_auth_material` ("expected one of base64_auth_material_bootstrap,
# base64_auth_material_relay, …"), and once that key is dropped it fails on
# `missing field relay_url`, which 0.6.0 never writes.
#
# The gate compares the FULL version, not the major.minor "minor line": 0.6.0
# and 0.6.3 agree on 0.6 and are still schema-incompatible — a minor-line check
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
  echo "  conductor is handed a file it may refuse to parse (0.6.0 hc + 0.6.3" >&2
  echo "  conductor: 'unknown field base64_auth_material', then 'missing field" >&2
  echo "  relay_url'). Point HOLOCHAIN_BIN at a DIRECTORY holding both binaries:" >&2
  echo "     HOLOCHAIN_BIN=/path/to/fork-bin ./hc-mesh.sh start" >&2
  echo "  (MESH_ALLOW_TOOLCHAIN_SKEW=1 to proceed anyway — deliberate skew only.)" >&2
  echo "" >&2
  [ "${MESH_ALLOW_TOOLCHAIN_SKEW:-0}" = "1" ] || exit 1
  echo "WARN: proceeding on a skewed pair because MESH_ALLOW_TOOLCHAIN_SKEW=1" >&2
}

# ---------------------------------------------------------------------------
# The generate-time transport subcommand is NOT the same word on both lines.
# Stock 0.6.0 offers `webrtc <SIGNAL_URL>`; the fork's 0.6.3 `hc` replaced it
# with `quic <RELAY_URL>` (the iroh transport), and passing the stock word to
# the fork CLI dies before anything is written:
#     error: unrecognized subcommand 'webrtc'
# Read the grammar from the CLI that will actually run rather than branching on
# a version number — the CLI is the authority on its own subcommands.
#
# MESH_FORK_RELAY_URL is a placeholder on a loopback mesh: peers reach each
# other directly and the relay only matters for NAT traversal, so pointing it
# at the doorway (an http URL that parses) is enough. It must be a real URL —
# `relay_url: null` is rejected with `relative URL without a base: "null"`.
# ---------------------------------------------------------------------------
mesh_network_args() { # -> the `network …` tail for `hc sandbox generate`
  local relay="${MESH_FORK_RELAY_URL:-http://localhost:$DOORWAY_PORT}"
  if hc sandbox generate network --help 2>&1 | grep -qE '^\s+quic\b'; then
    echo "network --bootstrap http://localhost:$DOORWAY_PORT/bootstrap quic $relay"
  else
    echo "network --bootstrap http://localhost:$DOORWAY_PORT/bootstrap webrtc ws://signal.localhost:$DOORWAY_PORT"
  fi
}

stop_all() {
  # Kill by exact binary identity, NEVER by a pattern that could match the
  # caller's own command line (self-kill class, hit twice on 2026-08-16).
  kill $(pgrep -x holochain) 2>/dev/null
  kill $(pgrep -f "[h]c sandbox") 2>/dev/null
  kill $(pgrep -f "elohim-storag[e]") 2>/dev/null
  kill $(pgrep -f "(debug|release)/doorwa[y]") 2>/dev/null
  # mongod: exact dbpath identity, never a bare "mongod" pattern.
  kill $(pgrep -f "mongod --dbpath $MESH_DIR/mong[o]") 2>/dev/null
  sleep 1
  echo "mesh stopped"
}

status_all() {
  echo "conductors:"; ss -tln 2>/dev/null | grep -E "127.0.0.1:44[0-9]{2} " || echo "  (none)"
  local i=0
  for name in "${PEERS[@]}"; do
    printf "  %-8s admin=%s app=%s  storage=" "$name" "$(admin_port $i)" "$(app_port $i)"
    curl -s -m 2 "http://localhost:$(http_port $i)/health" >/dev/null && echo "UP :$(http_port $i)" || echo "down"
    i=$((i+1))
  done
  printf "doorway  :%s " "$DOORWAY_PORT"
  curl -s -m 2 "http://localhost:$DOORWAY_PORT/health" >/dev/null && echo UP || echo down
  printf "doorwayB :%s " "${DOORWAY_B_PORT:-8889}"
  curl -s -m 2 "http://localhost:${DOORWAY_B_PORT:-8889}/health" >/dev/null && echo UP || echo down
  printf "mongod   :%s " "$MONGO_PORT"
  if (exec 3<>"/dev/tcp/127.0.0.1/$MONGO_PORT") 2>/dev/null; then echo "UP (archive-backed doorways)"; else echo "down (doorways run archive-less: inert warm shell)"; fi
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
}

probe_all() {
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

  local name port pid envfile exefile cwd bin key k failed=""
  for name in "${targets[@]}"; do
    port=""; k=0
    for n2 in "${PEERS[@]}"; do [ "$n2" = "$name" ] && port="$(http_port $k)"; k=$((k+1)); done
    [ -n "$port" ] || { echo "  $name: not a mesh peer ($MESH_PEERS)" >&2; continue; }
    envfile="$workdir/$name.environ"
    exefile="$workdir/$name.exe"

    # Exact pid: the storage binary's argv carries --http-port <port>, which no
    # other process has, and which never appears in this script's own argv.
    pid="$(ps -eo pid=,args= | awk -v me="$$" -v pat="--http-port $port" \
      '$1 != me && index($0, "elohim-storage") && index($0, pat) { print $1; exit }')"

    # Capture the live environment when we can. A pid that exists but whose
    # /proc is unreadable (a process already exiting) is treated exactly like no
    # pid at all — fall through to the last good capture rather than give up,
    # which is the difference between a recoverable peer and a dead one.
    if [ -n "$pid" ] && cp "/proc/$pid/environ" "$envfile" 2>/dev/null; then
      cwd="$(readlink "/proc/$pid/cwd")"
      if ! bin="$(resolve_exe "$pid" "$STORAGE_BIN")"; then
        echo "  $name: binary is gone (was '$bin'); rebuild it or set STORAGE_BIN" >&2
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
      # Binary, in order of trust: the capture's own exe record (written by this
      # script's live branch and by the a2o drills' writeRestartCapture), the
      # exe of any sibling peer still running (same build), then STORAGE_BIN.
      bin="$(restore_binary_for "$exefile")"
      if [ -z "$bin" ]; then
        echo "  $name: no executable storage binary — $exefile absent, no sibling peer running, STORAGE_BIN=$STORAGE_BIN not executable" >&2
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
os.execve(binpath, [binpath, "--http-port", port], env)
' "$envfile" "$bin" "$port" "$cwd" "$(restart_env_overlay)" >> "$LOGDIR/$name.log" 2>&1 &
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
    curl -s -m 2 "http://localhost:$port/health" >/dev/null || {
      echo "  $n2 :$port did not come back within the wait" >&2; failed+=" $n2"; }
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

# Env keys layered over a restarted peer's captured environment, one K=V per
# line. Empty by default — the capture is the truth. Two opt-ins:
#   MESH_RESTART_APPLY_PROFILE=1   re-apply THIS script's dev-tier pacing profile
#                                  (the same knobs `start` exports), so a knob
#                                  added after boot reaches a running mesh
#                                  without regenerating it. Never touches
#                                  AGENT_PUBKEY or any non-profile key.
#   MESH_RESTART_ENV_OVERLAY="K=V K=V"   ad-hoc keys for one experiment.
restart_env_overlay() {
  if [ "${MESH_RESTART_APPLY_PROFILE:-0}" = "1" ]; then
    printf '%s\n' \
      "PROJECTION_RECONCILE_SECS=$PROJECTION_RECONCILE_SECS" \
      "ACQUISITION_RECONCILE_SECS=$ACQUISITION_RECONCILE_SECS" \
      "CONTEST_BACKOFF_SECONDS=$CONTEST_BACKOFF_SECONDS" \
      "HEAL_MISSING_BACKOFF_SECONDS=$HEAL_MISSING_BACKOFF_SECONDS" \
      "ELOHIM_EVIDENCE_ABSENT_BACKOFF_SECS=$ELOHIM_EVIDENCE_ABSENT_BACKOFF_SECS" \
      "ELOHIM_HEAD_CORPUS_DIGEST=$ELOHIM_HEAD_CORPUS_DIGEST" \
      "ELOHIM_ADOPT_BEFORE_AUTHOR=$ELOHIM_ADOPT_BEFORE_AUTHOR" \
      "ADOPT_CONTEST_FANOUT=$ADOPT_CONTEST_FANOUT" \
      "ELOHIM_NETWORK_STAKES=$ELOHIM_NETWORK_STAKES"
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
print(next((i["id"] for i in items if i.get("dhtAnchorHash")), ""))
' 2>/dev/null)"
      if [ -z "$probe_id" ]; then
        echo "serving, but no ANCHORED content to probe the zome path with (inconclusive)"
      else
        body="$(curl -s -m 15 -H "Authorization: Bearer ${MESH_API_KEY_ADMIN:-mesh-admin-dev-key}" \
          "http://localhost:$port/db/content/$probe_id/head-record" 2>/dev/null)"
        case "$body" in
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
  cd "$LOCAL_DEV_DIR" || exit 2

  local fports="" aports="" i=0
  for _ in "${PEERS[@]}"; do
    fports+="${fports:+,}$(admin_port $i)"; aports+="${aports:+,}$(app_port $i)"; i=$((i+1))
  done

  # A restart re-runs `hc sandbox run`, which rewrites conductor-config.yaml in
  # the CLI's schema before the conductor reads it — so the pair must agree here
  # too, not only at generate time.
  [ "${MESH_CONDUCTOR_LAUNCH:-hc}" = "direct" ] || assert_toolchain_parity

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
  if [ "${MESH_CONDUCTOR_LAUNCH:-hc}" = "direct" ]; then
    local hc_bin="${HOLOCHAIN_BIN:-$(command -v holochain)}"
    [ -x "$hc_bin" ] || { echo "no conductor binary: $hc_bin" >&2; return 1; }
    echo "  launch mode: direct (per-conductor logs, no hc CLI rewrite)"
    for name in "${PEERS[@]}"; do
      (
        export RUST_LOG="$MESH_RUST_LOG"
        cd "$LOCAL_DEV_DIR" || exit 1
        setsid nohup sh -c "echo test | '$hc_bin' --piped --structured=Log --config-path '$LOCAL_DEV_DIR/$name/conductor-config.yaml'" \
          >> "$LOCAL_DEV_DIR/.sandbox_run_log.$name" 2>&1 &
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
print(next((i["id"] for i in items if i.get("dhtAnchorHash")), ""))
' 2>/dev/null)"
      if [ -z "$probe_id" ]; then
        echo "serving, but no ANCHORED content to probe the zome path with (inconclusive)"
      else
        body="$(curl -s -m 15 -H "Authorization: Bearer ${MESH_API_KEY_ADMIN:-mesh-admin-dev-key}" \
          "http://localhost:$port/db/content/$probe_id/head-record" 2>/dev/null)"
        case "$body" in
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

start_all() {
  # The CLI writes the config the conductor must parse — refuse a mismatched
  # pair before anything is generated, not after three conductors panic.
  assert_toolchain_parity
  mkdir -p "$MESH_DIR" "$LOGDIR" "$LOCAL_DEV_DIR"

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

  for bin in "$STORAGE_BIN" "$DOORWAY_BIN"; do
    [ -x "$bin" ] || { echo "missing binary: $bin (build it first — see CLAUDE.md pool-slot paths)"; exit 1; }
  done

  # Repack the happ when any DNA is newer than the bundle (stale-bundle trap:
  # elohim.happ predated lamad.dna by 3 months on 2026-08-16).
  if [ ! -f "$HAPP_PATH" ] || [ -n "$(find "$HAPP_WORKDIR" -name '*.dna' -newer "$HAPP_PATH" 2>/dev/null)" ]; then
    echo "repacking elohim.happ (DNA newer than bundle)"
    (cd "$HAPP_WORKDIR" && hc app pack . -o elohim.happ) || exit 1
  fi

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
      echo "mongod up on :$MONGO_PORT (dbpath $MONGO_DIR)"
    else
      echo "mongod already up on :$MONGO_PORT"
    fi
  else
    echo "mongod not found (MONGOD_BIN unset/absent) — doorways will run archive-less (inert warm shell)"
  fi

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
    DOORWAY_ID="${DOORWAY_ID:-alpha-elohim-host}" \
    DOORWAY_HEALTH_PORT="${DOORWAY_A_HEALTH_PORT:-8079}" \
    # Membrane thresholds (per client IP per 60s window; doorway membrane.rs defaults
    # 300/600/1200, ban 900s). Every a2o request on this mesh arrives from ONE loopback
    # address, and the destructive/load scenarios the owned lane runs (warm-up budgets,
    # reconnect churn, coalescing cold reads) exceed 1200/min — 2026-08-22 a re-exec'd
    # doorway A answered 403 {"error":"Forbidden"} x-membrane:deny to the rest of the lane.
    # Override per run; the fleet keeps the binary defaults.
    MONGODB_URI="mongodb://127.0.0.1:$MONGO_PORT" MONGODB_DB="doorway-a" \
    ELOHIM_NETWORK_STAKES="$ELOHIM_NETWORK_STAKES" \
    API_KEY_ADMIN="${MESH_API_KEY_ADMIN:-mesh-admin-dev-key}" \
    DOORWAY_MEMBRANE_SHAPE_THRESHOLD="${DOORWAY_MEMBRANE_SHAPE_THRESHOLD:-100000}" \
    DOORWAY_MEMBRANE_CHALLENGE_THRESHOLD="${DOORWAY_MEMBRANE_CHALLENGE_THRESHOLD:-200000}" \
    DOORWAY_MEMBRANE_BAN_THRESHOLD="${DOORWAY_MEMBRANE_BAN_THRESHOLD:-400000}" \
    SSR_BUNDLE_PATH="${SSR_BUNDLE_PATH:-$REPO_ROOT/app/elohim-app/dist/elohim-app/server/main.server.mjs}" \
    SSR_BUNDLE_SLUG="${SSR_BUNDLE_SLUG:-elohim-host-landing}" \
    SSR_BUNDLE_SLUGS="${SSR_BUNDLE_SLUGS:-elohim-host-landing,lamad-spa}" \
    nohup "$DOORWAY_BIN" --dev-mode --dev-signal-subscriber --listen "0.0.0.0:$DOORWAY_PORT" \
      --conductor-url "ws://localhost:$(admin_port 0)" \
      --app-port-min "$(app_port 0)" \
      --storage-url "$primary" ${extras:+--storage-urls "$extras"} \
      --bootstrap-enabled --signal-enabled > "$LOGDIR/doorway.log" 2>&1 &
    for _ in $(seq 1 20); do
      curl -s -m 2 "http://localhost:$DOORWAY_PORT/health" >/dev/null && break; sleep 1
    done
    echo "doorway up on :$DOORWAY_PORT (bootstrap+signal enabled)"
  else
    echo "doorway already up on :$DOORWAY_PORT"
  fi

  # 1b. Doorway B (apex/elohim.host stand-in): jessica-primary, NO bootstrap/
  # signal (A owns discovery — two mem-bootstrap doorways would partition the
  # island DHT). Gives the saga's cross-doorway legs a LOCAL target instead of
  # bleeding to the live production doorway (E2E_DOORWAY_B).
  DOORWAY_B_PORT="${DOORWAY_B_PORT:-8889}"
  if ! curl -s -m 2 "http://localhost:$DOORWAY_B_PORT/health" >/dev/null; then
    DOORWAY_ID="${DOORWAY_B_ID:-apex-elohim-host}" \
    DOORWAY_HEALTH_PORT="${DOORWAY_B_HEALTH_PORT:-8089}" \
    MONGODB_URI="mongodb://127.0.0.1:$MONGO_PORT" MONGODB_DB="doorway-b" \
    ELOHIM_NETWORK_STAKES="$ELOHIM_NETWORK_STAKES" \
    API_KEY_ADMIN="${MESH_API_KEY_ADMIN:-mesh-admin-dev-key}" \
    DOORWAY_MEMBRANE_SHAPE_THRESHOLD="${DOORWAY_MEMBRANE_SHAPE_THRESHOLD:-100000}" \
    DOORWAY_MEMBRANE_CHALLENGE_THRESHOLD="${DOORWAY_MEMBRANE_CHALLENGE_THRESHOLD:-200000}" \
    DOORWAY_MEMBRANE_BAN_THRESHOLD="${DOORWAY_MEMBRANE_BAN_THRESHOLD:-400000}" \
    SSR_BUNDLE_PATH="${SSR_BUNDLE_PATH:-$REPO_ROOT/app/elohim-app/dist/elohim-app/server/main.server.mjs}" \
    SSR_BUNDLE_SLUG="${SSR_BUNDLE_SLUG:-elohim-host-landing}" \
    SSR_BUNDLE_SLUGS="${SSR_BUNDLE_SLUGS:-elohim-host-landing,lamad-spa}" \
    nohup "$DOORWAY_BIN" --dev-mode --dev-signal-subscriber --listen "0.0.0.0:$DOORWAY_B_PORT" \
      --conductor-url "ws://localhost:$(admin_port 1)" \
      --app-port-min "$(app_port 1)" \
      --storage-url "http://127.0.0.1:$(http_port 1)" \
      --storage-urls "http://127.0.0.1:$(http_port 0),http://127.0.0.1:$(http_port 2)" \
      > "$LOGDIR/doorway-b.log" 2>&1 &
    for _ in $(seq 1 20); do
      curl -s -m 2 "http://localhost:$DOORWAY_B_PORT/health" >/dev/null && break; sleep 1
    done
    echo "doorway B up on :$DOORWAY_B_PORT (apex stand-in, jessica-primary)"
  else
    echo "doorway B already up on :$DOORWAY_B_PORT"
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
      python3 - "$LOCAL_DEV_DIR/$name/conductor-config.yaml" <<'PYEOF'
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

# The fork's `hc` (0.6.3 line, iroh/quic transport) writes a DEFAULT
# `signal_url` pointing at a PUBLIC Holo server — measured 2026-08-21:
# `signal_url: wss://dev-test-bootstrap2.holochain.org/` alongside the
# loopback bootstrap_url and relay_url this script passes. It is almost
# certainly inert while irohTransport is the active transport, so this
# only SAYS SO rather than rewriting it: an isolated mesh should not
# carry a public endpoint, and whether tx5 ever reads it on the fork has
# not been measured. Settle it before the fork mesh is trusted offline.
_sig = network.get("signal_url") or ""
if _sig and "localhost" not in _sig and "127.0.0.1" not in _sig:
    print(f"  WARN {path}: signal_url points off-box ({_sig}) — isolated-mesh sovereignty unverified on the fork line", file=sys.stderr)
network["advanced"] = advanced

with open(path, "w") as f:
    yaml.safe_dump(cfg, f, default_flow_style=False, sort_keys=False)
PYEOF
      if [ $? -ne 0 ]; then
        echo "gossip-config patch failed for $name — see $LOCAL_DEV_DIR/$name/conductor-config.yaml"
        exit 1
      fi
    done
    echo "dev-tier gossip config patched into ${#PEERS[@]} conductor-config.yaml (k2Gossip initiate=1000ms)"

    (
      export RUST_LOG="$MESH_RUST_LOG"
      holochain_bin_export
      setsid nohup sh -c "echo test | hc sandbox --piped -f $fports run -a -p=$rports" > .sandbox_run_log 2>&1 &
    )
    echo -n "waiting for ${#PEERS[@]} conductors to boot"
    for _ in $(seq 1 90); do
      [ "$(ss -tln | grep -cE "127.0.0.1:($(echo "$fports" | tr ',' '|')) ")" -ge ${#PEERS[@]} ] && break
      grep -qa "Payload: Could not" .sandbox_run_log && { echo; echo "conductor run failed — see $LOCAL_DEV_DIR/.sandbox_run_log"; exit 1; }
      printf "."; sleep 3
    done
    echo " up"
  else
    echo "conductors already up"
  fi

  # 3. Storage peers: one per conductor, agent key read from its conductor.
  local i=0
  for name in "${PEERS[@]}"; do
    if ! curl -s -m 2 "http://localhost:$(http_port $i)/health" >/dev/null; then
      local agent
      agent=$(hc sandbox call --running "$(admin_port $i)" list-apps 2>/dev/null \
        | grep -o '"agent_pub_key":"[^"]*"' | head -1 | cut -d'"' -f4)
      mkdir -p "$MESH_DIR/$name"
      HOLOCHAIN_ADMIN_URL="ws://localhost:$(admin_port $i)" \
      HOLOCHAIN_APP_URL="ws://localhost:$(app_port $i)" \
      STORAGE_DIR="$MESH_DIR/$name" \
      ENABLE_CONTENT_DB=true ENABLE_IMPORT_API=true \
      ENABLE_P2P=true P2P_PORT="$(p2p_port $i)" \
      AGENT_PUBKEY="$agent" RELAY_MODE=server \
      GENESIS_SELF_HEAL_IDENTITY=1 SELF_HUMAN_ID="$(human_id "$name")" \
      HOUSEHOLD_ID=household-dowell \
      DEVICE_ARCHETYPE=device-family-node-base \
      ELOHIM_STORAGE_PEER_POLICY_PATH="$MESH_DIR/peer-policy.toml" \
      PROJECTION_RECONCILE_SECS="$PROJECTION_RECONCILE_SECS" \
      ACQUISITION_RECONCILE_SECS="$ACQUISITION_RECONCILE_SECS" \
      CONTEST_BACKOFF_SECONDS="$CONTEST_BACKOFF_SECONDS" \
      HEAL_MISSING_BACKOFF_SECONDS="$HEAL_MISSING_BACKOFF_SECONDS" \
      ELOHIM_EVIDENCE_ABSENT_BACKOFF_SECS="$ELOHIM_EVIDENCE_ABSENT_BACKOFF_SECS" \
      ELOHIM_HEAD_CORPUS_DIGEST="$ELOHIM_HEAD_CORPUS_DIGEST" \
      ELOHIM_ADOPT_BEFORE_AUTHOR="$ELOHIM_ADOPT_BEFORE_AUTHOR" \
      ADOPT_CONTEST_FANOUT="$ADOPT_CONTEST_FANOUT" \
      ELOHIM_NETWORK_STAKES="$ELOHIM_NETWORK_STAKES" \
      ALLOW_SEED_NETWORK_STAKES=1 \
      ALLOW_SEED_DELEGATES_COMPUTE=1 \
      nohup "$STORAGE_BIN" --http-port "$(http_port $i)" > "$LOGDIR/$name.log" 2>&1 &
      echo "storage $name: http=$(http_port $i) p2p=$(p2p_port $i) agent=${agent:0:16}..."
    else
      echo "storage $name already up on :$(http_port $i)"
    fi
    i=$((i+1))
  done

  for _ in $(seq 1 30); do
    local ok=0 j=0
    for _n in "${PEERS[@]}"; do
      curl -s -m 2 "http://localhost:$(http_port $j)/health" >/dev/null && ok=$((ok+1)); j=$((j+1))
    done
    [ "$ok" -ge ${#PEERS[@]} ] && break; sleep 2
  done

  echo
  status_all
  echo
  echo "next: ./hc-mesh.sh probe   # run the CI Dataplane Validation probes here"
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
    conductors-restart) restart_conductors ;;
    storage-restart) shift; restart_storage "$@" ;;
    zome-probe) probe_zome_paths ;;
    fixture-refresh) refresh_fixture_pids ;;
    prologue) shift; exec bash "$SCRIPT_DIR/hc-mesh-prologue.sh" "$@" ;;
    *) echo "usage: hc-mesh.sh [start|stop|status|probe|prologue|conductors-restart|storage-restart [peer...]|zome-probe|fixture-refresh]"; exit 2 ;;
  esac
fi
