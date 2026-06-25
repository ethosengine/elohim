#!/usr/bin/env python3
# .claude/hooks/epr-meta-resolver.py
"""PreToolUse resolver for the .epr-meta compose-gate. Thin: stdin -> _lib.epr_meta -> verdict JSON.
Fail-open: a guard bug never blocks dev."""
import json
import sys
from pathlib import Path

# --- _lib bootstrap (clone of managed-surface-context.py:26-32) ---
_here = Path(__file__).resolve()
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        break
    _here = _here.parent

from _lib import epr_meta  # noqa: E402


def _emit_deny(reason: str):
    print(json.dumps({"hookSpecificOutput": {
        "hookEventName": "PreToolUse", "permissionDecision": "deny",
        "permissionDecisionReason": reason}}))
    sys.exit(0)


def _emit_ask(reason: str):
    print(json.dumps({"hookSpecificOutput": {
        "hookEventName": "PreToolUse", "permissionDecision": "ask",
        "permissionDecisionReason": reason}}))
    sys.exit(0)


def _emit_advise(text: str):
    print(json.dumps({"hookSpecificOutput": {
        "hookEventName": "PreToolUse", "additionalContext": text}}))
    sys.exit(0)


def main():
    raw = sys.stdin.read()
    try:
        data = json.loads(raw)
    except Exception:
        sys.exit(0)  # malformed stdin -> fail open silently

    tool = data.get("tool_name", "")
    if tool not in ("Write", "Edit"):
        sys.exit(0)
    ti = data.get("tool_input", {}) or {}
    fp = ti.get("file_path", "")
    if not fp:
        sys.exit(0)

    target = Path(fp)
    is_new = (tool == "Write") and not target.exists()
    # No separate dir-create event in Claude Code: a Write whose parent dir does not yet exist
    # is the new-subdir signal (drives no-new-subdirs / require-sibling).
    is_new_subdir = (tool == "Write") and not target.parent.exists()
    # Write carries full content; Edit does not -> read on-disk for frontmatter checks.
    content = ti.get("content")
    if content is None and target.exists():
        try:
            content = target.read_text(errors="replace")
        except Exception:
            content = None

    chain = epr_meta.collect_cascade(target)
    if not chain:
        sys.exit(0)  # no governance here

    if not epr_meta.yaml_available():
        _emit_advise("[.epr-meta] PyYAML unavailable — compose-gate rules NOT enforced for this "
                     "write. Install PyYAML to re-enable the gate. (failing open, not silent.)")

    # schema-validate every .epr-meta in the cascade; a malformed manifest is itself a deny.
    metas = [(m, epr_meta.load_meta(m)) for m in chain]
    for meta, cfg in metas:
        errs = epr_meta.validate_meta(cfg)
        if errs:
            _emit_deny(f"malformed `.epr-meta` at {meta}: {'; '.join(errs)}")

    # Recursion-guard surface 6.2: a governed subtree whose cascade reached the repo/depth bound
    # without a `root: true` constitutional base is a misconfiguration. v1 ADVISES (fail-open
    # friendly); the spec's stricter `deny` is a hardening once the root is repo-wide.
    advisories = []
    if not any(cfg.get("root") is True for _, cfg in metas):
        advisories.append("[.epr-meta] no `root: true` constitutional base in this cascade — "
                          "add one (this subtree's governance has no anchor).")

    merged = epr_meta.merge_rules(chain)
    write = {"path": fp, "content": content, "is_new": is_new,
             "is_new_subdir": is_new_subdir}
    verdict = epr_meta.combine(epr_meta.evaluate(merged, write))
    if verdict is None:
        if advisories:
            _emit_advise(" ".join(advisories))
        sys.exit(0)  # silent allow
    src = merged["sources"][-1] if merged["sources"] else "?"
    msg = f"{verdict.reason} [rule `{verdict.rule_id}` from {src}]"
    if verdict.cls == "deny":
        _emit_deny(msg)
    elif verdict.cls == "ask":
        _emit_ask(msg)
    else:
        _emit_advise(" ".join([msg, *advisories]))


if __name__ == "__main__":
    try:
        main()
    except SystemExit:
        raise
    except Exception as e:  # fail-open: internal error, NOT a deny
        print(f"epr-meta-resolver internal error: {e}", file=sys.stderr)
        sys.exit(1)
