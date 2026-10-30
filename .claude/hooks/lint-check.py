#!/usr/bin/env python3
"""
Post-edit lint check hook for Claude Code.
Runs linters on edited files and checks test coverage.
Only outputs if there are issues to minimize context usage.

Supports: elohim-app (ESLint/Stylelint), doorway-app (ESLint), sophia (ESLint), and — for
          ANY file in a declared formatter contract — the shared formatting check
          (_lib/epr_format: pinned rustfmt for Cargo packages, the owning package's Prettier for
          `format:check` globs). The formatting line is advisory here (worktree bytes); the
          commit decision is made on the index by the pre-commit ratchet (commit-format-ratchet@1).
          Clippy no longer runs on the edit path: a 60 s cargo run cannot fit this hook's 10 s
          budget, and clippy stays the project gate's obligation.

Hook Type: PostToolUse
Matcher: Edit|Write
"""

import json
import os
import subprocess
import sys
import re
from pathlib import Path

# Real monorepo layout, derived from CLAUDE_PROJECT_DIR (fixed 2026-07-02 —
# the prior hardcoded '/projects/elohim/elohim-app' etc. pointed at paths that
# never existed here, so every runner raised FileNotFoundError into a blanket
# except and the whole lint layer was silently dead).
PROJECT = os.environ.get('CLAUDE_PROJECT_DIR', '/projects/elohim')
APP_DIR = os.path.join(PROJECT, 'app', 'elohim-app')
DOORWAY_APP_DIR = os.path.join(PROJECT, 'doorway', 'doorway-app')
SOPHIA_DIR = os.path.join(PROJECT, 'sophia')


_SCRIPTS = Path(__file__).resolve().parent.parent / 'scripts'
if (_SCRIPTS / '_lib').is_dir() and str(_SCRIPTS) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS))


def repo_root_of(file_path: str) -> Path | None:
    """The checkout (main or linked worktree) that owns `file_path`."""
    here = Path(file_path).resolve().parent
    for _ in range(40):
        if (here / '.git').exists():
            return here
        if here.parent == here:
            return None
        here = here.parent
    return None


def format_advisory(file_path: str) -> str:
    """One `[Format]` line for a file inside a formatter contract that is not formatted (or could
    not be measured), via the SAME resolver the commit gate uses — on worktree bytes. '' when the
    file is clean or outside every contract. Never raises."""
    try:
        from _lib import epr_format
    except Exception:
        return ''
    root = repo_root_of(file_path)
    if root is None:
        return ''
    res = epr_format.check_worktree_file(root, file_path)
    if res is None or res.state in ('clean', 'ignored', 'outside'):
        return ''
    name = os.path.basename(file_path)
    if res.state == 'drift':
        return (f"[Format] {name}: not {res.formatter}-formatted — the pre-commit ratchet refuses "
                f"this if the file is new or was formatted at HEAD. Fix: {res.cure}")
    return f"[Format] {name}: formatting unmeasured — {res.reason}" + (f" Cure: {res.cure}" if res.cure else '')


def project_root_exists(cwd: str) -> bool:
    """A missing project root is a loud configuration error, never a silent no-op."""
    if os.path.isdir(cwd):
        return True
    print(f"lint-check: project root missing: {cwd}", file=sys.stderr)
    return False


# ============================================================================
# PROJECT DETECTION
# ============================================================================

def detect_project(file_path: str) -> str | None:
    """Route a file path to its project for linting."""
    if '/elohim-app/' in file_path:
        return 'elohim-app'
    if '/doorway-app/' in file_path:
        return 'doorway-app'
    if '/sophia/' in file_path:
        return 'sophia'
    if '/doorway/' in file_path and file_path.endswith('.rs'):
        return 'doorway'  # coverage signal only; formatting is the shared advisory
    return None


# ============================================================================
# ESLINT RUNNERS
# ============================================================================

