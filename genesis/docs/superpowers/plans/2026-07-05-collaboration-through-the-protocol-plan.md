---
id: "plan-collaboration-through-the-protocol"
title: "Collaboration Through the Protocol — harness-agnostic .epr-meta gate + feat/eprfs integration"
status: Draft
cites:
  - epr-meta-compose-gate | `.epr-meta` | sha256:6052ce071bfec509 | path: genesis/docs/superpowers/specs/2026-06-25-epr-meta-compose-gate-design.md
  - epr-meta-policy-registry-measure | `.epr-meta` Policy Registry + Measure Tier | sha256:474eee1686e3123b | path: genesis/docs/superpowers/specs/2026-07-02-epr-meta-policy-registry-measure-design.md
  - ci-detection-convergence-epr-meta-fold | CI Change-Detection Convergence + `.ci-ignore`→`.epr-meta` Fold (P6) | sha256:c2c141e379cb5672 | path: genesis/docs/superpowers/specs/2026-06-25-ci-detection-convergence-epr-meta-fold-design.md
  - epr-meta-compose-gate-plan | `.epr-meta` Compose-Gate | sha256:7f6d5e5c6f8351e6 | path: genesis/docs/superpowers/plans/2026-06-25-epr-meta-compose-gate.md
  - .claude/hooks/epr-meta-resolver.py
  - .claude/scripts/_lib/epr_meta.py
domain: D-governance
sprint: testable-now
# no requires_env — pure tooling (Python stdlib) + a native Rust workspace build; nothing needs shem/alpha/harbor
---

# Collaboration Through the Protocol — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the `.epr-meta` compose-gate govern *every* agent's commit — Codex, Gemini, a human, or Claude driving `git` via Bash — the same way `fmt`/`clippy` already do, by binding the existing pure rule engine to the harness-agnostic git-hook layer; and integrate the Codex-authored `feat/eprfs` projection workspace as the first branch to land *through* that governed substrate.

**Architecture:** The move is NOT a port. All governance logic already lives in the pure, harness-neutral library `.claude/scripts/_lib/epr_meta.py` (721 lines, stdlib+PyYAML, zero Claude coupling), already imported by three non-Claude callers. The Claude PreToolUse hook is one thin I/O adapter over it. This plan adds a **fourth adapter** — a git-invoked evaluator — plus a `.husky/pre-commit` that calls it over staged files, and a CI backstop that calls it over the PR diff. Same engine, same verdicts, new invocation surface. This is the endpoint the code *already declares* (`.claude/hooks/.epr-meta`: "hooks stay THIN … so the transport can be swapped without losing the rules") and that the `2026-07-02` policy-registry design *already models* ("define-once-bind-many … edit-time gate and census can never disagree: same registry, same expansion"). We compose from that design; we do not fork it.

**Tech Stack:** Python 3 (stdlib + PyYAML soft-dep), Bash (husky v9, `core.hooksPath=.husky/_`), git plumbing (`git diff --cached`, `git show :path`, `git ls-tree`), Rust (native, `RUSTFLAGS=""`) for the eprfs workspace.

## Global Constraints

- **Fail-open is inviolable.** Any internal error in the git adapter → `exit 0`, never block a commit on a guard bug. Mirror `epr-meta-resolver.py:327-333` and the library's `{}`-on-failure contract. A governance bug must never brick a non-Claude author's ability to commit.
- **Restorative, not carceral (Mishpat, not punishment).** A malformed `.epr-meta` in the cascade downgrades `deny`→`ask` (never hard-denies the subtree, which would brick the fix itself); editing an `.epr-meta` is never blocked. Consequence lands on the *write* (rejectable, fixable), never on the agent. This is what keeps it ToS-clean: no agent drives another; the shared substrate governs all equally.
- **Single source of rule logic.** The git adapter MUST call `_lib/epr_meta.py`. Zero rule logic in bash; do NOT re-derive rules; do NOT use the nascent Rust `eprfs-meta` parser (it resolves but does not enforce, and its own header says "It does not replace the existing hook resolver yet"). Three implementations would drift.
- **PVC-exempt by omission.** The compose-gate leg is pure-Python, ~milliseconds; it must NEVER be added to `HEAVY_GATES` (`.husky/pre-push:473`) nor routed through `run_gate`/`drop_project`. It is an unconditional top-level validation block, never deferrable.
- **Native Rust env for eprfs:** `RUSTFLAGS=""` (the WASM getrandom flag breaks the native link); its own `CARGO_TARGET_DIR` under `/tmp` (the `/projects` pool slot triggers the `invoked.timestamp` ENOENT fingerprint bug on this container).
- **`ask` maps to block-with-acknowledgement.** Git pre-commit is non-interactive; Claude's `ask` (prompt-to-proceed) has no analog. Map: `deny`→block (exit 1); `ask`→block unless `EPR_META_ACK=1` (a per-gate conscious acknowledgement, narrower than `--no-verify` which skips ALL hooks); `inject`/advise→stderr + allow; `measure`→stderr advisory + allow (the Claude hook owns the fingerprinted ledger side-channel; the git gate does not duplicate the flock'd writer in v1).

