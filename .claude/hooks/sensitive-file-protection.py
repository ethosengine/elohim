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

# Sensitive paths are context, not proof. Path names alone never interrupt development; a
# high-confidence content detector below owns the operator boundary. This avoids treating
# `.env.example`, public certificates, test fixtures, and identifiers named `private_key` as
# secrets while still warning the editing agent to inspect the destination carefully.
WARN_PATTERNS = [
    r'\.env(?:\.[^/]+)?$',
    r'credentials\.json$',
    r'secrets?\.json$',
    r'\.(?:pem|key)$',
    r'/\.ssh/',
    r'/secrets/',
    r'/private/',
    r'id_rsa',
    r'id_ed25519',
    r'Jenkinsfile$',           # May contain credential references
    r'\.gitlab-ci\.yml$',
    r'docker-compose.*\.yml$',
    r'Dockerfile$',
    r'/manifests/.*\.ya?ml$',  # Kubernetes manifests
    r'package-lock\.json$',    # Usually auto-generated
    r'Cargo\.lock$',           # Usually auto-generated
    r'flake\.lock$',           # Nix lock file
]

# Detector IDs are safe to disclose in hook output. The matched values never are.
PRIVATE_KEY_BLOCK = re.compile(
    r"-----BEGIN\s+(?:(?:RSA|EC|OPENSSH|DSA|ENCRYPTED)\s+)?PRIVATE\s+KEY-----",
    re.IGNORECASE,
)
LIVE_TOKEN_PATTERNS = {
    "github-token": re.compile(r"\b(?:ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9]{30,}\b"),
    "github-fine-grained-token": re.compile(r"\bgithub_pat_[A-Za-z0-9_]{40,}\b"),
    "aws-access-key": re.compile(r"\b(?:AKIA|ASIA)[A-Z0-9]{16}\b"),
    "slack-token": re.compile(r"\bxox[baprs]-[A-Za-z0-9-]{20,}\b"),
    "stripe-live-key": re.compile(r"\b(?:sk|rk)_live_[A-Za-z0-9]{20,}\b"),
}
ASSIGNMENT = re.compile(
    r'''(?imx)
    ["']?(?P<name>
      password|passwd|pwd|api[_-]?key|secret(?:[_-]?key)?|access[_-]?token|
      refresh[_-]?token|auth[_-]?token|client[_-]?secret|private[_-]?key
    )["']?\s*(?:=|:)\s*
    (?P<quote>["']?)(?P<value>[^\s,"'\]}#;]+)(?P=quote)
    ''',
)
PLACEHOLDER_WORDS = {
    "", "changeme", "change-me", "dummy", "example", "fake", "fixture", "placeholder",
    "redacted", "replace-me", "secret", "test", "todo", "unset", "your-key-here",
}
PLACEHOLDER_FRAGMENTS = ("example", "dummy", "fake", "fixture", "placeholder", "redact", "test-")


def matches_pattern(file_path: str, patterns: list) -> tuple[bool, str]:
    """Check if file path matches any pattern."""
    for pattern in patterns:
        if re.search(pattern, file_path, re.IGNORECASE):
            return True, pattern
    return False, ""


def _looks_like_live_value(value: str) -> bool:
    """Precision-biased secret-value test. Ambiguous short values remain advisory-only."""
    value = value.strip().strip('"\'')
    lowered = value.lower()
    if lowered in PLACEHOLDER_WORDS or any(part in lowered for part in PLACEHOLDER_FRAGMENTS):
        return False
    if value.startswith(("${", "{{", "<")) or lowered.startswith(("env:", "vault:")):
        return False
    if len(value) < 20:
        return False
    classes = sum(
        bool(re.search(pattern, value))
        for pattern in (r"[a-z]", r"[A-Z]", r"\d", r"[^A-Za-z0-9]")
    )
    return classes >= 2 and len(set(value)) >= 8


def check_content_for_secrets(content: str) -> list[str]:
    """Return redaction-safe IDs for high-confidence secret material only."""
    found = []
    if PRIVATE_KEY_BLOCK.search(content):
        found.append("private-key-block")
    for detector_id, pattern in LIVE_TOKEN_PATTERNS.items():
        if pattern.search(content):
            found.append(detector_id)
    if any(_looks_like_live_value(match.group("value")) for match in ASSIGNMENT.finditer(content)):
        found.append("high-entropy-secret-assignment")
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

        # For Write/Edit operations, check if new content contains secrets
        # (Edit carries the pasted text in new_string — the 2026-07-02 review
        # found the Edit path entirely uninspected).
        if tool_name in ('Write', 'Edit'):
            content = tool_input.get('content') or tool_input.get('new_string') or ''
            detector_ids = check_content_for_secrets(content)
            if detector_ids:
                output = {
                    "hookSpecificOutput": {
                        "hookEventName": "PreToolUse",
                        "permissionDecision": "ask",
                        "permissionDecisionReason": (
                            f"CONFIDENTIALITY STOP: the proposed content for '{rel_path}' contains "
                            "high-confidence secret material (detectors: "
                            f"{', '.join(detector_ids[:3])}). The matched value is intentionally "
                            "redacted. Confirm whether writing real credential/private-key material "
                            "is authorized."
                        )
                    }
                }
                print(json.dumps(output))
                sys.exit(0)

        # A sensitive-looking path or deployment config is useful context, but path names and
        # credential references are not secret material. Advise without stopping development.
        is_warning, warn_pattern = matches_pattern(file_path, WARN_PATTERNS)
        if is_warning:
            output = {
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "additionalContext": (
                        f"CAUTION: '{rel_path}' is a sensitive configuration path "
                        f"(matched: {warn_pattern}). No high-confidence secret material was "
                        "detected; proceed carefully."
                    )
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
