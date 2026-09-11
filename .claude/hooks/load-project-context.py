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
    placement-audit --headline runs ONCE per SessionStart instead of once per consumer."""
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


def get_habits_status(project_dir: str) -> str:
    """Delivery-habits headline (deterministic; from habits-status.py --headline).
    The habits (genesis/manifests/habits.yaml) is the session's selection surface:
    open on the top red contract instead of re-synthesizing the corpus."""
    import subprocess
    habits = os.path.join(project_dir, '.claude', 'scripts', 'habits-status.py')
    if not os.path.exists(habits):
        return ""
    try:
        r = subprocess.run([sys.executable, habits, '--headline'],
                           capture_output=True, text=True, timeout=10)
        return r.stdout.strip()
    except Exception:
        return ""


def get_saga_status(project_dir: str) -> str:
    """Resiliency-saga headline (deterministic; from saga-status.py, no args = one-liner).
    Sibling of get_habits_status: same subprocess/timeout/silent-failure posture — a failed
    or slow saga-status.py emits nothing rather than blocking session start."""
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

        seed_memory_injection_flag(str(data.get('session_id') or 'nosess'))

        context_parts = []

        # Always-on memory budget (deterministic) — fetched UNCONDITIONALLY (before any
        # schema/relationship gating) so every session opens knowing the state of the memory
        # surface + what's testable, per genesis/docs/PLACEMENT.md.
        budget = get_memory_budget(project_dir)
        if budget:
            context_parts.append(budget)
            context_parts.append("")

        # Delivery habits — the selection surface (top red contract) that makes
        # session start a selection problem, not a synthesis problem.
        habits = get_habits_status(project_dir)
        if habits:
            context_parts.append(habits)
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

        if not context_parts:
            sys.exit(0)

        # Output context for Claude
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