---

## Context (born oriented)

- **MAP / domain:** governance concern — the born-governed-emission enforcement field (compose-gate → managed surfaces → pre-push gates → merge topology). This plan relocates the first of those from Claude-private runtime to shared community law.
- **Prior art composed from (not forked):** the compose-gate design + impl plan (`2026-06-25`), the policy-registry "define-once-bind-many" design (`2026-07-02`, status Implemented — the enabler that makes a second harness adapter free), and the CI-detection `.epr-meta`-fold (`2026-06-25`, which established `.husky/pre-push` *already reading* the `.epr-meta` cascade for `.ci-ignore` freshness). The new leg adds *author-time rule evaluation* the pre-push does not yet do.
- **Philosophy is on-grain, not bolt-on.** "Identity sovereignty backstopped by community" ≡ "agents keep total agency, the protocol backstops coherence." The gate today is Claude-private = sovereignty-as-apex (each agent its own uncoordinated law). Moving it to `.husky` relocates the backstop to shared law — qahal/Mishpat backstopping every author regardless of which harness they carry. The policy registry is *already* named the `Mishpat::Precedent` shape (`epr_meta.py:328-333`).

---

## Task 1: eprfs — fix the one clippy defect the verify caught

**Files:**
- Modify: `elohim/eprfs/eprfs-core/src/projection.rs:270`

**Interfaces:**
- Consumes: nothing (leaf fix)
- Produces: an eprfs workspace that passes `cargo clippy -D warnings` — the pre-push gate's Rust bar.

Context: the isolated-worktree verify (fmt ✅ / build ✅ / test ✅ 12/12) found exactly one gate-failing lint: `clippy::cmp_owned` — `path == PathBuf::from("current")` allocates a `PathBuf` purely to compare.

- [ ] **Step 1: Reproduce the failing gate**

Run (in the verify worktree or after eprfs lands):
```bash
cd elohim/eprfs && CARGO_TARGET_DIR=/tmp/eprfs-verify-target RUSTFLAGS="" cargo clippy --all-targets -- -D warnings
```
Expected: FAIL — `error: this creates an owned instance just for comparison … projection.rs:270`

- [ ] **Step 2: Apply the minimal fix**

Replace the owned-comparison at `projection.rs:270`. Change:
```rust
Err(EprfsError::MissingBlob(path)) if path == PathBuf::from("current")
```
to compare against a borrowed `Path` (no allocation):
```rust
Err(EprfsError::MissingBlob(path)) if path == Path::new("current")
```
Ensure `use std::path::Path;` is in scope (it is, alongside the existing `PathBuf` import).

- [ ] **Step 3: Verify the gate passes**

Run: `cd elohim/eprfs && CARGO_TARGET_DIR=/tmp/eprfs-verify-target RUSTFLAGS="" cargo clippy --all-targets -- -D warnings && cargo fmt --check && cargo test`
Expected: clippy clean (exit 0), fmt clean, 12 tests pass.

- [ ] **Step 4: Commit (on the eprfs integration branch, not dev)**

```bash
git add elohim/eprfs/eprfs-core/src/projection.rs
git commit -m "fix(eprfs): borrow Path in cmp instead of allocating PathBuf (clippy::cmp_owned)"
```

---

## Task 2: The git-native compose-gate adapter (the fourth binding surface)

**Files:**
- Create: `.claude/hooks/epr-meta-git-gate.py`
- Test: `.claude/hooks/tests/test_epr_meta_git_gate.py`

