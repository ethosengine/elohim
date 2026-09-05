"""config_provenance.py — cross-references env-var READS (production code) against
env-var WRITES (the deployment/config surface) to compute the class of gap that let
`ELOHIM_TRANSPORT_BACKEND=dual` ship fleet-wide while `ELOHIM_IROH_RELAY_URL` — the knob
the iroh transport reads to decide whether it gets a relay — was set by NO manifest, NO
CI script, and NO env file anywhere in the repo. The feature booted green and stayed
configured-off; three reviewer agents each hand-grepped the repo to find this,
independently. This module makes that class computable instead of grep-luck.

MODEL: a key's provenance is the union of every place code READS it (Rust `env::var`/
`var_os`/`option_env!`/`env!`; TS/JS `process.env.*` / `import.meta.env.*`; Python
`os.environ`/`os.getenv`; and — scoped to `scripts/` trees only, because shell is noisy
and a shell script is usually a SETTER — `${KEY}`/`$KEY`) and every place the deployment
surface WRITES it (k8s manifest `env:` entries, Dockerfile `ENV`, `.env`-style files,
shell `export KEY=`/`KEY=value`, Jenkinsfile `withEnv([...])`/`environment { }`). A key
read by code but written nowhere is `unset` — the signal this module exists to surface.
A key read only in a form that supplies its own fallback (`.unwrap_or(...)`, `${K:-...}`,
`os.getenv("K", ...)`, `process.env.K || ...`) and set nowhere is `defaulted`: lower
confidence than `unset` because it degrades quietly instead of failing loud — which is
exactly what made the iroh gap invisible, but a real read with no default is the louder,
more actionable finding, so the two are reported separately rather than merged.

SCOPE, deliberately narrow: this is a static regex/line-based cross-reference, not a
parser for any of the five source languages it reads. Extractors favor being permissive
on WRITE detection (a missed write misreports a real `set` key as `unset` — the worse
failure mode for a tool whose whole job is flagging `unset`) and are conservative, with
the shape documented inline, on default-detection. False negatives on read/write SHAPES
not listed here are expected; when genuinely unsure whether a shape supplies a default,
extractors classify it as a plain read (never invent a default that isn't there).

Read-only, side-effect-free, never raises: a per-file failure is recorded in
`last_errors()` and the file is skipped — one bad file must never empty the whole scan
(the EprRouter poisoned-scope lesson: a fail-closed collect over rows degrades a whole
scope to silence; this module degrades per-file instead).
"""
from __future__ import annotations

import os
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

try:
    import yaml
except ImportError:  # pragma: no cover - PyYAML is a repo dep; degrade to line-scanner-only
    yaml = None


# ───────────────────────────── data model ─────────────────────────────

@dataclass(frozen=True)
class Read:
    key: str
    path: str  # repo-relative POSIX
    line: int  # 1-indexed
    defaulted: bool = False  # True iff this occurrence's shape supplies its own fallback


@dataclass(frozen=True)
class Write:
    key: str
    path: str  # repo-relative POSIX
    line: int  # 1-indexed
    kind: str = ""  # k8s-env | dockerfile-env | env-file | shell-assign | jenkinsfile-env


@dataclass(frozen=True)
class KeyProvenance:
    key: str
    reads: list  # list[Read]
    writes: list  # list[Write]
    verdict: str  # unset | set | orphan-set | defaulted


_LAST_ERRORS: list = []


def last_errors() -> list:
    """Per-file scan/read failures from the most recent scan_reads/scan_writes/provenance
    call (path-prefixed messages). Never raised — this is the recoverable-diagnostics
    out-channel a caller checks when it wants to know WHY a file's evidence is missing."""
    return list(_LAST_ERRORS)


# ───────────────────────────── key admissibility ─────────────────────────────

