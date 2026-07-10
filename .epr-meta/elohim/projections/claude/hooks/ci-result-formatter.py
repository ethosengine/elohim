#!/usr/bin/env python3
"""
CI Result Formatter - Minimal hook for mcp__jenkins__getBuild failures.
Only injects context when build failed/unstable.

PostToolUse payload carries the MCP result under `tool_response` (NOT
`tool_result` — the 2026-07-02 review found this hook dead since birth on that
key). The response may be the parsed build dict or an MCP content wrapper
({"content":[{"type":"text","text":"{...json...}"}]}); both are handled.
"""
import json
import sys

HINTS = {
    'elohim-holochain': ('DNA_BUILD', 'Check Rust/WASM. cargo build locally.'),
    'elohim-edge': ('INFRASTRUCTURE', 'Check container. Verify hApp artifact.'),
    'elohim': ('APP_BUILD', 'Check TypeScript. npm run build locally.'),
    'elohim-genesis': ('SEEDING', 'Check doorway-alpha.elohim.host/health'),
    'elohim-orchestrator': ('ORCHESTRATOR', 'Fetch ci-summary.json artifact.'),
    'elohim-sophia': ('SOPHIA_BUILD', 'cd sophia && pnpm lint && pnpm test'),
    'doorway-quality': ('DOORWAY_QUALITY', 'cd doorway && RUSTFLAGS="" cargo clippy && cargo fmt --check'),
}


def build_dict(response):
    """Normalize tool_response into the build dict, tolerating the MCP content wrapper."""
    if isinstance(response, dict) and 'content' in response and isinstance(response['content'], list):
        for item in response['content']:
            if isinstance(item, dict) and item.get('type') == 'text':
                try:
                    parsed = json.loads(item.get('text') or '')
                except (json.JSONDecodeError, TypeError):
                    continue
                if isinstance(parsed, dict):
                    return parsed
        return {}
    return response if isinstance(response, dict) else {}


def main():
    try:
        data = json.load(sys.stdin)
        result = build_dict(data.get('tool_response'))
        if not result:
            sys.exit(0)

        status = result.get('result', '')
        if status not in ['FAILURE', 'UNSTABLE']:
            sys.exit(0)

        # Find pipeline from job name — longest key wins so 'elohim-genesis'
        # never falls through to the bare 'elohim' hint.
        job = data.get('tool_input', {}).get('jobFullName', '')
        pipeline = max((p for p in HINTS if p in job), key=len, default=None)

        parts = [f"BUILD: {status} ({result.get('duration', 0)/1000:.0f}s)"]
        if pipeline:
            cat, hint = HINTS[pipeline]
            parts.append(f"{cat}: {hint}")

        print(json.dumps({
            "hookSpecificOutput": {
                "hookEventName": "PostToolUse",
                "additionalContext": " | ".join(parts)
            }
        }))
    except Exception:
        sys.exit(0)

if __name__ == "__main__":
    main()
