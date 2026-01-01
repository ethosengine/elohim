#!/usr/bin/env python3
"""
CI Result Formatter - Minimal hook for mcp__jenkins__getBuild failures.
Only injects context when build failed/unstable.
"""
import json
import sys

HINTS = {
    'elohim-holochain': ('DNA_BUILD', 'Check Rust/WASM. cargo build locally.'),
    'elohim-edge': ('INFRASTRUCTURE', 'Check container. Verify hApp artifact.'),
    'elohim': ('APP_BUILD', 'Check TypeScript. npm run build locally.'),
    'elohim-genesis': ('SEEDING', 'Check doorway-dev.elohim.host/health'),
    'elohim-orchestrator': ('ORCHESTRATOR', 'Fetch ci-summary.json artifact.')
}

def main():
    try:
        data = json.load(sys.stdin)
        result = data.get('tool_result', {})

        if not isinstance(result, dict):
            sys.exit(0)

        status = result.get('result', '')
        if status not in ['FAILURE', 'UNSTABLE']:
            sys.exit(0)

        # Find pipeline from job name
        job = data.get('tool_input', {}).get('jobFullName', '')
        pipeline = next((p for p in HINTS if p in job), None)

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
    except:
        sys.exit(0)

if __name__ == "__main__":
    main()
