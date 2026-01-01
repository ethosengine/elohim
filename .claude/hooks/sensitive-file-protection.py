#!/usr/bin/env python3
"""
Sensitive File Protection Hook

Blocks or warns before modifying sensitive files like .env, credentials,
secrets directories, and critical configuration files.

Hook Type: PreToolUse
Matcher: Edit|Write
"""
import json
import sys
import os
import re
from pathlib import Path

# Files that should be BLOCKED (never edit automatically)
BLOCKED_PATTERNS = [
    r'\.env$',
    r'\.env\.[^/]+$',
    r'credentials\.json$',
    r'secrets?\.json$',
    r'\.pem$',
    r'\.key$',
    r'/\.ssh/',
    r'/secrets/',
    r'/private/',
    r'id_rsa',
    r'id_ed25519',
]

# Files that should trigger a WARNING (proceed with caution)
WARN_PATTERNS = [
    r'Jenkinsfile$',           # May contain credential references
    r'\.gitlab-ci\.yml$',
    r'docker-compose.*\.yml$',
    r'Dockerfile$',
    r'/manifests/.*\.ya?ml$',  # Kubernetes manifests
    r'package-lock\.json$',    # Usually auto-generated
    r'Cargo\.lock$',           # Usually auto-generated
    r'flake\.lock$',           # Nix lock file
]

# Specific patterns in content that indicate secrets
SECRET_CONTENT_PATTERNS = [
    r'password\s*[=:]\s*["\'][^"\']+["\']',
    r'api[_-]?key\s*[=:]\s*["\'][^"\']+["\']',
    r'secret\s*[=:]\s*["\'][^"\']+["\']',
    r'token\s*[=:]\s*["\'][^"\']+["\']',
    r'private[_-]?key',
    r'BEGIN\s+(RSA|EC|OPENSSH)\s+PRIVATE\s+KEY',
]


def matches_pattern(file_path: str, patterns: list) -> tuple[bool, str]:
    """Check if file path matches any pattern."""
    for pattern in patterns:
        if re.search(pattern, file_path, re.IGNORECASE):
            return True, pattern
    return False, ""


def check_content_for_secrets(content: str) -> list[str]:
    """Check if content contains potential secrets."""
    found = []
    for pattern in SECRET_CONTENT_PATTERNS:
        if re.search(pattern, content, re.IGNORECASE):
            found.append(pattern)
    return found


def main():
    try:
        # Read hook input from stdin
        data = json.load(sys.stdin)

        tool_name = data.get('tool_name', '')
        tool_input = data.get('tool_input', {})
        file_path = tool_input.get('file_path', '')

        if not file_path:
            sys.exit(0)

        # Get relative path for cleaner output
        project_dir = os.environ.get('CLAUDE_PROJECT_DIR', '/projects/elohim')
        try:
            rel_path = os.path.relpath(file_path, project_dir)
        except ValueError:
            rel_path = file_path

        # Check for blocked patterns
        is_blocked, blocked_pattern = matches_pattern(file_path, BLOCKED_PATTERNS)
        if is_blocked:
            output = {
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "permissionDecision": "deny",
                    "permissionDecisionReason": f"BLOCKED: '{rel_path}' matches sensitive file pattern '{blocked_pattern}'. This file likely contains secrets or credentials and should not be modified automatically. If you need to modify this file, please ask the user to do it manually."
                }
            }
            print(json.dumps(output))
            sys.exit(0)

        # Check for warning patterns
        is_warning, warn_pattern = matches_pattern(file_path, WARN_PATTERNS)
        if is_warning:
            # Don't block, but add context
            output = {
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "additionalContext": f"CAUTION: '{rel_path}' is a sensitive configuration file (matched: {warn_pattern}). Proceed carefully and verify changes don't expose secrets."
                }
            }
            print(json.dumps(output))
            sys.exit(0)

        # For Write operations, check if new content contains secrets
        if tool_name == 'Write':
            content = tool_input.get('content', '')
            secret_patterns = check_content_for_secrets(content)
            if secret_patterns:
                output = {
                    "hookSpecificOutput": {
                        "hookEventName": "PreToolUse",
                        "permissionDecision": "ask",
                        "permissionDecisionReason": f"WARNING: The content being written to '{rel_path}' appears to contain sensitive data (matched patterns: {', '.join(secret_patterns[:3])}). Please confirm this is intentional."
                    }
                }
                print(json.dumps(output))
                sys.exit(0)

        # No issues found
        sys.exit(0)

    except json.JSONDecodeError:
        sys.exit(0)
    except Exception as e:
        print(f"sensitive-file-protection hook error: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
