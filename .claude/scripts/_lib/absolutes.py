"""Absolutes guard — the classifier both halves read (`epr:validator-absolutes-guard`).

WHY (operator, 2026-10-15): "'always' is a serious word — it needs to really mean something."
The protocol is graduated, governed, revocable and plural. Reach narrows, gates re-admit, stewards
act under a StewardshipGrant for people who hold their rights through others, and trust lags
delivery. Prose on a trust surface that says always / never / forever / guaranteed / no one can
usually promises more than the system keeps, and a false absolute there is a trust defect. An
absolute is legitimate when it names the invariant's ENFORCER on the same sentence or line.

The lexicon, the citation markers and the reason clause live in the frame atom
`elohim/sdk/schemas/v1/frames/frame-absolute-promise.json`, read through `_lib.frame_atoms` (the
same loader and normalization table the sovereignty and ownership guards use). This module owns
what the atom's generic substring classifier cannot express:

  * WORD-BOUNDED matching over the folded shadow (lowercase, NFKC, the ontology's confusables), so
    `never` does not fire inside `nevertheless`. A hyphen counts as part of the word, so
    compounds such as `always-on` (a device class) and `never-ending` are not members.
  * PROSE ONLY. `.md` and `.feature` prose minus code spans, fenced blocks, frontmatter, comments,
    tables and doc strings; `.html` text nodes and user-visible attribute values (aria-label,
    title, alt, placeholder, label) minus Angular interpolation and control flow; in `.ts`/`.js`
    UI sources, string and template literals that read as prose (two or more words, not a log
    line, an import, a selector or an error). Code comments are never scanned.
  * EXEMPTIONS. (a) an ENFORCER CITATION in the same sentence (prose, which may wrap across
    lines of one paragraph) or on the same or the preceding line (code and templates): `(enforced: <ref>)` in prose, `// absolute: <ref>` or
    `<!-- absolute: <ref> -->` beside a UI string. `<ref>` is a repo path (an optional `#anchor`,
    `:line` or `::name` suffix is allowed), `policy:<id>@<version>` or `seam:<id>`, and it MUST
    resolve: a citation to nothing is reported, not honoured. (b) testimony: the ontology atom's
    `testimony_exempt` (path prefix or `testimony: true` frontmatter), Markdown blockquotes and
    quoted speech. (c) a negated capability ("no one can …") is an absolute like any other and
    needs the same citation; there is no free pass for a refusal the text merely asserts.
  * NET-NEW only: lines are diffed as multisets (the frame classifier's notion), and the write
    fires when its added lines carry more uncited absolutes than its removed ones.

Advisory by construction: the PRE half is a `dispatch` policy row (a background review) and the
POST half (`.claude/hooks/absolutes-guard-signal.py`) only logs and folds. Nothing here blocks.
"""
from __future__ import annotations

import bisect
import re
import subprocess
import unicodedata
from pathlib import Path

from _lib import frame_atoms

VALIDATOR = "epr:validator-absolutes-guard"

# ── which writes are trust surfaces ──────────────────────────────────────────────────────────
# (prefix, suffixes) → the prose kind. The first match wins. Tests and stories are not surfaces.
_SURFACES: tuple[tuple[str, tuple[str, ...], str], ...] = (
    ("genesis/docs/", (".md",), "md"),
    ("genesis/a2o/features/", (".feature",), "feature"),
    ("genesis/a2o/features/", (".md",), "md"),
    ("elohim/elohim-chrome-asset/src/omni-element.js", (".js",), "code"),
    ("app/elohim-elements/", (".ts", ".js"), "code"),
    ("app/elohim-elements/", (".html",), "html"),
    ("app/", (".html",), "html"),
)
_NOT_SURFACE = re.compile(r"(\.spec\.|\.test\.|\.stories\.|/__tests__/|/node_modules/|/dist/)")


def surface_kind(rel: str) -> str | None:
    """The prose kind of a repo-relative path on a trust surface, or None."""
    rel = rel.replace("\\", "/")
    if rel.startswith("./"):
        rel = rel[2:]
    if _NOT_SURFACE.search("/" + rel):
        return None
    for prefix, suffixes, kind in _SURFACES:
        if rel.startswith(prefix) and rel.endswith(suffixes):
            return kind
    return None


