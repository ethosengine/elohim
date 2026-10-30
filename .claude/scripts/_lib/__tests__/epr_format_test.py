"""Commit-time formatting ratchet (commit-format-ratchet@1) — engine, adapter and REAL hooks.

Every behavioural case runs `git commit` / `git merge` in an isolated fixture repository whose
core.hooksPath is a copy of this checkout's tracked .husky entrypoints, so what is proven is hook
coverage, not just a Python function. Fixture git children get a scrubbed environment (inherited
GIT_DIR/GIT_INDEX_FILE would otherwise aim them at THIS repository); the real hook never scrubs.

Run: python3 .claude/scripts/_lib/__tests__/epr_format_test.py   (exit 0 = pass)
Bespoke assert-based harness — matches this __tests__ dir's convention (pytest is not installed).
"""
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

_here = Path(__file__).resolve()
REPO = None
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        REPO = _here
        break
    _here = _here.parent
if REPO is None:  # pragma: no cover
    print("FAIL: could not locate repo root", file=sys.stderr)
    sys.exit(1)

from _lib import epr_format, epr_meta, epr_meta_git  # noqa: E402

_passed = 0
_failures: list[str] = []


def check(label: str, cond: bool, detail: str = "") -> None:
    global _passed
    if cond:
        _passed += 1
        print(f"  ✅ {label}")
    else:
        _failures.append(label)
        print(f"  ❌ {label}")
        for line in (detail or "").splitlines()[-25:]:
            print(f"       {line}")


# ── environment ────────────────────────────────────────────────────────────────────────────────
SCRUB = ("GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE", "GIT_PREFIX", "GIT_COMMON_DIR",
         "GIT_OBJECT_DIRECTORY", "GIT_ALTERNATE_OBJECT_DIRECTORIES", "EPR_META_ACK", "HUSKY",
         "CLAUDE_SESSION_ID", "ELOHIM_SESSION_ID", "GIT_CONFIG_PARAMETERS")
TMP = Path(tempfile.mkdtemp(prefix="epr-format-test-"))
BASE_ENV = {k: v for k, v in os.environ.items() if k not in SCRUB}
BASE_ENV.update({
    "HOME": str(TMP), "GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": "/dev/null",
    "GIT_AUTHOR_NAME": "F", "GIT_AUTHOR_EMAIL": "f@example.test",
    "GIT_COMMITTER_NAME": "F", "GIT_COMMITTER_EMAIL": "f@example.test",
    "EPR_BIN": str(TMP / "no-native-epr"),  # native evaluator absent unless a case stubs it
})
RUSTUP_DIR = os.path.dirname(shutil.which("rustup") or "/nonexistent/rustup")
NODE_DIR = os.path.dirname(shutil.which("node") or "/nonexistent/node")


def _prettier_dir() -> Path | None:
    roots = [REPO]
    try:
        common = subprocess.run(["git", "rev-parse", "--path-format=absolute", "--git-common-dir"],
                                cwd=REPO, env=BASE_ENV, capture_output=True, text=True).stdout.strip()
        if common:
            roots.append(Path(common).parent)
    except OSError:
        pass
    for r in roots:
        cand = r / "node_modules" / "prettier"
        if (cand / "package.json").is_file():
            return cand.resolve()
    return None


PRETTIER = _prettier_dir()
HAVE_RUSTFMT = bool(shutil.which("rustup"))
HAVE_NODE = bool(shutil.which("node"))

# Husky v9's shim runner, verbatim in behaviour (the generated .husky/_/h is not tracked).
HUSKY_H = """#!/usr/bin/env sh
[ "$HUSKY" = "2" ] && set -x
n=$(basename "$0")
s=$(dirname "$(dirname "$0")")/$n
[ ! -f "$s" ] && exit 0
[ "${HUSKY-}" = "0" ] && exit 0
export PATH="node_modules/.bin:$PATH"
sh -e "$s" "$@"
c=$?
[ $c != 0 ] && echo "husky - $n script failed (code $c)"
exit $c
"""


def run(cwd, *args, env=None, input=None):
    return subprocess.run(list(args), cwd=cwd, env=env or BASE_ENV, capture_output=True,
                          text=True, input=input)


def git(cwd, *args, env=None, ok=True):
    r = run(cwd, "git", *args, env=env)
    if ok and r.returncode != 0:
        raise RuntimeError(f"git {' '.join(args)} failed: {r.stderr}")
    return r


def commit(cwd, msg="c", env=None, *extra):
    return git(cwd, "commit", "-q", "-m", msg, *extra, env=env, ok=False)


def write(root: Path, rel: str, text: str):
    p = root / rel
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(text)


RS_CLEAN = "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n"
RS_BAD = "pub fn add(a:i32,b:i32)->i32{a+b}\n"
RS_DEBT = "pub fn debt( ) -> i32 {  1 }\n"
RS_DEBT_WORSE = "pub fn debt( ) -> i32 {  1 }\npub fn more( )->i32{2}\n"
TS_CLEAN = "export const answer = 'yes';\n"
TS_BAD = 'export const answer   =  "yes"\n'
ROOT_MANIFEST = """---
epr-meta-version: 1
id: fixture-root
root: true
rules:
  - { id: commit-format-ratchet, policy: commit-format-ratchet@1 }
---
Fixture governance root.
"""

TEMPLATE = TMP / "template"


