"""cite_graph — content-addressed citation primitives for the doc/spec/memory graph.

The cite ENVELOPE is a single human-readable LINE (back-compatible with the minimal _lib.frontmatter
string-list parser, so every existing cites-reader keeps working structurally — no nested-YAML rewrite):

    cites:
      - <ref-slug> | <one-sentence desc> | <fingerprint> [| status: <health hint>]

A legacy path-string cite (no "|") parses as ref=<path>, legacy_path=True. IDENTITY is the slug (survives
moves + edits); FINGERPRINT is sha256 of the content-BODY (frontmatter EXCLUDED, so a metadata edit does
not trip STALE); the optional `status:` is tool-managed (the epr-head edge hint, set by the propagation
pass). Pure-stdlib; imports _lib.frontmatter.
"""
from __future__ import annotations

import hashlib
import os
import re
from pathlib import Path

from _lib import frontmatter as _fm

SEP = " | "
_SLUG_BAD = re.compile(r"[^a-z0-9]+")
_DASHES = re.compile(r"-{2,}")
_DATE_PREFIX = re.compile(r"^\d{4}-\d{2}-\d{2}-")


def slugify(text: str) -> str:
    s = _SLUG_BAD.sub("-", (text or "").lower()).strip("-")
    return _DASHES.sub("-", s)


_GENERIC = {"design", "plan", "readme", "index", "claude", "spec"}
# bare content filenames repeated under many dirs (governance/epic.md, value_scanner/epic.md, …) —
# disambiguate + describe via the parent dir.
_CONTENT_AMBIGUOUS = {"epic", "overview", "summary", "notes", "vision", "manifesto", "story"}


def derive_slug(path, title: str | None = None) -> str:
    """A readable, stable, unique slug. PREFER the filename stem minus its date prefix (unique-by-design;
    keeps the -design/-plan suffix that disambiguates a spec from its plan). A bare content-ambiguous stem
    (epic, overview, …) takes a <parent>- prefix (governance/epic.md -> governance-epic). A truly-generic
    stem (design, plan, …) falls back to the title's first segment."""
    p = Path(path)
    stem = slugify(_DATE_PREFIX.sub("", p.stem))
    if stem in _CONTENT_AMBIGUOUS:
        parent = slugify(p.parent.name)
        if parent:
            return f"{parent}-{stem}"
    if stem and stem not in _GENERIC:
        return stem
    if title:
        t = slugify(re.split(r"\s+[—–-]\s+", title)[0])
        if t:
            return t
    return stem or slugify(p.stem)


def allocate_slug(candidate: str, taken) -> str:
    """Collision-guard: candidate, else candidate-2, candidate-3, …"""
    taken = set(taken)
    if candidate not in taken:
        return candidate
    i = 2
    while f"{candidate}-{i}" in taken:
        i += 1
    return f"{candidate}-{i}"


def body_of(path) -> str:
    return _fm.parse_file(path).body


def fingerprint_text(body: str) -> str:
    return "sha256:" + hashlib.sha256(body.strip().encode("utf-8", "replace")).hexdigest()[:16]


def fingerprint(path) -> str:
    """Content-BODY fingerprint (frontmatter excluded — a metadata edit must NOT change it)."""
    return fingerprint_text(body_of(path))


def build_slug_index(roots) -> dict:
    """{ slug -> repo-relative-or-abs path } from every doc declaring `id:`. Scans live (and held/) trees;
    a doc without `id:` is simply absent until the migration assigns one."""
    index: dict = {}
    for root in roots:
        rp = Path(root)
        if not rp.exists():
            continue
        for md in rp.rglob("*.md"):
            sid = _fm.parse_file(md).get("id")
            if sid:
                index[sid] = str(md)
    return index


# Gospel surfaces (2026-06-05): id-declaring CLAUDE.md files repo-wide join the
# doc graph so contextual-gospel concern routing can be content-addressed
# (slug | hint | fingerprint — move-safe, drift-audited). ONLY CLAUDE.md
# filenames are indexed; entity docs (genesis/data) deliberately stay
# plain-path cites. Only docs DECLARING id: enter the index, so the wide walk
# admits nothing accidental.
GOSPEL_EXCLUDE_DIRS = {"node_modules", "dist", ".angular", "coverage", "target", "sophia"}


def gospel_claude_md_paths(repo_root):
    """Yield the repo's CLAUDE.md gospel paths, pruning vendored/build/dot dirs."""
    rp = Path(repo_root)
    for dirpath, dirnames, filenames in os.walk(rp):
        dirnames[:] = [
            d for d in dirnames if d not in GOSPEL_EXCLUDE_DIRS and not d.startswith(".")
        ]
        if "CLAUDE.md" in filenames:
            yield Path(dirpath) / "CLAUDE.md"


def extend_index_with_gospels(index: dict, repo_root) -> dict:
    """Add id-declaring gospel CLAUDE.mds to a slug index. Existing entries win
    on collision (doc-root declarations take precedence)."""
    for md in gospel_claude_md_paths(repo_root):
        try:
            sid = _fm.parse_file(md).get("id")
        except OSError:
            continue
        if sid and sid not in index:
            index[sid] = str(md)
    return index


