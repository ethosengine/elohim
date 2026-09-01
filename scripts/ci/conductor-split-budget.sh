#!/usr/bin/env bash
# conductor-split-budget.sh — budget-neutral conductor/storage split for one
# human's compute envelope (rung 2, backlog upgrade-propagation-p2p-design-arc).
#
# WHY BASH: this arithmetic lived as Groovy helpers and was killed TWICE at
# runtime by the CPS script-security sandbox (build #1403 rejected
# Double.parseDouble, #1404 rejected DGM toBigDecimal). The repo rule already
# says bash bodies live in scripts/ci/*.sh; sh(returnStdout:) + readJSON are
# sandbox-free and proven in this pipeline.
#
# CONTRACT: conductor + storage shares sum EXACTLY to the input on every
# dimension (memory 5/8 : 3/8, cpu 1/2 : 1/2, floor rounding with the
# remainder always falling to the STORAGE side). Empty inputs pass through as
# empty (pre-split behaviour for a human that declares nothing).
#
# Usage: conductor-split-budget.sh <memReq> <memLim> <cpuReq> <cpuLim>
# Output: one line of JSON with conductor*/storage* values (Mi / m units).
set -euo pipefail

MEM_NUM=5; MEM_DEN=8
CPU_NUM=1; CPU_DEN=2

to_mi() { # k8s memory quantity -> integer Mi (floor)
  local v="$1"
  case "$v" in
    *Ti) awk -v n="${v%Ti}" 'BEGIN{printf "%d", n*1024*1024}' ;;
    *Gi) awk -v n="${v%Gi}" 'BEGIN{printf "%d", n*1024}' ;;
    *Mi) awk -v n="${v%Mi}" 'BEGIN{printf "%d", n}' ;;
    *Ki) awk -v n="${v%Ki}" 'BEGIN{printf "%d", n/1024}' ;;
    *) echo "unsupported memory quantity '$v' — use Ki/Mi/Gi/Ti" >&2; exit 65 ;;
  esac
}
to_m() { # k8s cpu quantity -> integer millicores (floor)
  local v="$1"
  case "$v" in
    *m) awk -v n="${v%m}" 'BEGIN{printf "%d", n}' ;;
    *)  awk -v n="$v" 'BEGIN{printf "%d", n*1000}' ;;
  esac
}
split_mem() { # -> "conductorMi storageMi"
  local mi; mi="$(to_mi "$1")"
  local c=$(( mi * MEM_NUM / MEM_DEN ))
  echo "$c $(( mi - c ))"
}
split_cpu() { # -> "conductorM storageM"
  local m; m="$(to_m "$1")"
  local c=$(( m * CPU_NUM / CPU_DEN ))
  echo "$c $(( m - c ))"
}

out() { # emit JSON; empty input -> empty strings for that pair
  local mr="$1" ml="$2" cr="$3" cl="$4"
  local cmr="" smr="" cml="" sml="" ccr="" scr="" ccl="" scl=""
  if [ -n "$mr" ]; then read -r a b <<<"$(split_mem "$mr")"; cmr="${a}Mi"; smr="${b}Mi"; fi
  if [ -n "$ml" ]; then read -r a b <<<"$(split_mem "$ml")"; cml="${a}Mi"; sml="${b}Mi"; fi
  if [ -n "$cr" ]; then read -r a b <<<"$(split_cpu "$cr")"; ccr="${a}m"; scr="${b}m"; fi
  if [ -n "$cl" ]; then read -r a b <<<"$(split_cpu "$cl")"; ccl="${a}m"; scl="${b}m"; fi
  printf '{"conductorMemoryRequest":"%s","storageMemoryRequest":"%s","conductorMemoryLimit":"%s","storageMemoryLimit":"%s","conductorCpuRequest":"%s","storageCpuRequest":"%s","conductorCpuLimit":"%s","storageCpuLimit":"%s"}\n' \
    "$cmr" "$smr" "$cml" "$sml" "$ccr" "$scr" "$ccl" "$scl"
}
out "${1-}" "${2-}" "${3-}" "${4-}"
