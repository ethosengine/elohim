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
    r'(^|/)\.npmrc$',          # registry _authToken lives here
    r'(^|/)\.cargo/(config|credentials)(\.toml)?$',  # cargo registry token
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
      refresh[_-]?token|auth[_-]?token|client[_-]?secret|private[_-]?key|token
    )(?:[_-][a-z0-9]+)*["']?\s*(?:=|:)\s*
    (?P<quote>["']?)(?:(?:Bearer|Basic|Token)\s+)?(?P<value>[^\s,"'\]}#;]+)(?P=quote)
    ''',
)
# An unquoted capture is syntactically ambiguous: it is equally likely to be a bare code
# expression (`self.args.jwt_secret.as_ref()?`, `process.env.API_KEY`, a type name) as it is
# to be a real credential. .env / YAML / shell all write live credentials unquoted, so the
# unquoted branch cannot simply be dropped — it is narrowed instead. Generated credential
# material is drawn from base64/base64url/hex alphabets and effectively always carries a
# digit; code expressions carry call/index/path punctuation and usually no digit.
# Admits generated credential alphabets AND hand-typed password punctuation, while still
# excluding the call/index/generic/path punctuation that makes a value a code expression:
# ( ) [ ] < > : ; \ | ? and whitespace never appear here.
CREDENTIAL_CHARSET = re.compile(r"^[A-Za-z0-9+/=_.~!@$%^&*-]+$")
PLACEHOLDER_WORDS = {
    "", "changeme", "change-me", "dummy", "example", "fake", "fixture", "placeholder",
    "redacted", "replace-me", "secret", "test", "todo", "unset", "your-key-here",
}
PLACEHOLDER_FRAGMENTS = (
    "example", "dummy", "fake", "fixture", "placeholder", "redact", "test-",
    "change-me", "change_me", "changeme", "replace-me", "replace_with", "your-",
)


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


def _has_generated_entropy(value: str) -> bool:
    """Word-slugs are not credentials, however long they are.

    `register-confirm-password` (a data-testid), `optional-bearer-token` (doc prose) and
    `correct-horse-battery-staple` (a unit fixture) all clear the length and character-class
    bars while being ordinary hyphenated English. Generated credential material carries a
    digit or mixes case; hyphenated identifiers carry neither.
    """
    return any(c.isdigit() for c in value) or (
        any(c.islower() for c in value) and any(c.isupper() for c in value)
    )


def _segment_is_credential_ish(segment: str) -> bool:
    """One dot-separated segment that looks generated rather than like an identifier.

    A digit is necessary but not sufficient — it must also either mix in uppercase (a JWT
    segment) or be long enough that no one typed it as a name (a hex token body). This is
    what separates `NpmToken.9f3a1c7e...` (a real registry token) from `process.env.KEY`.
    """
    if not any(c.isdigit() for c in segment):
        return False
    return any(c.isupper() for c in segment) or len(segment) >= 16


def _unquoted_looks_like_credential(value: str) -> bool:
    """Extra shape evidence demanded of an UNQUOTED value before it may interrupt.

    Quoted values are unambiguous string literals and skip this. Unquoted ones must look
    like generated credential material rather than code: credential alphabet only (no call,
    index, generic, or path-separator punctuation), at least one digit, and — for dotted
    values — at least one generated-looking segment, so `self.args.jwt_secret` and
    `process.env.DOORWAY_API_KEY` stay silent while `eyJhbGciOi....` and `NpmToken.9f3a...`
    do not.

    Known trade: an all-alphabetic unquoted passphrase (`DB_PASSWORD=correctHorseBattery`)
    stays advisory-only, because dropping the digit requirement re-admits every bare type
    name and env-var path in the tree. Quoted passphrases are still caught.
    """
    if not CREDENTIAL_CHARSET.match(value):
        return False
    if not any(char.isdigit() for char in value):
        return False
    # '=' is base64 PADDING, so it is terminal. An interior '=' means the capture ran into
    # a query string (`token=transfer-1&tab=usage`), not a credential.
    if "=" in value.rstrip("="):
        return False
    if "." in value:
        return any(_segment_is_credential_ish(seg) for seg in value.split("."))
    return True


def _assignment_is_secret(match: "re.Match") -> bool:
    value = match.group("value")
    if not _looks_like_live_value(value):
        return False
    if not _has_generated_entropy(value):
        return False
    return bool(match.group("quote")) or _unquoted_looks_like_credential(value)


def check_content_for_secrets(content: str) -> list[str]:
    """Return redaction-safe IDs for high-confidence secret material only."""
    found = []
    if PRIVATE_KEY_BLOCK.search(content):
        found.append("private-key-block")
    for detector_id, pattern in LIVE_TOKEN_PATTERNS.items():
        if pattern.search(content):
            found.append(detector_id)
    if any(_assignment_is_secret(match) for match in ASSIGNMENT.finditer(content)):
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
