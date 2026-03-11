#!/usr/bin/env python3
"""
Update @coverage annotations in source files from lcov.info.
Run after test suite to keep coverage annotations fresh.

Usage: python scripts/update-coverage-annotations.py
"""

import os
import re
from datetime import date
from pathlib import Path

LCOV_PATH = 'coverage/elohim-app/lcov.info'
SRC_DIR = 'src'
COVERAGE_PATTERN = r'(//\s*@coverage:\s*)(~?\d+(?:\.\d+)?%\s*(?:estimate\s*)?\([^)]+\))'
TODAY = date.today().isoformat()


def parse_lcov() -> dict[str, float]:
    """Parse lcov.info and return file -> coverage% mapping."""
    if not os.path.exists(LCOV_PATH):
        print(f"Warning: {LCOV_PATH} not found. Run tests first.")
        return {}

    coverage = {}
    current_file = None
    lines_found = 0
    lines_hit = 0

    with open(LCOV_PATH, 'r') as f:
        for line in f:
            line = line.strip()
            if line.startswith('SF:'):
                current_file = line[3:]
            elif line.startswith('LF:'):
                lines_found = int(line[3:])
            elif line.startswith('LH:'):
                lines_hit = int(line[3:])
            elif line == 'end_of_record':
                if current_file and lines_found > 0:
                    pct = round((lines_hit / lines_found) * 100, 1)
                    coverage[current_file] = pct
                current_file = None
                lines_found = 0
                lines_hit = 0

    return coverage


def update_file_annotation(file_path: str, coverage_pct: float) -> bool:
    """Update or add @coverage annotation in a file. Returns True if modified."""
    try:
        with open(file_path, 'r') as f:
            content = f.read()

        new_annotation = f"// @coverage: {coverage_pct}% ({TODAY})"

        # Check if annotation exists
        if re.search(COVERAGE_PATTERN, content):
            # Update existing annotation
            new_content = re.sub(
                COVERAGE_PATTERN,
                lambda m: f"{m.group(1)}{coverage_pct}% ({TODAY})",
                content
            )
            if new_content != content:
                with open(file_path, 'w') as f:
                    f.write(new_content)
                return True
        else:
            # Add annotation after imports (find first blank line after imports)
            lines = content.split('\n')
            insert_idx = 0

            # Find end of import block
            for i, line in enumerate(lines):
                if line.strip().startswith('import '):
                    insert_idx = i + 1
                elif insert_idx > 0 and line.strip() == '':
                    # Found blank line after imports
                    insert_idx = i
                    break

            # Insert annotation
            if insert_idx > 0:
                lines.insert(insert_idx, '')
                lines.insert(insert_idx + 1, new_annotation)
                with open(file_path, 'w') as f:
                    f.write('\n'.join(lines))
                return True

        return False
    except Exception as e:
        print(f"  Error updating {file_path}: {e}")
        return False


def main():
    print("Updating coverage annotations from lcov.info...")

    coverage = parse_lcov()
    if not coverage:
        return

    print(f"Found coverage data for {len(coverage)} files")

    updated = 0
    added = 0

    for rel_path, pct in coverage.items():
        # Convert lcov path to absolute path
        abs_path = os.path.join(os.getcwd(), rel_path)

        if not os.path.exists(abs_path):
            continue

        # Skip spec files
        if '.spec.' in rel_path:
            continue

        # Check if file has existing annotation
        with open(abs_path, 'r') as f:
            has_annotation = '@coverage:' in f.read()

        if update_file_annotation(abs_path, pct):
            if has_annotation:
                updated += 1
                print(f"  Updated: {rel_path} -> {pct}%")
            else:
                added += 1
                print(f"  Added:   {rel_path} -> {pct}%")

    print(f"\nSummary: {updated} updated, {added} added")


if __name__ == '__main__':
    main()