**Interfaces:**
- Consumes: `_lib.epr_meta` — `collect_cascade(Path)->[Path]`, `check_meta(Path)->[str]`, `merge_rules([Path])->dict`, `load_policies(root)->(dict,[str])`, `expand_policies(merged,policies)->[str]`, `evaluate(merged, write)->[Verdict]`, `combine([Verdict])->Verdict|None`, `find_repo_root(Path)->Path`, `MANIFEST_NAME`. The `write` dict shape is `{"path": str, "content": str|None, "is_new": bool, "is_new_subdir": bool}` (identical to `epr-meta-resolver.py:294`).
- Produces: a CLI `epr-meta-git-gate.py [--staged | --range A..B]` with exit 0 = allow, 1 = block; and two pure, unit-testable functions `build_write(path, status, content, head_has_parent)->dict` and `decide(verdicts, *, ack)->(int, list[str])` that the husky/CI adapters and the tests both drive.

The adapter derives the `write` dict from git instead of Claude's stdin, then runs the *exact* resolver sequence. Full source (this IS the deliverable):

```python
#!/usr/bin/env python3
# .claude/hooks/epr-meta-git-gate.py
"""Harness-agnostic .epr-meta compose-gate — the git-invoked adapter (4th binding surface over
_lib.epr_meta, after the Claude PreToolUse hook, ci-ignore-projector, and placement-audit).

Governs ANY author's commit/push — Codex, Gemini, a human, or Claude driving git via Bash — by
evaluating the SAME pure rule engine the Claude hook uses, over files derived from git (staged
blobs / a rev-range) rather than Claude tool_input. Collaboration THROUGH the protocol: no agent
drives another; the shared substrate governs all equally.

Fail-open: a guard bug NEVER blocks a commit. Restorative: a malformed manifest downgrades
deny->ask; editing an .epr-meta is never blocked; consequence lands on the write, never the agent.

Usage:
  epr-meta-git-gate.py --staged        # pre-commit: git diff --cached
  epr-meta-git-gate.py --range A..B    # pre-push / CI: files changed in the range
Exit 0 = allow (advisories to stderr); 1 = block (deny, or ask without EPR_META_ACK=1).
Bypass: EPR_META_ACK=1 downgrades ask->allow for THIS gate; `git commit --no-verify` skips ALL hooks.
"""
import os
import subprocess
import sys
from pathlib import Path

# --- _lib bootstrap (clone of epr-meta-resolver.py:16-24) ---
_here = Path(__file__).resolve()
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        break
    _here = _here.parent

from _lib import epr_meta  # noqa: E402

_SEVERITY = {"deny": 3, "ask": 2, "inject": 1, "measure": 0, "dispatch": 0}


def _git(*args: str) -> str | None:
    """Run a git plumbing command; return stdout text, or None on any failure (fail-open)."""
    try:
        out = subprocess.run(["git", *args], capture_output=True, text=True, check=False)
        return out.stdout if out.returncode == 0 else None
    except Exception:  # noqa: BLE001
        return None


def _changed(mode: str, rng: str | None) -> list[tuple[str, str]]:
    """(path, status) pairs for staged files (--staged) or a rev-range. ACMR only — deletes
    can't violate an author-time content rule."""
    if mode == "staged":
        raw = _git("diff", "--cached", "--name-status", "--diff-filter=ACMR")
    else:
        raw = _git("diff", "--name-status", "--diff-filter=ACMR", rng)
    if not raw:
        return []
    out: list[tuple[str, str]] = []
    for ln in raw.splitlines():
        parts = ln.split("\t")
        if len(parts) < 2:
            continue
        status = parts[0][0]              # A/M/C/R (R100 -> R)
        path = parts[-1]                  # rename dst is the last field
        out.append((path, status))
    return out


def _content(mode: str, rng: str | None, path: str) -> str | None:
    """Post-write content of `path`: the staged blob (--staged) or the range's head blob."""
    if mode == "staged":
        return _git("show", f":{path}")
    head = rng.split("..")[-1] if rng else "HEAD"
    return _git("show", f"{head}:{path}")


def _head_has_parent(mode: str, rng: str | None, path: str) -> bool:
    """True if the file's parent directory already exists in the base tree (so a new file in it is
    NOT a new-subdir signal). Base = HEAD for --staged, else the range's base."""
    parent = str(Path(path).parent)
    if parent in (".", ""):
        return True  # repo root always exists
    base = "HEAD" if mode == "staged" else (rng.split("..")[0] if rng and ".." in rng else "HEAD")
    listing = _git("ls-tree", base, f"{parent}/")
    return bool(listing and listing.strip())


def build_write(path: str, status: str, content: str | None, head_has_parent: bool) -> dict:
    """PURE: derive the resolver's `write` dict from git facts. `is_new` = added (A). `is_new_subdir`
    = added AND its parent dir is absent from the base tree (the git analog of the resolver's
    'parent dir does not yet exist')."""
    is_new = status == "A"
    return {"path": path, "content": content, "is_new": is_new,
            "is_new_subdir": is_new and not head_has_parent}


def _verdict_for(path: str, write: dict) -> object | None:
    """Run the full resolver sequence for one write; return a combined Verdict|None. Mirrors
    epr-meta-resolver.py:250-323 minus the Claude-only side-channels. Malformed-manifest downgrade
    is applied here (deny->ask) unless the target IS an .epr-meta (never blocked)."""
    target = Path(path)
    chain = epr_meta.collect_cascade(target)
    if not chain:
        return None
    target_is_manifest = target.name == epr_meta.MANIFEST_NAME
    if not target_is_manifest:
        problems = [(m, errs) for m in chain if (errs := epr_meta.check_meta(m))]
        if problems:
            detail = "; ".join(f"{m}: {', '.join(e)}" for m, e in problems)
            return epr_meta.Verdict("ask", f"governance manifest malformed — {detail}. "
                                    "Fix the manifest to restore full governance; proceeding "
                                    "now requires acknowledgement.", "epr-meta:malformed")
    merged = epr_meta.merge_rules(chain)
    root = epr_meta.find_repo_root(target)
    policies, _pol_errs = epr_meta.load_policies(root)
    epr_meta.expand_policies(merged, policies)
    verdicts = epr_meta.evaluate(merged, write)
    return epr_meta.combine(verdicts)


def decide(verdicts: list, *, ack: bool) -> tuple[int, list[str]]:
    """PURE: fold per-file combined verdicts into (exit_code, messages). Most-severe wins.
    deny->block; ask->block unless ack; inject/measure->advise+allow."""
    msgs: list[str] = []
    worst = None
    for v in verdicts:
        if v is None:
            continue
        if worst is None or _SEVERITY[v.cls] > _SEVERITY[worst.cls]:
            worst = v
        msgs.append(f"[{v.cls}] {v.reason} (rule `{v.rule_id}`)")
    if worst is None:
        return 0, msgs
    if worst.cls == "deny":
        return 1, msgs
    if worst.cls == "ask":
        if ack:
            return 0, [*msgs, "[.epr-meta] ask-class acknowledged via EPR_META_ACK=1 — allowed."]
        return 1, [*msgs, "[.epr-meta] This write needs acknowledgement. If reviewed and intended, "
                          "re-commit with EPR_META_ACK=1 (this gate only), or `git commit "
                          "--no-verify` (skips ALL hooks)."]
    return 0, msgs  # inject / measure — advisory