def build_template():
    t = TEMPLATE
    t.mkdir()
    git(t, "init", "-q", "-b", "main")
    write(t, ".epr-meta/manifest.md", ROOT_MANIFEST)
    (t / ".claude/epr-meta").mkdir(parents=True)
    shutil.copy(REPO / ".claude/epr-meta/policies.yaml", t / ".claude/epr-meta/policies.yaml")
    os.symlink(REPO / ".claude/scripts", t / ".claude/scripts")  # the gate under test
    (t / ".husky").mkdir()
    for h in ("pre-commit", "pre-merge-commit"):
        shutil.copy(REPO / ".husky" / h, t / ".husky" / h)
        os.chmod(t / ".husky" / h, 0o755)
    write(t, "genesis/agentic/pool-policy.json",
          json.dumps({"cargo_env_overrides": {"*": {"RUSTUP_TOOLCHAIN": "1.96.1"}}}))
    write(t, ".gitignore", ".claude/data/\n")
    # Rust: a virtual workspace whose member inherits its edition (elohim/eprfs's shape).
    write(t, "rs/Cargo.toml", '[workspace]\nmembers = ["clean"]\n[workspace.package]\nedition = "2021"\n')
    write(t, "rs/clean/Cargo.toml", '[package]\nname = "clean"\nversion = "0.1.0"\nedition.workspace = true\n')
    write(t, "rs/clean/src/lib.rs", "pub mod absent_child;\n" + RS_CLEAN)
    write(t, "rs/clean/src/debt.rs", RS_DEBT)
    write(t, "rs/clean/src/conf/a.rs", RS_CLEAN)
    # Explicit package.workspace path, async needing a 2018+ edition.
    write(t, "rs/outer/Cargo.toml", '[workspace]\n[workspace.package]\nedition = "2021"\n')
    write(t, "rs/outer/inner/Cargo.toml",
          '[package]\nname = "inner"\nversion = "0.1.0"\nworkspace = ".."\nedition.workspace = true\n')
    write(t, "rs/outer/inner/src/lib.rs", "pub async fn go() -> u8 {\n    1\n}\n")
    write(t, "rs/virtual-only/stray.rs", RS_BAD)  # under a virtual manifest: outside contract
    # Prettier, ESM package (genesis/a2o's shape) with a frozen ignore.
    write(t, "esm/package.json", json.dumps({"name": "esm", "type": "module", "scripts": {
        "format:check": 'prettier --check "**/*.{ts,js,json,mjs}"'}}, indent=2) + "\n")
    write(t, "esm/.prettierrc.js", "export default {\n  singleQuote: true,\n  printWidth: 100,\n};\n")
    write(t, "esm/.prettierignore", "frozen.ts\n")
    write(t, "esm/.gitignore", "node_modules/\n")
    write(t, "esm/src/ok.ts", TS_CLEAN)
    # Prettier, CJS package (app/elohim-app's shape) scoped to src/ and .editorconfig.
    write(t, "cjs/package.json", json.dumps({"name": "cjs", "scripts": {
        "format:check": 'prettier --check "src/**/*.{ts,html,css,scss}"'}}, indent=2) + "\n")
    write(t, "cjs/.prettierrc.js", "module.exports = {\n  singleQuote: true,\n};\n")
    write(t, "cjs/.editorconfig", "root = true\n[*]\nindent_style = tab\n")
    write(t, "cjs/.gitignore", "node_modules/\n")
    write(t, "cjs/src/ok.ts", "export function f() {\n\treturn 'x';\n}\n")
    write(t, "cjs/scripts/outside.ts", TS_BAD)  # not under the package's src/** contract
    # Prettier with explicit flags (app/elohim-elements/elohim-core's shape): `--config` names the
    # parent's config, so a nearer config is NOT discovered; `--ignore-path` REPLACES the
    # package's default ignore files.
    write(t, "elem/.prettierrc.js", "module.exports = {\n  singleQuote: true,\n};\n")
    write(t, "elem/.prettierignore", "**/skipped.ts\n")
    write(t, "elem/core/package.json", json.dumps({"name": "core", "scripts": {
        "format:check": 'prettier --config ../.prettierrc.js --ignore-path ../.prettierignore '
                        '--check "src/**/*.ts" "*.scss"'}}, indent=2) + "\n")
    write(t, "elem/core/.prettierrc.json", '{"singleQuote": false}\n')  # a decoy discovery finds
    write(t, "elem/core/.prettierignore", "src/default-ignored.ts\n")  # replaced by --ignore-path
    write(t, "elem/core/src/ok.ts", TS_CLEAN)
    write(t, "docs/readme.md", "# anything   goes\n")
    git(t, "add", "-A")
    git(t, "commit", "-q", "--no-verify", "-m", "template")


def fixture(name: str, *, prettier=True, hooks="direct") -> Path:
    d = TMP / name
    shutil.copytree(TEMPLATE, d, symlinks=True)
    if prettier and PRETTIER:
        for pkg in ("esm", "cjs", "elem/core"):
            (d / pkg / "node_modules").mkdir()
            os.symlink(PRETTIER, d / pkg / "node_modules" / "prettier")
    if hooks == "direct":
        git(d, "config", "core.hooksPath", str(d / ".husky"))
    elif hooks == "husky":
        shims = d / ".husky" / "_"
        shims.mkdir()
        (shims / "h").write_text(HUSKY_H)
        for h in ("pre-commit", "pre-merge-commit"):
            (shims / h).write_text('#!/usr/bin/env sh\n. "$(dirname "$0")/h"\n')
            os.chmod(shims / h, 0o755)
        git(d, "config", "core.hooksPath", str(shims))
    return d


def head(d):
    return git(d, "rev-parse", "HEAD").stdout.strip()


def env_with(**kw):
    e = dict(BASE_ENV)
    e.update(kw)
    return e


def restore(d, before, *paths):
    """Put paths back to `before`, whether or not the commit under test (wrongly) landed."""
    git(d, "checkout", before, "--", *paths)
    if head(d) != before:
        git(d, "commit", "-q", "--no-verify", "-m", "restore after a wrongly admitted commit")


def path_without(*dirs):
    keep = [p for p in BASE_ENV["PATH"].split(":") if p and p not in dirs]
    return ":".join(keep)


build_template()

