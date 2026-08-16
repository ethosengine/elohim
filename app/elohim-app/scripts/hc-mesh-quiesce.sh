#!/bin/bash
#
# hc-mesh-quiesce.sh - time-wrapped local quiesce measure for hc-mesh.sh
#
# Wraps scripts/ci/fleet-quiesce-gate.sh against the local 3-peer mesh
# (see hc-mesh.sh) with the dev-tier pacing profile's knobs, so a wall-clock
# quiesce number can be taken locally BEFORE the Jenkins hc-mesh-in-CI leg
# exists (minutes-quiesce plan W4.1: genesis/docs/superpowers/plans/
# 2026-08-16-minutes-quiesce-fixture-trust-swarm-plan.md §3 W4).
#
# The mesh must already be up (`hc-mesh.sh start`) — this script only
# measures quiescence, it never starts/stops the mesh itself.
#
# USAGE:
#   ./hc-mesh-quiesce.sh
#
# URL/content overrides (defaults match hc-mesh.sh's port scheme):
#   MESH_QUIESCE_DOORWAY_A   default http://localhost:${DOORWAY_PORT:-8888}
#   MESH_QUIESCE_DOORWAY_B   default http://localhost:${DOORWAY_B_PORT:-8889}
#   MESH_QUIESCE_STORAGE_A   default http://localhost:8090 (peer index 0)
#   MESH_QUIESCE_STORAGE_B   default http://localhost:8091 (peer index 1)
#   MESH_QUIESCE_CONTENT_ID  default elohim-host-landing
#
# Gate knobs this harness sets (override the gate's own env contract —
# see scripts/ci/fleet-quiesce-gate.sh header for what each does):
#   QUIESCE_POLL_SECS               10 (fixed — fast local poll)
#   QUIESCE_SUSTAIN_SECS            computed: MESH_RECONCILE_SECS + 10%,
#                                   floored at 33s, so a fresh reconcile
#                                   sweep is guaranteed between the two
#                                   anchor PASS observations
#   QUIESCE_ACTIONABLE_TOLERANCE    2 (fixed — matches the alpha gate's
#                                   steady-state tolerance decision)
#   QUIESCE_DEADLINE_SECS           1200 (20min bound; override to extend)
#
# MESH_RECONCILE_SECS (same var hc-mesh.sh reads for its dev-tier pacing
# profile — set it identically for both, or the sustain-window math below
# no longer matches the fleet's actual reconcile cadence).
#
# Result record: one line per run appended to
# ${MESH_DIR:-/tmp/elohim-local-mesh}/quiesce-log.txt AND printed —
# date, wall-clock, PASS/FAIL, knob values. This is the shift-visible
# record the minutes-quiesce plan's baseline compares against
# (baseline: 3,439 rows x 3 peers, ~90min, .claude/shifts/
# 2026-08-16T04-15-local-mesh-saga-delivery.journal.md line 127).
#
set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
GATE_SCRIPT="$REPO_ROOT/scripts/ci/fleet-quiesce-gate.sh"

MESH_DIR="${MESH_DIR:-/tmp/elohim-local-mesh}"
mkdir -p "$MESH_DIR"
LOG_FILE="$MESH_DIR/quiesce-log.txt"

DOORWAY_PORT="${DOORWAY_PORT:-8888}"
DOORWAY_B_PORT="${DOORWAY_B_PORT:-8889}"

DOORWAY_A="${MESH_QUIESCE_DOORWAY_A:-http://localhost:$DOORWAY_PORT}"
DOORWAY_B="${MESH_QUIESCE_DOORWAY_B:-http://localhost:$DOORWAY_B_PORT}"
STORAGE_A="${MESH_QUIESCE_STORAGE_A:-http://localhost:8090}"
STORAGE_B="${MESH_QUIESCE_STORAGE_B:-http://localhost:8091}"
CONTENT_ID="${MESH_QUIESCE_CONTENT_ID:-elohim-host-landing}"

if [ ! -x "$GATE_SCRIPT" ]; then
  echo "missing or non-executable gate script: $GATE_SCRIPT"
  exit 2