def run(mode: str, rng: str | None) -> int:
    ack = os.environ.get("EPR_META_ACK") == "1"
    verdicts = []
    for path, status in _changed(mode, rng):
        content = _content(mode, rng, path)
        write = build_write(path, status, content, _head_has_parent(mode, rng, path))
        verdicts.append(_verdict_for(path, write))
    code, msgs = decide(verdicts, ack=ack)
    for m in msgs:
        print(m, file=sys.stderr)
    return code


def main() -> int:
    args = sys.argv[1:]
    if "--staged" in args:
        return run("staged", None)
    if "--range" in args:
        i = args.index("--range")
        rng = args[i + 1] if i + 1 < len(args) else "HEAD~1..HEAD"
        return run("range", rng)
    print("usage: epr-meta-git-gate.py [--staged | --range A..B]", file=sys.stderr)
    return 0  # unknown invocation → fail-open


if __name__ == "__main__":
    try:
        sys.exit(main())
    except SystemExit:
        raise
    except Exception as e:  # fail-open: internal error is NOT a block
        print(f"epr-meta-git-gate internal error: {e}", file=sys.stderr)
        sys.exit(0)
```

- [ ] **Step 1: Write the failing tests** (`.claude/hooks/tests/test_epr_meta_git_gate.py`)

```python
import importlib.util, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]  # repo root
sys.path.insert(0, str(ROOT / ".claude" / "scripts"))
spec = importlib.util.spec_from_file_location(
    "git_gate", ROOT / ".claude" / "hooks" / "epr-meta-git-gate.py")
