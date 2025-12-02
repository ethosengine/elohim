#!/usr/bin/env python3
"""
Add a relationship between humans in the Lamad prototype.
Run from elohim-app root: python scripts/add_relationship.py

Usage:
  # Interactive mode
  python scripts/add_relationship.py

  # Quick add with arguments
  python scripts/add_relationship.py --from matthew-manager --to alice-activist --type neighbor --intimacy connection

  # With context (e.g., workplace relationship)
  python scripts/add_relationship.py --from matthew-manager --to dan-developer --type coworker --intimacy connection --context org-ethosengine

After adding, run: python scripts/import_humans.py
"""

import json
import argparse
from pathlib import Path
from datetime import datetime

HUMANS_FILE = Path("/projects/elohim/data/humans/humans.json")

# Relationship types mapped to governance layers
RELATIONSHIP_TYPES = {
    # Family layer
    "spouse": {"layer": "family", "typical_intimacy": "intimate"},
    "parent": {"layer": "family", "typical_intimacy": "intimate"},
    "child": {"layer": "family", "typical_intimacy": "intimate"},
    "sibling": {"layer": "family", "typical_intimacy": "intimate"},
    "grandparent": {"layer": "family", "typical_intimacy": "trusted"},
    "grandchild": {"layer": "family", "typical_intimacy": "trusted"},

    # Neighborhood layer
    "neighbor": {"layer": "neighborhood", "typical_intimacy": "connection"},
    "local_friend": {"layer": "neighborhood", "typical_intimacy": "trusted"},

    # Community layer
    "community_member": {"layer": "community", "typical_intimacy": "connection"},
    "acquaintance": {"layer": "community", "typical_intimacy": "recognition"},

    # Workplace layer
    "coworker": {"layer": "workplace", "typical_intimacy": "connection"},
    "manager": {"layer": "workplace", "typical_intimacy": "connection"},
    "direct_report": {"layer": "workplace", "typical_intimacy": "connection"},
    "business_partner": {"layer": "economy", "typical_intimacy": "trusted"},

    # Affinity layer
    "mentor": {"layer": "affinity", "typical_intimacy": "trusted"},
    "mentee": {"layer": "affinity", "typical_intimacy": "trusted"},
    "congregation_member": {"layer": "affinity", "typical_intimacy": "connection"},
    "interest_group_member": {"layer": "affinity", "typical_intimacy": "connection"},
    "learning_partner": {"layer": "affinity", "typical_intimacy": "connection"},

    # General
    "friend": {"layer": "personal", "typical_intimacy": "trusted"},
    "network_connection": {"layer": "network", "typical_intimacy": "connection"},
    "other": {"layer": "community", "typical_intimacy": "recognition"},
}

INTIMACY_LEVELS = ["intimate", "trusted", "connection", "recognition"]


def load_humans():
    """Load existing humans data."""
    with open(HUMANS_FILE) as f:
        return json.load(f)


def save_humans(data):
    """Save humans data."""
    data["generatedAt"] = datetime.now().isoformat()
    with open(HUMANS_FILE, "w") as f:
        json.dump(data, f, indent=2)
    print(f"\n✓ Saved to {HUMANS_FILE}")


def get_human_ids(data):
    """Get list of valid human IDs."""
    return {h["id"] for h in data["humans"]}


def normalize_human_id(human_id: str) -> str:
    """Ensure human ID has 'human-' prefix."""
    if not human_id.startswith("human-"):
        return f"human-{human_id}"
    return human_id


def create_relationship_entry(
    source_id: str,
    target_id: str,
    rel_type: str,
    intimacy: str,
    context: str = None
) -> dict:
    """Create a relationship entry for humans.json."""
    entry = {
        "source": normalize_human_id(source_id),
        "target": normalize_human_id(target_id),
        "type": rel_type,
        "intimacy": intimacy
    }

    if context:
        entry["context"] = context

    return entry


def print_relationship_types():
    """Print available relationship types grouped by layer."""
    print("\nRelationship Types:")
    print("-" * 50)

    by_layer = {}
    for rel_type, info in RELATIONSHIP_TYPES.items():
        layer = info["layer"]
        if layer not in by_layer:
            by_layer[layer] = []
        by_layer[layer].append((rel_type, info["typical_intimacy"]))

    for layer, types in by_layer.items():
        print(f"\n  {layer.upper()} LAYER:")
        for rel_type, typical in types:
            print(f"    {rel_type:<22} (typically: {typical})")