# ── folding (char-indexed twin of frame_atoms._Folder) ───────────────────────────────────────
def _fold(text: str, ontology: dict) -> tuple[str, list[int]]:
    """Folded shadow of `text` and, per folded char, the index of the ORIGINAL char."""
    norm = ontology["normalization"]
    conf = {k: v for k, v in norm.get("confusables", {}).items() if len(k) == 1}
    out: list[str] = []
    origin: list[int] = []
    for i, ch in enumerate(text):
        low = ch.lower() if norm.get("lowercase") else ch
        for c in low:
            n = unicodedata.normalize("NFKC", c) if norm.get("nfkc") else c
            for d in n:
                for e in conf.get(d, d):
                    out.append(e)
                    origin.append(i)
    return "".join(out), origin


def _phrase_res(phrases: list[str]) -> list[tuple[str, re.Pattern]]:
    out = []
    for phrase in phrases:
        body = r"\s+".join(re.escape(w) for w in phrase.split())
        out.append((phrase, re.compile(rf"(?<![a-z0-9_\-]){body}(?![a-z0-9_\-])")))
    return out


# ── prose masks: per document, a same-length string with non-prose chars blanked ────────────
_VISIBLE_ATTRS = {"aria-label", "title", "alt", "placeholder", "label"}
_ATTR_RE = re.compile(r"""([^\s=<>"'/]+)\s*=\s*("([^"]*)"|'([^']*)')""")
_CTRL_FLOW_RE = re.compile(r"@(?:if|else if|else|for|switch|case|default|defer|empty|placeholder|"
                           r"loading|let)\b[^{\n]*")


def _blank(keep: list[bool], start: int, end: int) -> None:
    for i in range(max(0, start), min(len(keep), end)):
        keep[i] = False


def _apply(text: str, keep: list[bool]) -> str:
    return "".join(c if (k or c == "\n") else " " for c, k in zip(text, keep))


def _line_starts(text: str) -> list[int]:
    starts, at = [0], 0
    for line in text.split("\n")[:-1]:
        at += len(line) + 1
        starts.append(at)
    return starts


def _mask_inline_quotes(text: str, keep: list[bool], start: int, end: int, straight: bool) -> None:
    """Blank quoted speech inside one line: “…” always, "…" when `straight`."""
    seg = text[start:end]
    pats = [r"“[^”]*”"] + ([r'"[^"\n]*"'] if straight else [])
    for pat in pats:
        for m in re.finditer(pat, seg):
            _blank(keep, start + m.start(), start + m.end())


def _mask_md(text: str) -> str:
    keep = [True] * len(text)
    lines = text.split("\n")
    starts = _line_starts(text)
    if text.startswith("---\n"):
        end = text.find("\n---", 3)
        _blank(keep, 0, (end + 4) if end != -1 else len(text))
    fence = None
    for idx, line in enumerate(lines):
        s, e = starts[idx], starts[idx] + len(line)
        stripped = line.lstrip()
        marker = stripped[:3]
        if fence is not None:
            _blank(keep, s, e)
            if marker == fence:
                fence = None
            continue
        if marker in ("```", "~~~"):
            fence = marker
            _blank(keep, s, e)
            continue
        if stripped.startswith(">"):  # a blockquote is testimony
            _blank(keep, s, e)
            continue
        for m in re.finditer(r"`[^`\n]*`", line):  # inline code
            _blank(keep, s + m.start(), s + m.end())
        for m in re.finditer(r"\]\([^)\n]*\)|https?://\S+", line):  # link targets, bare URLs
            _blank(keep, s + m.start(), s + m.end())
        _mask_inline_quotes(text, keep, s, e, straight=True)
    for m in re.finditer(r"<!--.*?-->", text, re.S):
        _blank(keep, m.start(), m.end())
    return _apply(text, keep)