gg = importlib.util.module_from_spec(spec); spec.loader.exec_module(gg)
from _lib.epr_meta import Verdict


def test_build_write_new_file_in_new_subdir():
    w = gg.build_write("a/b/new.md", "A", "x", head_has_parent=False)
    assert w == {"path": "a/b/new.md", "content": "x", "is_new": True, "is_new_subdir": True}

def test_build_write_modified_existing():
    w = gg.build_write("a/b.md", "M", "x", head_has_parent=True)
    assert w["is_new"] is False and w["is_new_subdir"] is False

def test_decide_deny_blocks():
    code, _ = gg.decide([Verdict("deny", "no", "r1")], ack=False)
    assert code == 1

def test_decide_ask_blocks_without_ack_allows_with_ack():
    assert gg.decide([Verdict("ask", "confirm", "r1")], ack=False)[0] == 1
    assert gg.decide([Verdict("ask", "confirm", "r1")], ack=True)[0] == 0

def test_decide_inject_and_none_allow():
    assert gg.decide([Verdict("inject", "fyi", "r1"), None], ack=False)[0] == 0

def test_decide_most_severe_wins():
    code, _ = gg.decide([Verdict("inject", "fyi", "r1"), Verdict("deny", "no", "r2")], ack=False)
    assert code == 1
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `python3 -m pytest .claude/hooks/tests/test_epr_meta_git_gate.py -v`
Expected: FAIL (module `epr-meta-git-gate.py` not created yet / import error).

- [ ] **Step 3: Create the adapter** — write `.claude/hooks/epr-meta-git-gate.py` with the full source above; `chmod +x`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `python3 -m pytest .claude/hooks/tests/test_epr_meta_git_gate.py -v`
Expected: 6 passed.

- [ ] **Step 5: Live integration test against a real governed dir**

Create a scratch violation in the verify worktree (or a temp repo) under a dir whose `.epr-meta` requires frontmatter, stage it, and run `--staged`:
```bash
python3 .claude/hooks/epr-meta-git-gate.py --staged; echo "exit=$?"
```
Expected: a `[deny]`/`[ask]` line on stderr and a non-zero exit for a violating staged `.md`; exit 0 for a compliant one. (Use `genesis/docs/superpowers/plans/` or another `require-frontmatter` tree as the fixture.)

- [ ] **Step 6: Commit**

```bash
git add .claude/hooks/epr-meta-git-gate.py .claude/hooks/tests/test_epr_meta_git_gate.py
git commit -m "feat(epr-meta): git-native compose-gate adapter — the 4th harness-agnostic binding surface"
```

---

## Task 3: Wire the gate into `.husky/pre-commit` (born-governed at commit)

**Files:**
- Create: `.husky/pre-commit`

**Interfaces:**
- Consumes: `.claude/hooks/epr-meta-git-gate.py --staged` (Task 2).
- Produces: commit-time governance for every author. `.husky/_/pre-commit` (the husky forwarder stub) already exists and will now find this repo-relative script.

- [ ] **Step 1: Create `.husky/pre-commit`** (mirror the `.ci-ignore` block idiom: HUSKY=0 honor + `command -v python3` fail-open guard)

```bash
#!/usr/bin/env bash
# Pre-commit hook: harness-agnostic .epr-meta compose-gate over STAGED files.
# Governs every author's commit (Codex, Gemini, human, Claude-via-Bash) the way the Claude
# PreToolUse hook governs Claude's edits — same pure engine (_lib/epr_meta.py), git-derived writes.
#
# Bypass this gate only: EPR_META_ACK=1 git commit …   (acknowledges ask-class; deny still blocks)
# Bypass ALL hooks:      git commit --no-verify        (or HUSKY=0 git commit)
#
# Honor HUSKY=0 — core.hooksPath points at .husky/ directly, so the husky shim that normally
# handles this is not in the call path (identical to .husky/pre-push:23-25).
[ "${HUSKY-}" = "0" ] && exit 0

# Fail-open if python3 is absent (degraded shell) — same posture as pre-push's guarded blocks.
if command -v python3 >/dev/null 2>&1; then
  python3 .claude/hooks/epr-meta-git-gate.py --staged || exit 1
fi
exit 0
```

