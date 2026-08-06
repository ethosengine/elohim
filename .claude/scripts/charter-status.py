#!/usr/bin/env python3
"""charter-status.py — emit the delivery-charter headline from genesis/manifests/charter.yaml.

The charter is the inversion-of-control seam for sessions: articles are runnable
contracts (interfaces); sessions implement "make the top red green". This script
turns the charter into the one line every session opens with, so session start is
SELECTION (top red) instead of SYNTHESIS (read 240 specs).

WHY "CHARTER" AND NOT "SPINE" (renamed 2026-08-06, operator directive). A spine is
vertebrae in sequence, and sequence is the one property this register does NOT have:
articles are unordered peers, each independently binding. The ordered structure in
this repo is the resiliency-SAGA, whose chapters run 01..11 and have a frontier — so
the old name described the sibling register, and a reader who trusted it inferred the
wrong shape. A charter is a set of ARTICLES: unordered, each binding on its own,
amended only by due process — which is exactly covenant rule 4 (flips require
evidence). It also names the obligation the old word hid: an article declares what
would COUNT AS PROOF, the question a test suite never asks about itself.

Usage:
  charter-status.py --headline   one line for the SessionStart context block
  charter-status.py --full       table: every article, status, checks, first moves

Deterministic, read-only, no network. Statuses are DECLARED in charter.yaml
(covenant rule 4: flips require evidence); this script renders, never infers.
"""
import sys
from pathlib import Path

try:
    import yaml
except ImportError:
    sys.exit(0)  # never break session start over a dependency

CHARTER = Path(__file__).resolve().parents[2] / "genesis" / "manifests" / "charter.yaml"


def load():
    if not CHARTER.exists():
        return None
    try:
        return yaml.safe_load(CHARTER.read_text(encoding="utf-8"))
    except yaml.YAMLError as e:
        print(f"CHARTER  ⚠ unparseable ({e}) — fix {CHARTER}", file=sys.stdout)
        return None


def articles(charter: dict) -> list:
    """The charter's articles. Also accepts the pre-2026-08-06 `nodes:` key so a stale
    checkout or an unmerged branch renders instead of silently reporting zero."""
    return charter.get("articles") or charter.get("nodes") or []


def headline(charter: dict) -> str:
    arts = articles(charter)
    greens = [n for n in arts if n.get("status") == "green"]
    reds = [n for n in arts if n.get("status") == "red"]
    unwired = [n for n in arts if n.get("status") == "unwired"]
    active = [n for n in arts if n.get("active")]

    # Top red: an active red beats an inactive red beats an active unwired.
    top = next((n for n in reds if n.get("active")), None) \
        or (reds[0] if reds else None) \
        or next((n for n in unwired if n.get("active")), None)
    if top is None:
        nxt = "all green — pull the next article from the covenant queue"
    elif top.get("status") == "red":
        check = (top.get("checks") or ["(check missing)"])[0]
        nxt = f"top red: {top['id']} → {check}"
    else:
        nxt = f"top move: {top['id']} is unwired → write its red ({(top.get('first_move') or '').strip().split(':')[0]}…)"

    counts = f"{len(greens)} green · {len(reds)} red · {len(unwired)} unwired"
    fence = ",".join(n["id"] for n in active) or "none"
    return (
        f"CHARTER ({CHARTER.name})  {counts} · active: {fence}\n"
        f"  {nxt}\n"
        f"  RULE: sessions serve the charter — move reds green (with evidence), file new reds runnable, "
        f"one-line delta. A plan citing no article belongs in held/."
    )


def full(charter: dict) -> str:
    lines = [f"Delivery charter — {CHARTER} (updated {charter.get('updated')})", ""]
    for n in articles(charter):
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
    charter = load()
    if not charter:
        return 0
    if "--full" in sys.argv:
        print(full(charter))
    else:
        print(headline(charter))
    return 0


if __name__ == "__main__":
    sys.exit(main())