# ════════════════════════════════════════════════════════════════════════════════════════════════
print("rustfmt ratchet through a real `git commit`")
if HAVE_RUSTFMT:
    d = fixture("rust")
    before = head(d)
    write(d, "rs/clean/src/lib.rs", "pub mod absent_child;\n" + RS_BAD)
    git(d, "add", "rs/clean/src/lib.rs")
    r = commit(d)
    check("RED: an unformatted staged .rs in a formatted crate file refuses",
          r.returncode != 0 and head(d) == before, r.stderr)
    check("the refusal names the path, the rule and a scoped cure",
          "rs/clean/src/lib.rs was rustfmt-formatted at HEAD" in r.stderr
          and "commit-format-ratchet" in r.stderr and "rustfmt --edition 2021" in r.stderr, r.stderr)
    check("lib.rs whose child module is absent is still measured (stdin, never the mod tree)",
          "unmeasured" not in r.stderr.lower(), r.stderr)
    r2 = commit(d, "again", env_with(EPR_META_ACK="1"))
    check("EPR_META_ACK=1 does NOT waive formatting drift", r2.returncode != 0 and head(d) == before,
          r2.stderr)
    write(d, "rs/clean/src/lib.rs", "pub mod absent_child;\n" + RS_CLEAN + "// v2\n")
    git(d, "add", "rs/clean/src/lib.rs")
    write(d, "rs/clean/src/lib.rs", "pub mod absent_child;\n" + RS_BAD)  # worktree only
    r = commit(d)
    check("GREEN: the same file formatted and re-staged passes", r.returncode == 0 and head(d) != before,
          r.stderr)

    # partial staging: the index is judged, never the worktree
    write(d, "rs/clean/src/conf/a.rs", RS_BAD)
    git(d, "add", "rs/clean/src/conf/a.rs")
    write(d, "rs/clean/src/conf/a.rs", RS_CLEAN + "// fixed only in the worktree\n")
    r = commit(d)
    check("a formatted WORKTREE cannot rescue an unformatted INDEX", r.returncode != 0, r.stderr)
    git(d, "add", "rs/clean/src/conf/a.rs")
    write(d, "rs/clean/src/conf/a.rs", RS_BAD)
    r = commit(d)
    check("a dirty WORKTREE cannot poison a formatted INDEX (partially staged file)",
          r.returncode == 0, r.stderr)
    # alternate index: `git commit <path>` commits the worktree copy through a temporary index
    write(d, "rs/clean/src/conf/a.rs", RS_CLEAN + "// v3\n")
    r = git(d, "commit", "-q", "-m", "partial", "--", "rs/clean/src/conf/a.rs", ok=False)
    check("`git commit <path>` (GIT_INDEX_FILE temp index) judges the committed content",
          r.returncode == 0, r.stderr)
    write(d, "rs/clean/src/conf/a.rs", RS_BAD)
    r = git(d, "commit", "-q", "-m", "partial-bad", "--", "rs/clean/src/conf/a.rs", ok=False)
    check("`git commit <path>` with unformatted content refuses", r.returncode != 0, r.stderr)
    git(d, "checkout", "--", "rs/clean/src/conf/a.rs")

    # debt
    write(d, "rs/clean/src/debt.rs", RS_DEBT_WORSE)
    git(d, "add", "rs/clean/src/debt.rs")
    r = commit(d)
    check("touched debt (unformatted at HEAD, still unformatted) passes with a notice",
          r.returncode == 0 and "[notice]" in r.stderr and "debt.rs" in r.stderr, r.stderr)
    write(d, "rs/clean/src/debt.rs", "pub fn debt() -> i32 {\n    1\n}\n")
    git(d, "add", "rs/clean/src/debt.rs")
    r = commit(d)
    check("dirty-to-clean passes (HEAD becomes the stronger baseline)", r.returncode == 0, r.stderr)
    write(d, "rs/clean/src/debt.rs", RS_DEBT)
    git(d, "add", "rs/clean/src/debt.rs")
    r = commit(d)
    check("…and once formatted, regressing it refuses", r.returncode != 0, r.stderr)
    git(d, "checkout", "HEAD", "--", "rs/clean/src/debt.rs")

    # new file, unborn-like semantics, renames, copies, deletions, symlinks, odd names
    write(d, "rs/clean/src/fresh new ü.rs", RS_BAD)
    git(d, "add", "rs/clean/src/fresh new ü.rs")
    r = commit(d)
    check("a NEW unformatted .rs (odd filename) refuses and is named",
          r.returncode != 0 and "fresh new ü.rs" in r.stderr, r.stderr)
    git(d, "rm", "-q", "--cached", "rs/clean/src/fresh new ü.rs")
    (d / "rs/clean/src/fresh new ü.rs").unlink()
    git(d, "mv", "rs/clean/src/conf/a.rs", "rs/clean/src/conf/b.rs")
    git(d, "rm", "-q", "rs/clean/src/debt.rs")
    os.symlink("b.rs", d / "rs/clean/src/conf/link.rs")
    git(d, "add", "rs/clean/src/conf/link.rs")
    r = commit(d)
    check("a clean rename + a deletion + a symlink to .rs pass", r.returncode == 0, r.stderr)
    # rename of a debt file keeps its debt baseline (source path)
    write(d, "rs/clean/src/old_debt.rs", RS_DEBT)
    git(d, "add", "rs/clean/src/old_debt.rs")
    git(d, "commit", "-q", "--no-verify", "-m", "seed debt")
    git(d, "mv", "rs/clean/src/old_debt.rs", "rs/clean/src/new_debt.rs")
    r = commit(d)
    check("renamed debt keeps the rename source as its baseline (passes as debt)",
          r.returncode == 0 and "new_debt.rs" in r.stderr, r.stderr)
    shutil.copy(d / "rs/clean/src/new_debt.rs", d / "rs/clean/src/copied_debt.rs")
    git(d, "add", "rs/clean/src/copied_debt.rs")
    r = commit(d)
    check("a copy of an unformatted file is a NEW file and refuses", r.returncode != 0, r.stderr)
    git(d, "rm", "-q", "--cached", "rs/clean/src/copied_debt.rs")
    (d / "rs/clean/src/copied_debt.rs").unlink()

    # contract identities
    write(d, "rs/outer/inner/src/lib.rs", "pub async fn go() -> u8 {\n    2\n}\n")
    write(d, "rs/virtual-only/stray.rs", RS_BAD + "// still outside\n")
    git(d, "add", "rs/outer/inner/src/lib.rs", "rs/virtual-only/stray.rs")
    r = commit(d)
    check("explicit package.workspace edition resolves (async fn parses as 2021); a file under a "
          "virtual manifest is outside the contract", r.returncode == 0, r.stderr)
    write(d, "rs/clean/rustfmt.toml", "max_width = 30\n")
    git(d, "add", "rs/clean/rustfmt.toml")
    r = commit(d)
    check("a staged rustfmt.toml that un-formats UNCHANGED files refuses (contract rescan)",
          r.returncode != 0 and "formatter contract changed" in r.stderr, r.stderr)
    git(d, "rm", "-q", "--cached", "rs/clean/rustfmt.toml")
    (d / "rs/clean/rustfmt.toml").unlink()
    write(d, "rs/clean/src/two/.rustfmt.toml", "tab_spaces = 2\n")
    write(d, "rs/clean/src/two/two.rs", "pub fn two() -> u8 {\n  2\n}\n")
    git(d, "add", "rs/clean/src/two/.rustfmt.toml", "rs/clean/src/two/two.rs")
    r = commit(d)
    check("the nearest committed .rustfmt.toml governs (2-space file passes under tab_spaces=2)",
          r.returncode == 0, r.stderr)
    write(d, "rs/clean/src/two/four.rs", RS_CLEAN)
    git(d, "add", "rs/clean/src/two/four.rs")
    r = commit(d)
    check("…and a 4-space new file under it refuses", r.returncode != 0 and "four.rs" in r.stderr,
          r.stderr)
    git(d, "rm", "-q", "--cached", "rs/clean/src/two/four.rs")
    (d / "rs/clean/src/two/four.rs").unlink()
    before = head(d)
    git(d, "rm", "-q", "rs/clean/src/two/.rustfmt.toml")
    r = commit(d, "drop the 2-space config")
    check("a DELETION-ONLY commit removing a rustfmt config that un-formats unchanged files "
          "refuses (contract rescan runs with no staged regular file)",
          r.returncode != 0 and head(d) == before and "two.rs" in r.stderr
          and "formatter contract changed" in r.stderr, r.stderr)
    restore(d, before, "rs/clean/src/two/.rustfmt.toml")

    # gitlink and stdin --check false-zero
    sub = TMP / "sub-src"
    sub.mkdir()
    git(sub, "init", "-q", "-b", "main")
    write(sub, "x.rs", RS_BAD)
    git(sub, "add", "x.rs")
    git(sub, "commit", "-q", "--no-verify", "-m", "s")
    git(d, "update-index", "--add", "--cacheinfo", f"160000,{head(sub)},rs/clean/vendored")
    r = commit(d)
    check("a gitlink is not a formatting subject", r.returncode == 0, r.stderr)
    proc = subprocess.run([shutil.which("rustup"), "run", "1.96.1", "rustfmt", "--check"],
                          input=b"fn main(){let a=1;}\n", capture_output=True)
    m = epr_format.Rustfmt("1.96.1").measure(
        epr_format.DiskTree(d), "rs/clean/src/lib.rs", b"fn main(){let a=1;}\n",
        epr_format.RustContract("2021", None, "x"))
    check(f"byte comparison catches what `rustfmt --check` on stdin reports as exit "
          f"{proc.returncode}", m.state == "unformatted", m.detail)

    # unborn HEAD
    u = TMP / "unborn"
    u.mkdir()
    git(u, "init", "-q", "-b", "main")
    for rel in (".epr-meta/manifest.md", ".claude/epr-meta/policies.yaml",
                "genesis/agentic/pool-policy.json", "rs/Cargo.toml", "rs/clean/Cargo.toml"):
        write(u, rel, (TEMPLATE / rel).read_text())
    os.symlink(REPO / ".claude/scripts", u / ".claude/scripts")
    shutil.copytree(TEMPLATE / ".husky", u / ".husky")
    git(u, "config", "core.hooksPath", str(u / ".husky"))
    write(u, "rs/clean/src/lib.rs", RS_BAD)
    git(u, "add", "-A")
    r = commit(u)
    check("unborn HEAD: every staged file is new, an unformatted one refuses",
          r.returncode != 0 and "new file" in r.stderr, r.stderr)
    write(u, "rs/clean/src/lib.rs", RS_CLEAN)
    git(u, "add", "-A")
    r = commit(u)
    check("unborn HEAD: formatted root commit passes", r.returncode == 0, r.stderr)

    # missing tools
    d2 = fixture("rust-missing")
    write(d2, "rs/clean/src/lib.rs", RS_CLEAN + "// touched\n")
    git(d2, "add", "rs/clean/src/lib.rs")
    nopath = env_with(PATH=path_without(RUSTUP_DIR))
    r = commit(d2, "m", nopath)
    check("missing rustfmt REFUSES with the install cure",
          r.returncode != 0 and "rustup component add rustfmt --toolchain 1.96.1" in r.stderr,
          r.stderr)
    r = commit(d2, "m", env_with(PATH=path_without(RUSTUP_DIR), EPR_META_ACK="1"))
    check("…and EPR_META_ACK=1 acknowledges a missing tool (ask-class, witnessed)",
          r.returncode == 0, r.stderr)
    ledger = d2 / ".claude/data/governance-findings.jsonl"
    rows = [json.loads(x) for x in ledger.read_text().splitlines()] if ledger.is_file() else []
    check("the acknowledged refusal is witnessed with its policy and acked reason",
          any(r_.get("gate") == "git-gate-format" and (r_.get("refer") or {}).get("reason") == "acked"
              for r_ in rows), json.dumps(rows[-2:]))

    # native permit cannot clear a formatting deny
    stub = TMP / "native-permit.sh"
    stub.write_text('#!/bin/sh\ncat >/dev/null\necho \'{"decision":"permit","winningClass":null,'
                    '"ruleId":null,"reason":"stub","verdicts":[],"evaluator":{"id":"stub"}}\'\n')
    os.chmod(stub, 0o755)
    write(d2, "rs/clean/src/lib.rs", RS_BAD)
    git(d2, "add", "rs/clean/src/lib.rs")
    r = commit(d2, "n", env_with(EPR_BIN=str(stub)))
    check("a native PERMIT never clears a formatting deny (and records no disagreement)",
          r.returncode != 0 and "DISAGREEMENT" not in r.stderr, r.stderr)
