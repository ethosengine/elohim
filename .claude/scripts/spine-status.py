#!/usr/bin/env python3
"""spine-status.py — emit the delivery-spine headline from genesis/manifests/spine.yaml.

The spine is the inversion-of-control seam for sessions: nodes are runnable
contracts (interfaces); sessions implement "make the top red green". This
script turns the spine into the one line every session opens with, so session
start is SELECTION (top red) instead of SYNTHESIS (read 240 specs).

Usage:
  spine-status.py --headline   one line for the SessionStart context block
  spine-status.py --full       table: every node, status, checks, first moves

Deterministic, read-only, no network. Statuses are DECLARED in spine.yaml
(covenant rule 4: flips require evidence); this script renders, never infers.
"""
import sys
from pathlib import Path

try:
    import yaml
except ImportError:
    sys.exit(0)  # never break session start over a dependency

SPINE = Path(__file__).resolve().parents[2] / "genesis" / "manifests" / "spine.yaml"


def load():
    if not SPINE.exists():
        return None
    try:
        return yaml.safe_load(SPINE.read_text(encoding="utf-8"))
    except yaml.YAMLError as e:
        print(f"SPINE  ⚠ unparseable ({e}) — fix {SPINE}", file=sys.stdout)
        return None


def headline(spine: dict) -> str:
    nodes = spine.get("nodes", [])
    greens = [n for n in nodes if n.get("status") == "green"]
    reds = [n for n in nodes if n.get("status") == "red"]
    unwired = [n for n in nodes if n.get("status") == "unwired"]
    active = [n for n in nodes if n.get("active")]

    # Top red: an active red beats an inactive red beats an active unwired.
    top = next((n for n in reds if n.get("active")), None) \
        or (reds[0] if reds else None) \
        or next((n for n in unwired if n.get("active")), None)
    if top is None:
        nxt = "all green — pull the next node from the covenant queue"
    elif top.get("status") == "red":
        check = (top.get("checks") or ["(check missing)"])[0]
        nxt = f"top red: {top['id']} → {check}"
    else:
        nxt = f"top move: {top['id']} is unwired → write its red ({(top.get('first_move') or '').strip().split(':')[0]}…)"

    counts = f"{len(greens)} green · {len(reds)} red · {len(unwired)} unwired"
    fence = ",".join(n["id"] for n in active) or "none"
    return (
        f"SPINE ({SPINE.name})  {counts} · active: {fence}\n"
        f"  {nxt}\n"
        f"  RULE: sessions serve the spine — move reds green (with evidence), file new reds runnable, "
        f"one-line delta. A plan citing no node belongs in held/."
    )


def full(spine: dict) -> str:
    lines = [f"Delivery spine — {SPINE} (updated {spine.get('updated')})", ""]
    for n in spine.get("nodes", []):
        mark = {"green": "✅", "red": "🔴", "unwired": "▫️"}.get(n.get("status"), "?")
        act = "  [ACTIVE]" if n.get("active") else ""
        lines.append(f"{mark} {n['id']}{act}")
        lines.append(f"    {' '.join(str(n.get('invariant', '')).split())}")
        for c in n.get("checks", []) or []:
            lines.append(f"    check: {c}")
        if n.get("first_move"):
            lines.append(f"    first move: {' '.join(str(n['first_move']).split())}")
        if n.get("evidence"):
            lines.append(f"    evidence: {' '.join(str(n['evidence']).split())}")
        lines.append("")
    return "\n".join(lines)


def main() -> int:
    spine = load()
    if not spine:
        return 0
    if "--full" in sys.argv:
        print(full(spine))
    else:
        print(headline(spine))
    return 0


if __name__ == "__main__":
    sys.exit(main())
