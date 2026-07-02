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


def get_memory_budget(project_dir: str) -> str:
    """Always-on memory budget headline (deterministic; from placement-audit.py --headline).

    Caches its stdout to a per-project /tmp file so delivery-gate.py (same SessionStart) can
    reuse it instead of re-running the same heavy audit a second time (fail-open on both sides).
    """
    import subprocess
    audit = os.path.join(project_dir, '.claude', 'scripts', 'memory-kit', 'placement-audit.py')
    if not os.path.exists(audit):
        return ""
    try:
        r = subprocess.run([sys.executable, audit, '--headline'],
                           capture_output=True, text=True, timeout=25)
        out = r.stdout.strip()
        if out:
            try:
                with open(_headline_cache_path(project_dir), 'w', encoding='utf-8') as fh:
                    fh.write(out)
            except OSError:
                pass  # cache is a bonus; never fail the budget fetch on a write error
        return out
    except Exception:
        return ""


def get_spine_status(project_dir: str) -> str:
    """Delivery-spine headline (deterministic; from spine-status.py --headline).
    The spine (genesis/manifests/spine.yaml) is the session's selection surface:
    open on the top red contract instead of re-synthesizing the corpus."""
    import subprocess
    spine = os.path.join(project_dir, '.claude', 'scripts', 'spine-status.py')
    if not os.path.exists(spine):
        return ""
    try:
        r = subprocess.run([sys.executable, spine, '--headline'],
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

        # Delivery spine — the selection surface (top red contract) that makes
        # session start a selection problem, not a synthesis problem.
        spine = get_spine_status(project_dir)
        if spine:
            context_parts.append(spine)
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
