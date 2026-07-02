#!/usr/bin/env python3
# .claude/hooks/epr-meta-resolver.py
"""PreToolUse resolver for the .epr-meta compose-gate. Thin: stdin -> _lib.epr_meta -> verdict JSON.
Fail-open: a guard bug never blocks dev."""
import json
import sys
import time
from pathlib import Path

# --- _lib bootstrap (clone of managed-surface-context.py:26-32) ---
_here = Path(__file__).resolve()
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        break
    _here = _here.parent

from _lib import epr_meta  # noqa: E402
from _lib import store  # noqa: E402

# Re-nudge a given gap-root at most once per ~working session, so the in-flight coverage signal
# INFORMS the agent-with-context without nagging every edit (the minimalism principle, applied to
# the nudge itself). Tunable; the gap self-closes the moment a `covers: subtree` manifest is authored.
_ADVICE_WINDOW = 4 * 3600


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


def _coverage_nudge(target: Path):
    """The IN-FLIGHT coverage signal: if `target` sits inside an UNCLAIMED substantial region, return a
    debounced advisory pointing the agent — who has the live context RIGHT NOW — at the gap-root to
    govern. This is the prevention-shaped, context-rich remediation path (vs a blind external sweep):
    the best `.epr-meta` is written by whoever is already intimately in the directory. None if the
    region is already owned, not substantial, or recently advised. Fail-open: any error → no nudge."""
    try:
        root = epr_meta.find_repo_root(target)
        cfg = epr_meta.governance_cfg(root)
        adv = epr_meta.coverage_advice(
            target, repo_root=root, min_files=cfg["min_files"], min_subdirs=cfg["min_subdirs"],
            min_exts=cfg["min_exts"], exclude_globs=cfg["exclude_globs"])
        if not adv:
            return None
        gap = adv["gap_root"]
        state_p = root / ".claude/data/epr-meta-advice.json"
        seen = store.load_json(state_p, {}) or {}
        now = time.time()
        last = seen.get(gap, 0)
        if isinstance(last, (int, float)) and (now - last) < _ADVICE_WINDOW:
            return None  # debounced — already nudged for this region this session
        seen[gap] = now
        store.save_json(state_p, seen)
        return (f"[.epr-meta coverage] You're working in `{gap}` — a substantial directory with no "
                f"governance ownership (no `covers: subtree` .epr-meta above it). You have the live "
                f"context NOW: consider authoring `{gap}/.epr-meta` declaring `covers: subtree` — a real "
                f"rule if there's a recurring, mechanizable drift here, else rules-free with a `why:` "
                f"recording the considered 'no edit-time gate needed' decision. See the "
                f"`elohim-epr-metafile` skill. (In-flight remediation; full queue: "
                f"`placement-audit.py --epr-meta`.)")
    except Exception:  # noqa: BLE001 — the nudge is advisory; never let it block or crash the hook
        return None


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

    # Resolve the content the rules will see.
    #   Write: the payload carries the full post-write content.
    #   Edit:  synthesize the POST-edit content — read on-disk, apply old_string->new_string
    #          (respecting replace_all) — so content-triggered rules (contains-any / validator)
    #          evaluate what the edit INTRODUCES, not the stale pre-edit pre-image (which both
    #          misses a newly-added match AND lets a match the edit REMOVES keep blocking).
    if tool == "Edit":
        if target.exists():
            try:
                disk = target.read_text(errors="replace")
            except Exception:
                disk = None
            if disk is not None:
                old = ti.get("old_string", "")
                new = ti.get("new_string", "")
                if not old or old not in disk:
                    # old_string absent from disk -> the Edit itself will fail; nothing to gate.
                    sys.exit(0)
                content = disk.replace(old, new) if ti.get("replace_all") else disk.replace(old, new, 1)
            else:
                content = None  # unreadable -> content rules see nothing (the coverage nudge still runs)
        else:
            # Edit on a not-yet-existing file (the Edit will fail): leave content unresolved so
            # content rules stay silent, but keep the hook alive for the coverage nudge / cascade.
            content = None
    else:  # Write
        content = ti.get("content")
        if content is None and target.exists():
            # Defensive: a Write lacking an explicit content field on an existing file.
            try:
                content = target.read_text(errors="replace")
            except Exception:
                content = None

    # IN-FLIGHT COVERAGE SIGNAL — independent of the rule cascade. Authoring an .epr-meta is itself
    # never nudged (you're already governing). Computed up-front so it can fire even when there is no
    # cascade at all (the most common gap: a wholly-ungoverned substantial region).
    cov_nudge = None if target.name == epr_meta.MANIFEST_NAME else _coverage_nudge(target)

    chain = epr_meta.collect_cascade(target)
    if not chain:
        if cov_nudge:
            _emit_advise(cov_nudge)  # the empty-chair: "no governance here" → deliver the signal
        sys.exit(0)

    if not epr_meta.yaml_available():
        _emit_advise("[.epr-meta] PyYAML unavailable — compose-gate rules NOT enforced for this "
                     "write. Install PyYAML to re-enable the gate. (failing open, not silent.)")

    advisories = []
    if cov_nudge:  # a governed-but-unclaimed region: the nudge rides along with any rule advisories
        advisories.append(cov_nudge)

    # Strict-but-recoverable governance: a MALFORMED .epr-meta in the cascade must NOT hard-deny the
    # whole subtree (which would brick authoring — including the fix itself). Instead:
    #   • editing the manifest itself is never blocked, so you can always fix the typo → advise;
    #   • other writes in the subtree downgrade deny → ASK (overridable) until it's fixed.
    # This encodes "proposed-but-not-yet-valid governance" as `ask`, not a binding `deny`.
    target_is_manifest = target.name == epr_meta.MANIFEST_NAME
    problems = [(m, errs) for m in chain if (errs := epr_meta.check_meta(m))]
    if problems:
        detail = "; ".join(f"{m}: {', '.join(e)}" for m, e in problems)
        if target_is_manifest:
            advisories.append(f"[.epr-meta] malformed governance manifest(s) — {detail}. "
                              "(editing an .epr-meta is never blocked, so you can fix it.)")
        else:
            _emit_ask(f"governance manifest malformed — {detail}. Fix the manifest to restore full "
                      "governance here; proceeding now requires confirmation.")

    metas = [(m, epr_meta.load_meta(m)) for m in chain]
    # Recursion-guard surface 6.2: a governed subtree whose cascade reached the repo/depth bound
    # without a `root: true` constitutional base is a misconfiguration. v1 ADVISES (fail-open
    # friendly); the spec's stricter `deny` is a hardening once the root is repo-wide.
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
