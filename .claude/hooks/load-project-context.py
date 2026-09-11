#!/usr/bin/env python3
"""
Load Project Context Hook

Runs at session start to provide Claude with project schema knowledge.
Loads schemas from file-relationships.json so Claude knows about
ContentNode, PathMetadata, etc. without needing to read the model files.

Hook Type: SessionStart
"""
import json
import sys
import os

RECALL_TIMEOUT_S = 6  # the measured budget the BOOTSTRAP block's `recall open` call must fit inside

def load_relationships(project_dir: str) -> dict:
    """Load the file relationships configuration."""
    rel_path = os.path.join(project_dir, '.claude', 'file-relationships.json')
    if not os.path.exists(rel_path):
        return {}

    with open(rel_path, 'r') as f:
        return json.load(f)

def format_schema_summary(schemas: dict) -> str:
    """Format schemas for context injection."""
    if not schemas:
        return ""

    lines = ["ELOHIM PROJECT SCHEMAS:"]
    lines.append("")

    for name, info in schemas.items():
        lines.append(f"• {name}: {info.get('description', 'No description')}")
        lines.append(f"  Defined in: {info.get('definedIn', 'Unknown')}")
        fields = info.get('fields', [])
        if fields:
            lines.append(f"  Fields: {', '.join(fields[:8])}")
            if len(fields) > 8:
                lines.append(f"          ...and {len(fields) - 8} more")
        lines.append("")

    return "\n".join(lines)

def get_sync_relationships_summary(relationships: dict) -> str:
    """Summarize file relationships for context."""
    lines = ["FILE SYNC RELATIONSHIPS:"]
    lines.append("")

    for group_name, group in relationships.get('relationships', {}).items():
        desc = group.get('description', group_name)
        lines.append(f"• {group_name}: {desc}")

        if 'skill' in group:
            lines.append(f"  Skill: {group['skill']}")
        if 'cli' in group:
            lines.append(f"  CLI: {group['cli']}")

        sync_rules = group.get('sync_rules', [])
        if sync_rules:
            lines.append(f"  Sync rules: {len(sync_rules)} patterns tracked")

        lines.append("")

    return "\n".join(lines)

def _headline_cache_path(project_dir: str) -> str:
    """A per-project /tmp path both this hook and delivery-gate.py agree on, so the heavy
    `epr flow report --headline` runs ONCE per SessionStart instead of once per consumer."""
    import re
    slug = re.sub(r'[^A-Za-z0-9]+', '-', project_dir).strip('-')
    return f"/tmp/claude-headline-{slug}.txt"


def _observation_module(project_dir: str):
    """The hooks' shared emitter, imported by path so this works however the hook was launched."""
    sys.path.insert(0, os.path.join(project_dir, '.claude', 'hooks'))
    try:
        import _observation
        return _observation
    except Exception:
        return None


def _epr_headline(project_dir: str) -> str:
    """`epr flow report --headline` — the native projection of the declared bounds.

    The bounds themselves live in .claude/epr-meta/{measures,policies}.yaml; this verb folds
    each one against the observations the drift hooks append and prints the same headline
    lines the SessionStart headline is made of. Returns "" when the verb is absent or fails;
    there is no second producer to fall back to. Binary resolution: $EPR_BIN, gate target, PATH.
    """
    import subprocess
    obs = _observation_module(project_dir)
    binary = obs.resolve_bin() if obs else None
    if not binary:
        return ""
    try:
        r = subprocess.run([binary, 'flow', 'report', '--headline', '--root', project_dir],
                           capture_output=True, text=True, timeout=25)
        return r.stdout.strip() if r.returncode == 0 else ""
    except Exception:
        return ""