- [ ] **Step 2: Make executable**

Run: `chmod +x .husky/pre-commit`

- [ ] **Step 3: Verify it fires on a real commit**

Stage a frontmatter-less `.md` under a `require-frontmatter` tree and attempt a commit:
```bash
git add <violating.md> && git commit -m "test" ; echo "commit exit=$?"
```
Expected: commit BLOCKED (non-zero), stderr shows the `[deny]`/`[ask]` reason. Then verify a compliant commit passes, and that `EPR_META_ACK=1 git commit …` proceeds past an ask, and `--no-verify` bypasses. Clean up the test artifacts.

- [ ] **Step 4: Verify fail-open** — temporarily break the engine import path (e.g. rename `_lib` in a scratch copy) and confirm a commit still succeeds with the internal-error line on stderr. Restore.

- [ ] **Step 5: Commit**

```bash
git add .husky/pre-commit
git commit -m "feat(husky): pre-commit .epr-meta compose-gate — governance binds every author, not just Claude"
```

---

## Task 4: CI backstop (belt-and-suspenders — `--no-verify` must not reach dev ungoverned)

**Files:**
- Modify: `genesis/Jenkinsfile` (add a lightweight compose-gate stage) OR `.husky/pre-push` (add a push-time stage as the nearer backstop).

**Interfaces:**
- Consumes: `.claude/hooks/epr-meta-git-gate.py --range origin/dev..HEAD`.

Rationale: `.husky/` is CI-ignored today and no pipeline runs `epr_meta.evaluate()`; a violation pushed with `--no-verify` reaches `dev` ungoverned. Add the same "husky governs commit, CI is the backstop" shape used for `sweettest-check`. The nearest, cheapest backstop is a **pre-push stage** (governs even a commit that slipped `--no-verify` at commit-time but not push-time); the durable one is a CI stage over the PR diff.

- [ ] **Step 1: Add a pre-push compose-gate stage** to `.husky/pre-push`, immediately after the `.ci-ignore` freshness block (`:254`), as an unconditional top-level block (NOT via `run_gate`/`HEAVY_GATES` — PVC-exempt by omission):

```bash
# ── .epr-meta compose-gate (author-time rule evaluation over the push range) ──
# The commit-time gate (.husky/pre-commit) is the primary; this is the backstop for commits made
# with --no-verify at commit time. Pure-python, never deferrable. Fail-open if python3 absent.
if command -v python3 >/dev/null 2>&1; then
  RANGE_BASE=$(git merge-base origin/dev HEAD 2>/dev/null || echo "HEAD~1")
  if ! python3 .claude/hooks/epr-meta-git-gate.py --range "${RANGE_BASE}..HEAD"; then
    echo "[pre-push] ERROR: .epr-meta compose-gate rejected a change in this push range."
    echo "  Acknowledge (ask-class): EPR_META_ACK=1 git push   |   bypass all: git push --no-verify"
    exit 1
  fi
  echo "[pre-push] .epr-meta compose-gate ✓"
fi
```

- [ ] **Step 2: Verify** — push a branch containing a governed violation to a scratch remote/ref; confirm the push is blocked with the reason, and `EPR_META_ACK=1` / `--no-verify` bypass as designed.

- [ ] **Step 3 (follow-up, capture to backlog):** a genesis-Jenkinsfile stage evaluating the same gate over the full PR diff at `dev`→`main`, for the durable backstop independent of local hooks. File to `genesis/data/timeline/backlog/`.

- [ ] **Step 4: Commit**

```bash
git add .husky/pre-push
git commit -m "feat(husky): pre-push .epr-meta compose-gate backstop for --no-verify commits"
```

---

## Task 5: Cleanup discovered in-flight (capture, don't bloat) + parity guard backlog

