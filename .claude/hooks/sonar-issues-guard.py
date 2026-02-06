#!/usr/bin/env python3
"""
SonarQube Issues Guard - PreToolUse hook for mcp__sonarqube__search_sonar_issues_in_projects.

This hook prevents agents from calling the SonarQube issues API directly, which
returns ~50KB+ per page and destroys context windows. Instead, agents should use:

  bash .claude/scripts/sonar-issues-summary.sh

Which saves raw JSON to disk and produces a compact per-rule summary.

Behavior:
  - If called from main context (not a subagent): WARN with additionalContext
    suggesting the script instead, but allow through.
  - The warning gives agents a strong nudge to prefer the script.
"""
import json
import os
import sys


def main():
    try:
        data = json.load(sys.stdin)
        tool_name = data.get("tool_name", "")

        if tool_name != "mcp__sonarqube__search_sonar_issues_in_projects":
            sys.exit(0)

        # Check if summary already exists and is recent (< 1 hour)
        summary_path = os.path.join(
            os.environ.get("CLAUDE_PROJECT_DIR", "/projects/elohim"),
            ".claude",
            "sonar-issues-summary.json",
        )
        summary_exists = os.path.exists(summary_path)

        if summary_exists:
            import time

            age_minutes = (time.time() - os.path.getmtime(summary_path)) / 60
            age_msg = f"Summary file exists ({age_minutes:.0f}m old): {summary_path}"
        else:
            age_msg = "No summary file found yet."

        warning = (
            "WARNING: search_sonar_issues_in_projects returns ~50KB per page and will "
            "consume significant context window space. PREFER using the script instead:\n\n"
            "  SONAR_TOKEN=$SONARQUBE_TOKEN bash .claude/scripts/sonar-issues-summary.sh\n\n"
            "Then read .claude/sonar-issues-summary.json (compact, ~2KB per rule).\n"
            f"{age_msg}\n\n"
            "Only use the MCP tool directly if you need real-time data that the script "
            "can't provide (e.g., filtering by pull request ID)."
        )

        print(
            json.dumps(
                {
                    "hookSpecificOutput": {
                        "hookEventName": "PreToolUse",
                        "additionalContext": warning,
                    }
                }
            )
        )
    except Exception:
        # Don't block on hook errors
        sys.exit(0)


if __name__ == "__main__":
    main()