# Real config knobs in this repo are SCREAMING_SNAKE_CASE (ELOHIM_IROH_RELAY_URL,
# ELOHIM_TRANSPORT_BACKEND); `key.isupper()` is true for any string with at least one
# cased character and no lowercase ones, so it cheaply screens out camelCase JS locals,
# Rust match-bindings, and shell loop variables that a broad capture regex would
# otherwise also catch, without needing a second identifier-shape regex.
_IGNORE_EXACT = {"PATH", "HOME", "USER", "PWD", "SHELL", "TERM", "LANG", "CI", "TMPDIR", "HOSTNAME"}
_IGNORE_EXTRA = {"RUSTFLAGS", "NODE_ENV"}
# Shell/runtime builtins read like config keys but are supplied by the interpreter, so
# they can never be "set by a manifest" and would permanently pollute the `unset` bucket
# that is the whole point of this module (BASH_SOURCE surfaced as a top finding on the
# first live run, 2026-08-06).
_IGNORE_SHELL_BUILTINS = {
    "BASH_SOURCE", "BASH_VERSION", "BASH_REMATCH", "BASHPID", "FUNCNAME", "IFS", "OLDPWD",
    "RANDOM", "SECONDS", "LINENO", "PPID", "UID", "EUID", "GROUPS", "HOSTTYPE", "OSTYPE",
    "MACHTYPE", "SHLVL", "REPLY", "PIPESTATUS", "COLUMNS", "LINES", "LC_ALL", "LOGNAME",
    "EDITOR", "PAGER", "TZ", "DISPLAY", "SHELLOPTS", "BASHOPTS", "PS1", "PS2", "PS4",
}
_IGNORE_PREFIXES = ("CARGO_", "npm_")


def _looks_like_config_key(key: str) -> bool:
    if len(key) <= 1 or not key.isupper():
        return False
    if key in _IGNORE_EXACT or key in _IGNORE_EXTRA or key in _IGNORE_SHELL_BUILTINS:
        return False
    if key.startswith(_IGNORE_PREFIXES):
        return False
    return True


# ───────────────────────────── discovery ─────────────────────────────

# Generic vendor/build/generated dirs, pruned by NAME wherever they occur.
_SKIP_DIR_NAMES = {
    ".git", "node_modules", "target", "target-pool", "dist", "build", "__pycache__",
    ".angular", ".pnpm-store", ".venv", "venv", ".cargo-target-pool", "coverage",
    "out-tsc", ".worktrees", "bindings", ".mempalace", ".eprfs", ".idea", ".vscode",
}
# Specific REL-PATH trees that are vendored/generated per .gitignore but whose directory
# NAME alone (e.g. "repos") is too generic to prune everywhere — pruned by exact
# repo-relative path instead. genesis/research/repos/ holds full third-party research
# checkouts (freenet-git, p2panda, ...) whose OWN env::var reads are not this repo's
# config surface; scanning them produces false "unset" findings for someone else's code.
# .claude/worktrees/ holds sibling git worktrees — duplicate checkouts of THIS repo, so
# every read and write site in them is a phantom copy of one already counted in the main
# tree (it reported ELOHIM_IROH_RELAY_URL set at six sites when one manifest sets it).
_SKIP_REL_PATHS = {"genesis/research/repos", ".claude/worktrees"}

_JS_EXTS = {".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs"}
_SHELL_EXTS = {".sh", ".bash"}
_YAML_EXTS = {".yaml", ".yml"}


def _kind_for(path: Path) -> str:
    name = path.name
    suf = path.suffix.lower()
    if suf == ".rs":
        return "rust"
    if suf in _JS_EXTS:
        return "js"
    if suf == ".py":
        return "python"
    if suf in _SHELL_EXTS:
        return "shell"
    if suf in _YAML_EXTS:
        return "yaml"
    if name == ".env" or name.startswith(".env."):
        return "envfile"
    if name == "Jenkinsfile":
        return "jenkins"
    if name == "Dockerfile" or name.startswith("Dockerfile."):
        return "dockerfile"
    return ""


def _discover_files(root: Path) -> list:
    out = []
    for dirpath, dirnames, filenames in os.walk(root):
        rel_dir = Path(dirpath).relative_to(root).as_posix()
        pruned = []
        for d in dirnames:
            rel_child = d if rel_dir == "." else f"{rel_dir}/{d}"
            if d in _SKIP_DIR_NAMES or rel_child in _SKIP_REL_PATHS:
                continue
            pruned.append(d)
        dirnames[:] = pruned
        for fn in filenames:
            p = Path(dirpath) / fn
            if _kind_for(p):
                out.append(p)
    return sorted(out)


def _in_shell_read_scope(rel_posix: str) -> bool:
    """Shell reads count only under a `scripts/` tree (covers `.claude/scripts/`,
    `genesis/a2o/scripts/`, and `**/scripts/ci/` as a subset of "any `scripts` segment").
    Outside that scope a shell script is presumed a SETTER of the vars it names, not a
    reader — counting every `${FOO}` in every `.sh` in the tree would be pure noise."""
    return "scripts" in Path(rel_posix).parts


