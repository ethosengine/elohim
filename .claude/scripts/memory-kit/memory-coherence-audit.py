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
from _lib import cite_graph as _cg  # noqa: E402

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
    """Does this cites path/glob match anything on disk (repo-relative)? (back-compat path/glob resolver)."""
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


# Doc-graph roots the slug-index scans (live + the scope-tree held/ tree, if it exists).
_DOC_ROOTS = [
    REPO_ROOT / "genesis" / "docs",
    MEMORY_DIR,
]


def _within_doc_roots(p: Path) -> bool:
    """True iff p sits inside a cite-graph doc root — the SAME scope cite-gen's
    migrator (_doc_roots) uses. A path-cite to an id-bearing .md OUTSIDE these
    roots (e.g. a genesis/data entity doc — human / presence / device / chronicle)
    is a legitimate plain-path reference, NOT a format-candidate: `cite-gen --into`
    would refuse to convert it, so flagging it emits an un-actionable signal."""
    rp = p.resolve()
    return any(rp == r.resolve() or r.resolve() in rp.parents for r in _DOC_ROOTS)


def _doc_target_has_id(ref: str) -> bool:
    """A legacy repo-relative cite path: does it point at a DOC that declares `id:`
    AND sit within the migrator's doc-root scope? (then it should be an envelope,
    not a path-string — a CITE-FORMAT-CANDIDATE). Id-bearing .md OUTSIDE the doc
    roots (genesis/data entity docs), code, and external files stay healthy
    plain-path cites — they are not migratable, so they are not candidates."""
    p = REPO_ROOT / ref.strip("/")
    if p.suffix == ".md" and p.is_file() and _within_doc_roots(p):
        try:
            return bool(_fm.parse_file(p).get("id"))
        except OSError:
            return False
    return False


def classify_cite(cite_str: str, slug_index: dict) -> tuple[str, str]:
    """The content-addressed verdict for one cite. Returns (verdict, ref):
      ok | held (in held/, NOT dead) | dead | stale (fingerprint drift) | format-candidate (legacy doc path).
    Slugs resolve by the slug-index across live + held/; legacy path-strings keep p.exists() back-compat."""
    c = _cg.parse_cite(cite_str)
    if c["legacy_path"]:
        if not cite_resolves(cite_str):
            return ("dead", c["ref"])
        # resolves on disk — but a legacy DOC path with an id: target should be migrated to an envelope
        return ("format-candidate", c["ref"]) if _doc_target_has_id(c["ref"]) else ("ok", c["ref"])
    ref = c["ref"]
    if ref in slug_index:
        path = str(slug_index[ref]).replace("\\", "/")
        if "/held/" in path:
            return ("held", ref)
        if c["fingerprint"]:
            try:
                if _cg.fingerprint(path) != c["fingerprint"]:
                    return ("stale", ref)
            except OSError:
                pass
        return ("ok", ref)
    return ("dead", ref)


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
    held_cites: list[str] = field(default_factory=list)        # cited doc sequestered in held/ — NOT dead
    stale_cites: list[str] = field(default_factory=list)       # cited fingerprint drifted — re-verify
    format_candidates: list[str] = field(default_factory=list)  # legacy doc path-string — migrate to envelope
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
    slug_index = _cg.build_slug_index([str(r) for r in _DOC_ROOTS] + [
        str(REPO_ROOT / "genesis" / "docs" / "superpowers" / "held"),  # held/ if scope-tree created it
    ])
    _bucket = {"dead": "dead_cites", "held": "held_cites", "stale": "stale_cites",
               "format-candidate": "format_candidates"}
    for slug, p, fm in iter_memory_entries():
        cites = entry_cites(fm)
        rep = EntryReport(slug=slug, path=str(p.relative_to(REPO_ROOT)), cites=cites)
        if cites:
            for c in cites:
                verdict, _ref = classify_cite(c, slug_index)
                if verdict in _bucket:
                    getattr(rep, _bucket[verdict]).append(c)
                index.setdefault(c, []).append(slug)
        else:
            rep.cite_candidate = body_has_codepath(fm)
        reports.append(rep)
    return reports, index


