#!/usr/bin/env python3
"""
Post-edit lint check hook for Claude Code.
Runs ESLint/Stylelint on edited files and checks test coverage.
Only outputs if there are issues to minimize context usage.

Hook Type: PostToolUse
Matcher: Edit|Write
"""

import json
import os
import subprocess
import sys
import re
from pathlib import Path

def run_eslint(file_path: str) -> str:
    """Run ESLint on a TypeScript/HTML file."""
    try:
        result = subprocess.run(
            ['npx', 'eslint', file_path, '--format', 'stylish', '--no-error-on-unmatched-pattern'],
            capture_output=True,
            text=True,
            timeout=30,
            cwd='/projects/elohim/elohim-app'
        )
        output = result.stdout.strip()
        if output and ('error' in output.lower() or 'warning' in output.lower()):
            # Parse and condense output - get just line:col rule message
            lines = output.split('\n')
            issues = []
            for line in lines:
                # Match lines like "  123:45  error  Message  rule-name"
                match = re.match(r'\s*(\d+:\d+)\s+(error|warning)\s+(.+?)\s{2,}(\S+)$', line)
                if match:
                    loc, severity, msg, rule = match.groups()
                    sev = 'E' if severity == 'error' else 'W'
                    issues.append(f"  {loc} {sev}: {msg} [{rule}]")
                # Also catch summary line
                elif 'problem' in line.lower():
                    issues.append(line.strip())
            return '\n'.join(issues[:12])  # Limit to 12 issues
        return ''
    except (subprocess.TimeoutExpired, FileNotFoundError, Exception):
        return ''

def run_stylelint(file_path: str) -> str:
    """Run Stylelint on a CSS/SCSS file."""
    try:
        result = subprocess.run(
            ['npx', 'stylelint', file_path, '--formatter', 'string'],
            capture_output=True,
            text=True,
            timeout=30,
            cwd='/projects/elohim/elohim-app'
        )
        output = result.stdout.strip()
        if output:
            # Limit to 12 lines
            lines = output.split('\n')
            return '\n'.join(lines[:12])
        return ''
    except (subprocess.TimeoutExpired, FileNotFoundError, Exception):
        return ''


COVERAGE_THRESHOLD = 70
LCOV_PATH = '/projects/elohim/elohim-app/coverage/elohim-app/lcov.info'
COVERAGE_SUMMARY_PATH = '/projects/elohim/elohim-app/coverage/elohim-app/coverage-summary.json'

# Coverage annotation format: // @coverage: 85.2% (2025-01-30)
COVERAGE_ANNOTATION_PATTERN = r'//\s*@coverage:\s*(\d+(?:\.\d+)?)\s*%\s*\((\d{4}-\d{2}-\d{2})\)'


def get_coverage_from_annotation(file_path: str) -> dict | None:
    """
    Read coverage from in-file annotation.
    Format: // @coverage: 85.2% (2025-01-30)
    Returns dict with coverage_pct, date or None if not found.
    """
    try:
        with open(file_path, 'r') as f:
            # Only check first 20 lines (annotation should be near top)
            for i, line in enumerate(f):
                if i > 20:
                    break
                match = re.search(COVERAGE_ANNOTATION_PATTERN, line)
                if match:
                    return {
                        'coverage_pct': float(match.group(1)),
                        'date': match.group(2)
                    }
        return None
    except Exception:
        return None


