"""Git-native adapter for the .epr-meta compose-gate — the harness-agnostic binding surface.

The 4th adapter over _lib.epr_meta (after the Claude PreToolUse hook, ci-ignore-projector, and
placement-audit). Governs ANY author's commit/push — Codex, Gemini, a human, or Claude driving git
via Bash — by evaluating the SAME pure rule engine over files derived from git (staged blobs / a
rev-range) rather than Claude tool_input. Collaboration THROUGH the protocol: no agent drives
another; the shared substrate governs every author equally.

Real logic lives HERE (in _lib, tested under __tests__) so .claude/hooks/ stays thin and the
transport can be swapped — exactly what .claude/hooks/.epr-meta declares. Driven by the thin CLI
.claude/scripts/epr-meta-git-gate.py and .husky/{pre-commit,pre-push}.

Fail-open: a guard bug NEVER blocks a commit. Restorative (Mishpat, not punishment): a malformed
manifest downgrades deny->ask; editing an .epr-meta is never blocked; consequence lands on the
write (rejectable, fixable), never on the agent.
"""
from __future__ import annotations

import subprocess
from pathlib import Path

from _lib import epr_meta

# Mirrors _lib.epr_meta severities; kept local so this adapter imports no private name.
_SEVERITY = {"deny": 3, "ask": 2, "inject": 1, "measure": 0, "dispatch": 0}


def git(*args: str) -> str | None:
    """Run a git plumbing command; return stdout, or None on any failure (fail-open)."""
    try:
        out = subprocess.run(["git", *args], capture_output=True, text=True, check=False)
        return out.stdout if out.returncode == 0 else None
    except Exception:  # noqa: BLE001
        return None


def changed_files(mode: str, rng: str | None, *, runner=git) -> list[tuple[str, str]]:
    """(path, status-letter) for staged files (--staged) or a rev-range. ACMR only — a delete
    can't violate an author-time content rule. `runner` is injectable for tests."""
    if mode == "staged":
        raw = runner("diff", "--cached", "--name-status", "--diff-filter=ACMR")
    else:
        raw = runner("diff", "--name-status", "--diff-filter=ACMR", rng or "HEAD~1..HEAD")
    if not raw:
        return []
    out: list[tuple[str, str]] = []
    for ln in raw.splitlines():
        parts = ln.split("\t")
        if len(parts) < 2:
            continue
        out.append((parts[-1], parts[0][0]))  # rename dst is last; status letter is first char
    return out


def content_of(mode: str, rng: str | None, path: str, *, runner=git) -> str | None:
    """Post-write content of `path`: the staged blob (--staged) or the range head blob."""
    if mode == "staged":
        return runner("show", f":{path}")
    head = rng.split("..")[-1] if rng and ".." in rng else "HEAD"
    return runner("show", f"{head}:{path}")


def head_has_parent(mode: str, rng: str | None, path: str, *, runner=git) -> bool:
    """True if the file's parent dir already exists in the base tree (so a new file in it is NOT a
    new-subdir signal). Base = HEAD for --staged, else the range base."""
    parent = str(Path(path).parent)
    if parent in (".", ""):
        return True  # repo root always exists
    base = "HEAD" if mode == "staged" else (rng.split("..")[0] if rng and ".." in rng else "HEAD")
    listing = runner("ls-tree", base, f"{parent}/")
    return bool(listing and listing.strip())


def build_write(path: str, status: str, content: str | None, head_has_parent: bool) -> dict:
    """PURE: derive the resolver's `write` dict from git facts. `is_new` = added (A);
    `is_new_subdir` = added AND its parent dir absent from the base tree (the git analog of the
    resolver's 'parent dir does not yet exist'). Shape identical to epr-meta-resolver.py:294."""
    is_new = status == "A"
    return {"path": path, "content": content, "is_new": is_new,
            "is_new_subdir": is_new and not head_has_parent}


def verdict_for(path: str, write: dict):
    """Full resolver sequence for one write -> combined Verdict|None. Mirrors
    epr-meta-resolver.py:250-323 minus the Claude-only side-channels. Malformed manifest downgrades
    deny->ask (unless the target IS an .epr-meta — legacy flat OR directory-form
    `.epr-meta/manifest.md` — which is never blocked so the fix is never bricked)."""
    target = Path(path)
    chain = epr_meta.collect_cascade(target)
    if not chain:
        return None
    if not epr_meta.is_manifest_path(target):
        problems = [(m, errs) for m in chain if (errs := epr_meta.check_meta(m))]
        if problems:
            detail = "; ".join(f"{m}: {', '.join(e)}" for m, e in problems)
            return epr_meta.Verdict("ask", f"governance manifest malformed — {detail}. Fix the "
                                    "manifest to restore full governance; proceeding now requires "
                                    "acknowledgement.", "epr-meta:malformed")
    merged = epr_meta.merge_rules(chain)
    root = epr_meta.find_repo_root(target)
    policies, _pol_errs = epr_meta.load_policies(root)
    epr_meta.expand_policies(merged, policies)
    return epr_meta.combine(epr_meta.evaluate(merged, write))


def decide(verdicts: list, *, ack: bool) -> tuple[int, list[str]]:
    """PURE: fold per-file combined verdicts into (exit_code, messages). Most-severe wins.
    deny->block(1); ask->block unless ack; inject/measure->advise+allow(0)."""
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
                          "re-run with EPR_META_ACK=1 (this gate only), or bypass ALL hooks with "
                          "--no-verify."]
    return 0, msgs  # inject / measure — advisory only, never blocks


def run(mode: str, rng: str | None, *, ack: bool, runner=git) -> tuple[int, list[str]]:
    """Evaluate every changed file through the real cascade; return (exit_code, messages)."""
    verdicts = []
    for path, status in changed_files(mode, rng, runner=runner):
        content = content_of(mode, rng, path, runner=runner)
        parent_exists = head_has_parent(mode, rng, path, runner=runner)
        verdicts.append(verdict_for(path, build_write(path, status, content, parent_exists)))
    return decide(verdicts, ack=ack)