else:
    print("  ⚠ SKIP rustfmt cases: rustup not on PATH")

# ════════════════════════════════════════════════════════════════════════════════════════════════
print("Prettier ratchet (ESM and CJS packages)")
if HAVE_NODE and PRETTIER:
    d = fixture("prettier")
    write(d, "esm/src/new.ts", TS_BAD)
    git(d, "add", "esm/src/new.ts")
    r = commit(d)
    check("RED: a NEW unformatted .ts refuses (ESM `export default` config)",
          r.returncode != 0 and "esm/src/new.ts is a new file" in r.stderr
          and "pnpm exec prettier --write" in r.stderr, r.stderr)
    write(d, "esm/src/new.ts", TS_CLEAN)
    git(d, "add", "esm/src/new.ts")
    r = commit(d)
    check("GREEN: formatted and re-staged it passes", r.returncode == 0, r.stderr)
    write(d, "esm/frozen.ts", TS_BAD)
    write(d, "cjs/scripts/outside.ts", TS_BAD + "// still outside src/**\n")
    write(d, "docs/readme.md", "# no   contract   here\n")
    git(d, "add", "esm/frozen.ts", "cjs/scripts/outside.ts", "docs/readme.md")
    r = commit(d)
    check("a frozen .prettierignore entry, a path outside format:check globs, and markdown with "
          "no owning contract all pass", r.returncode == 0, r.stderr)
    write(d, "cjs/src/new.ts", "export function g() {\n  return 'y';\n}\n")
    git(d, "add", "cjs/src/new.ts")
    r = commit(d)
    check("CJS `module.exports` config + .editorconfig (tabs) is honoured: 2-space file refuses",
          r.returncode != 0 and "cjs/src/new.ts" in r.stderr, r.stderr)
    write(d, "cjs/src/new.ts", "export function g() {\n\treturn 'y';\n}\n")
    git(d, "add", "cjs/src/new.ts")
    r = commit(d)
    check("…and the tab-indented file passes", r.returncode == 0, r.stderr)
    write(d, "esm/.prettierrc.js", "export default {\n  singleQuote: false,\n};\n")
    git(d, "add", "esm/.prettierrc.js")
    r = commit(d)
    check("config-only drift: a staged Prettier config that un-formats unchanged files refuses",
          r.returncode != 0 and "formatter contract changed" in r.stderr, r.stderr)
    git(d, "checkout", "HEAD", "--", "esm/.prettierrc.js")
    write(d, "esm/.prettierrc.js", "export default {\n  singleQuote: true,\n  printWidth: 100,\n};\n")
    git(d, "add", "esm/.prettierrc.js")
    before = head(d)
    git(d, "rm", "-q", "esm/.prettierrc.js")
    r = commit(d, "drop the esm config")
    check("a DELETION-ONLY commit removing a Prettier config that un-formats unchanged files refuses",
          r.returncode != 0 and head(d) == before and "esm/src/ok.ts" in r.stderr, r.stderr)
    restore(d, before, "esm/.prettierrc.js")

    # explicit --config / --ignore-path (app/elohim-elements/elohim-core's script, verbatim shape)
    write(d, "elem/core/src/new.ts", TS_CLEAN.replace("answer", "fresh"))
    git(d, "add", "elem/core/src/new.ts")
    r = commit(d, "elem clean")
    check("GREEN: `--config ../.prettierrc.js --ignore-path ../.prettierignore` is modelled — a "
          "single-quoted file formatted under the NAMED config passes (no `does not model` refusal)",
          r.returncode == 0 and "does not model" not in r.stderr, r.stderr)
    write(d, "elem/core/src/bad.ts", TS_CLEAN.replace("'yes'", '"yes"'))
    git(d, "add", "elem/core/src/bad.ts")
    r = commit(d, "elem bad")
    check("RED: a double-quoted file refuses under the named config (the nearer decoy config is "
          "not discovered) and the cure carries the package's flags",
          r.returncode != 0 and "elem/core/src/bad.ts is a new file" in r.stderr
          and "--config ../.prettierrc.js --ignore-path ../.prettierignore --write" in r.stderr,
          r.stderr)
    git(d, "rm", "-q", "--cached", "--ignore-unmatch", "elem/core/src/bad.ts")
    if "bad.ts" in git(d, "ls-files", "elem/core/src").stdout:
        git(d, "rm", "-q", "elem/core/src/bad.ts")
        git(d, "commit", "-q", "--no-verify", "-m", "drop wrongly admitted bad.ts")
    write(d, "elem/core/src/skipped.ts", TS_BAD)
    git(d, "add", "elem/core/src/skipped.ts")
    r = commit(d, "elem skipped")
    check("a file the `--ignore-path` file ignores passes unformatted", r.returncode == 0, r.stderr)
    write(d, "elem/core/src/default-ignored.ts", TS_BAD)
    git(d, "add", "elem/core/src/default-ignored.ts")
    r = commit(d, "elem default-ignored")
    check("`--ignore-path` REPLACES the package's default .prettierignore (as the CLI does): a file "
          "only the default ignore names is checked and refuses",
          r.returncode != 0 and "default-ignored.ts" in r.stderr, r.stderr)
    if "default-ignored.ts" in git(d, "ls-files", "elem/core/src").stdout:
        git(d, "rm", "-q", "--cached", "elem/core/src/default-ignored.ts")
        if "default-ignored.ts" in git(d, "ls-tree", "-r", "--name-only", "HEAD").stdout:
            git(d, "commit", "-q", "--no-verify", "-m", "drop wrongly admitted file")
    before = head(d)
    write(d, "elem/.prettierrc.js", "module.exports = {\n  singleQuote: false,\n};\n")
    git(d, "add", "elem/.prettierrc.js")
    r = commit(d, "flip the named config")
    check("a staged change to the `--config` file un-formatting unchanged files refuses (rescan)",
          r.returncode != 0 and "elem/core/src/ok.ts" in r.stderr
          and "formatter contract changed" in r.stderr, r.stderr)
    restore(d, before, "elem/.prettierrc.js")

    for deleted in (False, True):
        p = fixture(f"explicit-config-rescan-{deleted}")
        package = json.loads((p / "elem/core/package.json").read_text())
        package["scripts"]["format:check"] = (
            'prettier --config ../style.cjs --check "src/**/*.ts"')
        write(p, "elem/core/package.json", json.dumps(package) + "\n")
        write(p, "elem/style.cjs", "module.exports = { singleQuote: true };\n")
        git(p, "add", "elem/core/package.json", "elem/style.cjs")
        git(p, "commit", "-q", "--no-verify", "-m", "custom parent config baseline")
        before = head(p)
        if deleted:
            git(p, "rm", "-q", "elem/style.cjs")
        else:
            write(p, "elem/style.cjs", "module.exports = { singleQuote: false };\n")
            git(p, "add", "elem/style.cjs")
        r = commit(p, "change only the named parent config")
        check(f"explicit parent config with a custom name, deleted={deleted}: unchanged "
              "governed files are rescanned and refuse",
              r.returncode != 0 and head(p) == before and "elem/core/src/ok.ts" in r.stderr
              and (deleted or "formatter contract changed" in r.stderr), r.stderr)

    for ignore in (".prettierignore", "format-excludes"):
        for deleted in (False, True):
            p = fixture(f"explicit-ignore-rescan-{ignore}-{deleted}")
            package = json.loads((p / "elem/core/package.json").read_text())
            package["scripts"]["format:check"] = (
                f'prettier --config ../.prettierrc.js --ignore-path ../{ignore} '
                '--check "src/**/*.ts"')
            write(p, "elem/core/package.json", json.dumps(package) + "\n")
            write(p, f"elem/{ignore}", "**/ok.ts\n")
            git(p, "add", "elem/core/package.json", f"elem/{ignore}")
            git(p, "commit", "-q", "--no-verify", "-m", "parent ignore baseline")
            if deleted:
                git(p, "rm", "-q", f"elem/{ignore}")
            else:
                write(p, f"elem/{ignore}", "")
                git(p, "add", f"elem/{ignore}")
            with epr_format.StagedFormatter(p, epr_format.Git(p, env=BASE_ENV)) as sf:
                changes = epr_format.staged_changes(sf.git, sf.head)
                sf.prefetch_closure(changes)
                rescans = sf.contract_rescans(changes)
            check(f"explicit parent ignore {ignore}, deleted={deleted}: unchanged governed "
                  "files are selected for rescan", "elem/core/src/ok.ts" in
                  {c.path for c in rescans}, repr(rescans))

    for adding in (False, True):
        p = fixture(f"explicit-reference-{adding}")
        package = json.loads((p / "elem/core/package.json").read_text())
        scripts = ['prettier --check "src/**/*.ts"',
                   'prettier --config ../style.cjs --check "src/**/*.ts"']
        package["scripts"]["format:check"] = scripts[not adding]
        write(p, "elem/core/package.json", json.dumps(package) + "\n")
        write(p, "elem/style.cjs", "module.exports = { singleQuote: true };\n")
        git(p, "add", "elem/core/package.json", "elem/style.cjs")
        git(p, "commit", "-q", "--no-verify", "-m", "reference baseline")
        package["scripts"]["format:check"] = scripts[adding]
        write(p, "elem/core/package.json", json.dumps(package) + "\n")
        write(p, "elem/style.cjs", "module.exports = { singleQuote: false };\n")
        git(p, "add", "elem/core/package.json", "elem/style.cjs")
        with epr_format.StagedFormatter(p, epr_format.Git(p, env=BASE_ENV)) as sf:
            # Isolate dependency discovery from package.json's own syntactic rescan root.
            changes = [c for c in epr_format.staged_changes(sf.git, sf.head)
                       if c.path == "elem/style.cjs"]
            sf.prefetch_closure(changes)
            rescans = sf.contract_rescans(changes)
        check(f"explicit reference added={adding}: dependencies are read from index AND base",
              "elem/core/src/ok.ts" in {c.path for c in rescans}, repr(rescans))

    for nearby in ('{broken json\n', '{"plugins": ["prettier-plugin-not-installed"]}\n'):
        p = fixture("explicit-config-nearby-" + ("syntax" if "broken" in nearby else "plugin"))
        write(p, "elem/core/src/.prettierrc", nearby)
        git(p, "add", "elem/core/src/.prettierrc")
        git(p, "commit", "-q", "--no-verify", "-m", "broken nearby config baseline")
        write(p, "elem/core/src/new.ts", TS_CLEAN)
        git(p, "add", "elem/core/src/new.ts")
        r = commit(p, "explicit config bypasses nearby config")
        check(f"an explicit --config ignores a broken nearby config ({nearby.strip()})",
              r.returncode == 0, r.stderr)

    p = fixture("explicit-config-plugin-parser")
    package = json.loads((p / "elem/core/package.json").read_text())
    package["scripts"]["format:check"] = (
        'prettier --config ../.prettierrc.js --check "src/**/*.custom"')
    write(p, "elem/core/package.json", json.dumps(package) + "\n")
    write(p, "elem/.prettierrc.js", """module.exports = {
  plugins: [{
    languages: [{ name: 'custom', parsers: ['custom'], extensions: ['.custom'] }],
    parsers: { custom: {
      parse: (text) => text, astFormat: 'custom', locStart: () => 0,
      locEnd: (text) => text.length,
    } },
    printers: { custom: { print: (path) => path.getValue().trim() + '\\n' } },
  }],
};
""")
    git(p, "add", "elem/core/package.json", "elem/.prettierrc.js")
    git(p, "commit", "-q", "--no-verify", "-m", "explicit parser plugin baseline")
    write(p, "elem/core/src/new.custom", "hello   \n")
    git(p, "add", "elem/core/src/new.custom")
    before = head(p)
    r = commit(p)
    check("parser inference uses plugins from the explicit config: an unformatted custom "
          "extension refuses instead of being outside",
          r.returncode != 0 and head(p) == before
          and "elem/core/src/new.custom is a new file" in r.stderr, r.stderr)
    write(p, "elem/core/src/new.custom", "hello\n")
    git(p, "add", "elem/core/src/new.custom")
    r = commit(p)
    check("the custom extension passes once formatted with the explicit config's plugin",
          r.returncode == 0, r.stderr)

    # merges: clean automatic merge runs pre-merge-commit; conflicted merge runs pre-commit
    git(d, "switch", "-q", "-c", "side")
    write(d, "esm/src/side.ts", TS_BAD)
    git(d, "add", "esm/src/side.ts")
    git(d, "commit", "-q", "--no-verify", "-m", "side brings drift")
    git(d, "switch", "-q", "main")
    write(d, "esm/src/main.ts", TS_CLEAN)
    git(d, "add", "esm/src/main.ts")
    git(d, "commit", "-q", "-m", "main moves")
    before = head(d)
    r = git(d, "merge", "--no-ff", "-m", "merge side", "side", ok=False)
    check("RED: a conflict-free `git merge --no-ff` bringing an unformatted file refuses "
          "(pre-merge-commit)", r.returncode != 0 and head(d) == before
          and "esm/src/side.ts" in (r.stderr + r.stdout), r.stderr + r.stdout)
    write(d, "esm/src/side.ts", TS_CLEAN)
    git(d, "add", "esm/src/side.ts")
    r = commit(d, "merge side (formatted)")
    check("GREEN: formatting the merged index and `git commit` completes the merge",
          r.returncode == 0 and len(git(d, "rev-list", "--parents", "-1", "HEAD").stdout.split()) == 3,
          r.stderr)
    git(d, "switch", "-q", "-c", "conflict")
    write(d, "esm/src/ok.ts", "export const answer = 'side';\n")
    git(d, "commit", "-q", "-am", "side edit")
    git(d, "switch", "-q", "main")
    write(d, "esm/src/ok.ts", "export const answer = 'main';\n")
    git(d, "commit", "-q", "-am", "main edit")
    git(d, "merge", "conflict", ok=False)
    write(d, "esm/src/ok.ts", 'export const answer  = "both"\n')
    git(d, "add", "esm/src/ok.ts")
    r = commit(d, "resolved")
    check("a manually resolved merge with unformatted resolution refuses (pre-commit)",
          r.returncode != 0, r.stderr)
    write(d, "esm/src/ok.ts", "export const answer = 'both';\n")
    git(d, "add", "esm/src/ok.ts")
    r = commit(d, "resolved")
    check("…formatted resolution commits", r.returncode == 0, r.stderr)

    # linked worktree + husky shims
    h = fixture("husky", hooks="husky")
    wt = TMP / "husky-wt"
    git(h, "worktree", "add", "-q", str(wt))
    for pkg in ("esm", "cjs"):
        (wt / pkg / "node_modules").mkdir()
        os.symlink(PRETTIER, wt / pkg / "node_modules" / "prettier")
    write(wt, "esm/src/wt.ts", TS_BAD)
    git(wt, "add", "esm/src/wt.ts")
    r = commit(wt)
    check("a linked worktree through husky shims runs the gate against ITS index",
          r.returncode != 0 and "esm/src/wt.ts" in r.stderr, r.stderr + r.stdout)
    write(wt, "esm/src/wt.ts", TS_CLEAN)
    git(wt, "add", "esm/src/wt.ts")
    r = commit(wt)
    check("…and passes once formatted", r.returncode == 0, r.stderr)

    # missing Prettier / Node
    p = fixture("prettier-missing", prettier=False)
    write(p, "esm/src/n.ts", TS_CLEAN)
    git(p, "add", "esm/src/n.ts")
    r = commit(p)
    check("missing Prettier in the package REFUSES naming `pnpm install --frozen-lockfile`",
          r.returncode != 0 and "pnpm install --frozen-lockfile" in r.stderr, r.stderr)
    p2 = fixture("node-missing")
    write(p2, "esm/src/n.ts", TS_CLEAN)
    git(p2, "add", "esm/src/n.ts")
    r = commit(p2, "n", env_with(PATH=path_without(NODE_DIR)))
    check("missing Node REFUSES with its cure", r.returncode != 0 and "node is not on PATH" in r.stderr,
          r.stderr)
