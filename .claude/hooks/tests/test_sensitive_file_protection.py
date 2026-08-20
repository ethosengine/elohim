#!/usr/bin/env python3
"""Regression tests for the pilot interruption boundary in sensitive-file-protection.py."""

import importlib.util
import json
import re
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
        # ── The unquoted boundary, pinned in BOTH directions ──────────────
        # .env / YAML / shell write live credentials WITHOUT quotes, so a cure for the
        # code-expression false positives below must not be "require a quote" — that
        # silently disables the detector exactly where secrets are actually pasted.
        (
            "unquoted env secret asks",
            ".env.production",
            "API_KEY=sk_live_AbCDef0123456789xyz",
            "ask",
        ),
        (
            "unquoted shell export secret asks",
            "scripts/deploy.sh",
            "export CLIENT_SECRET=aG9sb2NoYWluU2VjcmV0MTIzNDU2Nzg5",
            "ask",
        ),
        (
            "unquoted hex auth token asks",
            "genesis/manifests/y.yaml",
            "auth_token: 7c9f1a2b3d4e5f60718293a4b5c6d7e8",
            "ask",
        ),
        # ...and the code-expression class it must stay silent on. These are verbatim
        # shapes from the repo corpus (doorway-service JWT plumbing, seeder env reads)
        # that false-positived an "ask" on every edit before the value-shape narrowing.
        (
            "rust field access is clean",
            "doorway/doorway-service/src/server/http.rs",
            "let secret = self.args.jwt_secret.as_ref()?;",
            "silent",
        ),
        (
            "rust generic type annotation is clean",
            "doorway/doorway-service/src/main.rs",
            "auth_token: Arc<RwLock<Option<String>>> = Default::default();",
            "silent",
        ),
        (
            "env var read is clean",
            "genesis/seeder/src/seed.ts",
            "const API_KEY = process.env.DOORWAY_API_KEY;",
            "silent",
        ),
        (
            "type name assignment is clean",
            "genesis/seeder/src/seed-test-admin.ts",
            "const apiKey: CreateApiKeyResponse = await res.json();",
            "silent",
        ),
        (
            # "advise" (not "ask") is the assertion: the manifest PATH still earns the
            # CAUTION, while the content detector correctly declines to interrupt.
            "vault interpolation does not escalate",
            "genesis/manifests/z.yaml",
            "password: ${VAULT_SECRET}",
            "advise",
        ),
        # ── Shapes this repo actually writes credentials into ─────────────
        # Each of these was silent until 2026-08-20 and is grounded in a real file:
        # .npmrc:22, the cargo registry developer-setup runbook, and
        # doorway/doorway-service/.env.example:28-29.
        (
            "registry token with dotted body asks",
            ".npmrc",
            "//nexus.example.com/repository/npm/:_authToken=NpmToken.9f3a1c7e2b8d4f6a0c5e2b7d9f1a3c5e",
            "ask",
        ),
        (
            "scheme-prefixed token asks despite the space",
            ".cargo/credentials.toml",
            'token = "Bearer NpmToken.7f3a9c21bd4e5061a2b3c4d5e6f70819"',
            "ask",
        ),
        (
            "suffixed credential name asks",
            "doorway/doorway-service/.env",
            "API_KEY_ADMIN=Kx7Qm2Vp9Ls4Rt6Yn8Bz3Wc5Hd1Jf0G",
            "ask",
        ),
        (
            "punctuation-bearing password asks",
            ".env.production",
            "NATS_PASSWORD=Tr0ub4dor&3xKlmNopQrs",
            "ask",
        ),
        # ...and the word-slug class that must never interrupt. A data-testid registry is
        # edited constantly; SonarJS already had to be suppressed in-source on this file.
        (
            "data-testid slug is clean",
            "genesis/a2o/src/framework/pages/selectors.ts",
            "CONFIRM_PASSWORD: 'register-confirm-password',",
            "silent",
        ),
        (
            "documentation placeholder is clean",
            "docs/skill.json",
            '"apiKey": "optional-bearer-token"',
            "silent",
        ),
        (
            "change-me scaffold is clean",
            "app/elohim-app/src/environments/environment.prod.ts",
            "API_KEY: 'CHANGE-ME-prod-elohim-auth-2024',",
            "silent",
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
        # A hook that echoed the live token scored 19/19 under the old check, because
        # `content not in ""` is trivially true for every silent/advise case. Assert
        # instead that no credential-shaped RUN from the content appears anywhere in the
        # hook's output — which is what "the matched value is redacted" actually claims.
        emitted = json.dumps(output)
        secretish = re.findall(r"[A-Za-z0-9+/=_.~!@$%^&*-]{12,}", content)
        leaked = [tok for tok in secretish if tok in emitted]
        redacted = not leaked
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
