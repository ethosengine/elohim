#!/usr/bin/env python3
# .claude/hooks/epr-meta-resolver.py
"""PreToolUse resolver for the .epr-meta compose-gate. Thin: stdin -> _lib.epr_meta -> verdict JSON.
Fail-open: a guard bug never blocks dev."""
import hashlib
import json
import sys
import time
from pathlib import Path

try:
    import fcntl
except ImportError:  # pragma: no cover — non-POSIX
    fcntl = None

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

# Measure-class findings (the observation tier — never blocks) land here, sentinel-style:
# one line per LIVE fingerprint; a fingerprint PRESENT suppresses re-dispatch; the entry is
# deleted when the finding is drained (file back under ceiling), never parked.
_ARCH_LEDGER_REL = ".claude/data/architecture-findings.jsonl"


def _advice_debounced(root: Path, key: str) -> bool:
    """True if `key` was advised within _ADVICE_WINDOW; records the advice time otherwise.
    Shares the coverage-nudge store so all epr-meta debounce state lives in one place.
    Read-check-write runs under store.locked_update so concurrent sessions can't lose each
    other's debounce keys (worst case on lock timeout: one extra advisory, never a suppression)."""
    try:
        state_p = root / ".claude/data/epr-meta-advice.json"
        now = time.time()
        hit = {"debounced": False}

        def upd(seen):
            seen = seen if isinstance(seen, dict) else {}
            last = seen.get(key, 0)
            if isinstance(last, (int, float)) and (now - last) < _ADVICE_WINDOW:
                hit["debounced"] = True
                return seen
            seen[key] = now
            return seen

        store.locked_update(state_p, upd, {})
        return hit["debounced"]
    except Exception:  # noqa: BLE001 — store trouble → advise rather than suppress
        return False


def _handle_measures(measures, merged, root: Path, fp_path: str) -> list[str]:
    """The measure side-channel: file each hard-ceiling verdict as a fingerprinted architecture
    finding (flag→agent→canon→stasis, the deprecation-sentinel pattern). NEW fingerprint →
    ledger append + dispatch directive; already-filed → debounced one-line citation."""
    notes: list[str] = []
    try:
        rel = str(Path(fp_path).resolve().relative_to(root))
    except (ValueError, OSError):
        rel = fp_path
    for v in measures:
        try:
            fpid = hashlib.sha256(f"{v.rule_id}|{rel}".encode()).hexdigest()[:12]
            ledger = root / _ARCH_LEDGER_REL
            rule = merged["rules"].get(v.rule_id, {})
            agent = (rule.get("measure") or {}).get("dispatch-agent", "general-purpose")
            # Read-check-append under an flock on the ledger fd itself, so concurrent sessions
            # can't both see "not filed" and duplicate the row ("one line per LIVE fingerprint").
            # Non-blocking with bounded retry — a hook never waits on a lock; on timeout we skip
            # (the next edit of the still-over-ceiling file retries).
            ledger.parent.mkdir(parents=True, exist_ok=True)
            is_new = None  # None = lock not acquired
            with open(ledger, "a+", encoding="utf-8", errors="replace") as fh:
                locked = fcntl is None
                if not locked:
                    deadline = time.monotonic() + 0.5
                    while time.monotonic() < deadline:
                        try:
                            fcntl.flock(fh.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
                            locked = True
                            break
                        except OSError:
                            time.sleep(0.02)
                if locked:
                    fh.seek(0)
                    known = set()
                    for ln in fh.read().splitlines():
                        try:
                            known.add(json.loads(ln).get("fp"))
                        except Exception:  # noqa: BLE001
                            continue
                    is_new = fpid not in known
                    if is_new:
                        entry = {"fp": fpid, "rule": v.rule_id, "policy": rule.get("policy-ref"),
                                 "path": rel, "detail": v.reason[:300],
                                 "first_seen": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
                                 "status": "open"}
                        fh.write(json.dumps(entry, ensure_ascii=False) + "\n")
            if is_new is None:
                continue  # lock contention — another session owns this moment; next edit retries
            if not is_new:
                if not _advice_debounced(root, f"arch-fp:{fpid}"):
                    notes.append(
                        f"[.epr-meta measure] `{rel}` remains over its hard LoC ceiling — already "
                        f"filed as architecture finding {fpid} ({_ARCH_LEDGER_REL}, status open). "
                        f"Drain via its backlog plan; do not re-file.")
                continue
            notes.append(
                f"[.epr-meta measure] NEW architecture finding {fpid}: {v.reason} Captured to "
                f"{_ARCH_LEDGER_REL}. DISPATCH NOW (do not derail the current task): launch the "
                f"`{agent}` agent via the Agent tool with run_in_background: true and the prompt "
                f"'Architecture finding {fpid}: `{rel}` is at/over its LoC hard ceiling. Scope its "
                f"module seams and canonicalize a modularization plan into "
                f"genesis/data/timeline/backlog/ (timeline-CONVENTIONS-conformant, "
                f"status-carrying) — plan the restructuring as bounded work; do NOT refactor "
                f"inline. Honor the refactor-safety notes in the policy why "
                f"(registry: .claude/epr-meta/policies.yaml).' If that agent type is unavailable "
                f"this session, use general-purpose with the same prompt. Then continue your "
                f"current task.")
        except Exception:  # noqa: BLE001 — observation tier must never break the gate
            continue
    return notes


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
    cov_nudge = None if epr_meta.is_manifest_path(target) else _coverage_nudge(target)

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
    target_is_manifest = epr_meta.is_manifest_path(target)
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
    root = epr_meta.find_repo_root(target)
    policies, pol_errs = epr_meta.load_policies(root)
    exp_errs = epr_meta.expand_policies(merged, policies)
    advisories += [f"[.epr-meta] {e}" for e in (*pol_errs, *exp_errs)]

    write = {"path": fp, "content": content, "is_new": is_new,
             "is_new_subdir": is_new_subdir}
    verdicts = epr_meta.evaluate(merged, write)

    # Measure side-channel (observation tier — never blocks): hard-ceiling verdicts become
    # fingerprinted architecture findings + a dispatch directive, sentinel-style.
    measures = [v for v in verdicts if v.cls == "measure"]
    if measures:
        advisories += _handle_measures(measures, merged, root, fp)

    # Soft-ceiling injects re-fire on EVERY edit of an over-soft file — debounce per (rule, path)
    # so the nudge informs once per working session instead of nagging each keystroke.
    def _soft_debounced(v) -> bool:
        r = merged["rules"].get(v.rule_id, {})
        return (v.cls == "inject" and "measure" in r
                and _advice_debounced(root, f"loc-soft:{v.rule_id}:{fp}"))

    verdict = epr_meta.combine([v for v in verdicts if not _soft_debounced(v)])
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
