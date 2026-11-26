#!/usr/bin/env bash
# MESH_RECOVERY_DRAW coverage: a draw chosen by another seat runs exactly its
# lines in order, every drawn record names who chose and which draw, and the
# expectation never travels into the run. No mesh: the matrix runs from a
# scratch copy whose sibling hc-mesh.sh / hc-mesh-recovery.sh are stubs.
set -u

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fail=0
t() { if eval "$2"; then echo "ok   $1"; else echo "FAIL $1"; fail=1; fi; }

tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
cp "$here/../hc-mesh-recovery-matrix.sh" "$tmp/"
mkdir -p "$tmp/bin"
printf '#!/bin/sh\nexit 0\n' > "$tmp/bin/sleep"; chmod +x "$tmp/bin/sleep"
cat > "$tmp/hc-mesh.sh" <<'STUB'
# stub: sourced -> PEERS from MESH_PEERS; executed -> no-op
IFS=',' read -ra PEERS <<< "$MESH_PEERS"
MESH_DIR="${MESH_DIR:-/nonexistent}"
if [ "${BASH_SOURCE[0]}" = "$0" ]; then exit 0; fi
STUB
chmod +x "$tmp/hc-mesh.sh"
cat > "$tmp/hc-mesh-recovery.sh" <<'STUB'
#!/usr/bin/env bash
# stub: record argv as labels in a timeline line; recover OK
shape="$1" peer="$2"; shift 2
printf '%s\n' "$shape $peer $*" >> "$RECOVERY_ARGV_LOG"
python3 - "$shape" "$peer" "$@" >> "$RECOVERY_TIMELINE" <<'PY'
import json, sys
shape, peer, rest = sys.argv[1], sys.argv[2], sys.argv[3:]
labels = {}
for i in range(0, len(rest), 2):
    k, v = rest[i + 1].split("=", 1); labels[k] = v
print(json.dumps({"shape": shape, "peer": peer, "labels": labels,
                  "failing_legs": [], "time_to_recover_s": 1}))
PY
STUB
chmod +x "$tmp/hc-mesh-recovery.sh"
cat > "$tmp/lib.tsv" <<'LIB'
# scenario	peers	doorways	survivor	recovering	expect
homo-a	matthew,jessica	0	libp2p	libp2p	recover
homo-b	matthew,jessica	0	iroh	iroh	recover
LIB

run_matrix() { # extra env as args; prints combined output, sets mrc
  out="$(env PATH="$tmp/bin:$PATH" MESH_RECOVERY_LIBRARY="$tmp/lib.tsv" \
    MESH_RECOVERY_LIVE_SHAPE="matthew,jessica/0" MESH_RECOVERY_CORPUS_REF="5/5" \
    MESH_RECOVERY_SURVIVOR_STUB=ok RECOVERY_TIMELINE="$tmp/timeline.jsonl" \
    RECOVERY_ARGV_LOG="$tmp/argv.log" "$@" bash "$tmp/hc-mesh-recovery-matrix.sh" 2>&1)"
  mrc=$?
}
reset() { rm -f "$tmp/timeline.jsonl" "$tmp/argv.log"; }

printf '# a draw\n# chosen_by: agent:drawer\nhomo-b\tcold\t0\nhomo-a\twarm\t1\nhomo-b\twarm\t1\n' > "$tmp/draw.txt"
sha="$(sha256sum "$tmp/draw.txt" | cut -d' ' -f1)"

# 1. The draw runs exactly its lines, in order, with its slots.
reset; run_matrix MESH_RECOVERY_DRAW="$tmp/draw.txt" MESH_RECOVERY_RUN_BY=agent:runner
t "draw run exits 0 (rc=$mrc)" '[ "$mrc" -eq 0 ]'
seq_got="$(python3 -c '
import json,sys
for l in open(sys.argv[1]):
    r=json.loads(l); print(r["labels"]["scenario"], r["shape"], r["peer"], r["labels"]["run"])
' "$tmp/timeline.jsonl" | paste -sd';')"
t "draw lines run in order with their slots: '$seq_got'" \
  '[ "$seq_got" = "homo-b cold matthew 1;homo-a warm jessica 2;homo-b warm jessica 3" ]'
prov="$(python3 -c '
import json,sys
print(sorted({(json.loads(l)["labels"]["chosen_by"], json.loads(l)["labels"]["run_by"], json.loads(l)["labels"]["draw_sha256"]) for l in open(sys.argv[1])}))
' "$tmp/timeline.jsonl")"
t "every drawn record carries chosen_by, run_by, draw_sha256: $prov" \
  "[ \"\$prov\" = \"[('agent:drawer', 'agent:runner', '$sha')]\" ]"
