#!/usr/bin/env python3
"""
Memory-coherence audit — walk the memory→code/spec/scenario edge.

Memory entries declare an optional `cites:` frontmatter list of repo-relative
paths or globs whose change should re-open the entry for re-verification. This
is the memory-side mirror of a story's `feature:`/`anchors_epics:` — the
capture-time discipline that makes memory a first-class node in the repo's
coherence graph. See genesis/docs/superpowers/specs/2026-05-28-in-flight-memory-coherence-design.md.

Two modes:

  audit (default): for every memory entry, verify each `cites:` path/glob still
    resolves on disk. Emit:
      - DEAD-CITE       a cites path no longer exists (lesson cites moved/deleted code)
      - CITE-CANDIDATE  entry has inline code-path backticks but no `cites:` field
                        (advisory; supports organic rollout of the edge)
    Writes a derived projection report (JSON + Markdown) and rebuilds the
    cites-index consumed by the memory-coherence-signal hook.

  changed-files (`--changed -` reads NUL/newline paths from stdin, or
    `--since <git-ref>` diffs against a ref): glob-match the changed-file list
    against every entry's `cites:` and emit STALE-CANDIDATE — "memory entry X
    cites Y which just changed; re-verify the lesson." This is graph-walker's
    walk shape applied to memory nodes; usable from a husky pre-push consumer.

Standalone-runnable. Read-only over memory; the only write is the derived
report + cites-index (single-writer projection, mirrors the sibling audits).
Pure stdlib.

Usage:
  python3 .claude/scripts/memory-kit/memory-coherence-audit.py
  python3 .claude/scripts/memory-kit/memory-coherence-audit.py --json-only
  git diff --name-only origin/dev | \\
    python3 .claude/scripts/memory-kit/memory-coherence-audit.py --changed -
  python3 .claude/scripts/memory-kit/memory-coherence-audit.py --since origin/dev
"""
from __future__ import annotations

import argparse
import fnmatch
import glob
import json
import subprocess
import sys
from dataclasses import dataclass, field, asdict
from datetime import date
from pathlib import Path

# Bootstrap _lib (matches sibling scripts)
_here = Path(__file__).resolve()
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir() and (_here / ".git").exists():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        break
    _here = _here.parent
from _lib import paths as _paths  # noqa: E402
from _lib import frontmatter as _fm  # noqa: E402
from _lib import store as _store  # noqa: E402

REPO_ROOT = _paths.repo_root_from_file(__file__)
MEMORY_DIR = _paths.memory_dir(REPO_ROOT)
CITES_INDEX_PATH = _paths.reports_root(REPO_ROOT) / "cites-index.json"

# Reuse the path-likeness heuristic shape from substrate-currency-audit so the
# two audits agree on what counts as a code-path token.
import re

PATH_TOKEN_RE = re.compile(r"`([A-Za-z0-9_./*-]+)`")
KNOWN_EXTS = (
    ".rs", ".ts", ".tsx", ".js", ".py", ".md", ".json", ".toml", ".yaml",
    ".yml", ".sh", ".html", ".css", ".feature", ".sql",
)
FENCED_CODE_RE = re.compile(r"```[^\n]*\n.*?\n```", re.DOTALL)


def looks_like_path(tok: str) -> bool:
    if tok.startswith(("http://", "https://", "//")):
        return False
    if tok.startswith(("-", "/api/", "/v1/")):
        return False
    if "::" in tok:
        return False
    if "/" in tok:
        return True
    if tok.endswith(KNOWN_EXTS):
        return True
    return tok.endswith("/")


def has_glob(pattern: str) -> bool:
    return any(c in pattern for c in "*?[")


def cite_resolves(pattern: str) -> bool:
    """Does this cites path/glob match anything on disk (repo-relative)?"""
    clean = pattern.strip()
    if not clean:
        return False
    if has_glob(clean):
        try:
            return bool(glob.glob(str(REPO_ROOT / clean), recursive=True))
        except OSError:
            return False
    p = REPO_ROOT / clean.strip("/")
    return p.exists()


def changed_matches_cite(changed: str, pattern: str) -> bool:
    """Does a changed repo-relative file path match a cites path/glob?"""
    changed = changed.strip().lstrip("./")
    pattern = pattern.strip().lstrip("./")
    if not changed or not pattern:
        return False
    if has_glob(pattern):
        # fnmatch's * spans '/', which is what we want for `dir/**` style globs.
        return fnmatch.fnmatch(changed, pattern)
    if changed == pattern.rstrip("/"):
        return True
    # Directory-prefix cite (e.g. `elohim/holochain/dna/mishpat/`)
    return changed.startswith(pattern.rstrip("/") + "/")


@dataclass
class EntryReport:
    slug: str
    path: str
    cites: list[str] = field(default_factory=list)
    dead_cites: list[str] = field(default_factory=list)
    cite_candidate: bool = False  # has code-path backticks but no cites:


def iter_memory_entries():
    """Yield (slug, Path, Frontmatter) for each topic file (skip the index)."""
    for p in sorted(MEMORY_DIR.glob("*.md")):
        if p.name == "MEMORY.md":
            continue
        try:
            fm = _fm.parse_file(p)
        except OSError:
            continue
        yield p.stem, p, fm


def entry_cites(fm) -> list[str]:
    v = fm.fields.get("cites")
    if isinstance(v, list):
        return [s for s in v if isinstance(s, str) and s.strip()]
    if isinstance(v, str) and v.strip():
        return [v.strip()]
    return []


def body_has_codepath(fm) -> bool:
    body = FENCED_CODE_RE.sub("", fm.body)
    for m in PATH_TOKEN_RE.finditer(body):
        tok = m.group(1)
        if looks_like_path(tok) and (REPO_ROOT / tok.strip("/")).exists():
            return True
    return False