def print_humans_list(data):
    """Print list of available humans."""
    print("\nAvailable Humans:")
    print("-" * 50)

    by_category = {}
    for h in data["humans"]:
        cat = h.get("category", "other")
        if cat not in by_category:
            by_category[cat] = []
        by_category[cat].append(h)

    for cat, humans in sorted(by_category.items()):
        print(f"\n  {cat.upper()}:")
        for h in humans:
            short_id = h["id"].replace("human-", "")
            print(f"    {short_id:<25} ({h['displayName']})")


def interactive_mode(data):
    """Interactively create a new relationship."""
    print("\n" + "=" * 50)
    print("Add New Relationship")
    print("=" * 50)

    valid_ids = get_human_ids(data)

    # Show available humans
    show_list = input("\nShow list of available humans? (y/n): ").strip().lower()
    if show_list == "y":
        print_humans_list(data)

    # Source human
    source = input("\nFrom human ID (e.g., matthew-manager): ").strip()
    source = normalize_human_id(source)
    if source not in valid_ids:
        print(f"Error: '{source}' not found in humans list")
        return None

    # Target human
    target = input("To human ID (e.g., susan-spouse): ").strip()
    target = normalize_human_id(target)
    if target not in valid_ids:
        print(f"Error: '{target}' not found in humans list")
        return None

    if source == target:
        print("Error: Cannot create relationship with self")
        return None

    # Relationship type
    show_types = input("\nShow relationship types? (y/n): ").strip().lower()
    if show_types == "y":
        print_relationship_types()

    print(f"\nCommon types: spouse, parent, child, neighbor, coworker, friend, mentor, mentee")
    rel_type = input("Relationship type: ").strip()

    if rel_type not in RELATIONSHIP_TYPES:
        print(f"Warning: '{rel_type}' is not a standard type, using 'other'")
        confirm = input("Continue anyway? (y/n): ").strip().lower()
        if confirm != "y":
            return None

    # Intimacy level
    type_info = RELATIONSHIP_TYPES.get(rel_type, {"typical_intimacy": "connection"})
    default_intimacy = type_info["typical_intimacy"]

    print(f"\nIntimacy levels: {', '.join(INTIMACY_LEVELS)}")
    intimacy = input(f"Intimacy level (default: {default_intimacy}): ").strip() or default_intimacy

    if intimacy not in INTIMACY_LEVELS:
        print(f"Error: Invalid intimacy level. Must be one of: {', '.join(INTIMACY_LEVELS)}")
        return None

    # Optional context
    context = input("\nContext (optional, e.g., org-ethosengine for workplace): ").strip() or None

    return create_relationship_entry(source, target, rel_type, intimacy, context)


def cli_mode(args):
    """Create relationship from command line arguments."""
    return create_relationship_entry(
        source_id=args.source,
        target_id=args.target,
        rel_type=args.type,
        intimacy=args.intimacy,
        context=args.context
    )


def check_duplicate(data, new_rel):
    """Check if relationship already exists."""
    for rel in data.get("relationships", []):
        if (rel["source"] == new_rel["source"] and
            rel["target"] == new_rel["target"] and
            rel["type"] == new_rel["type"]):
            return True
    return False


def update_intimacy_counts(data):
    """Recalculate intimacy counts in summary."""
    counts = {"intimate": 0, "trusted": 0, "connection": 0, "recognition": 0}
    for rel in data.get("relationships", []):
        intimacy = rel.get("intimacy", "connection")
        if intimacy in counts:
            counts[intimacy] += 1
    data["summary"]["byIntimacy"] = counts
    data["summary"]["totalRelationships"] = len(data.get("relationships", []))


