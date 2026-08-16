#!/usr/bin/env python3
"""Regression tests for the pilot interruption boundary in sensitive-file-protection.py."""

import importlib.util
import json
import subprocess
import sys
from pathlib import Path

HOOK = Path(__file__).resolve().parents[1] / "sensitive-file-protection.py"
spec = importlib.util.spec_from_file_location("sensitive_file_protection", HOOK)
hook = importlib.util.module_from_spec(spec)
spec.loader.exec_module(hook)


def run(path: str, content: str) -> dict:
    payload = {"tool_name": "Write", "tool_input": {"file_path": path, "content": content}}
    result = subprocess.run(
        [sys.executable, str(HOOK)],
        input=json.dumps(payload),
        capture_output=True,
        text=True,
        check=True,
    )
    return json.loads(result.stdout)["hookSpecificOutput"] if result.stdout.strip() else {}


def main() -> int:
    cases = [
        ("private_key identifier is clean", "src/model.py", "private_key: str", "silent"),
        ("comment is clean", "README.md", "# private_key is supplied by the operator", "silent"),
        ("obvious test credential is clean", "tests/x.py", "api_key='test-api-key'", "silent"),
        ("blank env example only advises", ".env.example", "API_KEY=", "advise"),
        ("public certificate only advises", "cert.pem", "-----BEGIN CERTIFICATE-----", "advise"),
        ("generic private PEM asks", "fixture.txt", "-----BEGIN PRIVATE KEY-----\nabc", "ask"),
        ("openssh private PEM asks", "fixture.txt", "-----BEGIN OPENSSH PRIVATE KEY-----", "ask"),
        (
            "provider token asks",
            "config.ts",
            "token='ghp_abcdefghijklmnopqrstuvwxyzABCDEF123456'",
            "ask",
        ),
        (
            "json secret asks",
            "config.json",
            '"client_secret": "AbCDef0123456789+/LongValue"',
            "ask",
        ),
        (
            "yaml secret in manifest asks",
            "genesis/manifests/x.yaml",
            "password: AbCDef0123456789+/LongValue",
            "ask",
        ),
        (
            "jenkins credential reference only advises",
            "Jenkinsfile",
            "credentials('deploy-id')",
            "advise",
        ),
    ]
    failures = []
    for label, path, content, expected in cases:
        output = run(path, content)
        actual = (
            "ask" if output.get("permissionDecision") == "ask"
            else "advise" if output.get("additionalContext")
            else "silent"
        )
        redacted = content not in output.get("permissionDecisionReason", "")
        ok = actual == expected and redacted
        print(f"  {'PASS' if ok else 'FAIL'}: {label}: {actual}")
        if not ok:
            failures.append((label, expected, actual))
    if failures:
        print(f"FAILURES: {failures}")
        return 1
    print(f"{len(cases)} sensitive-file boundary assertions passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
