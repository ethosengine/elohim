#!/usr/bin/env python3
"""memory-index-projector — MEMORY.md as a GENERATED projection of topic-file frontmatter.

The native memory index (.claude/memory/MEMORY.md) is loaded into every session's context
under a hard harness byte-cap; hand-maintained index entries historically bloated past that
cap (the 2026-07-02 context-leak: 26KB > ~24.4KB limit → truncated load). Root cause was
hand-authoring a derived view. This projector applies storage-as-projection to agent
context: each topic file carries `title:` + `description:` frontmatter (the one-line recall
hook), and MEMORY.md is deterministically rendered from them — it cannot drift over budget
by hand again. (Same pattern as `.ci-ignore` projected from the root `.epr-meta` by
ci-ignore-projector.py; designed to lift into the brit/eprfs projection layer unchanged.)

Per-entry discipline (enforced here, taught by .claude/memory/.epr-meta at birth):
  title: <= 80 chars · description: ONE line <= 200 chars (target <= 160) · files without a
  description project a visible placeholder and count as violations.

Budget escalation (dogfoods flag -> agent -> canon -> stasis): over-soft-budget and per-file
violations land in .claude/memory-kit/memory-index-drift.json (`items`), which
cleanup-pressure.py counts toward the memory-stasis-loop gate. The drift items are a
SNAPSHOT of current violations (self-healing: fixing a file removes its item), unlike the
pure since-last-reset accumulators. Real budget relief is population work — umbrella
entries, graduation to stories/managed homes — per memory-kit CLAUDE.md; per-entry
tightening alone bottoms out at ~100-150 chars.

Modes:
  (default)  status line: entries, projected bytes, budget state, violations, freshness
  --check    exit 2 if MEMORY.md differs from the projection or exceeds the hard budget
  --apply    write the projection to MEMORY.md (only if changed)
  --hook     PostToolUse transport (registered in settings.json, matcher Edit|Write):
             stdin hook JSON; on a memory-dir topic-file write -> re-project + advise;
             on a hand-write of MEMORY.md itself -> advise that it is generated
"""
from __future__ import annotations

import json
import os
import sys
from pathlib import Path

# Bootstrap _lib by walking up
_here = Path(__file__).resolve()
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        break
    _here = _here.parent
from _lib import frontmatter as _fm  # noqa: E402
from _lib import store as _store  # noqa: E402
from _lib.paths import repo_root_from_file  # noqa: E402

SOFT_BUDGET_BYTES = 20_000  # early-warning: start consolidating (umbrella/graduate)
HARD_BUDGET_BYTES = 24_000  # stay under the harness load cap with headroom
DESC_MAX = 200
TITLE_MAX = 80

HEADER = (
    "<!-- GENERATED — do not hand-edit. MEMORY.md is projected from .claude/memory/*.md\n"
    "     frontmatter (title: + description:) by memory-index-projector.py --apply. -->\n"
)


def _repo() -> Path:
    """Repo root; MEMORY_INDEX_ROOT env overrides (test seam — keeps tests off the live index)."""
    env = os.environ.get("MEMORY_INDEX_ROOT")
    return Path(env).resolve() if env else repo_root_from_file(__file__)


def memory_dir(repo: Path) -> Path:
    return repo / ".claude" / "memory"


def drift_path(repo: Path) -> Path:
    return repo / ".claude" / "memory-kit" / "memory-index-drift.json"


def _clean_line(value: str) -> str:
    """Frontmatter values arrive quote-stripped; unescape inner quotes, collapse whitespace."""
    return " ".join(value.replace('\\"', '"').split())


def collect_entries(mem_dir: Path) -> tuple[list[dict], dict[str, str]]:
    """Scan topic files -> (entries sorted by filename, violations {key: detail}).

    A topic file opts out of the index with `index: false` frontmatter (e.g. a
    memorialized entry whose story pointer carries the recall duty).
    """
    entries: list[dict] = []
    violations: dict[str, str] = {}
    for p in sorted(mem_dir.glob("*.md"), key=lambda q: q.name.lower()):
        if p.name == "MEMORY.md":
            continue
        fm = _fm.parse_file(p)
        if fm.get("index").lower() == "false":
            continue
        title = _clean_line(fm.get("title") or fm.get("name") or p.stem)
        if len(title) > TITLE_MAX:
            violations[f"title-overlong:{p.name}"] = f"{len(title)} chars (max {TITLE_MAX})"
            title = title[: TITLE_MAX - 1].rstrip() + "…"
        desc = _clean_line(fm.get("description"))
        if not desc:
            violations[f"desc-missing:{p.name}"] = "no description: frontmatter"
            desc = "⚠ no description: frontmatter — add a one-line recall hook (<=160 chars)"
        elif len(desc) > DESC_MAX:
            violations[f"desc-overlong:{p.name}"] = f"{len(desc)} chars (max {DESC_MAX})"
            desc = desc[: DESC_MAX - 1].rstrip() + "…"
        entries.append({"file": p.name, "title": title, "desc": desc})
    return entries, violations