else:
    print("  ⚠ SKIP Prettier cases: node or an installed prettier is unavailable")

# ════════════════════════════════════════════════════════════════════════════════════════════════
print("A missing formatter never discards what another formatter measured")
if HAVE_RUSTFMT:
    m = fixture("mixed-missing", prettier=False)
    before = head(m)
    write(m, "rs/clean/src/lib.rs", "pub mod absent_child;\n" + RS_BAD)
    write(m, "esm/src/n.ts", TS_CLEAN)
    git(m, "add", "rs/clean/src/lib.rs", "esm/src/n.ts")
    r = commit(m, "mixed", env_with(EPR_META_ACK="1"))
    check("RED→GREEN: Prettier absent + measured Rust drift — EPR_META_ACK=1 still refuses, naming "
          "the Rust drift, and the Prettier file is reported unmeasured with its cure",
          r.returncode != 0 and head(m) == before
          and "rs/clean/src/lib.rs was rustfmt-formatted at HEAD" in r.stderr
          and "pnpm install --frozen-lockfile" in r.stderr, r.stderr)
    write(m, "rs/clean/src/lib.rs", "pub mod absent_child;\n" + RS_CLEAN + "// fine\n")
    git(m, "add", "rs/clean/src/lib.rs")
    r = commit(m, "mixed-ok", env_with(EPR_META_ACK="1"))
    check("…and with the Rust file formatted, EPR_META_ACK=1 acknowledges only the unmeasured "
          "Prettier file", r.returncode == 0 and head(m) != before, r.stderr)
