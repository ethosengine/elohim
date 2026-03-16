#!/usr/bin/env python3
"""
Post-edit P2P schema audit hook for Claude Code.
Scans Rust migrations, models, views, and routes for P2P anti-patterns.

Non-blocking: emits informational warnings via additionalContext.

Hook Type: PostToolUse
Matcher: Edit|Write
"""

import json
import os
import re
import sys
from fnmatch import fnmatch

# Paths this hook cares about
AUDIT_PATTERNS = [
    '*/migrations/**/up.sql',
    '*/migrations/**/down.sql',
    '*/db/models.rs',
    '*/views.rs',
    '*/routes/*.rs',
]

# Entity name patterns that should be notarized (DHT-backed)
NOTARIZED_ENTITIES = [
    'content', 'economic_event', 'rea_commitment', 'human',
    'human_relationship', 'attestation', 'learning_path',
    'stewardship', 'agreement', 'economic_resource',
]


def matches_audit_path(relative_path: str) -> str | None:
    """Check if file matches any audit pattern. Returns the type."""
    for pattern in AUDIT_PATTERNS:
        if fnmatch(relative_path, pattern):
            if 'migrations' in relative_path:
                return 'migration'
            if 'models.rs' in relative_path:
                return 'model'
            if 'views.rs' in relative_path:
                return 'view'
            if 'routes/' in relative_path:
                return 'route'
    return None


def audit_migration(lines: list[str]) -> list[str]:
    """Audit SQL migration for P2P anti-patterns."""
    warnings = []
    full_text = ''.join(lines).lower()

    for m in re.finditer(
        r'create\s+table\s+(?:if\s+not\s+exists\s+)?(\w+)',
        full_text,
    ):
        table_name = m.group(1)

        is_notarized = any(entity in table_name for entity in NOTARIZED_ENTITIES)

        if is_notarized and 'dht_anchor_hash' not in full_text:
            line_num = full_text[: m.start()].count('\n') + 1
            warnings.append(
                f'  L{line_num}: CREATE TABLE {table_name} — notarized entity pattern '
                f'without dht_anchor_hash column. Add dht_anchor_hash TEXT NOT NULL '
                f'or document why this is operational-only.'
            )

    if 'entity_id' in full_text and 'cid' not in full_text and 'hash' not in full_text:
        warnings.append(
            '  entity_id column without CID/hash reference — '
            'should entity_id be a content address?'
        )

    return warnings


def audit_model(lines: list[str]) -> list[str]:
    """Audit Rust model structs for P2P anti-patterns."""
    warnings = []

    for i, line in enumerate(lines):
        struct_match = re.search(r'pub\s+struct\s+(\w+)', line)
        if not struct_match:
            continue

        struct_name = struct_match.group(1)

        # Look ahead for id field and dht_anchor_hash
        lookahead = ''.join(lines[i : min(len(lines), i + 30)])
        has_id = bool(re.search(r'pub\s+id\s*:', lookahead))
        has_anchor = 'dht_anchor_hash' in lookahead

        if has_id and not has_anchor:
            is_notarized = any(
                entity in struct_name.lower()
                for entity in NOTARIZED_ENTITIES
            )
            if is_notarized:
                warnings.append(
                    f'  L{i + 1}: struct {struct_name} has id field without '
                    f'dht_anchor_hash — notarized entity needs DHT anchor'
                )

    return warnings


def audit_view(lines: list[str]) -> list[str]:
    """Audit View structs at the API boundary."""
    warnings = []

    for i, line in enumerate(lines):
        view_match = re.search(r'pub\s+struct\s+(\w+View)', line)
        if not view_match:
            continue

        view_name = view_match.group(1)

        lookahead = ''.join(lines[i : min(len(lines), i + 30)])
        has_id = bool(re.search(r'pub\s+id\s*:', lookahead))
        has_anchor = 'dht_anchor_hash' in lookahead

        if has_id and not has_anchor:
            is_notarized = any(
                entity in view_name.lower()
                for entity in NOTARIZED_ENTITIES
            )
            if is_notarized:
                warnings.append(
                    f'  L{i + 1}: {view_name} exposes id without dht_anchor_hash — '
                    f'clients cannot verify DHT provenance'
                )

    return warnings


def audit_route(lines: list[str]) -> list[str]:
    """Audit route handlers for missing DHT context."""
    warnings = []

    for i, line in enumerate(lines):
        fn_match = re.search(r'pub\s+async\s+fn\s+(\w+)', line)
        if not fn_match:
            continue

        fn_name = fn_match.group(1)

        # Look at surrounding context for DHT references
        window_start = max(0, i - 5)
        window_end = min(len(lines), i + 20)
        window = ''.join(lines[window_start:window_end]).lower()

        is_crud = any(
            verb in fn_name.lower()
            for verb in ['create', 'update', 'delete', 'put', 'post']
        )

        has_dht_context = any(
            term in window
            for term in [
                'coordinator', 'zome', 'dht', 'entry_type',
                'post_commit', 'projection', 'operational',
            ]
        )

        if is_crud and not has_dht_context:
            warnings.append(
                f'  L{i + 1}: {fn_name} — write handler without DHT context. '
                f'Is this a projection write or direct storage?'
            )

    return warnings


def main():
    try:
        data = json.load(sys.stdin)

        tool_input = data.get('tool_input', {})
        file_path = tool_input.get('file_path', '')

        if not file_path:
            sys.exit(0)

        project_dir = os.environ.get('CLAUDE_PROJECT_DIR', '/projects/elohim')

        try:
            relative_path = os.path.relpath(file_path, project_dir)
        except ValueError:
            relative_path = file_path

        file_type = matches_audit_path(relative_path)
        if not file_type:
            sys.exit(0)

        try:
            with open(file_path) as f:
                lines = f.readlines()
        except (FileNotFoundError, PermissionError):
            sys.exit(0)

        auditors = {
            'migration': audit_migration,
            'model': audit_model,
            'view': audit_view,
            'route': audit_route,
        }

        auditor = auditors.get(file_type)
        if not auditor:
            sys.exit(0)

        warnings = auditor(lines)

        if warnings:
            basename = os.path.basename(file_path)
            msg = (
                f'[P2P SCHEMA AUDIT] {basename}:\n'
                + '\n'.join(warnings[:10])
                + '\n  Reference: .claude/skills/p2p-design-gate/SKILL.md'
            )
            result = {
                'hookSpecificOutput': {
                    'hookEventName': 'PostToolUse',
                    'additionalContext': msg,
                }
            }
            print(json.dumps(result))

        sys.exit(0)

    except json.JSONDecodeError:
        sys.exit(0)
    except Exception as e:
        print(f'p2p-schema-audit hook error: {e}', file=sys.stderr)
        sys.exit(1)


if __name__ == '__main__':
    main()