def _mask_feature(text: str) -> str:
    keep = [True] * len(text)
    starts = _line_starts(text)
    in_doc = False
    for idx, line in enumerate(text.split("\n")):
        s, e = starts[idx], starts[idx] + len(line)
        stripped = line.strip()
        if stripped.startswith(('"""', "```")):
            in_doc = not in_doc
            _blank(keep, s, e)
            continue
        if in_doc or stripped.startswith(("@", "|")) or stripped.startswith("# language:"):
            _blank(keep, s, e)  # tags, data tables, doc strings, the language pragma
            continue
        if stripped.startswith("#"):
            # A story's `#` comments are its narrative preamble — spec prose, so they ARE read
            # (only the marker is blanked).
            hash_at = line.index("#")
            _blank(keep, s + hash_at, s + hash_at + 1)
        # Straight-quoted step arguments are UI copy under assertion — scanned. Curly quotes are
        # a person's speech in the narrative — testimony.
        _mask_inline_quotes(text, keep, s, e, straight=False)
    return _apply(text, keep)


def _html_keep(text: str, base: int, keep: list[bool]) -> None:
    """Blank everything in text[...] (offset `base` into `keep`) that a reader never sees."""
    n = len(text)
    for pat in (r"<!--.*?-->", r"<script\b.*?</script\s*>", r"<style\b.*?</style\s*>",
                r"\{\{.*?\}\}"):
        for m in re.finditer(pat, text, re.S | re.I):
            _blank(keep, base + m.start(), base + m.end())
    for m in _CTRL_FLOW_RE.finditer(text):
        _blank(keep, base + m.start(), base + m.end())
    i = 0
    while i < n:
        if text[i] == "<" and i + 1 < n and (text[i + 1].isalpha() or text[i + 1] in "/!"):
            j, quote = i + 1, None
            while j < n:
                c = text[j]
                if quote:
                    if c == quote:
                        quote = None
                elif c in "\"'":
                    quote = c
                elif c == ">":
                    break
                j += 1
            tag = text[i:j + 1]
            _blank(keep, base + i, base + j + 1)
            for m in _ATTR_RE.finditer(tag):
                if m.group(1).lower() in _VISIBLE_ATTRS:
                    vs = m.start(3) if m.group(3) is not None else m.start(4)
                    ve = m.end(3) if m.group(3) is not None else m.end(4)
                    for k in range(base + i + vs, base + i + ve):
                        if 0 <= k < len(keep):
                            keep[k] = True
            i = j + 1
            continue
        i += 1


def _mask_html(text: str) -> str:
    keep = [True] * len(text)
    _html_keep(text, 0, keep)
    return _apply(text, keep)


# Lines whose string literals a person never reads: logs, imports, DOM plumbing, thrown errors.
_CODE_SKIP_LINE = re.compile(
    r"(console\.|logger\.|\blog\(|\bimport\b|\brequire\(|querySelector|getAttribute|setAttribute|"
    r"addEventListener|removeEventListener|customElements\.|new\s+CustomEvent|new\s+Error|"
    r"\bthrow\b|\bfetch\(|\.matches\(|\.closest\()")
_IDENT_BEFORE = re.compile(r"([A-Za-z_$][\w$.]*)\s*$")


def _code_literals(text: str) -> list[tuple[int, int, str]]:
    """(start, end, kind) content ranges of string literals (`str`) and template-literal
    segments (`tmpl:<tag>`) in JS/TS, skipping comments. Regex literals are not lexed (rare in UI
    sources; the cost is a missed or spurious literal, never a crash)."""
    out: list[tuple[int, int, str]] = []
    n = len(text)
    modes: list[dict] = [{"m": "code", "depth": 0}]
    i = 0
    while i < n:
        top = modes[-1]
        c = text[i]
        if top["m"] == "code":
            if text.startswith("//", i):
                j = text.find("\n", i)
                i = n if j == -1 else j
                continue
            if text.startswith("/*", i):
                j = text.find("*/", i + 2)
                i = n if j == -1 else j + 2
                continue
            if c in "'\"":
                j = i + 1
                while j < n and text[j] != c and text[j] != "\n":
                    j += 2 if text[j] == "\\" else 1
                out.append((i + 1, min(j, n), "str"))
                i = j + 1
                continue
            if c == "`":
                m = _IDENT_BEFORE.search(text[max(0, i - 40):i])
                tag = m.group(1).split(".")[-1] if m else ""
                modes.append({"m": "tmpl", "start": i + 1, "tag": tag})
                i += 1
                continue
            if c == "{":
                top["depth"] += 1
            elif c == "}":
                if len(modes) > 1 and top["depth"] == 0:
                    modes.pop()
                    modes[-1]["start"] = i + 1
                    i += 1
                    continue
                top["depth"] = max(0, top["depth"] - 1)
            i += 1
            continue
        # template literal content
        if c == "\\":
            i += 2
            continue
        if c == "`":
            out.append((top["start"], i, f"tmpl:{top['tag']}"))
            modes.pop()
            i += 1
            continue
        if text.startswith("${", i):
            out.append((top["start"], i, f"tmpl:{top['tag']}"))
            modes.append({"m": "code", "depth": 0})
            i += 2
            continue
        i += 1
    if modes[-1]["m"] == "tmpl":
        out.append((modes[-1]["start"], n, f"tmpl:{modes[-1]['tag']}"))
    return out