# ───────────────────────────── shared line helper ─────────────────────────────

def _line_of(text: str, pos: int) -> int:
    return text.count("\n", 0, pos) + 1


# ───────────────────────────── Rust reads ─────────────────────────────

_RUST_VAR_RE = re.compile(r'(?:std::)?env::var(_os)?\(\s*"([A-Za-z_][A-Za-z0-9_]*)"\s*\)')
_RUST_OPTION_ENV_RE = re.compile(r'\boption_env!\(\s*"([A-Za-z_][A-Za-z0-9_]*)"\s*\)')
_RUST_ENV_MACRO_RE = re.compile(r'\benv!\(\s*"([A-Za-z_][A-Za-z0-9_]*)"\s*\)')
# Chained immediately after the read call: var("K").unwrap_or(...) / .unwrap_or_else(...)
# / .unwrap_or_default() / .ok() — the shapes the task calls out as "supplies its own
# default"; an `if let Ok(v) = env::var("K") { ... }` shape (no chain) is a PLAIN read.
_RUST_DEFAULT_CHAIN_RE = re.compile(
    r'^\??\s*\.(?:unwrap_or_default\(\)|unwrap_or_else\(|unwrap_or\(|ok\(\))'
)


def _rust_has_default(text: str, end: int) -> bool:
    return bool(_RUST_DEFAULT_CHAIN_RE.match(text[end:end + 80]))


def _rust_reads(text: str) -> list:
    out = []
    for m in _RUST_VAR_RE.finditer(text):
        key = m.group(2)
        out.append((key, _line_of(text, m.start()), _rust_has_default(text, m.end())))
    for m in _RUST_OPTION_ENV_RE.finditer(text):
        key = m.group(1)
        out.append((key, _line_of(text, m.start()), _rust_has_default(text, m.end())))
    for m in _RUST_ENV_MACRO_RE.finditer(text):
        key = m.group(1)
        out.append((key, _line_of(text, m.start()), _rust_has_default(text, m.end())))
    return out


# ───────────────────────────── TS/JS reads ─────────────────────────────

_JS_DOT_RE = re.compile(r'process\.env\.([A-Za-z_][A-Za-z0-9_]*)')
_JS_BRACKET_RE = re.compile(r'process\.env\[\s*[\'"]([A-Za-z_][A-Za-z0-9_]*)[\'"]\s*\]')
_JS_IMPORT_META_RE = re.compile(r'import\.meta\.env\.([A-Za-z_][A-Za-z0-9_]*)')
_JS_DEFAULT_TAIL_RE = re.compile(r'^\s*(\|\||\?\?)')


def _js_has_default(text: str, end: int) -> bool:
    nl = text.find("\n", end)
    tail = text[end: nl if nl != -1 else len(text)]
    return bool(_JS_DEFAULT_TAIL_RE.match(tail))


def _js_reads(text: str) -> list:
    out = []
    for rx in (_JS_DOT_RE, _JS_BRACKET_RE, _JS_IMPORT_META_RE):
        for m in rx.finditer(text):
            out.append((m.group(1), _line_of(text, m.start()), _js_has_default(text, m.end())))
    return out


# ───────────────────────────── Python reads ─────────────────────────────

_PY_BRACKET_RE = re.compile(r'os\.environ\[\s*[\'"]([A-Za-z_][A-Za-z0-9_]*)[\'"]\s*\]')
_PY_GET_RE = re.compile(r'os\.(?:environ\.get|getenv)\(\s*[\'"]([A-Za-z_][A-Za-z0-9_]*)[\'"]\s*(,)?')


def _py_reads(text: str) -> list:
    out = []
    for m in _PY_BRACKET_RE.finditer(text):
        out.append((m.group(1), _line_of(text, m.start()), False))
    for m in _PY_GET_RE.finditer(text):
        out.append((m.group(1), _line_of(text, m.start()), m.group(2) == ","))
    return out


# ───────────────────────────── shell reads + writes ─────────────────────────────

_SHELL_ASSIGN_RE = re.compile(r'^\s*(?:export\s+)?([A-Za-z_][A-Za-z0-9_]*)=', re.MULTILINE)
_SHELL_BRACED_RE = re.compile(r'\$\{([A-Za-z_][A-Za-z0-9_]*)(:[-=])?[^}]*\}')
_SHELL_BARE_RE = re.compile(r'\$(?!\{)([A-Za-z_][A-Za-z0-9_]*)')