def run_audit() -> tuple[list[EntryReport], dict]:
    reports: list[EntryReport] = []
    index: dict[str, list[str]] = {}
    for slug, p, fm in iter_memory_entries():
        cites = entry_cites(fm)
        rep = EntryReport(slug=slug, path=str(p.relative_to(REPO_ROOT)), cites=cites)
        if cites:
            for c in cites:
                if not cite_resolves(c):
                    rep.dead_cites.append(c)
                index.setdefault(c, []).append(slug)
        else:
            rep.cite_candidate = body_has_codepath(fm)
        reports.append(rep)
    return reports, index


def collect_changed(args) -> list[str]:
    if args.changed == "-":
        raw = sys.stdin.read()
        return [ln.strip() for ln in raw.replace("\0", "\n").splitlines() if ln.strip()]
    if args.since:
        try:
            out = subprocess.run(
                ["git", "diff", "--name-only", f"{args.since}...HEAD"],
                cwd=str(REPO_ROOT), capture_output=True, text=True, check=True,
            ).stdout
            return [ln.strip() for ln in out.splitlines() if ln.strip()]
        except (subprocess.CalledProcessError, OSError) as e:
            print(f"git diff failed: {e}", file=sys.stderr)
            return []
    return []


def run_changed(changed: list[str]) -> list[dict]:
    """Return STALE-CANDIDATE findings: entries citing a changed file."""
    findings: list[dict] = []
    for slug, p, fm in iter_memory_entries():
        cites = entry_cites(fm)
        if not cites:
            continue
        hit = [
            {"changed": c, "cite": pat}
            for pat in cites
            for c in changed
            if changed_matches_cite(c, pat)
        ]
        if hit:
            findings.append({
                "slug": slug,
                "path": str(p.relative_to(REPO_ROOT)),
                "matches": hit,
            })
    return findings


def write_index(index: dict) -> None:
    payload = {"schema_version": 1, "generated_at": date.today().isoformat(), "cites": index}
    _store.save_json(CITES_INDEX_PATH, payload)


def write_reports(reports: list[EntryReport]) -> tuple[Path, Path]:
    out_dir = _paths.reports_dir_for_today(REPO_ROOT)
    out_dir.mkdir(parents=True, exist_ok=True)
    json_path = out_dir / "memory-coherence-audit.json"
    md_path = out_dir / "memory-coherence-audit.md"

    with_cites = [r for r in reports if r.cites]
    dead = [r for r in reports if r.dead_cites]
    candidates = [r for r in reports if r.cite_candidate]

    summary = {
        "generated_at": date.today().isoformat(),
        "total_entries": len(reports),
        "entries_with_cites": len(with_cites),
        "entries_with_dead_cites": len(dead),
        "cite_candidates": len(candidates),
    }
    _store.save_json(json_path, {"summary": summary, "entries": [asdict(r) for r in reports]})

    lines = [
        "# Memory-coherence audit",
        "",
        f"_generated {summary['generated_at']}_",
        "",
        f"- total memory entries: **{summary['total_entries']}**",
        f"- entries declaring `cites:`: **{summary['entries_with_cites']}**",
        f"- entries with DEAD-CITE (cited path gone): **{summary['entries_with_dead_cites']}**",
        f"- CITE-CANDIDATE (code paths in body, no `cites:` yet): **{summary['cite_candidates']}**",
        "",
    ]
    if dead:
        lines += ["## DEAD-CITE — cited path no longer resolves", ""]
        for r in dead:
            for c in r.dead_cites:
                lines.append(f"- `{r.slug}` → `{c}`")
        lines.append("")
    if candidates:
        lines += [
            "## CITE-CANDIDATE — entry depends on code but declares no `cites:` (advisory)",
            "",
            "Adding `cites:` to these makes the in-flight walker re-open them when their code changes.",
            "",
        ]
        for r in candidates:
            lines.append(f"- `{r.slug}` (`{r.path}`)")
        lines.append("")
    if with_cites:
        lines += ["## Entries with a declared `cites:` edge", ""]
        for r in with_cites:
            mark = " ⚠️ has dead cite" if r.dead_cites else ""
            lines.append(f"- `{r.slug}` → {', '.join(f'`{c}`' for c in r.cites)}{mark}")
        lines.append("")
    md_path.write_text("\n".join(lines), encoding="utf-8")
    return json_path, md_path


def main() -> int:
    ap = argparse.ArgumentParser(description="Memory-coherence audit / walker")
    ap.add_argument("--changed", metavar="-", help="read changed paths from stdin ('-')")
    ap.add_argument("--since", metavar="GITREF", help="diff changed paths against a git ref")
    ap.add_argument("--json-only", action="store_true", help="suppress stdout summary")
    args = ap.parse_args()

    if args.changed or args.since:
        changed = collect_changed(args)
        findings = run_changed(changed)
        print(json.dumps({"changed_count": len(changed), "stale_candidates": findings}, indent=2))
        return 0

    reports, index = run_audit()
    write_index(index)
    json_path, md_path = write_reports(reports)
    if not args.json_only:
        s_dead = sum(len(r.dead_cites) for r in reports)
        n_cites = sum(1 for r in reports if r.cites)
        n_cand = sum(1 for r in reports if r.cite_candidate)
        print(f"memory entries: {len(reports)}")
        print(f"  with cites: {n_cites}  |  dead-cites: {s_dead}  |  cite-candidates: {n_cand}")
        print(f"  cites-index: {CITES_INDEX_PATH}")
        print(f"  report: {json_path}")
        print(f"          {md_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