**Files:**
- Modify: `genesis/docs/superpowers/specs/2026-06-25-epr-meta-compose-gate-design.md` (status `Draft`→`Implemented`; add a short "Binding surfaces" note naming the git-hook as surface #2 over the same registry)
- Reconcile (commit the untracked): `genesis/docs/superpowers/plans/2026-06-25-epr-meta-compose-gate.md`, `genesis/docs/superpowers/plans/2026-06-25-ci-detection-convergence-epr-meta-fold-plan.md`
- Create: `genesis/data/timeline/backlog/epr-meta-python-rust-parser-parity.md` (guard the two `.epr-meta` parsers — Python `_lib/epr_meta.py` enforcer and Rust `eprfs-meta` resolver — from grammar drift)
- Create: `genesis/data/timeline/backlog/eprfs-address-reuse-brit-cid-codec.md` (eprfs `address.rs` reinvents CID as String newtypes; should reuse brit `BritCid`/storage `cid::Cid`)

- [ ] **Step 1:** Refresh the design-spec status and add the binding-surfaces note (one paragraph: "Surface #1 = Claude PreToolUse hook; Surface #2 = git pre-commit/pre-push via `epr-meta-git-gate.py`; both drive the same `_lib/epr_meta.py` over the same registry — they cannot disagree").
- [ ] **Step 2:** `python3 .claude/scripts/memory-kit/cite-gen.py --refresh` on the compose-gate plan (its design-spec cite fingerprint drifted; current is `sha256:6052ce071bfec509`).
- [ ] **Step 3:** Write the two backlog items (timeline-CONVENTIONS-conformant, status-carrying).
- [ ] **Step 4:** Commit the doc reconciliation + backlog together.

---

## Task 6 (OPERATOR-OWNED): brit submodule reconciliation + eprfs landing on dev

This leg is **operator-owned** — it pushes to the `brit` repo and merges to `dev` (the integrator is the single push/merge authority; the brit submodule-pointer bump is explicitly operator-owned per `project_brit_next_gen_epr_meta_foundation`). Documented here for completeness; NOT executed autonomously.

The `feat/eprfs` brit pin (`8ddf3099d`) is **divergent**, not a fast-forward: it carries 6 git-tree-projection commits but is missing brit main's *entire* epr-meta/cite-parity foundation (landed `4185b9a0dd`, 2026-06-30). Taking it verbatim would move dev's brit onto a sibling line that silently REGRESSES brit. Also, brit's `brit-eprfs` crate path-depends on `../../eprfs/eprfs-core` (assumes the monorepo layout).

- [ ] **Step 1:** In the brit repo, rebase/cherry-pick the `brit-eprfs` adapter (6 commits, `7a51e1ff2..8ddf3099d`) onto brit `main` (`fc77ff2ed` or later), producing ONE brit commit carrying BOTH the eprfs adapter AND the cite-parity engine. Verify brit builds.
- [ ] **Step 2:** Point `elohim/brit` at that reconciled commit.
- [ ] **Step 3:** Rebase `feat/eprfs` onto current `dev`; confirm the empty collision surface holds (dev touched neither `elohim/Cargo.toml` nor the brit pointer). Include Task-1's clippy fix.
- [ ] **Step 4:** Wire eprfs into the gates — add `eprfs` to a `build-manifest.json` and a matching `run_gate` fallback `case` (per CLAUDE.md: a new excluded workspace needs the fallback arm, else `*) Unknown project` aborts the whole push). Its gate: `RUSTFLAGS="" cargo fmt --check && cargo clippy -D warnings && cargo test`.
- [ ] **Step 5:** Local FF merge `feat/eprfs` → `dev`; the pre-commit/pre-push compose-gate now governs this very merge — eprfs becomes the first branch to land *through* the governed substrate.

---

## Self-Review

**Spec coverage:** (1) integrate feat/eprfs → Tasks 1, 6. (2) make governance harness-agnostic → Tasks 2, 3, 4. (3) compose-not-fork → uses `_lib/epr_meta.py` verbatim, extends the `2026-07-02` design; Task 5 refreshes it. (4) respect P1 / projection-not-truth / sovereignty-backstopped-by-community → eprfs verified as projection-plane only; the gate's restorative posture is the Mishpat framing. Covered.

**Placeholder scan:** the adapter source and tests are complete (no TBD/TODO in the deliverable). Task 6 is deliberately operator-owned steps, not agentic-worker steps.

**Type consistency:** `build_write`/`decide` signatures match between the adapter, the tests, and Task-2's Interfaces block. `write` dict shape matches `epr-meta-resolver.py:294`. `Verdict(cls, reason, rule_id)` matches `_lib/epr_meta.py:122`. `_SEVERITY` matches the library's.

## Execution Handoff

Plan complete. Given ultracode + the operator's explicit "integrate and accept updates," Tasks 1–5 are executed inline in this session (the buildable, ToS-clean, dev-untouching legs); Task 6 is surfaced for the operator (brit push + dev merge authority).
