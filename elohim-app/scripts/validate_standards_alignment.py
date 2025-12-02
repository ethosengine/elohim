#!/usr/bin/env python3
"""
Validate standards alignment coverage in generated Lamad data.
Run from elohim-app root: python scripts/validate_standards_alignment.py

Checks what percentage of content nodes have standards-aligned fields populated:
- did (W3C Decentralized Identifiers)
- activityPubType (ActivityPub/ActivityStreams)
- linkedData (JSON-LD for semantic web)
- openGraphMetadata (Open Graph Protocol for social sharing)
"""

import json
from pathlib import Path
from typing import Dict, List, Tuple


def validate_standards_coverage() -> Dict[str, Tuple[int, int, float]]:
    """Check what percentage of content has standards fields populated.

    Returns:
        Dict mapping field name to (count, total, percentage)
    """
    content_dir = Path("src/assets/lamad-data/content")

    if not content_dir.exists():
        print(f"Error: Content directory not found at {content_dir}")
        print("Please run generate_lamad_data.py and import_fct_content.py first.")
        return {}

    total = 0
    coverage = {
        "did": 0,
        "activityPubType": 0,
        "linkedData": 0,
        "openGraphMetadata": 0,
        "createdAt": 0,
        "updatedAt": 0,
    }

    errors: List[str] = []

    for json_file in content_dir.glob("*.json"):
        if json_file.name == "index.json":
            continue

        try:
            node = json.load(open(json_file))
            total += 1

            # Check each standards field
            if node.get("did"):
                coverage["did"] += 1
                # Validate DID format
                did = node["did"]
                if not did.startswith("did:"):
                    errors.append(f"{json_file.name}: Invalid DID format '{did}'")

            if node.get("activityPubType"):
                coverage["activityPubType"] += 1

            if node.get("linkedData"):
                coverage["linkedData"] += 1
                # Validate JSON-LD structure
                ld = node["linkedData"]
                if "@context" not in ld:
                    errors.append(f"{json_file.name}: JSON-LD missing @context")
                if "@type" not in ld:
                    errors.append(f"{json_file.name}: JSON-LD missing @type")

            if node.get("openGraphMetadata"):
                coverage["openGraphMetadata"] += 1
                # Validate required OG fields
                og = node["openGraphMetadata"]
                required = ["ogTitle", "ogDescription", "ogUrl"]
                for field in required:
                    if field not in og:
                        errors.append(f"{json_file.name}: Open Graph missing {field}")

            if node.get("createdAt"):
                coverage["createdAt"] += 1

            if node.get("updatedAt"):
                coverage["updatedAt"] += 1

        except json.JSONDecodeError as e:
            errors.append(f"{json_file.name}: JSON decode error - {e}")
        except Exception as e:
            errors.append(f"{json_file.name}: Unexpected error - {e}")

    # Calculate percentages
    results = {}
    for field, count in coverage.items():
        pct = (count / total * 100) if total else 0
        results[field] = (count, total, pct)

    return results, total, errors


def print_report(results: Dict[str, Tuple[int, int, float]], total: int, errors: List[str]):
    """Print a formatted coverage report."""
    print("\n" + "=" * 60)
    print("STANDARDS ALIGNMENT COVERAGE REPORT")
    print("=" * 60)
    print(f"\nTotal content nodes analyzed: {total}\n")

    if not results:
        print("No results to display.")
        return

    print("Field Coverage:")
    print("-" * 60)

    for field, (count, total_nodes, pct) in results.items():
        # Determine status indicator
        if pct >= 95:
            status = "✓"
            status_label = "EXCELLENT"
        elif pct >= 80:
            status = "✓"
            status_label = "GOOD"
        elif pct >= 50:
            status = "⚠"
            status_label = "NEEDS IMPROVEMENT"
        else:
            status = "✗"
            status_label = "POOR"

        print(f"{status} {field:25} {count:4}/{total_nodes:4} ({pct:5.1f}%) - {status_label}")

    print("\nTarget Coverage (per plan):")
    print("-" * 60)
    targets = {
        "did": 100.0,
        "activityPubType": 100.0,
        "linkedData": 80.0,
        "openGraphMetadata": 80.0,
    }

    all_targets_met = True
    for field, target in targets.items():
        if field in results:
            _, _, pct = results[field]
            met = pct >= target
            status = "✓ MET" if met else "✗ NOT MET"
            print(f"  {field:25} Target: {target:5.1f}%  Actual: {pct:5.1f}%  {status}")
            if not met:
                all_targets_met = False

    if errors:
        print(f"\n⚠ Validation Errors Found: {len(errors)}")
        print("-" * 60)
        for error in errors[:20]:  # Show first 20 errors
            print(f"  • {error}")
        if len(errors) > 20:
            print(f"  ... and {len(errors) - 20} more errors")
    else:
        print("\n✓ No validation errors found")

    print("\n" + "=" * 60)
    if all_targets_met and not errors:
        print("STATUS: ✓ All targets met! Standards alignment is excellent.")
    elif all_targets_met:
        print("STATUS: ⚠ Coverage targets met, but validation errors found.")
    else:
        print("STATUS: ✗ Some coverage targets not met. Review import scripts.")
    print("=" * 60 + "\n")


def main():
    """Run standards validation."""
    print("Validating standards alignment in Lamad content...")
    results, total, errors = validate_standards_coverage()

    if results:
        print_report(results, total, errors)
    else:
        print("\nCould not run validation. Check error messages above.")
        return 1

    # Return exit code based on whether targets are met
    targets = {
        "did": 100.0,
        "activityPubType": 100.0,
        "linkedData": 80.0,
        "openGraphMetadata": 80.0,
    }

    for field, target in targets.items():
        if field in results:
            _, _, pct = results[field]
            if pct < target:
                return 1  # Targets not met

    if errors:
        return 1  # Validation errors found

    return 0  # Success


if __name__ == "__main__":
    exit(main())