def _shell_writes(text: str) -> list:
    return [(m.group(1), _line_of(text, m.start())) for m in _SHELL_ASSIGN_RE.finditer(text)]


def _shell_reads(text: str, assigned_keys: set) -> list:
    out = []
    for m in _SHELL_BRACED_RE.finditer(text):
        key = m.group(1)
        if key in assigned_keys:
            continue
        out.append((key, _line_of(text, m.start()), m.group(2) is not None))
    for m in _SHELL_BARE_RE.finditer(text):
        key = m.group(1)
        if key in assigned_keys:
            continue
        out.append((key, _line_of(text, m.start()), False))
    return out


# ───────────────────────────── k8s manifest (YAML) writes ─────────────────────────────

if yaml is not None:
    class _LineSafeLoader(yaml.SafeLoader):
        pass

    def _construct_mapping_with_line(loader, node, deep=False):
        mapping = yaml.SafeLoader.construct_mapping(loader, node, deep=deep)
        if isinstance(mapping, dict):
            mapping["__line__"] = node.start_mark.line + 1
        return mapping

    _LineSafeLoader.add_constructor(
        yaml.resolver.BaseResolver.DEFAULT_MAPPING_TAG, _construct_mapping_with_line
    )
else:  # pragma: no cover - PyYAML absent
    _LineSafeLoader = None

_YAML_DOC_SEP_RE = re.compile(r'^---\s*(#.*)?$')
_YAML_ENV_KEY_RE = re.compile(r'^(\s*)env:\s*(#.*)?$')
_YAML_LIST_NAME_RE = re.compile(r"^(\s*)-\s*name:\s*['\"]?([A-Za-z_][A-Za-z0-9_]*)['\"]?\s*(#.*)?$")


def _split_yaml_docs(text: str) -> list:
    """Multi-document YAML (`---`-separated) split into (chunk_text, chunk_start_line)
    pairs, so a parse failure in one document degrades only that document, not the
    whole file, and line numbers stay correct across documents."""
    lines = text.split("\n")
    chunks = []
    start = 0
    for i, line in enumerate(lines):
        if _YAML_DOC_SEP_RE.match(line):
            if i > start:
                chunks.append(("\n".join(lines[start:i]), start + 1))
            start = i + 1
    if start < len(lines):
        chunks.append(("\n".join(lines[start:]), start + 1))
    return chunks


def _walk_env_blocks(node) -> list:
    """Recursively find every `env:` LIST (never a bare `name:` key elsewhere in the
    manifest) and return (key, local_line) for each `- name: KEY` item, using the
    `__line__` the loader stamped on every mapping node."""
    out = []
    if isinstance(node, dict):
        for k, v in node.items():
            if k == "env" and isinstance(v, list):
                for item in v:
                    if isinstance(item, dict) and isinstance(item.get("name"), str):
                        out.append((item["name"], item.get("__line__", 0)))
            elif isinstance(v, (dict, list)):
                out.extend(_walk_env_blocks(v))
    elif isinstance(node, list):
        for item in node:
            out.extend(_walk_env_blocks(item))
    return out


def _line_scan_yaml_chunk(text: str, start_line: int) -> list:
    """Fallback for a YAML document that failed to parse (Go-template placeholders,
    genuinely malformed, ...): an indentation-tracked line scanner. Only `- name: KEY`
    lines strictly more-indented than the nearest preceding `env:` line count."""
    out = []
    in_env = False
    env_indent = -1
    for i, raw in enumerate(text.split("\n")):
        line_no = start_line + i
        if not raw.strip():
            continue
        m_env = _YAML_ENV_KEY_RE.match(raw)
        if m_env:
            in_env = True
            env_indent = len(m_env.group(1))
            continue
        if in_env:
            indent = len(raw) - len(raw.lstrip(" "))
            if indent <= env_indent:
                in_env = False
                m_env2 = _YAML_ENV_KEY_RE.match(raw)
                if m_env2:
                    in_env = True
                    env_indent = len(m_env2.group(1))
                continue
            m_name = _YAML_LIST_NAME_RE.match(raw)
            if m_name:
                out.append((m_name.group(2), line_no))
    return out