def get_memory_budget(project_dir: str) -> str:
    """Always-on memory budget headline — `epr flow report --headline` is the OWNER.

    The last-resort kit re-run went at station six round (a); the PRODUCER BRIDGE went at
    round (b) (2026-09-11), when the last two bridged values — `memkit-report-tier-mb@1` and
    `mempalace-surfaces-changed@1` — got native producers (the first `status: superseded` with
    the report tier it measured, the second `derive: files-newer-than` over a declared surface
    walk). Every one of the five slots is now derived, so a bridge would DOUBLE a value rather
    than supply one. Caches its stdout to a per-project /tmp file so delivery-gate.py (same
    SessionStart) reuses it instead of recomputing. Fail-open: any failure returns ''.
    """
    out = _epr_headline(project_dir)
    if out:
        try:
            with open(_headline_cache_path(project_dir), 'w', encoding='utf-8') as fh:
                fh.write(out)
        except OSError:
            pass  # cache is a bonus; never fail the budget fetch on a write error
    return out


def _epr_bootstrap_block(project_dir: str, data: dict) -> str:
    """The SessionStart BOOTSTRAP block: the `minimal` lens of
    `epr flow memory recall open --purpose bootstrap` — the bootstrapping head declared in
    `.claude/hooks/.epr-meta` (rule `bootstrapping-head-is-recall-open`).

    This is the ONLY orientation this hook renders. The former `get_habits_status` (a bespoke
    `habits-status.py --headline` re-scan) is RETIRED, not kept beside this: the `open` call's
    `minimal` lens already carries a `Bootstrap: top red: <id> — <check>` line drawn from the
    same habits register, and keeping both would be exactly the "a hook renders it at a lens
    and never derives a second orientation" drift the rule exists to catch.

    Landing: this hook is SessionStart and SYNCHRONOUS, so its `hookSpecificOutput.
    additionalContext` is the documented landing path (verified empirically — see
    `main()`'s comment). The caller folds this block's return value INTO that same JSON
    wrapper alongside the memory-budget headline; nothing here is printed on its own.

    Session-id and binary resolution both come from `_observation` (`resolve_bin`,
    `bootstrap_session_id`) — the same shared helper `run-projection.py` uses, so the two
    hooks' fallback session ids always agree. When `_observation` itself cannot be imported
    (a missing/broken copy in this project dir), there is no binary to resolve either, so the
    same "no epr binary resolved" absence line covers both cases without a separate check.

    Honest absence, never a fallback renderer: a missing binary, a non-zero exit, or a run past
    the 6-second budget all return exactly one line, `bootstrap: skipped — <reason>`.
    """
    import subprocess
    obs = _observation_module(project_dir)
    binary = obs.resolve_bin() if obs else None
    if not binary:
        return "bootstrap: skipped — no epr binary resolved ($EPR_BIN, gate target, PATH)"
    session_id = obs.bootstrap_session_id(project_dir, data)
    try:
        r = subprocess.run(
            [binary, 'flow', 'memory', 'recall', 'open',
             '--purpose', 'bootstrap', '--lens', 'minimal',
             '--session', f'bootstrap-{session_id}', '--root', project_dir],
            capture_output=True, text=True, timeout=RECALL_TIMEOUT_S,
        )
    except subprocess.TimeoutExpired:
        return f"bootstrap: skipped — recall open exceeded the {RECALL_TIMEOUT_S}s budget"
    except Exception as exc:  # noqa: BLE001 — a SessionStart hook must never crash the session
        return f"bootstrap: skipped — {exc}"
    if r.returncode != 0:
        reason_lines = (r.stderr or r.stdout or "").strip().splitlines()
        reason = reason_lines[0] if reason_lines else f"exit {r.returncode}"
        return f"bootstrap: skipped — {reason}"
    return r.stdout.rstrip("\n")


def get_saga_status(project_dir: str) -> str:
    """Resiliency-saga headline (deterministic; from saga-status.py, no args = one-liner).
    A DIFFERENT derivation from the bootstrap block (the ten dataplane chapters' green/frontier
    state, not the habits register's top red) — same subprocess/timeout/silent-failure posture:
    a failed or slow saga-status.py emits nothing rather than blocking session start."""
    import subprocess
    saga = os.path.join(project_dir, '.claude', 'scripts', 'saga-status.py')
    if not os.path.exists(saga):
        return ""
    try:
        r = subprocess.run([sys.executable, saga],
                           capture_output=True, text=True, timeout=10)
        return r.stdout.strip()
    except Exception:
        return ""