def is_gospel_claude_md(path, repo_root) -> bool:
    """True iff path is a gospel CLAUDE.md inside the walk scope above."""
    p = Path(path).resolve()
    if p.name != "CLAUDE.md":
        return False
    try:
        rel = p.relative_to(Path(repo_root).resolve())
    except ValueError:
        return False
    return not any(
        part in GOSPEL_EXCLUDE_DIRS or part.startswith(".") for part in rel.parts[:-1]
    )


def _is_fingerprint(p: str) -> bool:
    return p.startswith("sha256:") or p.startswith("bafy")


_PATH_EXTS = (".md", ".yaml", ".yml", ".py", ".ts", ".tsx", ".js", ".rs", ".json",
              ".toml", ".sh", ".feature", ".css", ".html", ".sql", ".groovy", ".mjs")
_INLINE_COMMENT = re.compile(r"\s+#(\s.*)?$")


def parse_cite(s: str) -> dict:
    """One cite string -> {ref, legacy_path, desc, fingerprint, status}. Legacy path-string (no '|') ->
    ref=path, legacy_path=True. Strips a trailing YAML inline comment (the minimal frontmatter parser
    keeps `- path   # note` as one string)."""
    raw = _INLINE_COMMENT.sub("", (s or "").strip()).strip()
    if "|" not in raw:
        is_path = ("/" in raw) or raw.endswith(_PATH_EXTS)
        return {"ref": raw, "legacy_path": is_path, "desc": "", "fingerprint": "", "status": ""}
    parts = [p.strip() for p in raw.split("|")]
    ref = parts[0]
    desc = fp = status = ""
    for p in parts[1:]:
        if _is_fingerprint(p):
            fp = p
        elif p.lower().startswith("status:"):
            status = p.split(":", 1)[1].strip()
        elif not desc:
            desc = p
    return {"ref": ref, "legacy_path": False, "desc": desc, "fingerprint": fp, "status": status}


def parse_cites(fm) -> list:
    """fm: a _lib.frontmatter.Frontmatter (or its .fields dict). Returns the parsed cite list."""
    fields = getattr(fm, "fields", fm) or {}
    raw = fields.get("cites") or []
    return [parse_cite(c) for c in raw if isinstance(c, str)]


def serialize_cite(d: dict) -> str:
    if d.get("legacy_path"):
        return d["ref"]
    parts = [d["ref"], d.get("desc", "")]
    if d.get("fingerprint"):
        parts.append(d["fingerprint"])
    if d.get("status"):
        parts.append("status: " + d["status"])
    return SEP.join(p for p in parts if p is not None and p != "" or p == parts[0])


def serialize_cites(entries) -> list:
    return [serialize_cite(d) for d in entries]


def envelope_verdict(cite: dict, slug_index: dict) -> str:
    """Health of one ENVELOPE cite, purely from the slug-index + fingerprint (no path/REPO_ROOT needed):
    ok | held (target sequestered in held/) | stale (fingerprint drift) | dead (slug resolves nowhere).
    A legacy path-string cite is 'ok' (not propagated — it carries no status hint)."""
    if cite.get("legacy_path"):
        return "ok"
    ref = cite["ref"]
    if ref not in slug_index:
        return "dead"
    path = str(slug_index[ref]).replace("\\", "/")
    if "/held/" in path:
        return "held"
    if cite.get("fingerprint"):
        try:
            if fingerprint(path) != cite["fingerprint"]:
                return "stale"
        except OSError:
            pass
    return "ok"


def reverse_index(docs) -> dict:
    """{ cited-ref -> [citer doc path] } over every doc's cites — the compute surface for propagation
    (a node-state change fans OUT to the edges that point at it)."""
    rev: dict = {}
    for d in docs:
        for c in parse_cites(_fm.parse_file(d)):
            rev.setdefault(c["ref"], []).append(str(d))
    return rev


# ── frontmatter line-rewrite helpers (shared by cite-gen, cites-migrate, cite-propagate) ──
_FM_LIST_ITEM = re.compile(r"^\s*-\s+")


def split_frontmatter(text: str):
    """-> (frontmatter_lines, body) or None if no `---` block."""
    lines = text.splitlines()
    if not lines or lines[0].strip() != "---":
        return None
    end = next((i for i in range(1, len(lines)) if lines[i].strip() == "---"), None)
    if end is None:
        return None
    return lines[1:end], "\n".join(lines[end + 1:])


def reassemble(fm_lines, body: str) -> str:
    return "---\n" + "\n".join(fm_lines) + "\n---\n" + body + ("\n" if not body.endswith("\n") else "")


def set_cites_block(fm_lines, items):
    """Replace the cites: list with `items` (cite strings, no leading '- '). Returns (lines, replaced?)."""
    out, i, n, ok = [], 0, len(fm_lines), False
    while i < n:
        if fm_lines[i].strip() == "cites:":
            out.append("cites:")
            i += 1
            while i < n and _FM_LIST_ITEM.match(fm_lines[i]):
                i += 1
            out += [f"  - {it}" for it in items]
            ok = True
            continue
        out.append(fm_lines[i])
        i += 1
    return out, ok