def _reads_as_prose(visible: str) -> bool:
    s = visible.strip()
    if not s or s[0] in "./#@:[-$%<{" or "=>" in s or "{" in s or "}" in s:
        return False
    if re.search(r"[a-z-]+\s*:\s*[^;]+;", s):
        return False  # a CSS declaration list assembled as a plain string
    return len(re.findall(r"[A-Za-z]{2,}", s)) >= 2 and " " in s


def _mask_code(text: str) -> str:
    keep = [False] * len(text)
    starts = _line_starts(text)
    for start, end, kind in _code_literals(text):
        if kind == "tmpl:css":
            continue  # a Lit `css` template is a stylesheet, never copy
        line_no = bisect.bisect_right(starts, start) - 1
        line_end = text.find("\n", starts[line_no])
        line = text[starts[line_no]:line_end if line_end != -1 else len(text)]
        if kind == "str" and _CODE_SKIP_LINE.search(line):
            continue
        sub = [True] * (end - start)
        if kind.startswith("tmpl"):
            _html_keep(text[start:end], 0, sub)
        visible = "".join(c if k else " " for c, k in zip(text[start:end], sub))
        if not _reads_as_prose(visible):
            continue
        for k, v in enumerate(sub):
            if v:
                keep[start + k] = True
    return _apply(text, keep)


_MASKERS = {"md": _mask_md, "feature": _mask_feature, "html": _mask_html, "code": _mask_code}


def prose_mask(text: str, kind: str) -> str:
    """`text` with every non-prose character blanked to a space (newlines kept, so line and
    column numbers still index the author's bytes as characters)."""
    return _MASKERS[kind](text)


# ── enforcer citations ───────────────────────────────────────────────────────────────────────
CITE_RE = re.compile(r"\b(?:enforced|absolute):\s*`?([^\s)\]>,;`]+)")
_REF_SUFFIX = re.compile(r"(#.*|::.*|:\d+(?:-\d+)?)$")


def citation_resolves(ref: str, root: Path) -> bool:
    """Whether a cited enforcer exists: a repo path, `policy:<id>@<v>` or `seam:<id>`."""
    ref = ref.rstrip(".")
    if ref.startswith("policy:"):
        pid, _, ver = ref[len("policy:"):].partition("@")
        try:
            lines = (root / ".claude/epr-meta/policies.yaml").read_text().splitlines()
        except OSError:
            return False
        for i, raw in enumerate(lines):
            if raw.strip() == f"- id: {pid}":
                window = " ".join(l.strip() for l in lines[i + 1:i + 4])
                if not ver or f"version: {ver}" in window:
                    return True
        return False
    if ref.startswith("seam:"):
        sid = ref[len("seam:"):]
        try:
            files = subprocess.run(["git", "-C", str(root), "ls-files", "*.seam.json",
                                    "*seam-registry.yaml"], capture_output=True, text=True,
                                   timeout=2).stdout.split()
        except (OSError, subprocess.SubprocessError):
            return False
        for f in files:
            try:
                body = (root / f).read_text(errors="replace")
            except OSError:
                continue
            if f'"{sid}"' in body or re.search(rf"\bid:\s*{re.escape(sid)}\b", body):
                return True
        return False
    path = _REF_SUFFIX.sub("", ref)
    if path.startswith("./"):
        path = path[2:]
    if not path or path.startswith("/") or ".." in Path(path).parts:
        return False
    return (root / path).exists()