def main():
    parser = argparse.ArgumentParser(
        description="Add a relationship between humans in the Lamad prototype",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Interactive mode
  python scripts/add_relationship.py

  # Quick add (IDs can omit 'human-' prefix)
  python scripts/add_relationship.py --from matthew-manager --to alice-activist --type neighbor --intimacy connection

  # Workplace relationship with context
  python scripts/add_relationship.py --from matthew-manager --to dan-developer --type coworker --intimacy trusted --context org-ethosengine

  # Family relationship
  python scripts/add_relationship.py --from susan-spouse --to sammy-son --type parent --intimacy intimate

Relationship Types:
  Family:      spouse, parent, child, sibling, grandparent, grandchild
  Neighborhood: neighbor, local_friend
  Workplace:   coworker, manager, direct_report, business_partner
  Affinity:    mentor, mentee, congregation_member, learning_partner
  General:     friend, acquaintance, network_connection

Intimacy Levels:
  intimate    - Closest relationships (spouse, parent-child)
  trusted     - Deep trust (close friends, mentors, grandparents)
  connection  - Regular interaction (neighbors, coworkers)
  recognition - Know of each other (acquaintances)
        """
    )

    parser.add_argument("--from", dest="source", help="Source human ID")
    parser.add_argument("--to", dest="target", help="Target human ID")
    parser.add_argument("--type", help="Relationship type (e.g., neighbor, coworker, friend)")
    parser.add_argument("--intimacy", default="connection", help=f"Intimacy level: {', '.join(INTIMACY_LEVELS)}")
    parser.add_argument("--context", help="Optional context (e.g., org-ethosengine)")
    parser.add_argument("--list-humans", action="store_true", help="List all humans and exit")
    parser.add_argument("--list-types", action="store_true", help="List relationship types and exit")
    parser.add_argument("--dry-run", action="store_true", help="Show what would be added without saving")

    args = parser.parse_args()

    # Load data
    data = load_humans()

    # List modes
    if args.list_humans:
        print_humans_list(data)
        return

    if args.list_types:
        print_relationship_types()
        return

    # Validate IDs exist
    valid_ids = get_human_ids(data)

    # Determine mode
    if args.source and args.target and args.type:
        # Validate
        source = normalize_human_id(args.source)
        target = normalize_human_id(args.target)

        if source not in valid_ids:
            print(f"Error: '{source}' not found. Use --list-humans to see available humans.")
            return
        if target not in valid_ids:
            print(f"Error: '{target}' not found. Use --list-humans to see available humans.")
            return
        if args.intimacy not in INTIMACY_LEVELS:
            print(f"Error: Invalid intimacy. Must be one of: {', '.join(INTIMACY_LEVELS)}")
            return

        new_rel = cli_mode(args)
    else:
        new_rel = interactive_mode(data)

    if not new_rel:
        print("Aborted.")
        return

    # Check for duplicate
    if check_duplicate(data, new_rel):
        print(f"\nWarning: This relationship already exists!")
        confirm = input("Add duplicate anyway? (y/n): ").strip().lower()
        if confirm != "y":
            print("Aborted.")
            return

    # Preview
    print("\n" + "=" * 50)
    print("NEW RELATIONSHIP:")
    print("=" * 50)

    # Get display names for clarity
    source_name = next((h["displayName"] for h in data["humans"] if h["id"] == new_rel["source"]), new_rel["source"])
    target_name = next((h["displayName"] for h in data["humans"] if h["id"] == new_rel["target"]), new_rel["target"])

    print(f"\n  {source_name} --[{new_rel['type']}]--> {target_name}")
    print(f"  Intimacy: {new_rel['intimacy']}")
    if new_rel.get("context"):
        print(f"  Context: {new_rel['context']}")

    print("\nJSON:")
    print(json.dumps(new_rel, indent=2))

    if args.dry_run:
        print("\n(Dry run - not saved)")
        return

    # Confirm
    confirm = input("\nAdd this relationship? (y/n): ").strip().lower()
    if confirm != "y":
        print("Aborted.")
        return

    # Add and save
    if "relationships" not in data:
        data["relationships"] = []

    data["relationships"].append(new_rel)
    update_intimacy_counts(data)

    save_humans(data)

    print(f"\n✓ Added relationship: {source_name} --[{new_rel['type']}]--> {target_name}")
    print("\nNext step: Run the import script to update Lamad content:")
    print("  python scripts/import_humans.py")


if __name__ == "__main__":
    main()
