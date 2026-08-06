#!/usr/bin/env python3
"""habits-status.py — emit the delivery-habits headline from genesis/manifests/habits.yaml.

The habits register is the inversion-of-control seam for sessions: each habit is a
runnable contract (an interface); sessions implement "make the top red green". This
script turns it into the one line every session opens with, so session start is
SELECTION (top red) instead of SYNTHESIS (read 240 specs).

WHY "HABITS" (2026-08-06, operator directive). The manifesto already quotes James Clear:
"You don't rise to the level of your goals, you fall to the level of your systems." This
file is that level — not what we intend (intentions live in specs and plans) but what the
system is observed to do, with evidence. A habit is proven by repetition, never by
declaration, which is the discipline this register exists to enforce. Earlier names failed
on shape and namespace: "spine" implied a sequence this register does not have (the ordered
structure is the resiliency-saga), and "charter" collided with the shipped qahal collective
`charter` wire field.

Usage:
  habits-status.py --headline   one line for the SessionStart context block
  habits-status.py --full       table: every habit, status, checks, first moves

Deterministic, read-only, no network. Statuses are DECLARED in habits.yaml (covenant rule
4: flips require evidence); this script renders, never infers.
"""
import sys
from pathlib import Path

try:
    import yaml
except ImportError:
    sys.exit(0)  # never break session start over a dependency

HABITS = Path(__file__).resolve().parents[2] / "genesis" / "manifests" / "habits.yaml"


def load():
    if not HABITS.exists():
        return None
    try:
        return yaml.safe_load(HABITS.read_text(encoding="utf-8"))
    except yaml.YAMLError as e:
        print(f"HABITS  ⚠ unparseable ({e}) — fix {HABITS}", file=sys.stdout)
        return None


def habits_of(doc: dict) -> list:
    """The declared habits."""
    return doc.get("habits") or []


def headline(habits: dict) -> str:
    arts = habits_of(habits)
    greens = [n for n in arts if n.get("status") == "green"]
    reds = [n for n in arts if n.get("status") == "red"]
    unwired = [n for n in arts if n.get("status") == "unwired"]
    active = [n for n in arts if n.get("active")]

    # Top red: an active red beats an inactive red beats an active unwired.
    top = next((n for n in reds if n.get("active")), None) \
        or (reds[0] if reds else None) \
        or next((n for n in unwired if n.get("active")), None)
    if top is None:
        nxt = "all green — pull the next habit from the covenant queue"
    elif top.get("status") == "red":
        check = (top.get("checks") or ["(check missing)"])[0]
        nxt = f"top red: {top['id']} → {check}"
    else:
        nxt = f"top move: {top['id']} is unwired → write its red ({(top.get('first_move') or '').strip().split(':')[0]}…)"

    counts = f"{len(greens)} green · {len(reds)} red · {len(unwired)} unwired"
    fence = ",".join(n["id"] for n in active) or "none"
    return (
        f"HABITS ({HABITS.name})  {counts} · active: {fence}\n"
        f"  {nxt}\n"
        f"  RULE: sessions serve the habits — move reds green (with evidence), file new reds runnable, "
        f"one-line delta. A plan citing no habit belongs in held/."
    )


def full(habits: dict) -> str:
    lines = [f"Delivery habits — {HABITS} (updated {habits.get('updated')})", ""]
    for n in habits_of(habits):
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
    habits = load()
    if not habits:
        return 0
    if "--full" in sys.argv:
        print(full(habits))
    else:
        print(headline(habits))
    return 0


if __name__ == "__main__":
    sys.exit(main())