def _sentence_bounds(paragraph: str) -> list[int]:
    """Offsets in `paragraph` where a new sentence starts."""
    return [0] + [m.end() for m in re.finditer(r"[.!?](?=\s)", paragraph)]


def _cited(lines: list[str], masked: list[str], idx: int, col: int, kind: str,
           root: Path) -> tuple[str | None, bool]:
    """The citation that covers the absolute at (line idx, col) → (ref, resolves)."""
    candidates: list[str] = []
    if kind in ("md", "feature"):
        # The same sentence: walk the paragraph (contiguous non-blank lines) around idx.
        lo = idx
        while lo > 0 and lines[lo - 1].strip():
            lo -= 1
        hi = idx
        while hi + 1 < len(lines) and lines[hi + 1].strip():
            hi += 1
        para = "\n".join(lines[lo:hi + 1])
        at = sum(len(lines[k]) + 1 for k in range(lo, idx)) + col
        bounds = _sentence_bounds(para)
        def sentence(off: int) -> int:
            return max(k for k, b in enumerate(bounds) if b <= off)
        here = sentence(at)
        for m in CITE_RE.finditer(para):
            if sentence(m.start()) == here:
                candidates.append(m.group(1))
    else:
        for k in (idx, idx - 1):
            if 0 <= k < len(lines):
                candidates += [m.group(1) for m in CITE_RE.finditer(lines[k])]
    if not candidates:
        return None, False
    for ref in candidates:
        if citation_resolves(ref, root):
            return ref, True
    return candidates[0], False


# ── testimony (the ontology atom's testimony_exempt) ─────────────────────────────────────────
_FRONTMATTER = re.compile(r"\A---[ \t]*\n(.*?)\n---[ \t]*(?:\n|\Z)", re.S)


def is_testimony(rel: str, text: str, ontology: dict) -> bool:
    exempt = ontology.get("testimony_exempt") or {}
    if any(rel.startswith(p) for p in exempt.get("path_prefixes") or []):
        return True
    key, match = exempt.get("frontmatter_key"), _FRONTMATTER.match(text)
    if not key or not match:
        return False
    for line in match.group(1).splitlines():
        name, sep, value = line.partition(":")
        if sep and name.strip() == key:
            return value.split("#", 1)[0].strip().strip("\"'").lower() == "true"
    return False


# ── the scan ─────────────────────────────────────────────────────────────────────────────────
def scan(text: str, kind: str, root: Path, atom: dict, ontology: dict,
         only_lines: set[int] | None = None) -> list[dict]:
    """Every absolute in `text`'s prose → [{line, col, phrase, text, citation, resolves}], with
    `citation`/`resolves` set when a citation covers it (resolving ones are exempt and dropped).
    `only_lines` (0-based) restricts the scan to those lines."""
    lines = text.split("\n")
    masked = prose_mask(text, kind).split("\n")
    res = _phrase_res(atom["recall_signal"]["phrases"])
    out: list[dict] = []
    for idx, mline in enumerate(masked):
        if only_lines is not None and idx not in only_lines:
            continue
        if not mline.strip():
            continue
        folded, origin = _fold(mline, ontology)
        taken: list[tuple[int, int]] = []
        for phrase, pat in res:
            for m in pat.finditer(folded):
                s, e = origin[m.start()], origin[m.end() - 1] + 1
                if any(s < te and ts < e for ts, te in taken):
                    continue
                taken.append((s, e))
                ref, ok = _cited(lines, masked, idx, s, kind, root)
                if ok:
                    continue
                out.append({"line": idx + 1, "col": s + 1, "phrase": phrase,
                            "text": lines[idx].strip()[:160], "citation": ref,
                            "resolves": False if ref else None})
    out.sort(key=lambda h: (h["line"], h["col"]))
    return out


def _delta(prior: str, post: str) -> tuple[set[int], set[int]]:
    """0-based indexes of added lines in `post` and removed lines in `prior` (multiset diff)."""
    remaining: dict[str, list[int]] = {}
    for i, line in enumerate(prior.split("\n")):
        remaining.setdefault(line, []).append(i)
    added: set[int] = set()
    for i, line in enumerate(post.split("\n")):
        bucket = remaining.get(line)
        if bucket:
            bucket.pop(0)
        else:
            added.add(i)
    removed = {i for bucket in remaining.values() for i in bucket}
    return added, removed


