#!/usr/bin/env python3
"""
Pre-flight check for quality sessions.

Run at the start of a quality-orchestrator session to establish baseline.
Reports build status, known failures, and test health before any changes.

Usage (standalone):
  python3 .claude/hooks/pre-flight-check.py

Usage (from quality-orchestrator):
  Called automatically when quality-orchestrator skill is invoked.

Returns JSON with baseline status for the session.
"""

import json
import os
import subprocess
import sys
import time
from pathlib import Path

APP_DIR = '/projects/elohim/elohim-app'


def run_tsc_check() -> dict:
    """Run TypeScript type check (no emit) and capture errors."""
    start = time.time()
    try:
        result = subprocess.run(
            ['npx', 'ng', 'build', '--configuration=development'],
            capture_output=True,
            text=True,
            timeout=120,
            cwd=APP_DIR
        )
        elapsed = round(time.time() - start, 1)

        if result.returncode == 0:
            return {
                'status': 'green',
                'errors': [],
                'elapsed_s': elapsed,
            }

        # Parse build errors
        errors = []
        lines = (result.stdout + result.stderr).split('\n')
        for line in lines:
            if 'ERROR' in line and ('TS' in line or 'error' in line.lower()):
                # Clean up ANSI codes
                clean = line
                for code in ['\x1b[31m', '\x1b[0m', '\x1b[32m', '\x1b[37m',
                             '\x1b[41;31m', '\x1b[41;97m', '\x1b[1m', '\x1b[35m']:
                    clean = clean.replace(code, '')
                clean = clean.strip()
                if clean and clean not in errors:
                    errors.append(clean)

        return {
            'status': 'red',
            'errors': errors[:20],  # Cap at 20 errors
            'error_count': len(errors),
            'elapsed_s': elapsed,
        }
    except subprocess.TimeoutExpired:
        return {'status': 'timeout', 'errors': ['Build timed out after 120s'], 'elapsed_s': 120}
    except Exception as e:
        return {'status': 'error', 'errors': [str(e)], 'elapsed_s': 0}


def check_git_status() -> dict:
    """Check git working tree status."""
    try:
        result = subprocess.run(
            ['git', 'status', '--porcelain'],
            capture_output=True,
            text=True,
            timeout=10,
            cwd='/projects/elohim'
        )
        lines = [l for l in result.stdout.strip().split('\n') if l.strip()]
        modified = [l for l in lines if l.startswith(' M') or l.startswith('M ')]
        untracked = [l for l in lines if l.startswith('??')]
        staged = [l for l in lines if l[0] in 'MADR']

        return {
            'clean': len(lines) == 0,
            'modified_count': len(modified),
            'untracked_count': len(untracked),
            'staged_count': len(staged),
        }
    except Exception:
        return {'clean': False, 'error': 'Could not check git status'}


def check_test_coverage_annotations() -> dict:
    """Scan for coverage annotations to get a quick health picture."""
    import re
    pattern = re.compile(r'//\s*@coverage:\s*(\d+(?:\.\d+)?)\s*%')
    below_threshold = []
    total_files = 0
    total_above = 0

    src_dir = Path(APP_DIR) / 'src' / 'app'
    for ts_file in src_dir.rglob('*.ts'):
        if '.spec.' in str(ts_file) or 'node_modules' in str(ts_file):
            continue
        try:
            with open(ts_file, 'r') as f:
                head = f.read(2000)  # Only check first 2000 chars
            match = pattern.search(head)
            if match:
                total_files += 1
                pct = float(match.group(1))
                if pct >= 70:
                    total_above += 1
                else:
                    rel = str(ts_file).split('/elohim-app/')[-1]
                    below_threshold.append({'file': rel, 'coverage': pct})
        except Exception:
            continue

    return {
        'annotated_files': total_files,
        'above_threshold': total_above,
        'below_threshold_count': len(below_threshold),
        'below_threshold': below_threshold[:10],  # Top 10 worst
    }


def main():
    """Run all pre-flight checks and output results."""
    print("=" * 60)
    print("PRE-FLIGHT CHECK - Quality Session Baseline")
    print("=" * 60)

    # Git status
    print("\n[1/3] Checking git status...")
    git = check_git_status()
    if git.get('clean'):
        print("  Git: CLEAN working tree")
    else:
        print(f"  Git: {git.get('modified_count', 0)} modified, "
              f"{git.get('untracked_count', 0)} untracked, "
              f"{git.get('staged_count', 0)} staged")

    # Build check
    print("\n[2/3] Running build check (ng build)...")
    build = run_tsc_check()
    if build['status'] == 'green':
        print(f"  Build: GREEN ({build['elapsed_s']}s)")
    else:
        print(f"  Build: RED - {build.get('error_count', len(build['errors']))} errors ({build['elapsed_s']}s)")
        for err in build['errors'][:5]:
            print(f"    - {err[:120]}")
        if build.get('error_count', 0) > 5:
            print(f"    ... and {build['error_count'] - 5} more")

    # Coverage summary
    print("\n[3/3] Scanning coverage annotations...")
    coverage = check_test_coverage_annotations()
    print(f"  Annotated files: {coverage['annotated_files']}")
    print(f"  Above 70%: {coverage['above_threshold']}")
    print(f"  Below 70%: {coverage['below_threshold_count']}")

    print("\n" + "=" * 60)

    # Build the summary report
    baseline = {
        'git': git,
        'build': build,
        'coverage': coverage,
        'recommendation': [],
    }

    if build['status'] != 'green':
        baseline['recommendation'].append(
            f"BUILD IS RED with {build.get('error_count', '?')} errors. "
            "Fix build errors BEFORE starting quality work to avoid masking regressions."
        )

    if not git.get('clean'):
        baseline['recommendation'].append(
            "Working tree has uncommitted changes. Consider committing or stashing first."
        )

    if baseline['recommendation']:
        print("RECOMMENDATIONS:")
        for rec in baseline['recommendation']:
            print(f"  - {rec}")
    else:
        print("BASELINE: All clear. Ready for quality work.")

    print("=" * 60)

    # Output structured JSON for programmatic consumption
    report_path = '/tmp/pre-flight-report.json'
    with open(report_path, 'w') as f:
        json.dump(baseline, f, indent=2)
    print(f"\nFull report saved to: {report_path}")

    # Exit with appropriate code
    sys.exit(0 if build['status'] == 'green' else 1)


if __name__ == '__main__':
    main()