def _scan_yaml_writes(text: str) -> list:
    out = []
    for chunk_text, start_line in _split_yaml_docs(text):
        if not chunk_text.strip():
            continue
        parsed = None
        if _LineSafeLoader is not None:
            try:
                parsed = yaml.load(chunk_text, Loader=_LineSafeLoader)
            except Exception:  # noqa: BLE001 - malformed doc; fall back to line scan below
                parsed = None
        if isinstance(parsed, (dict, list)):
            for key, local_line in _walk_env_blocks(parsed):
                abs_line = start_line + local_line - 1 if local_line else start_line
                out.append((key, abs_line))
        else:
            out.extend(_line_scan_yaml_chunk(chunk_text, start_line))
    return out


# ───────────────────────────── Dockerfile writes ─────────────────────────────

_DOCKER_ENV_LINE_RE = re.compile(r'^\s*ENV\s+(.+)$', re.IGNORECASE)
_DOCKER_KV_RE = re.compile(r'([A-Za-z_][A-Za-z0-9_]*)=')


def _scan_dockerfile_writes(text: str) -> list:
    out = []
    for i, line in enumerate(text.split("\n"), start=1):
        m = _DOCKER_ENV_LINE_RE.match(line)
        if not m:
            continue
        rest = m.group(1)
        kv = _DOCKER_KV_RE.findall(rest)
        if kv:
            out.extend((k, i) for k in kv)
        else:
            first = rest.strip().split()
            if first:
                out.append((first[0], i))
    return out


# ───────────────────────────── .env-style file writes ─────────────────────────────

_ENVFILE_LINE_RE = re.compile(r'^\s*(?:export\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*=')


def _scan_envfile_writes(text: str) -> list:
    out = []
    for i, line in enumerate(text.split("\n"), start=1):
        if line.lstrip().startswith("#"):
            continue
        m = _ENVFILE_LINE_RE.match(line)
        if m:
            out.append((m.group(1), i))
    return out


# ───────────────────────────── Jenkinsfile (Groovy) writes ─────────────────────────────

_JENKINS_WITHENV_RE = re.compile(r'withEnv\s*\(\s*\[')
_JENKINS_ENV_BLOCK_RE = re.compile(r'\benvironment\s*\{')
_JENKINS_STRING_KV_RE = re.compile(r'["\']([A-Za-z_][A-Za-z0-9_]*)\s*=')
_JENKINS_BARE_KV_RE = re.compile(r'^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=', re.MULTILINE)


def _extract_bracket_block(text: str, open_idx: int, open_ch: str, close_ch: str):
    """Naive same-char bracket-depth scan from `open_idx` (the opening bracket itself)
    to its matching close. Good enough for Groovy pipeline source; not a real parser."""
    depth = 0
    i = open_idx
    n = len(text)
    while i < n:
        c = text[i]
        if c == open_ch:
            depth += 1
        elif c == close_ch:
            depth -= 1
            if depth == 0:
                return text[open_idx:i + 1], open_idx
        i += 1
    return text[open_idx:], open_idx


def _scan_jenkins_writes(text: str) -> list:
    out = []
    for m in _JENKINS_WITHENV_RE.finditer(text):
        open_idx = m.end() - 1  # index of the '['
        block, start = _extract_bracket_block(text, open_idx, "[", "]")
        for km in _JENKINS_STRING_KV_RE.finditer(block):
            out.append((km.group(1), _line_of(text, start + km.start())))
    for m in _JENKINS_ENV_BLOCK_RE.finditer(text):
        open_idx = m.end() - 1  # index of the '{'
        block, start = _extract_bracket_block(text, open_idx, "{", "}")
        for km in _JENKINS_BARE_KV_RE.finditer(block):
            out.append((km.group(1), _line_of(text, start + km.start())))
    return out


# ───────────────────────────── per-file dispatch ─────────────────────────────