def get_coverage_from_lcov(file_path: str) -> dict | None:
    """
    Parse lcov.info to get coverage for a specific file.
    Returns dict with lines_found, lines_hit, coverage_pct or None if not found.
    """
    if not os.path.exists(LCOV_PATH):
        return None

    # Convert absolute path to src-relative path for matching
    # e.g., /projects/elohim/elohim-app/src/app/foo.ts -> src/app/foo.ts
    if '/elohim-app/' in file_path:
        rel_path = file_path.split('/elohim-app/')[-1]
    else:
        return None

    try:
        with open(LCOV_PATH, 'r') as f:
            content = f.read()

        # Split into file records
        records = content.split('end_of_record')

        for record in records:
            lines = record.strip().split('\n')
            sf_line = next((l for l in lines if l.startswith('SF:')), None)
            if not sf_line:
                continue

            record_path = sf_line[3:]  # Remove 'SF:' prefix
            if record_path == rel_path:
                # Found our file - extract LF (lines found) and LH (lines hit)
                lf = next((int(l[3:]) for l in lines if l.startswith('LF:')), 0)
                lh = next((int(l[3:]) for l in lines if l.startswith('LH:')), 0)

                if lf > 0:
                    coverage_pct = round((lh / lf) * 100, 1)
                    return {
                        'lines_found': lf,
                        'lines_hit': lh,
                        'coverage_pct': coverage_pct
                    }
        return None
    except Exception:
        return None


def check_spec_file_exists(file_path: str) -> bool:
    """Check if a corresponding .spec.ts file exists for the given file."""
    if not file_path.endswith('.ts') or file_path.endswith('.spec.ts'):
        return True  # Not applicable or already a spec file

    spec_path = file_path.replace('.ts', '.spec.ts')
    return os.path.exists(spec_path)


def get_coverage_signal(file_path: str) -> str:
    """
    Generate coverage signal for a file.

    Priority:
    1. In-file @coverage annotation (authoritative - updated by test runs)
    2. lcov.info fallback (if annotation missing)

    Returns empty string if coverage is OK or not applicable.
    """
    # Only check .ts files (not .spec.ts, .html, .css, etc.)
    if not file_path.endswith('.ts') or file_path.endswith('.spec.ts'):
        return ''

    basename = os.path.basename(file_path)

    # 1. Check in-file coverage annotation (primary source)
    annotation = get_coverage_from_annotation(file_path)
    if annotation:
        pct = annotation['coverage_pct']
        if pct < COVERAGE_THRESHOLD:
            return f"[Coverage] {basename}: {pct}% < {COVERAGE_THRESHOLD}% - use linter agent to write basic tests"
        return ''  # Above threshold, no signal

    # 2. No annotation - check lcov as fallback
    coverage = get_coverage_from_lcov(file_path)
    if coverage:
        pct = coverage['coverage_pct']
        if pct < COVERAGE_THRESHOLD:
            return f"[Coverage] {basename}: {pct}% - run `npm test` to update annotations"
        return ''  # Above threshold

    # 3. No data at all
    if not check_spec_file_exists(file_path):
        return f"[Coverage] {basename}: no spec file - use linter agent to add basic tests"

    return ''


def main():
    try:
        # Read hook input from stdin (Claude Code hook protocol)
        data = json.load(sys.stdin)

        tool_input = data.get('tool_input', {})
        file_path = tool_input.get('file_path', '')

        if not file_path:
            sys.exit(0)

        # Only lint elohim-app files
        if '/elohim-app/' not in file_path:
            sys.exit(0)

        # Determine linter based on file extension
        lint_output = ''
        basename = os.path.basename(file_path)

        if file_path.endswith('.ts') or file_path.endswith('.html'):
            lint_output = run_eslint(file_path)
        elif file_path.endswith('.css') or file_path.endswith('.scss'):
            lint_output = run_stylelint(file_path)

        # Check coverage for TypeScript files
        coverage_output = get_coverage_signal(file_path)

        # Combine outputs
        context_parts = []
        if lint_output:
            context_parts.append(f"[Lint] {basename}:\n{lint_output}")
        if coverage_output:
            context_parts.append(coverage_output)

        if context_parts:
            # Output in Claude Code hook format
            result = {
                "hookSpecificOutput": {
                    "hookEventName": "PostToolUse",
                    "additionalContext": '\n\n'.join(context_parts)
                }
            }
            print(json.dumps(result))

        sys.exit(0)

    except json.JSONDecodeError:
        # No valid JSON input - just exit
        sys.exit(0)
    except Exception as e:
        # Log error to stderr but don't block
        print(f"lint-check hook error: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == '__main__':
    main()