else:
    print("  ⚠ SKIP mixed missing-tool case: rustup not on PATH")

# ════════════════════════════════════════════════════════════════════════════════════════════════
print("Failure modes refuse; nothing degrades to success")
d = fixture("failures")
write(d, "rs/clean/src/lib.rs", RS_CLEAN + "// t\n")
git(d, "add", "rs/clean/src/lib.rs")
pybin = TMP / "py-noyaml"
pybin.mkdir()
(pybin / "python3").write_text(f'#!/bin/sh\nexec {sys.executable} -S "$@"\n')
os.chmod(pybin / "python3", 0o755)
r = commit(d, "y", env_with(PATH=f"{pybin}:{BASE_ENV['PATH']}"))
check("Python without PyYAML refuses (the policy is never resolved to an empty rule set)",
      r.returncode != 0 and "PyYAML" in r.stderr, r.stderr)
write(d, "rs/clean/Cargo.toml", "[package\nname = broken\n")
git(d, "add", "rs/clean/Cargo.toml")
r = commit(d)
check("a malformed Cargo.toml in the closure refuses as unmeasured, not exempt",
      r.returncode != 0 and "UNMEASURED" in r.stderr, r.stderr)
git(d, "checkout", "HEAD", "--", "rs/clean/Cargo.toml")
write(d, "rs/clean/src/lib.rs", "pub fn broken( {\n")
git(d, "add", "rs/clean/src/lib.rs")
r = commit(d)
check("a parse failure of the staged blob refuses (unmeasured)", r.returncode != 0
      and "could not measure the staged blob" in r.stderr, r.stderr)