def _scan_file_text(text: str, rel: str, kind: str):
    reads = []
    writes = []

    if kind == "rust":
        reads = [Read(k, rel, ln, d) for k, ln, d in _rust_reads(text)]
    elif kind == "js":
        reads = [Read(k, rel, ln, d) for k, ln, d in _js_reads(text)]
    elif kind == "python":
        reads = [Read(k, rel, ln, d) for k, ln, d in _py_reads(text)]
    elif kind == "shell":
        raw_writes = _shell_writes(text)
        writes = [Write(k, rel, ln, "shell-assign") for k, ln in raw_writes]
        if _in_shell_read_scope(rel):
            assigned = {k for k, _ in raw_writes}
            reads = [Read(k, rel, ln, d) for k, ln, d in _shell_reads(text, assigned)]
    elif kind == "yaml":
        writes = [Write(k, rel, ln, "k8s-env") for k, ln in _scan_yaml_writes(text)]
    elif kind == "dockerfile":
        writes = [Write(k, rel, ln, "dockerfile-env") for k, ln in _scan_dockerfile_writes(text)]
    elif kind == "envfile":
        writes = [Write(k, rel, ln, "env-file") for k, ln in _scan_envfile_writes(text)]
    elif kind == "jenkins":
        writes = [Write(k, rel, ln, "jenkinsfile-env") for k, ln in _scan_jenkins_writes(text)]

    reads = [r for r in reads if _looks_like_config_key(r.key)]
    writes = [w for w in writes if _looks_like_config_key(w.key)]
    return reads, writes


def _scan_files(root: Path, files, key_filter):
    """The single walk-and-dispatch core. Every public entry point (scan_reads,
    scan_writes, provenance) funnels through this so a full-repo provenance() pass pays
    for exactly one filesystem walk and one read of each matched file, never two."""
    root = Path(root).resolve()
    errors = []
    candidates = [Path(f) for f in files] if files is not None else _discover_files(root)

    reads_by_key = {}
    writes_by_key = {}

    for p in candidates:
        kind = _kind_for(p)
        if not kind:
            continue
        try:
            rel = p.resolve().relative_to(root).as_posix()
        except ValueError:
            rel = p.as_posix()
        try:
            raw = p.read_text(encoding="utf-8", errors="replace")
        except OSError as e:
            errors.append(f"{rel}: unreadable ({e})")
            continue
        if key_filter is not None and not any(k in raw for k in key_filter):
            continue
        try:
            file_reads, file_writes = _scan_file_text(raw, rel, kind)
        except Exception as e:  # noqa: BLE001 - one bad file must never kill the scan
            errors.append(f"{rel}: scan failed ({e})")
            continue
        for r in file_reads:
            reads_by_key.setdefault(r.key, []).append(r)
        for w in file_writes:
            writes_by_key.setdefault(w.key, []).append(w)

    return reads_by_key, writes_by_key, errors


# ───────────────────────────── public API ─────────────────────────────

def scan_reads(root: Path, files: Iterable = None) -> dict:
    """Every env-var READ found under `root` (or, if `files` is given, only those
    files — the synthetic-fixture-tree entry point tests use). key -> list[Read]."""
    global _LAST_ERRORS
    reads, _writes, errors = _scan_files(Path(root), files, None)
    _LAST_ERRORS = errors
    return reads


def scan_writes(root: Path, files: Iterable = None) -> dict:
    """Every env-var WRITE found under `root` (or only `files`, if given).
    key -> list[Write]."""
    global _LAST_ERRORS
    _reads, writes, errors = _scan_files(Path(root), files, None)
    _LAST_ERRORS = errors
    return writes


def _verdict(reads: list, writes: list) -> str:
    if writes and reads:
        return "set"
    if writes:
        return "orphan-set"
    if reads:
        return "defaulted" if all(r.defaulted for r in reads) else "unset"
    return "unset"  # unreachable: callers only build entries backed by >=1 read or write


def provenance(root: Path, keys: Iterable = None) -> dict:
    """The cross-reference: key -> KeyProvenance for every key seen in either reads or
    writes under `root`. `keys`, if given, both narrows the returned keys AND lets the
    scan skip the expensive per-shape regex pass on any file whose raw text contains
    none of the requested keys as a substring — so `provenance(root, keys=["ELOHIM_"])`-
    style narrow queries don't pay for the full-repo cross product."""
    global _LAST_ERRORS
    key_filter = set(keys) if keys is not None else None
    reads, writes, errors = _scan_files(Path(root), None, key_filter)
    _LAST_ERRORS = errors

    all_keys = set(reads) | set(writes)
    if key_filter is not None:
        all_keys &= key_filter

    out = {}
    for key in sorted(all_keys):
        r = reads.get(key, [])
        w = writes.get(key, [])
        out[key] = KeyProvenance(key=key, reads=r, writes=w, verdict=_verdict(r, w))
    return out


def unset_keys(prov: dict) -> list:
    """`unset` verdicts from a `provenance()` result, sorted by key — the signal that
    matters: read by code, set by nothing."""
    return sorted((kp for kp in prov.values() if kp.verdict == "unset"), key=lambda kp: kp.key)
