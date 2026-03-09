#!/usr/bin/env python3
"""
Annotate genesis content with stewardedBy based on tag-to-human mapping.

Each human stewards content that matches their story:
- Matthew: governance, protocol core, family learning
- Susan: family curriculum, relationship content
- Pastor Pete: faith community, pastoral care
- Timothy: tutorials, mentorship, learning paths
- Frank: agriculture, supply chain, local economy

Content with no matching rule defaults to Matthew (founder, backwards compat).
Idempotent: overwrites existing stewardedBy on each run.

Usage: python3 genesis/scripts/annotate-stewardship.py [--dry-run]
"""

import json
import os
import sys
from pathlib import Path
from typing import TypedDict


class Steward(TypedDict):
    humanId: str
    affinity: float
    role: str


# ─── Stewardship Rules ─────────────────────────────────────────────────────
# Each rule: (tag_pattern, stewards)
# First matching rule wins. More specific rules first.
# A content node can match multiple rules — stewards accumulate.

STEWARDSHIP_RULES: list[tuple[set[str], list[Steward]]] = [
    # Assessments — Timothy (tutor) is primary, Susan (homeschool) curates
    ({"assessment"}, [
        {"humanId": "human-timothy-tutor", "affinity": 0.8, "role": "author"},
        {"humanId": "human-susan-partner", "affinity": 0.5, "role": "curator"},
    ]),

    # Faith/pastoral content — Pastor Pete primary
    ({"faith", "pastoral", "spiritual"}, [
        {"humanId": "human-pastor-pete-pastor", "affinity": 0.9, "role": "author"},
    ]),

    # Bible/scripture content — Pastor Pete stewards, Matthew endorses
    ({"fct"}, [
        {"humanId": "human-pastor-pete-pastor", "affinity": 0.7, "role": "steward"},
        {"humanId": "human-matthew-manager", "affinity": 0.3, "role": "endorser"},
    ]),

    # Governance — Matthew primary, Pete endorses community governance
    ({"governance"}, [
        {"humanId": "human-matthew-manager", "affinity": 0.8, "role": "author"},
        {"humanId": "human-pastor-pete-pastor", "affinity": 0.4, "role": "endorser"},
    ]),

    # Family-layer governance — Susan primary
    ({"governance_layer:family"}, [
        {"humanId": "human-susan-partner", "affinity": 0.8, "role": "author"},
        {"humanId": "human-matthew-manager", "affinity": 0.6, "role": "curator"},
    ]),

    # Economic/autonomous entity content — Frank (farmer, producer)
    ({"autonomous-entity", "economic-coordination"}, [
        {"humanId": "human-frank-farmer", "affinity": 0.7, "role": "author"},
        {"humanId": "human-matthew-manager", "affinity": 0.4, "role": "endorser"},
    ]),

    # Community/neighborhood content — Nancy (neighbor) + Pete
    ({"governance_layer:neighborhood"}, [
        {"humanId": "human-nancy-neighbor", "affinity": 0.6, "role": "steward"},
        {"humanId": "human-pastor-pete-pastor", "affinity": 0.4, "role": "endorser"},
    ]),

    # Learning paths, education — Timothy primary
    ({"learning", "tutorial", "path", "education"}, [
        {"humanId": "human-timothy-tutor", "affinity": 0.7, "role": "author"},
    ]),

    # Elohim agent content — Matthew (protocol architect)
    ({"elohim_agents:personal_agent"}, [
        {"humanId": "human-matthew-manager", "affinity": 0.9, "role": "author"},
    ]),

    # Value scanner scenarios — distributed across economy humans
    ({"value-scanner"}, [
        {"humanId": "human-frank-farmer", "affinity": 0.5, "role": "steward"},
        {"humanId": "human-georgina-grocer", "affinity": 0.4, "role": "steward"},
        {"humanId": "human-matthew-manager", "affinity": 0.3, "role": "endorser"},
    ]),
]

# Default steward when no rule matches
DEFAULT_STEWARD: list[Steward] = [
    {"humanId": "human-matthew-manager", "affinity": 1.0, "role": "author"},
]

CONTENT_DIR = Path(__file__).parent.parent / "data" / "lamad" / "content"


def match_stewards(tags: list[str]) -> list[Steward]:
    """Find stewards for a content node based on its tags."""
    tag_set = set(tags)
    matched_stewards: dict[str, Steward] = {}

    for required_tags, stewards in STEWARDSHIP_RULES:
        if required_tags & tag_set:  # any tag matches
            for s in stewards:
                # Keep highest affinity per human
                existing = matched_stewards.get(s["humanId"])
                if existing is None or s["affinity"] > existing["affinity"]:
                    matched_stewards[s["humanId"]] = s

    if not matched_stewards:
        return list(DEFAULT_STEWARD)

    # Sort by affinity descending
    return sorted(matched_stewards.values(), key=lambda s: -s["affinity"])


def annotate_file(filepath: Path, dry_run: bool = False) -> tuple[str, list[Steward]]:
    """Annotate a single content JSON with stewardedBy."""
    with open(filepath) as f:
        data = json.load(f)

    tags = data.get("tags", [])
    stewards = match_stewards(tags)
    data["stewardedBy"] = stewards

    if not dry_run:
        with open(filepath, "w") as f:
            json.dump(data, f, indent=2, ensure_ascii=False)
            f.write("\n")

    return data.get("id", filepath.stem), stewards


def main():
    dry_run = "--dry-run" in sys.argv

    if not CONTENT_DIR.exists():
        print(f"Content directory not found: {CONTENT_DIR}")
        sys.exit(1)

    # Exclude humans/ and graph/ subdirectories
    json_files = [
        f for f in CONTENT_DIR.glob("*.json")
        if f.is_file()
    ]

    print(f"{'DRY RUN: ' if dry_run else ''}Annotating {len(json_files)} content files...")

    steward_counts: dict[str, int] = {}
    for filepath in sorted(json_files):
        content_id, stewards = annotate_file(filepath, dry_run)
        for s in stewards:
            steward_counts[s["humanId"]] = steward_counts.get(s["humanId"], 0) + 1

    print(f"\nStewardship distribution:")
    for human_id, count in sorted(steward_counts.items(), key=lambda x: -x[1]):
        print(f"  {human_id}: {count} content nodes")

    print(f"\nTotal: {len(json_files)} files {'would be ' if dry_run else ''}annotated")


if __name__ == "__main__":
    main()