def render(entries: list[dict]) -> str:
    lines = [HEADER]
    for e in entries:
        lines.append(f"- [{e['title']}]({e['file']}) — {e['desc']}\n")
    return "".join(lines)


def project(repo: Path) -> dict:
    """Pure projection: returns text + stats without touching MEMORY.md."""
    mem = memory_dir(repo)
    entries, violations = collect_entries(mem)
    text = render(entries)
    size = len(text.encode("utf-8"))
    disk = mem / "MEMORY.md"
    on_disk = disk.read_text(encoding="utf-8", errors="replace") if disk.is_file() else ""
    return {
        "text": text,
        "bytes": size,
        "entries": len(entries),
        "violations": violations,
        "fresh": on_disk == text,
        "over_soft": size > SOFT_BUDGET_BYTES,
        "over_hard": size > HARD_BUDGET_BYTES,
    }


def status_line(p: dict) -> str:
    budget = "over-HARD ⛔" if p["over_hard"] else ("over-soft ⚠" if p["over_soft"] else "under ✅")
    fresh = "fresh ✅" if p["fresh"] else "STALE ⚠ (run --apply)"
    return (
        f"memory-index: {p['entries']} entries · {p['bytes']}B "
        f"(soft {SOFT_BUDGET_BYTES} / hard {HARD_BUDGET_BYTES}: {budget}) · "
        f"{len(p['violations'])} violation(s) · {fresh}"
    )


def sync_drift(repo: Path, p: dict) -> None:
    """Snapshot current violations + budget state into the cleanup-pressure accumulator."""
    items: dict[str, str] = dict(p["violations"])
    if p["over_soft"]:
        items["index-over-soft-budget"] = f"{p['bytes']}B > {SOFT_BUDGET_BYTES}B — consolidate (umbrella/graduate)"
    _store.save_json(drift_path(repo), {"schema_version": 1, "items": items})


def apply(repo: Path, quiet: bool = False) -> dict:
    p = project(repo)
    if not p["fresh"]:
        (memory_dir(repo) / "MEMORY.md").write_text(p["text"], encoding="utf-8")
        p["fresh"] = True
    sync_drift(repo, p)
    if not quiet:
        print(status_line(p))
    return p


def hook() -> int:
    """PostToolUse Edit|Write transport. Fail-open: any error exits 0 silently."""
    try:
        payload = json.load(sys.stdin)
    except (json.JSONDecodeError, ValueError):
        return 0
    edited = (payload.get("tool_input", {}) or {}).get("file_path") or ""
    if not edited:
        return 0
    repo = _repo()
    mem = memory_dir(repo)
    try:
        ep = Path(edited).resolve()
        if ep.parent != mem.resolve() or ep.suffix != ".md":
            return 0
    except OSError:
        return 0

    msgs: list[str] = []
    if ep.name == "MEMORY.md":
        p = project(repo)
        if not p["fresh"]:
            msgs.append(
                "[memory-index] MEMORY.md is GENERATED — this hand-edit will be overwritten on the "
                "next projection. Put the change in the topic file's title:/description: frontmatter, "
                "then run: python3 .claude/scripts/memory-kit/memory-index-projector.py --apply"
            )
    else:
        p = apply(repo, quiet=True)
        mine = [k for k in p["violations"] if k.endswith(f":{ep.name}")]
        for k in mine:
            msgs.append(f"[memory-index] {k} — {p['violations'][k]}; the recall hook projects truncated.")
        # The over-soft advisory is a standing-pressure signal, not a per-write toll:
        # emit once per process tree (violations on YOUR file always emit).
        soft_flag = Path(f"/tmp/claude-memindex-softwarn-{os.getppid()}")
        if p["over_soft"] and not soft_flag.exists():
            try:
                soft_flag.touch()
            except OSError:
                pass
            msgs.append(
                f"[memory-index] index at {p['bytes']}B (soft budget {SOFT_BUDGET_BYTES}B"
                f"{', HARD CAP EXCEEDED' if p['over_hard'] else ''}) — consolidation due: fold related "
                "entries under an umbrella or graduate durable knowledge to its managed home "
                "(memory-kit CLAUDE.md pair-off discipline); pressure feeds the memory-stasis-loop gate."
            )
    if msgs:
        print(json.dumps({"hookSpecificOutput": {"hookEventName": "PostToolUse", "additionalContext": "\n".join(msgs)}}))
    return 0


def main() -> int:
    if "--hook" in sys.argv:
        try:
            return hook()
        except Exception:  # noqa: BLE001 — hooks are fail-open by contract
            return 0
    repo = _repo()
    if "--apply" in sys.argv:
        apply(repo, quiet="--quiet" in sys.argv)
        return 0
    p = project(repo)
    print(status_line(p))
    if p["violations"]:
        for k, v in sorted(p["violations"].items()):
            print(f"  ⚠ {k} — {v}")
    if "--check" in sys.argv:
        return 2 if (not p["fresh"] or p["over_hard"]) else 0
    return 0


if __name__ == "__main__":
    sys.exit(main())