def collect_changed(args) -> list[str]:
    # Sort + dedupe so the output JSON is deterministic regardless of stdin/git
    # ordering (idempotent for consumers that hash/diff the report).
    if args.changed == "-":
        raw = sys.stdin.read()
        return sorted({ln.strip() for ln in raw.replace("\0", "\n").splitlines() if ln.strip()})
    if args.since:
        try:
            out = subprocess.run(
                ["git", "diff", "--name-only", f"{args.since}...HEAD"],
                cwd=str(REPO_ROOT), capture_output=True, text=True, check=True,
            ).stdout
            return sorted({ln.strip() for ln in out.splitlines() if ln.strip()})
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


def write_index(index: dict, today: date) -> None:
    payload = {"schema_version": 1, "generated_at": today.isoformat(), "cites": index}
    _store.save_json(CITES_INDEX_PATH, payload)


def write_reports(reports: list[EntryReport], today: date) -> tuple[Path, Path]:
    out_dir = _paths.reports_dir_for_today(REPO_ROOT)
    out_dir.mkdir(parents=True, exist_ok=True)
    json_path = out_dir / "memory-coherence-audit.json"
    md_path = out_dir / "memory-coherence-audit.md"

    with_cites = [r for r in reports if r.cites]
    dead = [r for r in reports if r.dead_cites]
    candidates = [r for r in reports if r.cite_candidate]
    held = [r for r in reports if r.held_cites]
    stale = [r for r in reports if r.stale_cites]
    fmt = [r for r in reports if r.format_candidates]
    cites_legacy = sum(len(r.format_candidates) for r in reports)  # the migration backlog (stasis cites_legacy)

    summary = {
        "generated_at": today.isoformat(),
        "total_entries": len(reports),
        "entries_with_cites": len(with_cites),
        "entries_with_dead_cites": len(dead),
        "cite_candidates": len(candidates),
        "held_cites": sum(len(r.held_cites) for r in reports),
        "stale_candidates": sum(len(r.stale_cites) for r in reports),
        "cite_format_candidates": cites_legacy,  # legacy doc path-strings to migrate to envelopes
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
        f"- HELD-CITE (cited doc sequestered in held/ — NOT dead): **{summary['held_cites']}**",
        f"- STALE-CANDIDATE (cited fingerprint drifted — re-verify): **{summary['stale_candidates']}**",
        f"- CITE-FORMAT-CANDIDATE (legacy doc path-string → migrate to envelope): **{summary['cite_format_candidates']}**",
        f"- CITE-CANDIDATE (code paths in body, no `cites:` yet): **{summary['cite_candidates']}**",
        "",
    ]
    if dead:
        lines += ["## DEAD-CITE — cited path no longer resolves", ""]
        for r in dead:
            for c in r.dead_cites:
                lines.append(f"- `{r.slug}` → `{c}`")
        lines.append("")
    if held:
        lines += ["## HELD-CITE — cited doc is sequestered in held/ (informational, NOT dead)", ""]
        for r in held:
            for c in r.held_cites:
                lines.append(f"- `{r.slug}` → `{c}`")
        lines.append("")
    if stale:
        lines += ["## STALE-CANDIDATE — cited content fingerprint drifted; re-verify the lesson", ""]
        for r in stale:
            for c in r.stale_cites:
                lines.append(f"- `{r.slug}` → `{c}`")
        lines.append("")
    if fmt:
        lines += ["## CITE-FORMAT-CANDIDATE — legacy doc path-string; migrate via `cite-gen --into`", ""]
        for r in fmt:
            for c in r.format_candidates:
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
    _today = date.today()
    write_index(index, _today)
    json_path, md_path = write_reports(reports, _today)
    if not args.json_only:
        s_dead = sum(len(r.dead_cites) for r in reports)
        s_held = sum(len(r.held_cites) for r in reports)
        s_stale = sum(len(r.stale_cites) for r in reports)
        s_fmt = sum(len(r.format_candidates) for r in reports)
        n_cites = sum(1 for r in reports if r.cites)
        n_cand = sum(1 for r in reports if r.cite_candidate)
        print(f"memory entries: {len(reports)}")
        print(f"  with cites: {n_cites}  |  dead: {s_dead}  |  held: {s_held}  |  stale: {s_stale}"
              f"  |  format-candidate (cites_legacy): {s_fmt}  |  cite-candidates: {n_cand}")
        print(f"  cites-index: {CITES_INDEX_PATH}")
        print(f"  report: {json_path}")
        print(f"          {md_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
