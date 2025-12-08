#!/usr/bin/env python3
"""
Sync Check Hook

Runs after Edit/Write operations to check if related files need updates.
Uses .claude/file-relationships.json to determine sync requirements.

Hook Type: PostToolUse
Matcher: Edit|Write
"""
import json
import sys
import os
from pathlib import Path
from fnmatch import fnmatch

def load_relationships(project_dir: str) -> dict:
    """Load the file relationships configuration."""
    rel_path = os.path.join(project_dir, '.claude', 'file-relationships.json')
    if not os.path.exists(rel_path):
        return {}

    with open(rel_path, 'r') as f:
        return json.load(f)

def get_relative_path(file_path: str, project_dir: str) -> str:
    """Get path relative to project directory."""
    try:
        return os.path.relpath(file_path, project_dir)
    except ValueError:
        return file_path

def find_matching_rules(relative_path: str, relationships: dict) -> list:
    """Find sync rules that match the edited file."""
    matching = []

    for group_name, group in relationships.get('relationships', {}).items():
        for rule in group.get('sync_rules', []):
            pattern = rule.get('trigger_pattern', '')
            # Check if the file matches the pattern
            if fnmatch(relative_path, f'**/{pattern}') or fnmatch(relative_path, pattern):
                matching.append({
                    'group': group_name,
                    'pattern': pattern,
                    'notify': rule.get('notify', []),
                    'message': rule.get('message', 'Related files may need updates')
                })

    return matching

def get_file_descriptions(notify_list: list, group: dict) -> list:
    """Get human-readable descriptions of files that need updates."""
    descriptions = []

    for target in notify_list:
        if target == 'skill':
            skill_path = group.get('skill', '')
            if skill_path:
                descriptions.append(f"Skill documentation: {skill_path}")
        elif target == 'cli':
            cli_path = group.get('cli', '')
            if cli_path:
                descriptions.append(f"CLI commands: {cli_path}")
        elif target == 'models':
            for model_type, paths in group.get('models', {}).items():
                if isinstance(paths, list):
                    for p in paths:
                        descriptions.append(f"Model ({model_type}): {p}")

    return descriptions

def check_model_sync(relative_path: str, relationships: dict) -> list:
    """Check if this file is part of a model sync pair."""
    warnings = []

    model_sync = relationships.get('relationships', {}).get('model-sync', {})
    for pair in model_sync.get('pairs', []):
        source = pair.get('source', '')
        target = pair.get('target', '')

        if relative_path.endswith(source) or source in relative_path:
            warnings.append({
                'type': 'model-sync',
                'message': pair.get('message', 'Model sync required'),
                'target': target,
                'direction': pair.get('sync_direction', 'bidirectional')
            })

    return warnings

def main():
    try:
        # Read hook input from stdin
        data = json.load(sys.stdin)

        tool_input = data.get('tool_input', {})
        file_path = tool_input.get('file_path', '')

        if not file_path:
            sys.exit(0)

        # Get project directory from environment
        project_dir = os.environ.get('CLAUDE_PROJECT_DIR', '/projects/elohim')

        # Load relationships configuration
        relationships = load_relationships(project_dir)
        if not relationships:
            sys.exit(0)

        relative_path = get_relative_path(file_path, project_dir)

        # Find matching sync rules
        matching_rules = find_matching_rules(relative_path, relationships)
        model_warnings = check_model_sync(relative_path, relationships)

        if not matching_rules and not model_warnings:
            sys.exit(0)

        # Build context message
        context_parts = []

        if matching_rules:
            context_parts.append("FILE SYNC REMINDER:")
            context_parts.append(f"Modified: {relative_path}")
            context_parts.append("")

            for rule in matching_rules:
                if rule['notify']:
                    context_parts.append(f"• {rule['message']}")
                    group = relationships.get('relationships', {}).get(rule['group'], {})
                    file_descs = get_file_descriptions(rule['notify'], group)
                    for desc in file_descs:
                        context_parts.append(f"  → {desc}")

        if model_warnings:
            if context_parts:
                context_parts.append("")
            context_parts.append("MODEL SYNC WARNING:")
            for warning in model_warnings:
                context_parts.append(f"• {warning['message']}")
                context_parts.append(f"  → Target: {warning['target']}")

        # Output context for Claude
        output = {
            "hookSpecificOutput": {
                "hookEventName": "PostToolUse",
                "additionalContext": "\n".join(context_parts)
            }
        }

        print(json.dumps(output))

    except json.JSONDecodeError:
        # No valid JSON input - just exit
        sys.exit(0)
    except Exception as e:
        # Log error to stderr but don't block
        print(f"sync-check hook error: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()
