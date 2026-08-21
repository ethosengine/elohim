#!/usr/bin/env bash
# hc-mesh-perf-watch.sh — continuous mesh timing watch (15 s cadence).
# Writes ${MESH_DIR:-/tmp/elohim-local-mesh}/perf/watch.jsonl (one JSON object per tick) and appends
# `SPIKE` lines to perf/watch.spikes when: a probe exceeds 1.0 s or answers >=500/0, a service exceeds
# 1.5 cores over the tick, doorway A's breaker is not closed, or a storage zome path reads dead.
# Probes are read-only. Born 2026-08-21 when the "storage stall" that opened the doorway breaker turned
# out to be the doorway's own first SSR render parking its runtime (12.00 s via doorway, 1.6 ms direct).
OUT="${MESH_DIR:-/tmp/elohim-local-mesh}/perf"; mkdir -p "$OUT"; HZ=$(getconf CLK_TCK); INTERVAL="${PERF_WATCH_INTERVAL:-15}"
declare -A prev
# cpu_of sets $CPU (cores over the last tick) — a function, not a $(…) substitution, because the
# prev[] ticks must persist across iterations (a subshell would lose them and read 0.0 forever).
cpu_of() { local pid=$1 key=$2; CPU=-1; [ -r /proc/$pid/stat ] || return; local t; t=$(awk '{print $14+$15}' /proc/$pid/stat); local p=${prev[$key]:-$t}; prev[$key]=$t; CPU=$(awk -v a=$p -v b=$t -v hz=$HZ -v i=$INTERVAL 'BEGIN{printf "%.2f",(b-a)/hz/i}'); }
while true; do
  ts=$(date -u +%H:%M:%S); line="{\"t\":\"$ts\""
  for spec in "storage-m:elohim-storage --http-port 8090" "storage-j:elohim-storage --http-port 8091" "storage-s:elohim-storage --http-port 8092" "doorway-a:doorway --dev-mode --listen 0.0.0.0:8888" "doorway-b:doorway --dev-mode --listen 0.0.0.0:8889"; do
    name=${spec%%:*}; pat=${spec#*:}; pid=$(ps -eo pid=,args= | awk -v p="$pat" 'index($0,p){print $1; exit}'); c=-1; [ -n "$pid" ] && { cpu_of $pid $name; c=$CPU; }
    line="$line,\"cpu_$name\":$c"; awk -v c=$c 'BEGIN{exit !(c>1.5)}' && echo "$ts SPIKE cpu $name=$c" >> $OUT/watch.spikes
  done
  i=0; for pid in $(ps -eo pid=,args= | awk '/[h]olochain --piped/{print $1}' | head -3); do cpu_of $pid c$i; c=$CPU; line="$line,\"cpu_conductor$i\":$c"; awk -v c=$c 'BEGIN{exit !(c>1.5)}' && echo "$ts SPIKE cpu conductor$i=$c" >> $OUT/watch.spikes; i=$((i+1)); done
  for probe in "direct_content:http://localhost:8090/db/content/elohim-host-landing" "doorway_content:http://localhost:8888/db/content/elohim-host-landing" "doorway_root:http://localhost:8888/" "doorwayB_lamad:http://localhost:8889/lamad"; do
    name=${probe%%:*}; url=${probe#*:}; r=$(curl -s -m 15 -o /dev/null -w '%{http_code} %{time_total}' "$url" 2>/dev/null); code=${r%% *}; tt=${r##* }; code=${code:-0}; tt=${tt:-0}; [[ $code =~ ^[0-9]+$ ]] || code=0; [[ $tt =~ ^[0-9.]+$ ]] || tt=0
    line="$line,\"$name\":{\"code\":$code,\"s\":$tt}"; awk -v t=$tt -v c=$code 'BEGIN{exit !(t>1.0 || c>=500 || c==0)}' && echo "$ts SPIKE $name code=$code t=${tt}s" >> $OUT/watch.spikes
  done
  brk=$(curl -s -m 5 http://localhost:8888/admin/self-healing 2>/dev/null | python3 -c "import sys,json; u=json.load(sys.stdin).get('upstreams') or []; print(','.join(x.get('circuit','?') for x in u) or '?')" 2>/dev/null || echo '?')
  zp=$(for p in 8090 8091 8092; do curl -s -m 5 http://localhost:$p/health 2>/dev/null | python3 -c "import sys,json; print((json.load(sys.stdin).get('conductor') or {}).get('zomePath','?'))" 2>/dev/null || echo '?'; done | paste -sd,)
  line="$line,\"breakerA\":\"$brk\",\"zomePath\":\"$zp\"}"; echo "$line" >> $OUT/watch.jsonl
  case ",$brk," in *,closed,*) ;; esac; [ "$brk" != "?" ] && [ -n "$(tr "," "\n" <<<"$brk" | grep -vx closed)" ] && echo "$ts SPIKE breakerA=$brk" >> $OUT/watch.spikes
  case "$zp" in *dead*) echo "$ts SPIKE zomePath=$zp" >> $OUT/watch.spikes;; esac
  sleep "$INTERVAL"
done