def seed_memory_injection_flag(session_id: str) -> None:
    """Pre-seed pre-tool-memory.py's flag for the MAIN session tree.

    The harness injects MEMORY.md natively for the main session (claudeMd
    system-reminder), so the PreToolUse injector would only duplicate ~22KB.
    SessionStart runs in the same process tree as the main loop's hooks, so
    touching the (session_id, ppid) flag here suppresses the duplicate while
    leaving subagent trees (different ppid) to get their injection.
    """
    try:
        from pathlib import Path
        Path(f"/tmp/claude-memory-loaded-{session_id[:64]}-{os.getppid()}").touch()
    except OSError:
        pass


def main():
    try:
        # Read hook input from stdin
        data = json.load(sys.stdin)

        # Get project directory
        project_dir = os.environ.get('CLAUDE_PROJECT_DIR', '/projects/elohim')

        session_id = str(data.get('session_id') or 'nosess')
        seed_memory_injection_flag(session_id)

        context_parts = []

        # Always-on memory budget (deterministic) — fetched UNCONDITIONALLY (before any
        # schema/relationship gating) so every session opens knowing the state of the memory
        # surface + what's testable, per genesis/docs/PLACEMENT.md.
        budget = get_memory_budget(project_dir)
        if budget:
            context_parts.append(budget)
            context_parts.append("")

        # Resiliency-saga headline — the ten dataplane chapters' green/frontier state
        # (genesis/a2o/features/dataplane/resiliency-saga/), joined from flows.jsonl +
        # the (often-absent-locally) dataplane sprint-report. See saga-sync.sh to pull
        # the report down and refresh the flow state.
        saga = get_saga_status(project_dir)
        if saga:
            context_parts.append(saga)
            context_parts.append("")

        # Schema + sync-relationship data lives in .claude/file-relationships.json and is
        # re-delivered at edit time by the sync-check hook; a one-line pointer replaces the
        # ~2.5KB static dump every session paid before 2026-07-02.
        relationships = load_relationships(project_dir)
        if relationships:
            context_parts.append(
                "SYNC HOOKS ACTIVE: edit-time hooks surface related-file reminders from "
                ".claude/file-relationships.json (schemas + sync rules; read it when you need the map)."
            )
            context_parts.append("")

        # The bootstrap block: the `minimal` lens of `epr flow memory recall open --purpose
        # bootstrap` (the bootstrapping head; see `.claude/hooks/.epr-meta`, rule
        # `bootstrapping-head-is-recall-open`). This hook is SessionStart and SYNCHRONOUS
        # (unlike run-projection.py's `--event session`, which is registered `async: true` and
        # so is never relied on to land anything) — the ONE documented landing path for a
        # SessionStart hook's output is the `hookSpecificOutput.additionalContext` JSON wrapper,
        # so the BOOTSTRAP block is folded INTO that same wrapper below, never printed as a
        # separate plain-text section. Always computed (context_parts is never empty once this
        # is appended, even on the honest-absence `bootstrap: skipped` line) — no second
        # orientation is derived here (no habits.yaml re-scan; see the retired
        # `get_habits_status`, replaced by `_epr_bootstrap_block`).
        bootstrap_block = _epr_bootstrap_block(project_dir, data)
        context_parts.append("BOOTSTRAP:")
        context_parts.append(bootstrap_block)

        # Output context for Claude — one JSON hookSpecificOutput wrapper, always emitted (the
        # bootstrap block above guarantees context_parts is never empty).
        output = {
            "hookSpecificOutput": {
                "hookEventName": "SessionStart",
                "additionalContext": "\n".join(context_parts)
            }
        }
        print(json.dumps(output))

    except json.JSONDecodeError:
        sys.exit(0)
    except Exception as e:
        print(f"load-project-context hook error: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()