def _context_changed(prior: str, post: str, kind: str, root: Path, atom: dict,
                     ontology: dict) -> set[int]:
    """0-based lines of `post` whose absolute became uncited with no text change on that line:
    the enforcer citation sat on an adjacent line or elsewhere in the sentence and was removed,
    so a line-multiset delta alone would never rescan it. Compares the full uncited hits of both
    versions (keyed by phrase and line text) and returns the lines of `post` hits the prior lacks."""
    before: dict[tuple[str, str], int] = {}
    for h in scan(prior, kind, root, atom, ontology):
        key = (h["phrase"], h["text"])
        before[key] = before.get(key, 0) + 1
    changed: set[int] = set()
    for h in scan(post, kind, root, atom, ontology):
        key = (h["phrase"], h["text"])
        if before.get(key, 0) > 0:
            before[key] -= 1
        else:
            changed.add(h["line"] - 1)
    return changed


def classify(rel: str, post: str, prior: str, root: Path | None = None) -> dict | None:
    """The net-new uncited absolutes a write introduces on a trust surface, or None.

    Returns `{frameRef, verdict, netNew, hits, reason, kind}`. Raises `frame_atoms.FrameAtomError`
    when the atom or the ontology cannot be read (the caller decides how to degrade)."""
    kind = surface_kind(rel)
    if kind is None:
        return None
    root = root or frame_atoms.repo_root()
    # Citations resolve against `root`; the atoms come from `root` when it carries them, else the
    # loader's own fallback (a hook-test project dir has no atoms of its own).
    atoms_root = root if (root / frame_atoms.FRAMES_REL).is_dir() else frame_atoms.repo_root()
    frames = frame_atoms.load_frames(atoms_root)
    if VALIDATOR not in frames and atoms_root != frame_atoms._LIB_REPO_ROOT:
        # A checkout whose atoms predate this frame (an older worktree): read the atoms that
        # ship beside this module (the atom and this classifier land together).
        atoms_root = frame_atoms._LIB_REPO_ROOT
        frames = frame_atoms.load_frames(atoms_root)
    if VALIDATOR not in frames:
        raise frame_atoms.FrameAtomError(f"no frame atom binds validator `{VALIDATOR}`")
    atom, ref = frames[VALIDATOR]
    ontology = frame_atoms.load_ontology(atoms_root)
    if is_testimony(rel, post, ontology):
        return None
    cap = atom["recall_signal"]["scan_cap_bytes"]
    post_s, prior_s = post[:cap], prior[:cap]
    added, removed = _delta(prior_s, post_s)
    added |= _context_changed(prior_s, post_s, kind, root, atom, ontology)
    if not added:
        return None
    gained = scan(post_s, kind, root, atom, ontology, added)
    lost = scan(prior_s, kind, root, atom, ontology, removed) if removed else []
    surplus: dict[str, int] = {}
    for h in gained:
        surplus[h["phrase"]] = surplus.get(h["phrase"], 0) + 1
    for h in lost:
        surplus[h["phrase"]] = surplus.get(h["phrase"], 0) - 1
    net_new = len(gained) - len(lost)
    if net_new < atom["recall_signal"]["min_net_new"]:
        return None
    hits = [h for h in gained if surplus.get(h["phrase"], 0) > 0]
    return {"frameRef": ref, "verdict": "abstain", "netNew": net_new, "hits": hits,
            "kind": kind, "reason": frame_atoms.reason_line(atom, ref, None, "abstain")}


REPAIRS = ("cite the enforcer on the same sentence or line — `(enforced: <test path | "
           "policy:<id>@<v> | seam:<id> | schema path>)` in prose, `// absolute: <enforcer>` or "
           "`<!-- absolute: <enforcer> -->` beside a UI string (the citation must resolve); or say "
           "what actually holds — \"by default\", \"unless …\", \"until …\", \"as far as this "
           "device knows\"")


def describe(hits: list[dict]) -> str:
    parts = []
    for h in hits:
        note = f" (cited `{h['citation']}` does not resolve)" if h.get("citation") else ""
        parts.append(f"L{h['line']} \"{h['phrase']}\"{note}: {h['text']}")
    return "; ".join(parts)
