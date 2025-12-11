#!/usr/bin/env python3
"""
Add a new human to the Lamad prototype.
Run from elohim-app root: python scripts/add_human.py

Usage:
  # Interactive mode
  python scripts/add_human.py

  # Quick add with arguments
  python scripts/add_human.py --name "Alice" --id "alice-activist" --bio "Community organizer fighting for justice" --category "community"

  # With more details
  python scripts/add_human.py \
    --name "Bob" \
    --id "bob-baker" \
    --bio "Local bakery owner, feeding the community" \
    --category "local-economy" \
    --location "Valley Town" \
    --layer "municipality" \
    --affinities "baking,small-business,community-economics" \
    --org "org-bobs-bakery:Bob's Bakery:owner"

After adding, run: python scripts/import_humans.py
"""

import json
import argparse
from pathlib import Path
from datetime import datetime

HUMANS_FILE = Path("/projects/elohim/data/humans/humans.json")

CATEGORIES = [
    "core-family",
    "workplace",
    "community",
    "affinity",
    "local-economy",
    "newcomer",
    "visitor",
    "red-team",
    "edge-case"
]

REACH_LEVELS = ["hidden", "network", "community", "public"]

LAYERS = [
    "household",
    "neighborhood",
    "municipality",
    "county_regional",
    "state_provincial",
    "national",
    "global"
]


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


def create_human_entry(
    human_id: str,
    display_name: str,
    bio: str,
    category: str,
    profile_reach: str = "community",
    location_name: str = None,
    location_layer: str = None,
    affinities: list = None,
    organizations: list = None,
    communities: list = None,
    is_minor: bool = False,
    guardian_ids: list = None,
    is_pseudonymous: bool = False,
    notes: str = None
) -> dict:
    """Create a human entry for humans.json."""
    entry = {
        "id": f"human-{human_id}",
        "displayName": display_name,
        "bio": bio,
        "category": category,
        "profileReach": profile_reach
    }

    if location_name and location_layer:
        entry["location"] = {
            "layer": location_layer,
            "name": location_name
        }

    if organizations:
        entry["organizations"] = organizations

    if communities:
        entry["communities"] = communities

    if affinities:
        entry["affinities"] = affinities

    if is_minor:
        entry["ageCategory"] = "minor"
        if guardian_ids:
            entry["guardianIds"] = guardian_ids

    if is_pseudonymous:
        entry["isPseudonymous"] = True
        entry["acceptingConnections"] = False

    if notes:
        entry["notes"] = notes

    return entry


def interactive_mode():
    """Interactively create a new human."""
    print("\n" + "=" * 50)
    print("Add New Human to Lamad Prototype")
    print("=" * 50)

    # Required fields
    display_name = input("\nDisplay name: ").strip()
    if not display_name:
        print("Error: Display name is required")
        return None

    default_id = display_name.lower().replace(" ", "-")
    human_id = input(f"ID (default: {default_id}): ").strip() or default_id

    bio = input("Bio (short description): ").strip()
    if not bio:
        print("Error: Bio is required")
        return None

    # Category
    print(f"\nCategories: {', '.join(CATEGORIES)}")
    category = input("Category (default: community): ").strip() or "community"
    if category not in CATEGORIES:
        print(f"Warning: '{category}' is not a standard category")

    # Profile reach
    print(f"\nReach levels: {', '.join(REACH_LEVELS)}")
    profile_reach = input("Profile reach (default: community): ").strip() or "community"

    # Location
    location_name = input("\nLocation name (optional, e.g., 'Tech Valley'): ").strip()
    location_layer = None
    if location_name:
        print(f"Layers: {', '.join(LAYERS)}")
        location_layer = input("Location layer (default: neighborhood): ").strip() or "neighborhood"

    # Affinities
    affinities_str = input("\nAffinities (comma-separated, e.g., 'coding,music,gardening'): ").strip()
    affinities = [a.strip() for a in affinities_str.split(",")] if affinities_str else None

    # Organizations
    orgs = []
    add_org = input("\nAdd an organization? (y/n): ").strip().lower()
    while add_org == "y":
        org_id = input("  Organization ID (e.g., org-my-company): ").strip()
        org_name = input("  Organization name: ").strip()
        org_role = input("  Your role (e.g., developer, owner, member): ").strip()
        if org_id and org_name:
            orgs.append({"id": org_id, "name": org_name, "role": org_role or "member"})
        add_org = input("Add another organization? (y/n): ").strip().lower()

    # Communities
    communities_str = input("\nCommunities (comma-separated IDs, e.g., 'community-local-church,community-tech-meetup'): ").strip()
    communities = [c.strip() for c in communities_str.split(",")] if communities_str else None

    # Special flags
    is_minor = input("\nIs this a minor? (y/n, default: n): ").strip().lower() == "y"
    guardian_ids = None
    if is_minor:
        guardians_str = input("Guardian IDs (comma-separated, e.g., 'human-parent-one,human-parent-two'): ").strip()
        guardian_ids = [g.strip() for g in guardians_str.split(",")] if guardians_str else None

    is_pseudonymous = input("Is this pseudonymous/anonymous? (y/n, default: n): ").strip().lower() == "y"

    notes = input("\nInternal notes (optional, for design purposes): ").strip() or None

    return create_human_entry(
        human_id=human_id,
        display_name=display_name,
        bio=bio,
        category=category,
        profile_reach=profile_reach,
        location_name=location_name,
        location_layer=location_layer,
        affinities=affinities,
        organizations=orgs if orgs else None,
        communities=communities,
        is_minor=is_minor,
        guardian_ids=guardian_ids,
        is_pseudonymous=is_pseudonymous,
        notes=notes
    )