t "expect never travels into a drawn run" '! grep -q "expect=" "$tmp/argv.log"'
pre_rows="${out%%scenario	shape	run*}"
t "no expected outcome printed before the runs finish" \
  '! grep -qE "recover|no-shared-transport|expect" <<< "$(grep -v "^matrix: draw\|survivor health\|corpus reference\|live shape" <<< "$pre_rows" | grep -v "^===")"'

# 2. In a fanout scenario (more than two peers) the last peer recovers, drawn or not:
#    the line's slot chooses only between the two peers of a two-peer scenario.
cat > "$tmp/lib-fan.tsv" <<'LIB'
# scenario	peers	doorways	survivor	recovering	expect
fan-3	matthew,jessica,james	0	dual	dual	recover
LIB
printf '# chosen_by: agent:drawer\nfan-3\tcold\t0\n' > "$tmp/fan.txt"
reset; run_matrix MESH_RECOVERY_LIBRARY="$tmp/lib-fan.tsv" MESH_RECOVERY_LIVE_SHAPE="matthew,jessica,james/0" \
  MESH_RECOVERY_DRAW="$tmp/fan.txt" MESH_RECOVERY_RUN_BY=agent:runner
fan_peer="$(python3 -c 'import json,sys; print(",".join(json.loads(l)["peer"] for l in open(sys.argv[1])))' "$tmp/timeline.jsonl" 2>/dev/null)"
t "a drawn fanout run recovers the last peer, whatever its slot says (rc=$mrc, peer=$fan_peer)" \
  '[ "$mrc" -eq 0 ] && [ "$fan_peer" = "james" ]'

# 3. Refusals: exit 2, a reason, and no run.
refuse() { # <label> <expected-reason-substring> <env...>
  local label="$1" want="$2"; shift 2
  reset; run_matrix "$@"
  t "refuses $label (rc=$mrc)" '[ "$mrc" -eq 2 ] && [[ "$out" == *"$want"* ]] && [ ! -e "$tmp/argv.log" ]'
}
refuse "draw together with MESH_RECOVERY_SCENARIOS" "exclusive" MESH_RECOVERY_DRAW="$tmp/draw.txt" MESH_RECOVERY_SCENARIOS=homo-a
refuse "an unreadable draw" "not readable" MESH_RECOVERY_DRAW="$tmp/missing.txt"
printf '# chosen_by: agent:drawer\n# nothing else\n' > "$tmp/empty.txt"
refuse "a draw with no run lines" "no run lines" MESH_RECOVERY_DRAW="$tmp/empty.txt"
printf '# chosen_by: agent:drawer\nnope\twarm\t0\n' > "$tmp/unknown.txt"
refuse "a scenario not in the library" "not in the library" MESH_RECOVERY_DRAW="$tmp/unknown.txt"
printf '# chosen_by: agent:drawer\nhomo-a\ttepid\t0\n' > "$tmp/shape.txt"
refuse "an invalid shape" "invalid shape" MESH_RECOVERY_DRAW="$tmp/shape.txt"
printf '# chosen_by: agent:drawer\nhomo-a\twarm\t2\n' > "$tmp/slot.txt"
refuse "an invalid slot" "invalid recovering slot" MESH_RECOVERY_DRAW="$tmp/slot.txt"
printf '# a draw\nhomo-a\twarm\t0\n' > "$tmp/nochooser.txt"
refuse "a draw with no chooser" "chosen_by" MESH_RECOVERY_DRAW="$tmp/nochooser.txt"
refuse "a draw chosen by the runner" "not a draw" MESH_RECOVERY_DRAW="$tmp/draw.txt" MESH_RECOVERY_RUN_BY=agent:drawer
refuse "a drawn run whose runner is not named" "runner is not named" MESH_RECOVERY_DRAW="$tmp/draw.txt"

# 4. Without a draw the cycle runs as before, minus the expect label.
reset; run_matrix MESH_RECOVERY_RUNS=2 MESH_RECOVERY_SHAPES=warm
cyc="$(python3 -c '
import json,sys
for l in open(sys.argv[1]):
    r=json.loads(l); print(r["labels"]["scenario"], r["peer"], r["labels"]["run"], sorted(r["labels"]))
' "$tmp/timeline.jsonl" | paste -sd';')"
t "cycle mode unchanged, labels scenario/shape/run only: '$cyc'" \
  "[ \"\$cyc\" = \"homo-a jessica 1 ['run', 'scenario', 'shape'];homo-a matthew 2 ['run', 'scenario', 'shape'];homo-b jessica 1 ['run', 'scenario', 'shape'];homo-b matthew 2 ['run', 'scenario', 'shape']\" ] && [ \"\$mrc\" -eq 0 ]"

exit "$fail"