git(d, "checkout", "HEAD", "--", "rs/clean/src/lib.rs")

# in-process: timeout, worker crash, concurrent index change, selected-without-result
fenv = {k: v for k, v in BASE_ENV.items()}
write(d, "rs/clean/src/lib.rs", RS_CLEAN + "// t2\n")
write(d, "esm/src/x.ts", TS_CLEAN)
git(d, "add", "rs/clean/src/lib.rs", "esm/src/x.ts")


def phase():
    return epr_meta_git.format_phase(d, git=epr_format.Git(d, env=fenv))


saved = (epr_format.RUSTFMT_TIMEOUT_S, epr_format.WORKER, epr_format.StagedFormatter.judge)
try:
    if HAVE_RUSTFMT:
        epr_format.RUSTFMT_TIMEOUT_S = 0.000001
        v, _n, _s = phase()
        check("a formatter timeout is unmeasured (ask), never success",
              any(x[1].cls == "ask" and "timed out" in x[1].reason for x in v), str(v))
        epr_format.RUSTFMT_TIMEOUT_S = saved[0]
    if HAVE_NODE:
        crash = TMP / "crash.mjs"
        crash.write_text("process.exit(3);\n")
        epr_format.WORKER = crash
        v, _n, _s = phase()
        check("a Prettier worker crash is unmeasured (ask), never success",
              any(x[0] == "esm/src/x.ts" and x[1].cls == "ask" for x in v), str(v))
        epr_format.WORKER = saved[1]

    def racing_judge(self, changes):
        write(d, "docs/race.md", "raced\n")
        git(d, "add", "docs/race.md")
        return saved[2](self, changes)

    epr_format.StagedFormatter.judge = racing_judge
    v, _n, _s = phase()
    check("a concurrent index change refuses with a retry (deny)",
          any(x[1].cls == "deny" and "re-run the commit" in x[1].reason for x in v), str(v))
    epr_format.StagedFormatter.judge = lambda self, changes: {}
    v, _n, _s = phase()
    check("a selected file with no completed result is an enforcement error (deny)",
          any(x[1].cls == "deny" and "no result came back" in x[1].reason for x in v), str(v))
finally:
    epr_format.RUSTFMT_TIMEOUT_S, epr_format.WORKER, epr_format.StagedFormatter.judge = saved

u2 = TMP / "unmerged"
u2.mkdir()
git(u2, "init", "-q", "-b", "main")
write(u2, "a.ts", "x\n")
git(u2, "add", "a.ts")
git(u2, "commit", "-q", "--no-verify", "-m", "a")
blob = git(u2, "rev-parse", "HEAD:a.ts").stdout.strip()
subprocess.run(["git", "update-index", "--index-info"], cwd=u2, env=BASE_ENV, text=True,
               input=f"0 {'0' * 40}\ta.ts\n100644 {blob} 1\ta.ts\n100644 {blob} 2\ta.ts\n",
               check=True)
try:
    epr_format.staged_changes(epr_format.Git(u2, env=BASE_ENV), head(u2))
    check("unresolved index entries refuse", False)
except epr_format.EnforcementError as e:
    check("unresolved index entries refuse", "unresolved index entries" in str(e), str(e))

# missing gate file: the wrapper keeps its fail-open posture, but says so
g = fixture("no-gate")
os.unlink(g / ".claude/scripts")
write(g, "rs/clean/src/lib.rs", RS_BAD)
git(g, "add", "rs/clean/src/lib.rs")
r = commit(g)
check("an absent gate script is fail-open in the wrapper (task ruling) but prints UNGOVERNED",
      r.returncode == 0 and "UNGOVERNED" in r.stderr, r.stderr)
r = git(g, "commit", "--allow-empty", "-q", "-m", "h", env=env_with(HUSKY="0"), ok=False)
check("HUSKY=0 still bypasses the hook", r.returncode == 0, r.stderr)

# ════════════════════════════════════════════════════════════════════════════════════════════════
print("Outside the staged adapter the validator is not applicable")
check("_staged_format with no commit evidence returns None (never 'checked')",
      epr_meta._staged_format({"path": "x.rs", "content": RS_BAD}) is None)
real = REPO / "elohim/eprfs/epr-cli/src/flow/scope.rs"
if real.is_file():
    v = epr_meta_git.verdict_for(str(real), epr_meta_git.build_write(str(real), "M", RS_BAD, True))
    check("the resolver/range path over a real repo file fires no formatting verdict",
          v is None or v.rule_id != "commit-format-ratchet", str(v))
reg = json.loads((REPO / "elohim/sdk/schemas/v1/registries/governance-validators.json").read_text())
check("the shared validator registry scopes the staged-format validator to python",
      any(x.get("ref") == epr_format.STAGED_FORMAT_VALIDATOR and x.get("runtime") == "python"
          for x in reg["validators"]))
pols, errs = epr_meta.load_policies(REPO)
pol = pols.get("commit-format-ratchet@1") or {}
check("commit-format-ratchet@1 loads clean, pinned, deny, operator-established",
      pol.get("class") == "deny" and pol.get("validator") == epr_format.STAGED_FORMAT_VALIDATOR
      and str(pol.get("established_by", "")).startswith("operator-")
      and not [e for e in errs if "commit-format-ratchet" in e], str(errs))

# coverage: every independent cascade root binds the ratchet (fixtures excepted)
listing = subprocess.run(["git", "ls-files", "-z"], cwd=REPO, env=BASE_ENV, capture_output=True,
                         text=True).stdout.split("\0")
roots, unbound = [], []
for rel in listing:
    if not rel or "/__tests__/fixtures/" in rel:
        continue
    if not (rel.endswith("/.epr-meta") or rel == ".epr-meta" or rel.endswith(".epr-meta/manifest.md")):
        continue
    cfg = epr_meta.load_meta(REPO / rel)
    if cfg.get("root") is True:
        roots.append(rel)
        if not any(r_.get("policy") == "commit-format-ratchet@1" for r_ in cfg.get("rules") or []):
            unbound.append(rel)
check(f"every independent cascade root ({len(roots)}) binds commit-format-ratchet@1",
      len(roots) >= 5 and not unbound, f"unbound: {unbound}")

