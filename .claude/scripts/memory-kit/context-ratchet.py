#!/usr/bin/env python3
"""
context-ratchet.py — the CONTEXT-COVERAGE gate (the code-coverage ratchet, applied to context).

Context coverage is to the memory/doc graph what code coverage is to source: a measured % of the
graph that is "covered" (decomposed, stated, linked with path+explainer, …). This tool is the
RATCHET — the same pattern every coverage library uses: enforce "coverage must not decrease," and
ratchet the baseline UP as the loop improves it. It is the GATE, separate from the measurement tool
(`placement-audit.py`), exactly as a coverage gate in CI is separate from the coverage instrument.

  - reads the coverage report from `placement-audit.py --stasis --json` (per-dimension + score)
  - compares each dimension to `.claude/memory-kit/context-coverage-baseline.json`
  - FAILS (exit 1) if any dimension dropped below baseline − EPSILON — a regression (a change lowered
    context coverage: a new doc with no status, a removed link, a code file with no trace)
  - on no-regression, ratchets the baseline UP (max per dimension) and exits 0
  - first run (no baseline) establishes it

Run:  context-ratchet.py            # gate + ratchet-up (local)
      context-ratchet.py --check    # gate only, never writes the baseline (for CI / pre-push)
"""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
BASELINE = ROOT / ".claude/memory-kit/context-coverage-baseline.json"
AUDIT = ROOT / ".claude/scripts/memory-kit/placement-audit.py"
CHECK_ONLY = "--check" in sys.argv[1:]


def _epsilon():
    """Ratchet epsilon from the context-coverage manifest (the tuning surface); default 0.005."""
    import re
    p = ROOT / ".claude/memory-kit/context-coverage.yaml"
    if p.exists():
        m = re.search(r"ratchet_epsilon:\s*\n\s+value:\s*([0-9.]+)", p.read_text())
        if m:
            return float(m.group(1))
    return 0.005


EPSILON = _epsilon()


def current():
    r = subprocess.run([sys.executable, str(AUDIT), "--stasis", "--json"],
                       capture_output=True, text=True)
    try:
        data = json.loads(r.stdout)
    except Exception:  # noqa: BLE001
        print("context-ratchet: could not read coverage report from placement-audit.py --stasis --json",
              file=sys.stderr)
        print(r.stdout[-400:], file=sys.stderr)
        raise SystemExit(2)
    return data.get("dimensions", {}), float(data.get("score", 0.0))


def main() -> int:
    dims, score = current()
    if not BASELINE.exists():
        BASELINE.write_text(json.dumps({"score": round(score, 4), "dimensions": dims}, indent=1))
        print(f"context-coverage BASELINE established: score {score:.3f}")
        for n, v in sorted(dims.items()):
            print(f"   {v*100:5.1f}%  {n}")
        return 0

    base = json.loads(BASELINE.read_text())
    base_dims = base.get("dimensions", {})
    regressions = [(n, base_dims[n], dims.get(n, 0.0))
                   for n in base_dims if dims.get(n, 0.0) < base_dims[n] - EPSILON]
    if regressions:
        print("❌ CONTEXT-COVERAGE REGRESSION — gate FAILS (a change lowered coverage):")
        for n, b, v in sorted(regressions, key=lambda x: x[1] - x[2], reverse=True):
            print(f"   {n}:  {b*100:.1f}% → {v*100:.1f}%   (down {(b-v)*100:.1f} pts)")
        print("   Add the missing status / link / explained-trace, or mark the artifact no-cover.")
        return 1

    new_dims = {n: max(dims.get(n, 0.0), base_dims.get(n, 0.0)) for n in set(dims) | set(base_dims)}
    new_floor = sum(new_dims.values()) / len(new_dims) if new_dims else 0.0
    gained = score - base.get("score", 0.0)
    if not CHECK_ONLY:
        BASELINE.write_text(json.dumps({"score": round(new_floor, 4), "dimensions": new_dims}, indent=1))
    print(f"✅ no regression — score {base.get('score', 0.0):.3f} → {score:.3f}  ({gained:+.3f})"
          + ("" if CHECK_ONLY else f"; baseline ratcheted to {new_floor:.3f}"))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
