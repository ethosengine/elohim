#!/usr/bin/env python3
"""
prep-brainstorm.py "<topic>" — the DETERMINISTIC pre-step for /brainstorm.

Composes three cheap deterministic reads into ONE context preload the wrapper injects
BEFORE superpowers:brainstorming, so the session composes from existing canon and targets
only the testable surface:

  1. prior art          (spec-coherence-index.py --query)  → compose, don't re-spec
  2. testable surface   (placement-audit.py --focus)        → what's in scope / blocked-by-env
  3. budget headline    (placement-audit.py --ledger --json)→ outstanding pressure

Sub-second, no LLM. The EXPENSIVE agent-driven restructure is NOT run here — it fires only
when --check-drift trips a threshold (so brainstorms don't each pay for a full restructure).

Usage:  prep-brainstorm.py "doorway ssr routing"
        prep-brainstorm.py --check-drift "topic"   # also prints a drift advisory
"""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ARGS = [a for a in sys.argv[1:] if not a.startswith("--")]
CHECK_DRIFT = "--check-drift" in sys.argv[1:]
TOPIC = " ".join(ARGS).strip()
DRIFT_THRESHOLD = 120  # pressure items above which an agent-driven structuring pass is advised


def run(script, *args):
    try:
        return subprocess.run([sys.executable, str(HERE / script), *args],
                              capture_output=True, text=True, timeout=60).stdout.strip()
    except Exception as e:  # noqa: BLE001
        return f"(prep: {script} failed: {e})"


def main() -> int:
    if not TOPIC:
        print("usage: prep-brainstorm.py \"<topic>\"", file=sys.stderr)
        return 2

    prior = run("spec-coherence-index.py", "--query", TOPIC)
    focus = run("placement-audit.py", "--focus")
    try:
        led = json.loads(run("placement-audit.py", "--ledger", "--json") or "{}")
        # Must match placement-audit ledger_mode pressure_order: BLOCKED-BY-ENV is
        # HELD (Cascade: NONE per PLACEMENT.md), NOT actionable pressure, so it is
        # excluded here too — otherwise held work falsely trips the drift advisory.
        pressure = sum(1 for r in led.get("rows", [])
                       if r["state"] in ("NEEDS-TRIAGE", "MEM-UNLINKED", "CLAIMED-ONLY",
                                         "REGRESSED", "SUPERSEDED", "UNKNOWN-STATUS"))
    except Exception:  # noqa: BLE001
        pressure = -1

    print("=" * 78)
    print(f'BRAINSTORM PRELOAD — "{TOPIC}"   (deterministic; COMPOSE, do not duplicate)')
    print("=" * 78)
    print("\n## 1. PRIOR ART (compose from these; only ADD to canonical; standalone ONLY if empty)\n")
    print(prior)
    print("\n## 2. TESTABLE SURFACE (plan only what's in scope; BLOCKED-BY-ENV is held, not regressed)\n")
    print(focus)
    print(f"\n## 3. BUDGET — {pressure} pressure items outstanding "
          f"(run `placement-audit.py --ledger` for the per-file queue)\n")
    print("## RULE FOR THIS SESSION")
    print("  - If PRIOR ART shows a CANONICAL/done match → COMPOSE from it; extend canonical, don't fork.")
    print("  - If it shows a SUPERSEDED match → do NOT revive; read its history record for the gotcha.")
    print("  - If it shows a claimed-UNVERIFIED match → note it needs verification, don't assume it works.")
    print("  - Create a standalone spec ONLY if PRIOR ART is empty. The /brainstorm post-step will then")
    print("    decompose the result into the state system so the next plan targets real gaps.")

    if CHECK_DRIFT and pressure > DRIFT_THRESHOLD:
        print(f"\n⚠ DRIFT ADVISORY: {pressure} > {DRIFT_THRESHOLD} pressure items. Consider an agent-driven")
        print("  structuring pass (classify needs-triage / link unlinked memory) BEFORE brainstorming, so")
        print("  this session composes against a clean surface. (Cheap reads above already ran; the agent")
        print("  pass is the expensive step — fire it deliberately, not every session.)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