def run_eslint(file_path: str, cwd: str = APP_DIR) -> str:
    """Run ESLint on a TypeScript/HTML file."""
    if not project_root_exists(cwd):
        return ''
    try:
        result = subprocess.run(
            ['npx', 'eslint', file_path, '--format', 'stylish', '--no-error-on-unmatched-pattern'],
            capture_output=True,
            text=True,
            timeout=30,
            cwd=cwd
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
    if not project_root_exists(APP_DIR):
        return ''
    try:
        result = subprocess.run(
            ['npx', 'stylelint', file_path, '--formatter', 'string'],
            capture_output=True,
            text=True,
            timeout=30,
            cwd=APP_DIR
        )
        output = result.stdout.strip()
        if output:
            # Limit to 12 lines
            lines = output.split('\n')
            return '\n'.join(lines[:12])
        return ''
    except (subprocess.TimeoutExpired, FileNotFoundError, Exception):
        return ''


# ============================================================================
# COVERAGE CHECKS
# ============================================================================

COVERAGE_THRESHOLD = 70
LCOV_PATH = os.path.join(APP_DIR, 'coverage', 'elohim-app', 'lcov.info')

# Coverage annotation format: // @coverage: 85.2% (2025-01-30)
COVERAGE_ANNOTATION_PATTERN = r'//\s*@coverage:\s*(\d+(?:\.\d+)?)\s*%\s*\((\d{4}-\d{2}-\d{2})\)'


def get_coverage_from_annotation(file_path: str) -> dict | None:
    """Read coverage from in-file annotation."""
    try:
        with open(file_path, 'r') as f:
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
    """Parse lcov.info to get coverage for a specific file."""
    if not os.path.exists(LCOV_PATH):
        return None

    if '/elohim-app/' in file_path:
        rel_path = file_path.split('/elohim-app/')[-1]
    else:
        return None

    try:
        with open(LCOV_PATH, 'r') as f:
            content = f.read()

        records = content.split('end_of_record')
        for record in records:
            lines = record.strip().split('\n')
            sf_line = next((l for l in lines if l.startswith('SF:')), None)
            if not sf_line:
                continue

            record_path = sf_line[3:]
            if record_path == rel_path:
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
    """Check if a corresponding test file exists for the given file."""
    if not file_path.endswith('.ts') or file_path.endswith('.spec.ts') or file_path.endswith('.test.ts') or file_path.endswith('.test.tsx'):
        return True

    # elohim-app and doorway-app use .spec.ts
    if '/elohim-app/' in file_path or '/doorway-app/' in file_path:
        spec_path = file_path.replace('.ts', '.spec.ts')
        return os.path.exists(spec_path)

    # sophia uses .test.ts / .test.tsx
    if '/sophia/' in file_path:
        for ext in ['.test.ts', '.test.tsx']:
            test_path = file_path.rsplit('.', 1)[0] + ext
            if os.path.exists(test_path):
                return True
        return False

    return True


def check_rust_test_module(file_path: str) -> bool:
    """Check if a Rust source file has a #[cfg(test)] module."""
    try:
        with open(file_path, 'r') as f:
            content = f.read()
        return '#[cfg(test)]' in content
    except Exception:
        return True  # Don't report if we can't read


def get_coverage_signal(file_path: str, project: str) -> str:
    """Generate coverage signal for a file based on its project."""
    basename = os.path.basename(file_path)

    if project == 'elohim-app':
        if not file_path.endswith('.ts') or file_path.endswith('.spec.ts'):
            return ''

        # 1. Check in-file coverage annotation
        annotation = get_coverage_from_annotation(file_path)
        if annotation:
            pct = annotation['coverage_pct']
            if pct < COVERAGE_THRESHOLD:
                return f"[Coverage] {basename}: {pct}% < {COVERAGE_THRESHOLD}% - use quality-sweep agent to write basic tests"
            return ''

        # 2. Check lcov fallback
        coverage = get_coverage_from_lcov(file_path)
        if coverage:
            pct = coverage['coverage_pct']
            if pct < COVERAGE_THRESHOLD:
                return f"[Coverage] {basename}: {pct}% - run `npm test` to update annotations"
            return ''

        # 3. No data
        if not check_spec_file_exists(file_path):
            return f"[Coverage] {basename}: no spec file - use quality-sweep agent to add basic tests"

    elif project == 'doorway':
        if file_path.endswith('.rs') and '/src/' in file_path:
            if not check_rust_test_module(file_path):
                return f"[Coverage] {basename}: no #[cfg(test)] module"

    elif project == 'sophia':
        if file_path.endswith('.ts') or file_path.endswith('.tsx'):
            if not file_path.endswith('.test.ts') and not file_path.endswith('.test.tsx'):
                if not check_spec_file_exists(file_path):
                    return f"[Coverage] {basename}: no .test.ts/.test.tsx file"

    elif project == 'doorway-app':
        if file_path.endswith('.ts') and not file_path.endswith('.spec.ts'):
            if not check_spec_file_exists(file_path):
                return f"[Coverage] {basename}: no .spec.ts file"

    return ''


# ============================================================================
# TYPE SIGNATURE CHECK (elohim-app only)
# ============================================================================

def check_type_signature_change(file_path: str) -> str:
    """Detect type signature patterns that could break callers."""
    if not file_path.endswith('.ts') or file_path.endswith('.spec.ts'):
        return ''

    try:
        with open(file_path, 'r') as f:
            content = f.read()

        warnings = []

        export_from_matches = re.findall(
            r'export\s+type\s*\{([^}]+)\}\s*from\s*[\'"]', content
        )
        for match in export_from_matches:
            exported_names = [n.strip() for n in match.split(',')]
            for name in exported_names:
                if not name:
                    continue
                lines = content.split('\n')
                usage_count = 0
                for line in lines:
                    stripped = line.strip()
                    if stripped.startswith('import ') or stripped.startswith('export type {'):
                        continue
                    if re.search(rf'\b{re.escape(name)}\b', stripped):
                        usage_count += 1
                if usage_count > 0:
                    has_local_import = bool(re.search(
                        rf'import\s+type\s*\{{[^}}]*\b{re.escape(name)}\b[^}}]*\}}\s*from',
                        content
                    ))
                    if not has_local_import:
                        warnings.append(
                            f"export-type-from '{name}' used locally but not imported - will cause TS2304"
                        )

        if warnings:
            basename = os.path.basename(file_path)
            return f"[TypeGuard] {basename}: " + '; '.join(warnings)

        return ''
    except Exception:
        return ''


# ============================================================================
# MAIN
# ============================================================================

def main():
    try:
        # Read hook input from stdin (Claude Code hook protocol)
        data = json.load(sys.stdin)

        tool_input = data.get('tool_input', {})
        file_path = tool_input.get('file_path', '')

        if not file_path:
            sys.exit(0)

        # The shared formatting advisory runs for EVERY file, before the project routing below:
        # most formatter contracts (eprfs, a2o, storage, ...) belong to no lint project here.
        format_output = format_advisory(file_path)

        # Detect which project this file belongs to
        project = detect_project(file_path)
        if not project and not format_output:
            sys.exit(0)

        lint_output = ''
        coverage_output = ''
        typeguard_output = ''
        basename = os.path.basename(file_path)

        # Route to appropriate linters
        if project == 'elohim-app':
            if file_path.endswith('.ts') or file_path.endswith('.html'):
                lint_output = run_eslint(file_path)
            elif file_path.endswith('.css') or file_path.endswith('.scss'):
                lint_output = run_stylelint(file_path)
            typeguard_output = check_type_signature_change(file_path)

        elif project == 'doorway-app':
            if file_path.endswith('.ts') or file_path.endswith('.html'):
                lint_output = run_eslint(file_path, cwd=DOORWAY_APP_DIR)

        elif project == 'sophia':
            if file_path.endswith('.ts') or file_path.endswith('.tsx'):
                lint_output = run_eslint(file_path, cwd=SOPHIA_DIR)

        # Check coverage
        coverage_output = get_coverage_signal(file_path, project) if project else ''

        # Combine outputs
        context_parts = []
        if format_output:
            context_parts.append(format_output)
        if lint_output:
            context_parts.append(f"[Lint] {basename}:\n{lint_output}")
        if typeguard_output:
            context_parts.append(typeguard_output)
        if coverage_output:
            context_parts.append(coverage_output)

        if context_parts:
            result = {
                "hookSpecificOutput": {
                    "hookEventName": "PostToolUse",
                    "additionalContext": '\n\n'.join(context_parts)
                }
            }
            print(json.dumps(result))

        sys.exit(0)

    except json.JSONDecodeError:
        sys.exit(0)
    except Exception as e:
        print(f"lint-check hook error: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == '__main__':
    main()