# ════════════════════════════════════════════════════════════════════════════════════════════════
print("Edit-time advisory routing (lint-check.py, worktree bytes, never blocking)")
LINT = REPO / ".claude/hooks/lint-check.py"
a = fixture("advisory")


def lint(path):
    r = run(a, sys.executable, str(LINT), env=env_with(CLAUDE_PROJECT_DIR=str(a)),
            input=json.dumps({"tool_input": {"file_path": str(path)}}))
    ctx = json.loads(r.stdout)["hookSpecificOutput"]["additionalContext"] if r.stdout.strip() else ""
    return r.returncode, ctx


if HAVE_RUSTFMT:
    write(a, "rs/clean/src/lib.rs", RS_BAD)
    code, ctx = lint(a / "rs/clean/src/lib.rs")
    check("an unformatted .rs OUTSIDE doorway/ now gets a [Format] advisory (exit 0)",
          code == 0 and "[Format] lib.rs" in ctx and "rustfmt" in ctx, ctx)
    write(a, "rs/clean/src/lib.rs", RS_CLEAN)
    code, ctx = lint(a / "rs/clean/src/lib.rs")
    check("a formatted .rs gets no advisory", code == 0 and "[Format]" not in ctx, ctx)
if HAVE_NODE and PRETTIER:
    write(a, "esm/src/ok.ts", TS_BAD)
    code, ctx = lint(a / "esm/src/ok.ts")
    check("an unformatted .ts in a format:check package gets a [Format] advisory",
          code == 0 and "[Format] ok.ts" in ctx and "prettier" in ctx, ctx)
code, ctx = lint(a / "docs/readme.md")
check("a file outside every contract gets nothing and exits 0", code == 0 and ctx == "", ctx)
check("clippy is gone from the edit path", "cargo', 'clippy" not in LINT.read_text())

# ════════════════════════════════════════════════════════════════════════════════════════════════
print("Cost: a 30-file mixed staged set")
if HAVE_RUSTFMT and HAVE_NODE and PRETTIER:
    c = fixture("cost")
    for i in range(15):
        write(c, f"rs/clean/src/gen{i}.rs", RS_CLEAN.replace("add", f"add{i}"))
        write(c, f"esm/src/gen{i}.ts", TS_CLEAN.replace("answer", f"answer{i}"))
    git(c, "add", "-A")
    gitc = epr_format.Git(c, env=BASE_ENV)
    t0 = time.monotonic()
    v, notes, stats = epr_meta_git.format_phase(c, git=gitc)
    dt = time.monotonic() - t0
    print(f"     30 files: {dt:.2f}s, stats {stats}")
    check("30 staged files: one Prettier worker, bounded git launches, under the 30 s ceiling",
          not v and stats.get("prettier_workers") == 1 and stats.get("git_calls", 99) <= 15
          and dt < 30, f"{dt:.2f}s {stats} {v}")

print("Cost: a configuration rescan across many directories")
if HAVE_NODE and PRETTIER:
    c = fixture("rescan-cost")
    for i in range(80):
        write(c, f"cjs/src/d{i}/x{i}.ts", f"export const v{i} = 'x';\n")
    git(c, "add", "-A")
    git(c, "commit", "-q", "--no-verify", "-m", "eighty directories")
    write(c, "package.json", json.dumps({"name": "root", "private": True}, indent=2) + "\n")
    git(c, "add", "package.json")
    gitc = epr_format.Git(c, env=BASE_ENV)
    t0 = time.monotonic()
    v, notes, stats = epr_meta_git.format_phase(c, git=gitc)
    dt = time.monotonic() - t0
    print(f"     root package.json over ~90 dirs: {dt:.2f}s, stats {stats}")
    check("a root package.json change rescans every directory with bounded git launches "
          "(one listing + one batch read per side, not one launch per directory)",
          not v and gitc.calls <= 20, f"{dt:.2f}s git_calls={gitc.calls} {stats} {v}")

# ════════════════════════════════════════════════════════════════════════════════════════════════
print("This repository's declared contracts resolve (worktree bytes)")
wt = epr_format.DiskTree(REPO)
core = "app/elohim-elements/elohim-core/src/epr-format-probe.ts"
if (REPO / "app/elohim-elements/elohim-core/package.json").is_file():
    try:
        c_core = epr_format.contract_for(wt, core)
    except epr_format.ContractError as e:
        c_core = e
    check("elohim-core's `--config ../.prettierrc.js --ignore-path ../.prettierignore` resolves "
          "to a Prettier contract naming both files",
          isinstance(c_core, epr_format.PrettierContract)
          and c_core.config == "app/elohim-elements/.prettierrc.js"
          and c_core.ignore_paths == ("app/elohim-elements/.prettierignore",), repr(c_core))
lib = "app/elohim-library/projects/lamad-ui/src/epr-format-probe.ts"
if (REPO / "app/elohim-library/package.json").is_file():
    got = {rel: epr_format.contract_for(wt, rel) for rel in (
        lib, "app/elohim-library/projects/elohim-service/src/x.ts",
        "app/elohim-library/projects/html5-app-plugin/src/x.ts",
        "app/elohim-library/projects/lamad-ui/src/x.html",
        "app/elohim-library/projects/lamad-ui/src/x.spec.ts",
        "app/elohim-library/projects/perseus-plugin/src/x.ts",
        "app/elohim-library/src/x.ts")}
    want_in = [lib, "app/elohim-library/projects/elohim-service/src/x.ts",
               "app/elohim-library/projects/html5-app-plugin/src/x.ts",
               "app/elohim-library/projects/lamad-ui/src/x.html"]
    check("elohim-library's ESLint `prettier/prettier` scope is a declared Prettier contract "
          "(lamad-ui, elohim-service, html5-app-plugin TS; lamad-ui HTML)",
          all(isinstance(got[r], epr_format.PrettierContract) for r in want_in), repr(got))
    check("…and what that ESLint scope ignores (spec files, perseus-plugin, the root src/) stays "
          "outside", all(isinstance(got[r], epr_format.Outside) for r in got if r not in want_in),
          repr(got))
check("negated globs follow the Prettier CLI (`!` excludes)",
      epr_format.glob_match(("src/**/*.ts", "!**/*.spec.ts"), "src/a/b.ts")
      and not epr_format.glob_match(("src/**/*.ts", "!**/*.spec.ts"), "src/a/b.spec.ts"))
pc = epr_format.parse_prettier_check(
    'prettier --config=../c.js --ignore-path a --ignore-path b --check "src/**/*.ts"')
check("`--flag=value` and repeated `--ignore-path` parse",
      pc is not None and pc.config == "../c.js" and pc.ignore_paths == ("a", "b"), repr(pc))
for bad in ('prettier --check --cache "src/**/*.ts"', 'prettier --check --config'):
    try:
        epr_format.parse_prettier_check(bad)
        check(f"an unmodelled script refuses: {bad}", False)
    except epr_format.ContractError:
        check(f"an unmodelled script refuses: {bad}", True)

shutil.rmtree(TMP, ignore_errors=True)
print(f"\n{_passed} checks passed, {len(_failures)} failed")
if _failures:
    for f in _failures:
        print(f"  FAILED: {f}")
    sys.exit(1)
