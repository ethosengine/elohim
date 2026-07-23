#!/usr/bin/env python3
"""Thin CLI for the harness-agnostic .epr-meta compose-gate — sibling to ci-ignore-projector.py.
All logic lives in _lib.epr_meta_git (tested under _lib/__tests__). Invoked by
.husky/{pre-commit,pre-push} and (later) a CI stage, so the SAME pure rule engine governs every
author's commit — Codex, Gemini, a human, or Claude driving git — not just Claude's tool edits.

Usage: epr-meta-git-gate.py [--staged | --range A..B]
Exit 0 = allow (advisories to stderr); 1 = block (deny, or ask without EPR_META_ACK=1).
Bypass: EPR_META_ACK=1 downgrades ask->allow for THIS gate; git --no-verify skips ALL hooks.
Fail-open by design (callers guard on `command -v python3`); absence never blocks a commit.

Witnesses each changed file's decision to governance-findings.jsonl via the SAME
_lib.epr_meta.witness helper the Claude PreToolUse resolver drives (runtime="git-gate") — one
governance ledger, every author. Re-derives the per-file verdict sequence `_lib.epr_meta_git.run()`
computes internally (same functions, same order) so each file's `path` is available alongside its
Verdict for the witness `subject` — `run()`'s aggregate (exit_code, messages) return doesn't expose
that pairing, and this module is a thin CLI (real logic stays in _lib.epr_meta_git, untouched)."""
import os
import sys
from pathlib import Path

_here = Path(__file__).resolve()
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        break
    _here = _here.parent

from _lib import epr_meta  # noqa: E402
from _lib import epr_meta_git  # noqa: E402


def _witness_file(root: Path, path: str, verdict, *, ack: bool) -> None:
    """Witness ONE file's combined Verdict (or None) via the shared epr_meta.witness helper.
    Mirrors the resolver hook's decision mapping (epr_meta.decision_for), plus the git-only
    EPR_META_ACK downgrade: an acked ask is witnessed as permit + refer{reason: "acked"} — the ask
    still fired (that's recorded), the author/operator chose to proceed. A clean allow (verdict is
    None) is never witnessed — mirrors the resolver's no-rule-clean-allow silence."""
    if verdict is None:
        return
    if verdict.cls == "ask" and ack:
        epr_meta.witness(root, runtime="git-gate", gate="git-gate", subject=path,
                          decision="permit", cls="ask", rule_id=verdict.rule_id,
                          refer={"layer": "operator", "reason": "acked"}, checks=[verdict.reason])
        return
    info = epr_meta.decision_for(verdict)
    epr_meta.witness(root, runtime="git-gate", gate="git-gate", subject=path,
                      decision=info["decision"], cls=info["cls"], rule_id=info["rule_id"],
                      refer=info["refer"], checks=[verdict.reason])


def main(argv: list[str]) -> int:
    ack = os.environ.get("EPR_META_ACK") == "1"
    if "--staged" in argv:
        mode, rng = "staged", None
    elif "--range" in argv:
        i = argv.index("--range")
        mode, rng = "range", (argv[i + 1] if i + 1 < len(argv) else "HEAD~1..HEAD")
    else:
        print("usage: epr-meta-git-gate.py [--staged | --range A..B]", file=sys.stderr)
        return 0  # unknown invocation -> fail-open

    root = epr_meta.find_repo_root(Path.cwd())
    verdicts_by_path: list[tuple[str, object]] = []
    for path, status in epr_meta_git.changed_files(mode, rng):
        content = epr_meta_git.content_of(mode, rng, path)
        parent_exists = epr_meta_git.head_has_parent(mode, rng, path)
        write = epr_meta_git.build_write(path, status, content, parent_exists)
        verdicts_by_path.append((path, epr_meta_git.verdict_for(path, write)))

    for path, v in verdicts_by_path:
        try:
            _witness_file(root, path, v, ack=ack)
        except Exception:  # noqa: BLE001 — witnessing must never break the gate
            pass

    code, msgs = epr_meta_git.decide([v for _, v in verdicts_by_path], ack=ack)
    for m in msgs:
        print(m, file=sys.stderr)
    return code


if __name__ == "__main__":
    try:
        sys.exit(main(sys.argv[1:]))
    except SystemExit:
        raise
    except Exception as e:  # fail-open: an internal error is NOT a block
        print(f"epr-meta-git-gate internal error: {e}", file=sys.stderr)
        sys.exit(0)