fi

# Sustain window: reconcile cadence + 10%, floored at 33s (default
# MESH_RECONCILE_SECS=30 -> 30+3=33, exactly the floor). Must exceed one
# reconcile sweep by construction, or the gate's two anchor PASS
# observations could straddle the same stale sweep and false-PASS.
RECONCILE_SECS="${MESH_RECONCILE_SECS:-30}"
sustain_bump=$(( (RECONCILE_SECS + 9) / 10 ))
SUSTAIN_SECS=$((RECONCILE_SECS + sustain_bump))
[ "$SUSTAIN_SECS" -lt 33 ] && SUSTAIN_SECS=33

export QUIESCE_POLL_SECS="10"
export QUIESCE_SUSTAIN_SECS="$SUSTAIN_SECS"
export QUIESCE_ACTIONABLE_TOLERANCE="2"
export QUIESCE_DEADLINE_SECS="${QUIESCE_DEADLINE_SECS:-1200}"

echo "hc-mesh-quiesce: doorway-a=$DOORWAY_A doorway-b=$DOORWAY_B storage-a=$STORAGE_A storage-b=$STORAGE_B content=$CONTENT_ID"
echo "hc-mesh-quiesce: QUIESCE_POLL_SECS=$QUIESCE_POLL_SECS QUIESCE_SUSTAIN_SECS=$QUIESCE_SUSTAIN_SECS QUIESCE_ACTIONABLE_TOLERANCE=$QUIESCE_ACTIONABLE_TOLERANCE QUIESCE_DEADLINE_SECS=$QUIESCE_DEADLINE_SECS (reconcile=$RECONCILE_SECS)"

# One-shot shared-substrate I/O baseline (plan W4.4 #5): all mesh peers share
# ONE volume, so every run records the substrate's write throughput as its
# infra context. 64MB fdatasync'd write into the mesh workdir, then removed.
# A degraded baseline vs prior log lines means the run was infra-contended —
# discard it, don't tune against it. Skip with MESH_IO_PROBE=0.
io_baseline="skipped"
if [ "${MESH_IO_PROBE:-1}" = "1" ]; then
  probe_file="$MESH_DIR/.io-probe.tmp"
  io_baseline=$(dd if=/dev/zero of="$probe_file" bs=1M count=64 conv=fdatasync 2>&1 \
    | tail -1 | grep -o '[0-9.]* [MG]B/s' || echo "unmeasured")
  rm -f "$probe_file"
fi

start_iso="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
start_epoch=$(date +%s)

TIME_FILE="$(mktemp)"
{ time bash "$GATE_SCRIPT" "$DOORWAY_A" "$DOORWAY_B" "$CONTENT_ID" "$STORAGE_A" "$STORAGE_B"; } 2> "$TIME_FILE"
gate_exit=$?
cat "$TIME_FILE" >&2

end_epoch=$(date +%s)
wall_secs=$((end_epoch - start_epoch))
rm -f "$TIME_FILE"

if [ "$gate_exit" -eq 0 ]; then
  verdict="PASS"
else
  verdict="FAIL(exit=$gate_exit)"
fi

record="$start_iso wall=${wall_secs}s verdict=$verdict content=$CONTENT_ID reconcile=${RECONCILE_SECS}s contest_backoff=${MESH_CONTEST_BACKOFF:-120}s heal_missing_backoff=${MESH_HEAL_MISSING_BACKOFF:-60}s evidence_absent_backoff=${MESH_EVIDENCE_ABSENT_BACKOFF:-600}s head_corpus_digest=${MESH_HEAD_CORPUS_DIGEST:-1} sustain=${SUSTAIN_SECS}s poll=${QUIESCE_POLL_SECS}s tolerance=${QUIESCE_ACTIONABLE_TOLERANCE} deadline=${QUIESCE_DEADLINE_SECS}s io_baseline=${io_baseline// /}"

echo "$record" | tee -a "$LOG_FILE"

exit "$gate_exit"
