"""Commit-time formatting ratchet — the engine behind validator `epr:validator-staged-format`.

Steward ruling (2026-10-30): "we should never commit unlinted code". Policy
`commit-format-ratchet@1` (.claude/epr-meta/policies.yaml) declares the rule and the .epr-meta
cascade selects which files it governs; this module measures them. It runs only inside the staged
git adapter (_lib.epr_meta_git.format_phase): outside it the validator is not applicable, never
"checked".

The guarantee, with hooks enabled: a commit cannot add an unformatted file inside a declared
formatter contract, and cannot make a previously formatted file unformatted. A file that was
already unformatted at HEAD and is still unformatted is permitted and reported (file-level debt).

  refuse  iff  staged_is_unformatted AND (file_is_new OR head_is_formatted)

What is judged is the INDEX, never the worktree. Every byte comes from `git cat-file --batch`:
the staged blob, its HEAD (or rename-source) baseline, and the configuration closure each was
committed with (Cargo.toml chain for the edition, rustfmt.toml, package.json, Prettier configs,
ignore files, .editorconfig, the .epr-meta cascade and policy registry). Each side is
materialized into its own disposable tree, so a formatted worktree cannot rescue a bad index and
a dirty worktree cannot poison a good one. Nothing here formats, stashes or re-stages.

Contracts are derived from what each project already declares, never imposed:
  * rustfmt  — any `.rs` file whose nearest Cargo.toml is a package (edition resolved literally or
               through `edition.workspace`), formatted with the pinned toolchain
               (genesis/agentic/pool-policy.json `cargo_env_overrides["*"].RUSTUP_TOOLCHAIN`) and
               the nearest committed rustfmt.toml/.rustfmt.toml (or an explicit empty config, so
               a home-directory config cannot interfere).
  * Prettier — a file matched by the `prettier --check <globs>` in the `format:check` script of an
               ancestor package.json (`!` globs exclude, as in the CLI), with that package's own
               installed Prettier and .editorconfig. `--config <file>` names the config (else the
               nearest is discovered); `--ignore-path <file>…` replaces the default
               .gitignore/.prettierignore. Any other flag is unmodelled and refuses as unmeasured.
A file in neither is `outside` the contract: not checked, and never reported as checked-clean.

States: clean · drift · debt · ignored · outside · unmeasured. Anything that prevents a
measurement (missing tool, malformed config, parse failure, unreadable object, timeout, crash) is
`unmeasured` and refuses; it never becomes success.
"""
from __future__ import annotations

import concurrent.futures
import dataclasses
import hashlib
import json
import os
import posixpath
import re
import shlex
import shutil
import subprocess
import tempfile
import threading
import queue
from pathlib import Path

try:  # Python >= 3.11
    import tomllib
except Exception:  # pragma: no cover — declared environment ships 3.11+
    tomllib = None

STAGED_FORMAT_VALIDATOR = "epr:validator-staged-format"
FORMAT_POLICY_ID = "commit-format-ratchet"

POOL_POLICY_REL = "genesis/agentic/pool-policy.json"
POLICY_REGISTRY_REL = ".claude/epr-meta/policies.yaml"

RUST_CONFIG_NAMES = ("rustfmt.toml", ".rustfmt.toml")
PRETTIER_CONFIG_NAMES = (
    ".prettierrc", ".prettierrc.json", ".prettierrc.yaml", ".prettierrc.yml", ".prettierrc.json5",
    ".prettierrc.js", ".prettierrc.cjs", ".prettierrc.mjs", ".prettierrc.ts", ".prettierrc.cts",
    ".prettierrc.mts", ".prettierrc.toml", "prettier.config.js", "prettier.config.cjs",
    "prettier.config.mjs", "prettier.config.ts", "prettier.config.cts", "prettier.config.mts",
)
PRETTIER_IGNORE_NAMES = (".prettierignore", ".gitignore")
MANIFEST_CANDIDATES = (".epr-meta/manifest.md", ".epr-meta")
# Extensions a formatter contract can plausibly claim. Used ONLY to decide whether a degraded
# environment (no YAML/TOML parser) matters for this commit — never to select files.
FORMAT_CANDIDATE_EXTS = {".rs", ".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".mts", ".cts",
                         ".json", ".json5", ".html", ".css", ".scss", ".less", ".vue", ".md",
                         ".yaml", ".yml", ".graphql"}

