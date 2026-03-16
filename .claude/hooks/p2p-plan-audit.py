#!/usr/bin/env python3
"""
Post-edit P2P design audit hook for Claude Code.
Scans plan documents (genesis/plans/*.md) for relational-DB anti-patterns
that suggest P2P-native design thinking was skipped.

Non-blocking: emits informational warnings via additionalContext.

Hook Type: PostToolUse
Matcher: Edit|Write
"""

import json
import os
import re
import sys


# Each rule: (flag_pattern, antidote_pattern, warning_message)
# Flag fires if flag_pattern found without antidote_pattern within 10 lines
RULES = [
    (
        r'(?i)PRIMARY\s+KEY',
        r'(?i)dht_anchor_hash|anchor.hash|DHT.anchor',
        'PRIMARY KEY without dht_anchor_hash — notarized entities need a DHT anchor column',
    ),
    (
        r'(?i)\bUUID\b',
        r'(?i)source.chain|DHT|EntryHash|operational|content.address',
        'UUID identity without P2P anchoring discussion — should this be content-addressed?',
    ),
    (
        r'(?i)(?:API\s+endpoint|REST\s+route|GET\s+/|POST\s+/|PUT\s+/|DELETE\s+/)',
        r'(?i)entry.type|coordinator.function|zome|DHT|source.chain',
        'API route without entry type — routes should follow from DHT design, not precede it',
    ),
    (
        r'(?i)(?:CREATE\s+TABLE|new\s+table|add\s+(?:a\s+)?table|schema\b)',
        r'(?i)source.of.truth|DHT|projection|operational|notarized|agent.scoped',
        'New storage schema without source-of-truth declaration',
    ),
    (
        r'(?i)entity_id.*(?:CID|cid|bafk)|store.*(?:CID|cid).*(?:as|in)',
        r'(?i)content.address(?:ed)?|EntryHash|identity.IS',
        'CID used as foreign key — should the entity BE content-addressed instead?',
    ),
]

ANTIDOTE_WINDOW = 10  # lines before/after to search for antidote


def scan_plan(lines: list[str]) -> list[str]:
    """Scan plan lines for P2P anti-patterns."""
    warnings = []

    for i, line in enumerate(lines):
        for flag_re, antidote_re, message in RULES:
            if not re.search(flag_re, line):
                continue

            # Check window around the flag for the antidote
            window_start = max(0, i - ANTIDOTE_WINDOW)
            window_end = min(len(lines), i + ANTIDOTE_WINDOW + 1)
            window = ' '.join(lines[window_start:window_end])

            if not re.search(antidote_re, window):
                warnings.append(f'  L{i + 1}: {message}')

    # Deduplicate (same warning can fire on adjacent lines)
    return list(dict.fromkeys(warnings))


def main():
    try:
        data = json.load(sys.stdin)

        tool_input = data.get('tool_input', {})
        file_path = tool_input.get('file_path', '')

        if not file_path:
            sys.exit(0)

        # Only audit plan documents
        if '/plans/' not in file_path or not file_path.endswith('.md'):
            sys.exit(0)

        try:
            with open(file_path) as f:
                lines = f.readlines()
        except (FileNotFoundError, PermissionError):
            sys.exit(0)

        warnings = scan_plan(lines)

        if warnings:
            basename = os.path.basename(file_path)
            msg = (
                f'[P2P DESIGN AUDIT] {basename}:\n'
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
        print(f'p2p-plan-audit hook error: {e}', file=sys.stderr)
        sys.exit(1)


if __name__ == '__main__':
    main()
