#!/bin/bash
#
# Holochain Conductor Runner
#
# Modes:
#   fresh    - Generate fresh ephemeral sandbox (old behavior)
#   snapshot - Run from pre-seeded snapshot data
#
# Usage:
#   ./run-conductor.sh          # Uses snapshot if available, else fresh
#   ./run-conductor.sh fresh    # Force fresh sandbox
#   ./run-conductor.sh snapshot # Force snapshot mode (fails if not available)
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CONDUCTOR_DATA_DIR="$SCRIPT_DIR/conductor-data"
HAPP_PATH="/projects/elohim/holochain/dna/lamad-spike/workdir/lamad-spike.happ"
APP_ID="elohim"
APP_PORT=4445

MODE="${1:-auto}"

# Determine mode
if [ "$MODE" = "auto" ]; then
  if [ -d "$CONDUCTOR_DATA_DIR" ] && [ "$(ls -A $CONDUCTOR_DATA_DIR 2>/dev/null)" ]; then
    MODE="snapshot"
    echo "   Auto-detected: Using snapshot data"
  else
    MODE="fresh"
    echo "   Auto-detected: No snapshot, generating fresh"
  fi
fi

case "$MODE" in
  fresh)
    echo "=========================================="
    echo "   Starting FRESH Holochain Sandbox"
    echo "=========================================="
    echo ""
    echo "   This creates a new ephemeral conductor."
    echo "   Data will be lost on restart."
    echo ""
    exec hc sandbox generate \
      --app-id "$APP_ID" \
      --in-process-lair \
      -r="$APP_PORT" \
      "$HAPP_PATH"
    ;;

  snapshot)
    echo "=========================================="
    echo "   Starting from SNAPSHOT"
    echo "=========================================="

    if [ ! -d "$CONDUCTOR_DATA_DIR" ] || [ -z "$(ls -A $CONDUCTOR_DATA_DIR 2>/dev/null)" ]; then
      echo ""
      echo "   ERROR: No conductor data found!"
      echo ""
      echo "   To create a snapshot:"
      echo "     cd holochain/seeder && npm run snapshot:create"
      echo ""
      echo "   Or to restore from existing snapshot:"
      echo "     cd holochain/seeder && npm run snapshot:restore"
      echo ""
      exit 1
    fi

    echo ""
    echo "   Data: $CONDUCTOR_DATA_DIR"
    echo "   Port: $APP_PORT"
    echo ""

    exec hc sandbox run \
      --root "$CONDUCTOR_DATA_DIR" \
      -p="$APP_PORT"
    ;;

  *)
    echo "Usage: $0 [fresh|snapshot|auto]"
    echo ""
    echo "Modes:"
    echo "  auto     - Use snapshot if available, else fresh (default)"
    echo "  fresh    - Generate new ephemeral sandbox"
    echo "  snapshot - Run from pre-seeded snapshot data"
    exit 1
    ;;
esac