RUSTFMT_TIMEOUT_S = 20
PRETTIER_TIMEOUT_S = 60
RUST_POOL = max(1, min(8, os.cpu_count() or 2))
# One Prettier worker per ~PRETTIER_PER_WORKER files, at most PRETTIER_POOL (each costs ~0.6 s to start).
PRETTIER_POOL = max(1, min(4, (os.cpu_count() or 2) // 2))
PRETTIER_PER_WORKER = 16
EMPTY_RUSTFMT_REL = ".epr-format/empty-rustfmt.toml"

WORKER = Path(__file__).resolve().parent / "epr_format_prettier.mjs"

CURE_RUSTFMT = "rustup component add rustfmt --toolchain {pin}"
CURE_NODE = ("install the repository's declared Node toolchain, then `pnpm install "
             "--frozen-lockfile` at the repository root")
CURE_PRETTIER = "`pnpm install --frozen-lockfile` at the repository root"
CURE_PYTHON = "install the declared Python (3.11+, for tomllib) and PyYAML (`pip install PyYAML`)"
CURE_HOOK = "restore the tracked hook and gate files (`git checkout -- .husky .claude/scripts`) and run `pnpm run prepare`"


class GitError(Exception):
    """A git read failed — distinct from an object that is honestly absent."""


class EnforcementError(Exception):
    """The gate could not complete its own obligation (never a formatting verdict)."""


class MissingTool(Exception):
    """A parser or formatter the contract requires is not installed."""

    def __init__(self, message: str, cure: str):
        super().__init__(message)
        self.cure = cure


# ── git reads (byte-preserving; honours GIT_INDEX_FILE through the inherited environment) ──

class Git:
    def __init__(self, cwd: Path, env: dict | None = None):
        self.cwd = Path(cwd)
        self.env = env
        self.calls = 0

    def run(self, *args: str, input: bytes | None = None, ok_codes=(0,)) -> tuple[int, bytes]:
        self.calls += 1
        try:
            proc = subprocess.run(["git", *args], cwd=self.cwd, env=self.env, input=input,
                                  capture_output=True, check=False)
        except OSError as e:
            raise GitError(f"git {' '.join(args)}: {e}") from e
        if proc.returncode not in ok_codes:
            err = proc.stderr.decode("utf-8", "replace").strip()
            raise GitError(f"git {' '.join(args)} exited {proc.returncode}: {err}")
        return proc.returncode, proc.stdout

    def out(self, *args: str, **kw) -> bytes:
        return self.run(*args, **kw)[1]

    def batch(self, specs: list[str]) -> dict[str, tuple[str, bytes] | None]:
        """`git cat-file --batch -z -Z`: spec -> (type, bytes), or None when the object is absent.
        A protocol failure raises GitError — absence and failure are never collapsed."""
        result: dict[str, tuple[str, bytes] | None] = {}
        if not specs:
            return result
        payload = b"".join(s.encode("utf-8", "surrogateescape") + b"\0" for s in specs)
        data = self.out("cat-file", "--batch", "-z", "-Z", input=payload)
        pos = 0
        for spec in specs:
            end = data.find(b"\0", pos)
            if end < 0:
                raise GitError(f"cat-file --batch: truncated output at {spec!r}")
            header = data[pos:end].decode("utf-8", "surrogateescape")
            pos = end + 1
            if header.endswith(" missing") or header.endswith(" ambiguous"):
                result[spec] = None
                continue
            parts = header.split(" ")
            if len(parts) != 3 or not parts[2].isdigit():
                raise GitError(f"cat-file --batch: unexpected header {header!r} for {spec!r}")
            size = int(parts[2])
            body = data[pos:pos + size]
            if len(body) != size:
                raise GitError(f"cat-file --batch: short read for {spec!r}")
            pos += size + 1  # trailing delimiter
            result[spec] = (parts[1], body)
        return result


def head_commit(git: Git) -> str | None:
    code, out = git.run("rev-parse", "--verify", "-q", "HEAD^{commit}", ok_codes=(0, 1))
    return out.decode().strip() if code == 0 and out.strip() else None


@dataclasses.dataclass
class Change:
    path: str              # repo-relative POSIX path in the index
    status: str            # A M C R T
    src_path: str | None   # rename/copy source (the HEAD baseline path)
    src_mode: str
    dst_mode: str
    src_blob: str | None
    dst_blob: str
    is_new: bool           # no formatted baseline can exist (added, copied, type-changed)
    contract_rescan: bool = False   # unchanged file re-selected because its contract changed


_ZERO = re.compile(r"^0+$")
REGULAR_MODES = ("100644", "100755")


def staged_changes(git: Git, head: str | None) -> list[Change]:
    """Every staged addition/modification/copy/rename/type-change whose result is a REGULAR file.
    Deletions, gitlinks and symlink payloads are not formatting subjects. Rename sources are kept
    (they are the HEAD baseline). Unresolved index entries refuse."""
    unmerged = git.out("ls-files", "-u", "-z")
    if unmerged.strip(b"\0"):
        paths = sorted({rec.split(b"\t", 1)[-1].decode("utf-8", "surrogateescape")
                        for rec in unmerged.split(b"\0") if b"\t" in rec})
        raise EnforcementError("unresolved index entries (" + ", ".join(paths[:5])
                               + ("…" if len(paths) > 5 else "") + "); resolve them and `git add` "
                               "the result before committing")
    args = ["diff", "--cached", "--raw", "-z", "--no-abbrev", "-M", "-C", "--no-ext-diff"]
    if head:
        args.append(head)
    raw = git.out(*args)
    toks = raw.split(b"\0")
    out: list[Change] = []
    i = 0
    while i < len(toks):
        meta = toks[i]
        if not meta:
            i += 1
            continue
        if not meta.startswith(b":"):
            raise GitError(f"diff --raw: unexpected record {meta[:60]!r}")
        fields = meta[1:].decode().split(" ")
        if len(fields) < 5:
            raise GitError(f"diff --raw: short record {meta!r}")
        src_mode, dst_mode, src_blob, dst_blob, status = fields[:5]
        letter = status[0]
        if letter in ("R", "C"):
            src = toks[i + 1].decode("utf-8", "surrogateescape")
            dst = toks[i + 2].decode("utf-8", "surrogateescape")
            i += 3
        else:
            src = dst = toks[i + 1].decode("utf-8", "surrogateescape")
            i += 2
        if letter in ("D", "U", "X"):
            if letter == "U":
                raise EnforcementError(f"unresolved index entry {dst}; resolve and `git add` it")
            continue
        if dst_mode not in REGULAR_MODES:
            continue  # gitlink (160000) or symlink (120000) — no formatting payload
        src_regular = src_mode in REGULAR_MODES and not _ZERO.match(src_blob)
        is_new = letter in ("A", "C") or not src_regular
        out.append(Change(path=dst, status=letter,
                          src_path=(src if letter in ("R", "M", "T") and src_regular else None),
                          src_mode=src_mode, dst_mode=dst_mode,
                          src_blob=(src_blob if src_regular else None), dst_blob=dst_blob,
                          is_new=is_new))
    return out


def index_fingerprint(git: Git) -> str:
    """HEAD + every index entry (mode, blob, stage, path). Captured before checking and compared
    before success: a concurrent `git add` refuses with a retry rather than a stale verdict."""
    h = hashlib.sha256()
    h.update((head_commit(git) or "unborn").encode())
    h.update(b"\n")
    h.update(git.out("ls-files", "-s", "-z"))
    return h.hexdigest()


# ── trees: a source of committed bytes rooted somewhere on the filesystem ──

def ancestors(rel: str) -> list[str]:
    """Directories from rel's parent up to the repository root ('' = root), nearest first."""
    out = []
    d = posixpath.dirname(rel)
    while True:
        out.append(d)
        if not d:
            break
        d = posixpath.dirname(d)
    return out


def join(d: str, name: str) -> str:
    return f"{d}/{name}" if d else name


class Tree:
    """Committed bytes for one side (index or HEAD), materialized under a disposable root."""

    def __init__(self, git: Git | None, prefix: str | None, root: Path, label: str):
        self.git, self.prefix, self.root, self.label = git, prefix, Path(root), label
        self._cache: dict[str, bytes | None] = {}
        self.parsed: dict[str, object] = {}   # rel -> parsed JSON/TOML (or the ContractError)
        self.memo: dict[tuple, object] = {}   # contract memo keyed by directory
        self.known: set[str] | None = None    # every path this side holds, once listed
        (self.root / ".git").mkdir(parents=True, exist_ok=True)  # project-root marker

    def know(self, paths: set[str]) -> None:
        """Record the complete path listing of this side: a later prefetch answers every absent
        candidate from it, so only blobs that exist cost a `cat-file` read."""
        self.known = paths

    def prefetch(self, rels) -> None:
        missing = [r for r in dict.fromkeys(rels) if r not in self._cache]
        if not missing:
            return
        if self.git is None or self.prefix is None:
            for r in missing:
                self._cache[r] = None
            return
        if self.known is not None:
            for r in missing:
                if r not in self.known:
                    self._cache[r] = None
            missing = [r for r in missing if r in self.known]
            if not missing:
                return
        got = self.git.batch([f"{self.prefix}{r}" for r in missing])
        for r in missing:
            hit = got.get(f"{self.prefix}{r}")
            self._cache[r] = hit[1] if hit and hit[0] == "blob" else None

    def read(self, rel: str) -> bytes | None:
        if rel not in self._cache:
            self.prefetch([rel])
        return self._cache[rel]

    def put(self, rel: str, data: bytes) -> None:
        self._cache[rel] = data

    def path(self, rel: str) -> Path:
        return self.root / rel

    def materialize(self, rel: str, data: bytes | None = None) -> Path:
        data = self.read(rel) if data is None else data
        dest = self.path(rel)
        if data is None:
            raise EnforcementError(f"{self.label}:{rel} is not readable for materialization")
        dest.parent.mkdir(parents=True, exist_ok=True)
        if not dest.exists():
            dest.write_bytes(data)
        return dest


class DiskTree(Tree):
    """The worktree on disk — used only by the edit-time advisory (lint-check), never at commit."""

    def __init__(self, root: Path):
        self.git, self.prefix, self.root, self.label = None, None, Path(root), "worktree"
        self._cache = {}
        self.parsed, self.memo, self.known = {}, {}, None

    def prefetch(self, rels) -> None:
        for r in rels:
            if r not in self._cache:
                p = self.root / r
                try:
                    self._cache[r] = p.read_bytes() if p.is_file() else None
                except OSError:
                    self._cache[r] = None

    def materialize(self, rel: str, data: bytes | None = None) -> Path:
        return self.path(rel)


# ── contracts ──

@dataclasses.dataclass(frozen=True)
class RustContract:
    edition: str
    config: str | None        # repo-relative rustfmt config, None = explicit empty config
    config_digest: str

    def identity(self):
        return ("rustfmt", self.edition, self.config, self.config_digest)


@dataclasses.dataclass(frozen=True)
class PrettierContract:
    package_dir: str
    globs: tuple[str, ...]
    closure_digest: str       # config, editorconfig, ignore files, package fields
    config: str | None = None                 # repo-relative `--config` file, None = discovered
    ignore_paths: tuple[str, ...] | None = None   # repo-relative `--ignore-path`s, None = defaults

    def identity(self):
        return ("prettier", self.package_dir, self.globs, self.closure_digest, self.config,
                self.ignore_paths)

    def ignore_files(self) -> tuple[str, ...]:
        """The ignore files the CLI reads: the declared --ignore-path list, else the defaults."""
        if self.ignore_paths is not None:
            return self.ignore_paths
        return tuple(join(self.package_dir, n) for n in PRETTIER_IGNORE_NAMES)


@dataclasses.dataclass(frozen=True)
class Outside:
    reason: str

    def identity(self):
        return ("outside",)


class ContractError(Exception):
    """A configuration file in the closure could not be read or parsed."""


def _digest(*parts: bytes | None) -> str:
    h = hashlib.sha256()
    for p in parts:
        h.update(b"\x00" if p is None else hashlib.sha256(p).digest())
    return h.hexdigest()[:16]


def _parsed(tree: Tree, rel: str, parse, kind: str):
    """Parse one closure file once per side; a parse failure is remembered and re-raised."""
    if rel in tree.parsed:
        hit = tree.parsed[rel]
        if isinstance(hit, ContractError):
            raise hit
        return hit
    data = tree.read(rel)
    if data is None:
        tree.parsed[rel] = None
        return None
    try:
        value = parse(data.decode("utf-8"))
    except Exception as e:  # noqa: BLE001
        err = ContractError(f"{tree.label}:{rel} is not valid {kind}: {e}")
        tree.parsed[rel] = err
        raise err from e
    tree.parsed[rel] = value
    return value


def _toml(tree: Tree, rel: str):
    if tomllib is None and tree.read(rel) is not None:
        raise MissingTool("Python tomllib is unavailable — Cargo.toml cannot be read", CURE_PYTHON)
    return _parsed(tree, rel, lambda t: tomllib.loads(t), "TOML")


def resolve_edition(tree: Tree, pkg_dir: str, manifest: dict) -> str:
    pkg = manifest.get("package") or {}
    ed = pkg.get("edition")
    if ed is None:
        return "2015"  # cargo's default
    if isinstance(ed, str):
        return ed
    if isinstance(ed, dict) and ed.get("workspace") is True:
        ws_dir = None
        if "workspace" in manifest:
            ws_dir = pkg_dir
        elif isinstance(pkg.get("workspace"), str):
            ws_dir = posixpath.normpath(join(pkg_dir, pkg["workspace"]))
            ws_dir = "" if ws_dir == "." else ws_dir
        else:
            for d in ancestors(join(pkg_dir, "Cargo.toml"))[1:]:
                m = _toml(tree, join(d, "Cargo.toml"))
                if m is not None and "workspace" in m:
                    ws_dir = d
                    break
        if ws_dir is None:
            raise ContractError(f"{join(pkg_dir, 'Cargo.toml')} inherits its edition from a "
                                f"workspace that cannot be found")
        ws = _toml(tree, join(ws_dir, "Cargo.toml")) or {}
        wed = ((ws.get("workspace") or {}).get("package") or {}).get("edition")
        if not isinstance(wed, str):
            raise ContractError(f"{join(ws_dir, 'Cargo.toml')} declares no "
                                f"[workspace.package] edition for {pkg_dir}")
        return wed
    raise ContractError(f"{join(pkg_dir, 'Cargo.toml')} has an unreadable `edition` value {ed!r}")


def rust_contract(tree: Tree, rel: str):
    key = ("rust", posixpath.dirname(rel))
    if key not in tree.memo:
        try:
            tree.memo[key] = _rust_contract(tree, rel)
        except ContractError as e:
            tree.memo[key] = e
    hit = tree.memo[key]
    if isinstance(hit, ContractError):
        raise hit
    return hit


def _rust_contract(tree: Tree, rel: str):
    """The rustfmt contract depends only on the file's directory (memoized by rust_contract)."""
    dirs = ancestors(rel)
    tree.prefetch([join(d, n) for d in dirs for n in ("Cargo.toml", *RUST_CONFIG_NAMES)])
    edition = None
    for d in dirs:
        m = _toml(tree, join(d, "Cargo.toml"))
        if m is None:
            continue
        if "package" not in m:
            return Outside(f"nearest Cargo.toml ({join(d, 'Cargo.toml')}) is a virtual manifest")
        edition = resolve_edition(tree, d, m)
        break
    if edition is None:
        return Outside("no owning Cargo package")
    for d in dirs:
        for name in RUST_CONFIG_NAMES:
            cfg = join(d, name)
            data = tree.read(cfg)
            if data is not None:
                return RustContract(edition, cfg, _digest(data))
    return RustContract(edition, None, _digest(None))


_SHELL_OPS = {"&&", "||", ";", "|", "&"}


@dataclasses.dataclass(frozen=True)
class PrettierCheck:
    """What a `prettier --check` script declares, paths relative to the owning package."""
    globs: tuple[str, ...]
    config: str | None = None                    # --config <path>
    ignore_paths: tuple[str, ...] | None = None  # --ignore-path <path>… (replaces the defaults)


# Flags that take a value and that this gate models; everything else unknown refuses.
_VALUE_FLAGS = ("--config", "--ignore-path")


def parse_prettier_check(script: str) -> PrettierCheck | None:
    """Globs and the configuration flags of a `prettier --check <globs>` script. None = the script
    is not a Prettier check. `--config` and `--ignore-path` are modelled exactly as the CLI applies
    them (an explicit config file; ignore files that REPLACE the default .gitignore/.prettierignore).
    Any other flag raises ContractError (unmeasured, never silently narrowed)."""
    try:
        toks = shlex.split(script)
    except ValueError as e:
        raise ContractError(f"format:check script is not parseable: {e}") from e
    for i, t in enumerate(toks):
        if t == "prettier" or t.endswith("/prettier"):
            globs, checking, config, ignores = [], False, None, []
            rest = toks[i + 1:]
            j = 0
            while j < len(rest):
                a = rest[j]
                j += 1
                if a in _SHELL_OPS:
                    break
                if a in ("--check", "-c"):
                    checking = True
                    continue
                flag, eq, val = a.partition("=")
                if flag in _VALUE_FLAGS:
                    if not eq:
                        if j >= len(rest) or rest[j] in _SHELL_OPS or rest[j].startswith("-"):
                            raise ContractError(f"format:check passes `{flag}` without a value")
                        val = rest[j]
                        j += 1
                    if flag == "--config":
                        if config is not None:
                            raise ContractError("format:check passes `--config` twice")
                        config = val
                    else:
                        ignores.append(val)
                elif a.startswith("-"):
                    raise ContractError(f"format:check passes `{a}` to prettier, which this gate "
                                        f"does not model")
                else:
                    neg = a.startswith("!")
                    g = a[1:] if neg else a
                    g = g[2:] if g.startswith("./") else g
                    globs.append(("!" if neg else "") + g)
            if not checking:
                return None
            for g in globs:
                if not any(c in g for c in "*?{["):
                    raise ContractError(f"format:check names `{g}` without a glob; this gate "
                                        f"models extension globs only")
            if globs and all(g.startswith("!") for g in globs):
                raise ContractError("format:check names only negated globs")
            return PrettierCheck(tuple(globs), config, tuple(ignores) if ignores else None)
    return None


def _brace_expand(pat: str) -> list[str]:
    m = re.search(r"\{([^{}]*)\}", pat)
    if not m:
        return [pat]
    out = []
    for alt in m.group(1).split(","):
        out.extend(_brace_expand(pat[:m.start()] + alt + pat[m.end():]))
    return out


def _glob_regex(pat: str) -> re.Pattern:
    i, out = 0, []
    while i < len(pat):
        c = pat[i]
        if pat.startswith("**/", i):
            out.append("(?:.*/)?")
            i += 3
        elif pat.startswith("**", i):
            out.append(".*")
            i += 2
        elif c == "*":
            out.append("[^/]*")
            i += 1
        elif c == "?":
            out.append("[^/]")
            i += 1
        elif c == "[":
            j = pat.find("]", i + 1)
            if j < 0:
                out.append(re.escape(c))
                i += 1
            else:
                body = pat[i + 1:j]
                if body.startswith("!"):
                    body = "^" + body[1:]
                out.append(f"[{body}]")
                i = j + 1
        else:
            out.append(re.escape(c))
            i += 1
    return re.compile("^" + "".join(out) + "$")


_GLOB_CACHE: dict[str, list[re.Pattern]] = {}


def _glob_hit(g: str, rel: str) -> bool:
    pats = _GLOB_CACHE.get(g)
    if pats is None:
        pats = _GLOB_CACHE[g] = [_glob_regex(p) for p in _brace_expand(g)]
    return any(p.match(rel) for p in pats)


def glob_match(globs, rel: str) -> bool:
    """Prettier CLI semantics: a file is selected by any positive glob and by no `!` glob."""
    pos = [g for g in globs if not g.startswith("!")]
    neg = [g[1:] for g in globs if g.startswith("!")]
    return any(_glob_hit(g, rel) for g in pos) and not any(_glob_hit(g, rel) for g in neg)


def _json(tree: Tree, rel: str):
    return _parsed(tree, rel, json.loads, "JSON")


def _package_path(tree: Tree, pkg_dir: str, p: str, flag: str) -> str:
    """A `--config`/`--ignore-path` value, resolved from the package directory to a repo path."""
    rel = posixpath.normpath(join(pkg_dir, p))
    if p.startswith("/") or rel == ".." or rel.startswith("../"):
        raise ContractError(f"{join(pkg_dir, 'package.json')} format:check `{flag} {p}` points "
                            f"outside the repository")
    return "" if rel == "." else rel


def _package_check(tree: Tree, d: str):
    """(package.json dict, PrettierCheck) for directory d, or None. Memoized per directory."""
    key = ("pcheck", d)
    if key not in tree.memo:
        try:
            pj = _json(tree, join(d, "package.json"))
            script = (pj.get("scripts") or {}).get("format:check") if isinstance(pj, dict) else None
            check = parse_prettier_check(script) if isinstance(script, str) else None
            tree.memo[key] = (pj, script, check) if check and check.globs else None
        except ContractError as e:
            tree.memo[key] = e
    hit = tree.memo[key]
    if isinstance(hit, ContractError):
        raise hit
    return hit


def prettier_contract(tree: Tree, rel: str):
    if "node_modules" in rel.split("/"):
        return Outside("node_modules")
    dirs = ancestors(rel)
    if ("pdir", dirs[0]) not in tree.memo:
        tree.prefetch([join(d, n) for d in dirs
                       for n in ("package.json", ".editorconfig", *PRETTIER_CONFIG_NAMES,
                                 *PRETTIER_IGNORE_NAMES)])
        tree.memo[("pdir", dirs[0])] = True
    for d in dirs:
        owner = _package_check(tree, d)
        if owner is None:
            continue
        _pj, _script, check = owner
        sub = rel[len(d) + 1:] if d else rel
        if not glob_match(check.globs, sub):
            continue
        key = ("prettier", dirs[0], d)
        if key not in tree.memo:
            try:
                tree.memo[key] = _prettier_closure(tree, dirs, d, owner)
            except ContractError as e:
                tree.memo[key] = e
        hit = tree.memo[key]
        if isinstance(hit, ContractError):
            raise hit
        return hit
    return Outside("no package declares a Prettier format:check contract for this path")


def _prettier_closure(tree: Tree, dirs: list[str], d: str, owner) -> PrettierContract:
    """The contract a package's format:check gives files in dirs[0]: it depends only on the
    directory, so it is computed once per (directory, owning package)."""
    pj, script, check = owner
    config = _package_path(tree, d, check.config, "--config") if check.config else None
    ignores = (tuple(_package_path(tree, d, p, "--ignore-path") for p in check.ignore_paths)
               if check.ignore_paths is not None else None)
    if config is not None and tree.read(config) is None:
        raise ContractError(f"{join(d, 'package.json')} format:check `--config {check.config}` "
                            f"names {config}, which {tree.label} does not hold")
    parts: list[bytes | None] = [json.dumps(
        {"type": pj.get("type"), "prettier": pj.get("prettier"), "script": script},
        sort_keys=True).encode()]
    for a in dirs:  # config + editorconfig chain from the file up to the root
        parts.append(a.encode())
        for n in (*PRETTIER_CONFIG_NAMES, ".editorconfig"):
            parts.append(tree.read(join(a, n)))
        if a != d:
            apj = _json(tree, join(a, "package.json"))
            if isinstance(apj, dict):
                parts.append(json.dumps({"type": apj.get("type"),
                                         "prettier": apj.get("prettier")},
                                        sort_keys=True).encode())
    if config is not None:
        parts.extend([b"config", config.encode(), tree.read(config)])
    contract = PrettierContract(d, check.globs, "", config, ignores)
    for p in contract.ignore_files():
        parts.extend([p.encode(), tree.read(p)])
    return dataclasses.replace(contract, closure_digest=_digest(*parts))


def contract_for(tree: Tree, rel: str):
    if rel.endswith(".rs"):
        return rust_contract(tree, rel)
    return prettier_contract(tree, rel)


# ── results ──

@dataclasses.dataclass
class FileResult:
    path: str
    formatter: str | None
    state: str                 # clean | drift | debt | ignored | outside | unmeasured
    reason: str = ""
    cure: str = ""
    missing_tool: bool = False
    evidence: dict = dataclasses.field(default_factory=dict)

    def as_dict(self) -> dict:
        return dataclasses.asdict(self)


@dataclasses.dataclass
class Measure:
    """One formatter run over one blob: formatted | unformatted | ignored | outside | error |
    missing (the formatter this contract needs is not installed; `cure` names the install)."""
    state: str
    detail: str = ""
    syntax: bool = False       # the formatter rejected the input itself (parse failure)
    tool: str = ""
    config: str | None = None
    cure: str = ""             # state "missing": the install cure for the absent formatter


# ── formatter adapters ──

def pinned_toolchain(tree: Tree) -> str | None:
    pol = _json(tree, POOL_POLICY_REL)
    try:
        pin = pol["cargo_env_overrides"]["*"]["RUSTUP_TOOLCHAIN"]
        return pin if isinstance(pin, str) and pin else None
    except Exception:  # noqa: BLE001
        return None


class Rustfmt:
    def __init__(self, pin: str | None):
        self.pin = pin
        self._binary = None
        self._version = None
        self._missing: MissingTool | None = None   # a failed resolution is not retried per side
        self._lock = threading.Lock()

    def cure(self) -> str:
        return CURE_RUSTFMT.format(pin=self.pin or "<pinned toolchain>")

    def binary(self) -> tuple[str, str]:
        with self._lock:
            if self._missing is not None:
                raise self._missing
            try:
                return self._resolve()
            except MissingTool as e:
                self._missing = e
                raise

    def _resolve(self) -> tuple[str, str]:
        if self._binary is None:
            args = ["rustup", "which", "rustfmt"] + (["--toolchain", self.pin] if self.pin else [])
            try:
                proc = subprocess.run(args, capture_output=True, text=True, timeout=15)
            except FileNotFoundError as e:
                raise MissingTool("rustup is not on PATH, so the pinned rustfmt cannot be "
                                  "resolved", self.cure()) from e
            except subprocess.TimeoutExpired as e:
                raise MissingTool("`rustup which rustfmt` timed out", self.cure()) from e
            path = proc.stdout.strip()
            if proc.returncode != 0 or not path or not os.access(path, os.X_OK):
                raise MissingTool(f"rustfmt is not installed for toolchain "
                                  f"{self.pin or '(active)'}: "
                                  f"{(proc.stderr or proc.stdout).strip()[:200]}", self.cure())
            try:
                ver = subprocess.run([path, "--version"], capture_output=True, text=True,
                                     timeout=15)
            except (OSError, subprocess.TimeoutExpired) as e:
                raise MissingTool(f"rustfmt at {path} does not run: {e}", self.cure()) from e
            if ver.returncode != 0:
                raise MissingTool(f"rustfmt at {path} does not run: {ver.stderr.strip()[:200]}",
                                  self.cure())
            self._binary, self._version = path, ver.stdout.strip()
        return self._binary, self._version

    def measure(self, tree: Tree, rel: str, data: bytes, contract: RustContract) -> Measure:
        binary, version = self.binary()
        if contract.config:
            cfg = tree.materialize(contract.config)
        else:
            cfg = tree.path(EMPTY_RUSTFMT_REL)
            if tree.git is None:  # disk mode: never write into the worktree
                cfg = Path(tempfile.gettempdir()) / "epr-format-empty-rustfmt.toml"
            cfg.parent.mkdir(parents=True, exist_ok=True)
            if not cfg.exists():
                cfg.write_bytes(b"")
        args = [binary, "--edition", contract.edition, "--config-path", str(cfg),
                "--emit", "stdout", "--color", "never"]
        try:
            proc = subprocess.run(args, input=data, capture_output=True, cwd=str(cfg.parent),
                                  timeout=RUSTFMT_TIMEOUT_S)
        except subprocess.TimeoutExpired:
            return Measure("error", f"rustfmt timed out after {RUSTFMT_TIMEOUT_S}s", tool=version)
        except OSError as e:
            return Measure("error", f"rustfmt failed to start: {e}", tool=version)
        if proc.returncode != 0:
            first = proc.stderr.decode("utf-8", "replace").strip().splitlines()
            return Measure("error", f"rustfmt exited {proc.returncode}: "
                                    f"{first[0] if first else 'no diagnostic'}",
                           syntax=True, tool=version, config=contract.config)
        state = "formatted" if proc.stdout == data else "unformatted"
        return Measure(state, tool=version, config=contract.config)


class PrettierWorker:
    """One Node process for the whole invocation; each batch is one JSON line each way."""

    def __init__(self):
        self.proc = None
        self.lines: queue.Queue = queue.Queue()
        self.launches = 0

    def _start(self):
        node = shutil.which("node")
        if not node:
            raise MissingTool("node is not on PATH, so Prettier cannot run", CURE_NODE)
        if not WORKER.is_file():
            raise MissingTool(f"the Prettier worker {WORKER} is missing", CURE_HOOK)
        self.proc = subprocess.Popen([node, str(WORKER)], stdin=subprocess.PIPE,
                                     stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        self.launches += 1
        threading.Thread(target=self._pump, daemon=True).start()

    def _pump(self):
        for line in self.proc.stdout:
            self.lines.put(line)
        self.lines.put(None)

    def run(self, items: list[dict]) -> list[dict]:
        if not items:
            return []
        if self.proc is None:
            self._start()
        try:
            self.proc.stdin.write(json.dumps({"items": items}).encode() + b"\n")
            self.proc.stdin.flush()
        except OSError as e:
            raise EnforcementError(f"the Prettier worker died before accepting work: {e}") from e
        try:
            line = self.lines.get(timeout=PRETTIER_TIMEOUT_S)
        except queue.Empty:
            self.close()
            return [{"id": it["id"], "state": "error",
                     "detail": f"prettier timed out after {PRETTIER_TIMEOUT_S}s"} for it in items]
        if line is None:
            err = self.proc.stderr.read().decode("utf-8", "replace").strip()[-300:]
            self.proc = None
            return [{"id": it["id"], "state": "error",
                     "detail": f"the Prettier worker exited: {err or 'no diagnostic'}"}
                    for it in items]
        try:
            return json.loads(line)["results"]
        except Exception as e:  # noqa: BLE001
            return [{"id": it["id"], "state": "error",
                     "detail": f"the Prettier worker answered unreadably: {e}"} for it in items]

    def close(self):
        if self.proc is not None:
            try:
                self.proc.stdin.close()
                self.proc.wait(timeout=5)
            except Exception:  # noqa: BLE001
                self.proc.kill()
            self.proc = None


def _prettier_item(tree: Tree, rel: str, data: bytes, contract: PrettierContract,
                   resolve_from: Path, ident: str) -> dict:
    target = tree.materialize(rel, data)
    for d in ancestors(rel):  # the config/editorconfig closure the worker will resolve
        for n in (*PRETTIER_CONFIG_NAMES, ".editorconfig", "package.json"):
            if tree.read(join(d, n)) is not None:
                tree.materialize(join(d, n))
    ignore = [str(tree.materialize(p)) for p in contract.ignore_files()
              if tree.read(p) is not None]
    item = {"id": ident, "file": str(target), "ignorePaths": ignore,
            "resolveFrom": str(resolve_from)}
    if contract.config:
        item["config"] = str(tree.materialize(contract.config))
    return item


def _measure_from_worker(r: dict) -> Measure:
    state = r.get("state")
    if state == "missing":
        return Measure("missing", f"Prettier is not installed for this package: "
                                  f"{r.get('detail', '')}", cure=CURE_PRETTIER)
    tool = f"prettier {r.get('version', '?')}"
    if state in ("formatted", "unformatted", "ignored"):
        return Measure(state, tool=tool, config=r.get("configFile"))
    if state == "outside":
        return Measure("outside", r.get("detail", ""), tool=tool)
    return Measure("error", r.get("detail", "prettier failed"), syntax=bool(r.get("syntax")),
                   tool=tool)


# ── the ratchet ──

def _cure_for(formatter: str, rel: str, contract) -> str:
    if formatter == "rustfmt":
        cfg = f" --config-path {contract.config}" if contract.config else ""
        return (f"`rustfmt --edition {contract.edition}{cfg} {rel}` (pinned toolchain), "
                f"review the result, then `git add {rel}`")
    pkg = contract.package_dir or "."
    sub = rel[len(contract.package_dir) + 1:] if contract.package_dir else rel
    flags = ""
    if contract.config:
        flags += f" --config {shlex.quote(posixpath.relpath(contract.config, pkg))}"
    for ip in contract.ignore_paths or ():
        flags += f" --ignore-path {shlex.quote(posixpath.relpath(ip, pkg))}"
    return (f"`(cd {pkg} && pnpm exec prettier{flags} --write {shlex.quote(sub)})`, review the "
            f"result, then `git add {rel}`")


@dataclasses.dataclass
class Item:
    change: Change
    contract: object = None
    head_contract: object = None


class StagedFormatter:
    """Plan → measure → judge over one invocation's staged index."""

    def __init__(self, repo: Path, git: Git | None = None, *, scratch: Path | None = None):
        self.repo = Path(repo)
        self.git = git or Git(self.repo)
        self._tmp = tempfile.TemporaryDirectory(prefix="epr-format-", dir=scratch)
        base = Path(self._tmp.name)
        self.head = head_commit(self.git)
        self.index = Tree(self.git, ":", base / "index", "index")
        self.base = Tree(self.git if self.head else None, f"{self.head}:" if self.head else None,
                         base / "head", "HEAD")
        self.fingerprint = index_fingerprint(self.git)
        self.rustfmt = Rustfmt(pinned_toolchain(self.index))
        self.prettier_pool: list[PrettierWorker] = []
        self.notices: list[str] = []

    @property
    def prettier_launches(self) -> int:
        return sum(w.launches for w in self.prettier_pool)

    def close(self):
        for w in self.prettier_pool:
            w.close()
        self._tmp.cleanup()

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        self.close()

    CLOSURE_NAMES = ("Cargo.toml", *RUST_CONFIG_NAMES, "package.json", ".editorconfig",
                     *PRETTIER_CONFIG_NAMES, *PRETTIER_IGNORE_NAMES, *MANIFEST_CANDIDATES)

    def prefetch_closure(self, changes: list[Change]) -> None:
        """One `cat-file --batch` per side for every blob and every closure candidate the plan can
        touch — each git launch costs ~60 ms in this repository, so per-file reads add up."""
        for tree, rels in ((self.index, [c.path for c in changes]),
                           (self.base, [c.src_path for c in changes if c.src_path])):
            want = [POLICY_REGISTRY_REL, POOL_POLICY_REL, *rels]
            for d in dict.fromkeys(a for r in rels for a in ancestors(r)):
                want.extend(join(d, n) for n in self.CLOSURE_NAMES)
            tree.prefetch(want)

    def verify_unchanged(self) -> None:
        if index_fingerprint(self.git) != self.fingerprint:
            raise EnforcementError("the index or HEAD changed while the formatting gate was "
                                   "checking it (a concurrent `git add`/commit); re-run the commit")

    # contract-change rescan: an unchanged file whose committed contract moved is re-judged
    def contract_rescans(self, changes: list[Change]) -> list[Change]:
        changed = {c.path for c in changes}
        roots: set[tuple[str, str]] = set()
        for c in changes + [Change(p, "D", None, "", "", None, "", False)
                            for p in self._deleted_paths()]:
            name = posixpath.basename(c.path)
            d = posixpath.dirname(c.path)
            if name in RUST_CONFIG_NAMES or name == "Cargo.toml":
                roots.add((d, "rust"))
            elif name in PRETTIER_CONFIG_NAMES or name in (".editorconfig", "package.json"):
                roots.add((d, "prettier"))
            elif name in PRETTIER_IGNORE_NAMES:
                pj = _json(self.index, join(d, "package.json")) or _json(self.base, join(d, "package.json"))
                if isinstance(pj, dict) and (pj.get("scripts") or {}).get("format:check"):
                    roots.add((d, "prettier"))
        if not roots:
            return []
        extra: dict[str, Change] = {}
        for d, kind in sorted(roots):
            listing = self.git.out("ls-files", "-s", "-z", "--", d or ".")
            for rec in listing.split(b"\0"):
                if not rec:
                    continue
                meta, path = rec.split(b"\t", 1)
                mode, blob, _stage = meta.decode().split(" ")
                rel = path.decode("utf-8", "surrogateescape")
                if rel in changed or rel in extra or mode not in REGULAR_MODES:
                    continue
                if (kind == "rust") != rel.endswith(".rs"):
                    continue
                extra[rel] = Change(rel, "M", rel, mode, mode, blob, blob, False, True)
        if extra:
            self._prime_listings(extra)
        out = []
        for rel, ch in extra.items():
            try:
                now, then = contract_for(self.index, rel), contract_for(self.base, rel)
            except (ContractError, MissingTool):
                out.append(ch)  # cannot prove the contract unchanged: judge it
                continue
            if isinstance(now, Outside):
                continue
            if now.identity() != then.identity():
                out.append(ch)
        return out

    def _prime_listings(self, extra: dict[str, Change]) -> None:
        """Before a rescan computes two contracts for each of possibly thousands of files, list
        each side once and fetch every closure file that exists in ONE `cat-file` per side —
        otherwise each new directory costs its own git launch per side (a root package.json
        change measured 3,436 launches and 155 s before this)."""
        if self.index.known is None:
            raw = self.git.out("ls-files", "-z", "--full-name")
            self.index.know({p.decode("utf-8", "surrogateescape") for p in raw.split(b"\0") if p})
        if self.head and self.base.known is None:
            raw = self.git.out("ls-tree", "-r", "-z", "--name-only", "--full-tree", self.head)
            self.base.know({p.decode("utf-8", "surrogateescape") for p in raw.split(b"\0") if p})
        dirs = dict.fromkeys(a for r in extra for a in ancestors(r))
        want = [join(d, n) for d in dirs for n in self.CLOSURE_NAMES]
        self.index.prefetch(want)
        self.base.prefetch(want)

    def _deleted_paths(self) -> list[str]:
        if not self.head:
            return []
        raw = self.git.out("diff", "--cached", "--name-only", "-z", "--diff-filter=D",
                           "--no-renames", self.head)
        return [p.decode("utf-8", "surrogateescape") for p in raw.split(b"\0") if p]

    def measure_side(self, tree: Tree, jobs: list[tuple[str, str, bytes, object]]) -> dict:
        """jobs: (id, rel, data, contract) → {id: Measure}. Rust in a bounded pool; Prettier in one
        batched worker, fanned out to at most PRETTIER_POOL workers for a large batch (a merge)."""
        out: dict[str, Measure] = {}
        rust = [j for j in jobs if isinstance(j[3], RustContract)]
        pret = [j for j in jobs if isinstance(j[3], PrettierContract)]
        if rust:
            try:
                self.rustfmt.binary()  # MissingTool surfaces once, before any pool work
            except MissingTool as e:
                for ident, *_ in rust:
                    out[ident] = Measure("missing", str(e), cure=e.cure)
                rust = []
            with concurrent.futures.ThreadPoolExecutor(max_workers=RUST_POOL) as pool:
                futs = {pool.submit(self.rustfmt.measure, tree, rel, data, c): ident
                        for ident, rel, data, c in rust}
                for fut, ident in futs.items():
                    out[ident] = fut.result()
        if pret:
            items = [_prettier_item(tree, rel, data, c, self.repo / c.package_dir, ident)
                     for ident, rel, data, c in pret]
            k = max(1, min(PRETTIER_POOL, len(items) // PRETTIER_PER_WORKER))
            while len(self.prettier_pool) < k:
                self.prettier_pool.append(PrettierWorker())
            chunks = [items[i::k] for i in range(k)]
            try:
                with concurrent.futures.ThreadPoolExecutor(max_workers=k) as pool:
                    answers = list(pool.map(lambda wc: wc[0].run(wc[1]),
                                            zip(self.prettier_pool[:k], chunks)))
            except MissingTool as e:  # node or the worker itself is absent
                answers = [[{"id": it["id"], "state": "missing-tool", "detail": str(e),
                             "cure": e.cure} for it in items]]
            for batch in answers:
                for r in batch:
                    if r.get("state") == "missing-tool":
                        out[r["id"]] = Measure("missing", r["detail"], cure=r["cure"])
                    else:
                        out[r["id"]] = _measure_from_worker(r)
        return out

    def judge(self, changes: list[Change]) -> dict[str, FileResult]:
        """Measure every selected change and apply the ratchet. Returns {path: FileResult}."""
        results: dict[str, FileResult] = {}
        self.index.prefetch([c.path for c in changes])
        self.base.prefetch([c.src_path for c in changes if c.src_path])
        staged_jobs, items = [], {}
        for c in changes:
            try:
                contract = contract_for(self.index, c.path)
            except ContractError as e:
                results[c.path] = FileResult(c.path, None, "unmeasured", str(e),
                                             "fix the configuration file named above")
                continue
            if isinstance(contract, Outside):
                results[c.path] = FileResult(c.path, None, "outside", contract.reason)
                continue
            data = self.index.read(c.path)
            if data is None:
                raise EnforcementError(f"staged blob for {c.path} is unreadable")
            items[c.path] = Item(c, contract)
            staged_jobs.append((c.path, c.path, data, contract))
        staged = self.measure_side(self.index, staged_jobs)

        head_jobs = []
        for path, item in items.items():
            m = staged.get(path)
            fmt = "rustfmt" if isinstance(item.contract, RustContract) else "prettier"
            ev = {"blob": item.change.dst_blob, "headBlob": item.change.src_blob,
                  "baselinePath": item.change.src_path, "tool": m.tool if m else None,
                  "config": (m.config if m else None), "contract": list(item.contract.identity())}
            if m is None:
                raise EnforcementError(f"{path} was selected but no formatting result came back")
            if m.state == "missing":
                results[path] = FileResult(path, fmt, "unmeasured",
                                           f"{m.detail} — the staged blob cannot be checked",
                                           m.cure, missing_tool=True, evidence=ev)
            elif m.state == "ignored":
                results[path] = FileResult(path, fmt, "ignored", "ignored by the package's "
                                           "ignore files", evidence=ev)
            elif m.state == "outside":
                results[path] = FileResult(path, fmt, "outside", m.detail, evidence=ev)
            elif m.state == "error":
                results[path] = FileResult(path, fmt, "unmeasured",
                                           f"{fmt} could not measure the staged blob: {m.detail}",
                                           "fix the file so it parses, then stage it",
                                           evidence=ev)
            elif m.state == "formatted":
                results[path] = FileResult(path, fmt, "clean", evidence=ev)
            elif item.change.is_new or not item.change.src_path:
                results[path] = FileResult(
                    path, fmt, "drift",
                    f"{path} is a new file and its staged content is not {fmt}-formatted",
                    _cure_for(fmt, path, item.contract), evidence=ev)
            else:
                try:
                    item.head_contract = contract_for(self.base, item.change.src_path)
                except ContractError:
                    item.head_contract = Outside("HEAD configuration unreadable")
                if isinstance(item.head_contract, Outside) or (
                        type(item.head_contract) is not type(item.contract)):
                    results[path] = FileResult(
                        path, fmt, "debt",
                        f"{path} is not {fmt}-formatted, and at HEAD it was outside this "
                        f"contract — permitted as existing debt", _cure_for(fmt, path, item.contract),
                        evidence=ev)
                    continue
                hdata = self.base.read(item.change.src_path)
                if hdata is None:
                    raise EnforcementError(f"HEAD blob for {item.change.src_path} is unreadable")
                head_jobs.append((path, item.change.src_path, hdata, item.head_contract))
        if head_jobs:
            headm = self.measure_side(self.base, head_jobs)
            for path, src, _data, _c in head_jobs:
                item = items[path]
                fmt = "rustfmt" if isinstance(item.contract, RustContract) else "prettier"
                m = headm.get(path)
                ev = {"blob": item.change.dst_blob, "headBlob": item.change.src_blob,
                      "baselinePath": src, "contract": list(item.contract.identity()),
                      "headState": m.state if m else None, "tool": m.tool if m else None}
                if m is None:
                    raise EnforcementError(f"{path}: no HEAD baseline result came back")
                if m.state == "missing":
                    results[path] = FileResult(path, fmt, "unmeasured",
                                               f"{m.detail} — the HEAD baseline cannot be checked",
                                               m.cure, missing_tool=True, evidence=ev)
                elif m.state == "error" and not m.syntax:
                    results[path] = FileResult(path, fmt, "unmeasured",
                                               f"{fmt} could not measure the HEAD baseline: "
                                               f"{m.detail}", "re-run the commit", evidence=ev)
                elif m.state == "formatted":
                    why = ("its formatter contract changed in this commit" if item.change.contract_rescan
                           else "this commit")
                    results[path] = FileResult(
                        path, fmt, "drift",
                        f"{path} was {fmt}-formatted at HEAD and {why} leaves the staged "
                        f"content unformatted", _cure_for(fmt, path, item.contract), evidence=ev)
                else:
                    results[path] = FileResult(
                        path, fmt, "debt",
                        f"{path} was already unformatted at HEAD and is still unformatted — "
                        f"permitted as existing file-level debt; formatting it makes HEAD the "
                        f"stronger baseline", _cure_for(fmt, path, item.contract), evidence=ev)
        return results


# ── edit-time advisory (worktree bytes; never the commit decision) ──

def check_worktree_file(repo: Path, path: str | Path) -> FileResult | None:
    """Format status of one worktree file for the PostToolUse advisory. None when the file is
    outside every contract. Never raises; failures come back as `unmeasured`."""
    repo = Path(repo)
    p = Path(path)
    try:
        rel = p.resolve().relative_to(repo.resolve()).as_posix() if p.is_absolute() else p.as_posix()
    except ValueError:
        return None
    tree = DiskTree(repo)
    try:
        contract = contract_for(tree, rel)
        if isinstance(contract, Outside):
            return None
        data = tree.read(rel)
        if data is None:
            return None
        fmt = "rustfmt" if isinstance(contract, RustContract) else "prettier"
        if fmt == "rustfmt":
            m = Rustfmt(pinned_toolchain(tree)).measure(tree, rel, data, contract)
        else:
            worker = PrettierWorker()
            item = {"id": rel, "file": str(repo / rel),
                    "ignorePaths": [str(repo / p) for p in contract.ignore_files()
                                    if tree.read(p) is not None],
                    "resolveFrom": str(repo / contract.package_dir)}
            if contract.config:
                item["config"] = str(repo / contract.config)
            try:
                [r] = worker.run([item])
            finally:
                worker.close()
            m = _measure_from_worker(r)
        if m.state == "missing":
            return FileResult(rel, fmt, "unmeasured", m.detail, m.cure, missing_tool=True)
        if m.state == "formatted":
            return FileResult(rel, fmt, "clean")
        if m.state in ("ignored", "outside"):
            return FileResult(rel, fmt, m.state)
        if m.state == "unformatted":
            return FileResult(rel, fmt, "drift", f"{rel} is not {fmt}-formatted",
                              _cure_for(fmt, rel, contract).replace(", then `git add", "; stage with `git add"))
        return FileResult(rel, fmt, "unmeasured", m.detail)
    except MissingTool as e:
        return FileResult(rel, None, "unmeasured", str(e), e.cure, missing_tool=True)
    except Exception as e:  # noqa: BLE001 — an advisory never breaks the edit
        return FileResult(rel, None, "unmeasured", f"format check failed: {e}")