def cli_mode(args):
    """Create human from command line arguments."""
    orgs = None
    if args.org:
        orgs = []
        for org_str in args.org:
            parts = org_str.split(":")
            if len(parts) >= 2:
                orgs.append({
                    "id": parts[0],
                    "name": parts[1],
                    "role": parts[2] if len(parts) > 2 else "member"
                })

    communities = None
    if args.communities:
        communities = [c.strip() for c in args.communities.split(",")]

    affinities = None
    if args.affinities:
        affinities = [a.strip() for a in args.affinities.split(",")]

    guardian_ids = None
    if args.guardians:
        guardian_ids = [g.strip() for g in args.guardians.split(",")]

    return create_human_entry(
        human_id=args.id,
        display_name=args.name,
        bio=args.bio,
        category=args.category,
        profile_reach=args.reach,
        location_name=args.location,
        location_layer=args.layer,
        affinities=affinities,
        organizations=orgs,
        communities=communities,
        is_minor=args.minor,
        guardian_ids=guardian_ids,
        is_pseudonymous=args.pseudonymous,
        notes=args.notes
    )


def main():
    parser = argparse.ArgumentParser(
        description="Add a new human to the Lamad prototype",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Interactive mode
  python scripts/add_human.py

  # Quick add
  python scripts/add_human.py --name "Alice" --id "alice-activist" --bio "Community organizer" --category "community"

  # Full example
  python scripts/add_human.py \\
    --name "Bob" \\
    --id "bob-baker" \\
    --bio "Local bakery owner" \\
    --category "local-economy" \\
    --location "Valley Town" \\
    --layer "municipality" \\
    --affinities "baking,small-business" \\
    --org "org-bobs-bakery:Bob's Bakery:owner" \\
    --communities "community-local-business,community-farmers-market"
        """
    )

    parser.add_argument("--name", help="Display name")
    parser.add_argument("--id", help="Human ID (without 'human-' prefix)")
    parser.add_argument("--bio", help="Short biography")
    parser.add_argument("--category", default="community", help=f"Category: {', '.join(CATEGORIES)}")
    parser.add_argument("--reach", default="community", help=f"Profile reach: {', '.join(REACH_LEVELS)}")
    parser.add_argument("--location", help="Location name")
    parser.add_argument("--layer", default="neighborhood", help=f"Location layer: {', '.join(LAYERS)}")
    parser.add_argument("--affinities", help="Comma-separated affinities")
    parser.add_argument("--org", action="append", help="Organization as 'id:name:role' (can repeat)")
    parser.add_argument("--communities", help="Comma-separated community IDs")
    parser.add_argument("--minor", action="store_true", help="Mark as minor")
    parser.add_argument("--guardians", help="Comma-separated guardian human IDs")
    parser.add_argument("--pseudonymous", action="store_true", help="Mark as pseudonymous")
    parser.add_argument("--notes", help="Internal design notes")
    parser.add_argument("--dry-run", action="store_true", help="Show what would be added without saving")

    args = parser.parse_args()

    # Determine mode
    if args.name and args.id and args.bio:
        new_human = cli_mode(args)
    else:
        new_human = interactive_mode()

    if not new_human:
        print("Aborted.")
        return

    # Preview
    print("\n" + "=" * 50)
    print("NEW HUMAN ENTRY:")
    print("=" * 50)
    print(json.dumps(new_human, indent=2))

    if args.dry_run:
        print("\n(Dry run - not saved)")
        return

    # Confirm
    confirm = input("\nAdd this human? (y/n): ").strip().lower()
    if confirm != "y":
        print("Aborted.")
        return

    # Load, add, save
    data = load_humans()

    # Check for duplicate
    existing_ids = {h["id"] for h in data["humans"]}
    if new_human["id"] in existing_ids:
        print(f"Error: Human with ID '{new_human['id']}' already exists!")
        return

    data["humans"].append(new_human)
    data["summary"]["totalHumans"] = len(data["humans"])

    # Update category count
    cat = new_human["category"]
    if cat in data["categories"]:
        data["categories"][cat]["count"] = data["categories"][cat].get("count", 0) + 1
    else:
        data["categories"][cat] = {"description": "", "count": 1}

    save_humans(data)

    print(f"\n✓ Added {new_human['displayName']} ({new_human['id']})")
    print("\nNext step: Run the import script to generate Lamad content:")
    print("  python scripts/import_humans.py")


if __name__ == "__main__":
    main()
