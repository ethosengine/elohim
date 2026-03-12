#!/usr/bin/env python3
"""
Validate that all humanId references across genesis artifacts match
the canonical registry in genesis/docs/humans/humans.json.

Fast preflight check (~2s for 3500 files). Run before seeding or pushing.

Usage: python3 genesis/scripts/validate-human-ids.py
Exit:  0 = all valid, 1 = mismatches found
"""

import json
import sys
from pathlib import Path

GENESIS_DIR = Path(__file__).parent.parent
HUMANS_JSON = GENESIS_DIR / "docs" / "humans" / "humans.json"
CONTENT_DIR = GENESIS_DIR / "data" / "lamad" / "content"
MANIFESTS_DIR = GENESIS_DIR / "manifests" / "humans"


def load_registry() -> set[str]:
    if not HUMANS_JSON.exists():
        print(f"ERROR: {HUMANS_JSON} not found")
        sys.exit(1)
    with open(HUMANS_JSON) as f:
        data = json.load(f)
    return {h["id"] for h in data["humans"]}


def check_content(valid_ids: set[str]) -> dict[str, int]:
    """Check stewardedBy humanIds in content files."""
    invalid: dict[str, int] = {}
    for f in CONTENT_DIR.glob("*.json"):
        try:
            data = json.loads(f.read_text())
        except (json.JSONDecodeError, UnicodeDecodeError):
            continue
        for s in data.get("stewardedBy", []):
            hid = s.get("humanId", "")
            if hid and hid not in valid_ids:
                invalid[hid] = invalid.get(hid, 0) + 1
    return invalid


def check_manifests(valid_ids: set[str]) -> dict[str, str]:
    """Check HUMAN_ID values in k8s manifests."""
    invalid: dict[str, str] = {}
    if not MANIFESTS_DIR.exists():
        return invalid
    for f in MANIFESTS_DIR.glob("*.yaml"):
        text = f.read_text()
        for line in text.splitlines():
            if "HUMAN_ID" in line and "value:" in line:
                # Extract value from:  value: "human-xxx"
                val = line.split("value:")[-1].strip().strip('"')
                if val and val not in valid_ids:
                    invalid[val] = str(f.name)
    return invalid


def check_annotation_script(valid_ids: set[str]) -> list[str]:
    """Check humanIds hardcoded in annotate-stewardship.py."""
    script = GENESIS_DIR / "scripts" / "annotate-stewardship.py"
    if not script.exists():
        return []
    invalid = []
    text = script.read_text()
    # Find all human-*-* patterns in the script
    import re
    for match in re.finditer(r'"(human-[a-z]+-[a-z]+(?:-[a-z]+)*)"', text):
        hid = match.group(1)
        if hid not in valid_ids:
            invalid.append(hid)
    return list(set(invalid))


def main():
    valid_ids = load_registry()
    print(f"Registry: {len(valid_ids)} canonical humanIds from {HUMANS_JSON.name}")

    errors = False

    # Check content stewardedBy
    bad_content = check_content(valid_ids)
    if bad_content:
        errors = True
        print(f"\n  Content files reference unknown humanIds:")
        for hid, count in sorted(bad_content.items(), key=lambda x: -x[1]):
            print(f"    {hid}: {count} files")

    # Check k8s manifests
    bad_manifests = check_manifests(valid_ids)
    if bad_manifests:
        errors = True
        print(f"\n  K8s manifests reference unknown humanIds:")
        for hid, fname in sorted(bad_manifests.items()):
            print(f"    {hid} in {fname}")

    # Check annotation script
    bad_script = check_annotation_script(valid_ids)
    if bad_script:
        errors = True
        print(f"\n  annotate-stewardship.py references unknown humanIds:")
        for hid in sorted(bad_script):
            print(f"    {hid}")

    if errors:
        print(f"\nFAILED: Fix humanIds to match {HUMANS_JSON.name}")
        print(f"Valid IDs: {', '.join(sorted(valid_ids))}")
        sys.exit(1)

    print("All humanId references valid ✓")
    sys.exit(0)


if __name__ == "__main__":
    main()
