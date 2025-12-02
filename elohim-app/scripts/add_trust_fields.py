#!/usr/bin/env python3
"""
Add trust fields to existing content nodes.
Run from elohim-app root: python scripts/add_trust_fields.py

This script:
1. Loads all content JSON files
2. Adds authorId, reach, trustScore, activeAttestationIds fields
3. Links content to attestations from attestations/index.json
4. Updates content/index.json with trust summaries
"""

import os
import json
from pathlib import Path
from datetime import datetime
from typing import Optional

# Paths (relative to elohim-app root)
CONTENT_DIR = Path("src/assets/lamad-data/content")
ATTESTATIONS_FILE = Path("src/assets/lamad-data/attestations/index.json")

# Default trust values for content without attestations
DEFAULT_AUTHOR = "system"
DEFAULT_REACH = "commons"  # Existing content is public
DEFAULT_TRUST_SCORE = 0.8   # High baseline for existing curated content

# Reach level ordering
REACH_LEVELS = {
    "private": 0,
    "invited": 1,
    "local": 2,
    "community": 3,
    "federated": 4,
    "commons": 5
}

# Attestation type weights for trust score calculation
ATTESTATION_WEIGHTS = {
    "author-verified": 0.1,
    "steward-approved": 0.3,
    "community-endorsed": 0.2,
    "peer-reviewed": 0.4,
    "governance-ratified": 0.5,
    "curriculum-canonical": 0.5,
    "safety-reviewed": 0.2,
    "accuracy-verified": 0.3,
    "accessibility-checked": 0.1,
    "license-cleared": 0.2
}


def load_attestations() -> dict:
    """Load attestations and organize by content ID."""
    attestations_by_content = {}

    if not ATTESTATIONS_FILE.exists():
        print(f"Warning: {ATTESTATIONS_FILE} not found")
        return attestations_by_content

    with open(ATTESTATIONS_FILE, "r", encoding="utf-8") as f:
        data = json.load(f)

    for att in data.get("attestations", []):
        content_id = att.get("contentId")
        if content_id:
            if content_id not in attestations_by_content:
                attestations_by_content[content_id] = []
            attestations_by_content[content_id].append(att)

    return attestations_by_content


def calculate_trust_score(attestations: list) -> float:
    """Calculate trust score from attestations."""
    if not attestations:
        return DEFAULT_TRUST_SCORE

    total_weight = 0
    for att in attestations:
        if att.get("status") == "active":
            att_type = att.get("attestationType", "")
            weight = ATTESTATION_WEIGHTS.get(att_type, 0.1)
            total_weight += weight

    # Normalize to 0-1 range (max possible ~1.8 if all attestations present)
    return min(1.0, total_weight / 1.5)


def get_effective_reach(attestations: list) -> str:
    """Determine effective reach from attestations."""
    if not attestations:
        return DEFAULT_REACH

    highest_reach = "private"
    highest_level = 0

    for att in attestations:
        if att.get("status") == "active":
            reach = att.get("reachGranted", "private")
            level = REACH_LEVELS.get(reach, 0)
            if level > highest_level:
                highest_level = level
                highest_reach = reach

    return highest_reach


def add_trust_fields_to_content(content: dict, attestations: list) -> dict:
    """Add trust fields to a content node."""
    active_attestation_ids = [
        att["id"] for att in attestations
        if att.get("status") == "active"
    ]

    content["authorId"] = content.get("authorId", DEFAULT_AUTHOR)
    content["reach"] = get_effective_reach(attestations)
    content["trustScore"] = round(calculate_trust_score(attestations), 2)
    content["activeAttestationIds"] = active_attestation_ids
    content["trustComputedAt"] = datetime.utcnow().isoformat() + "Z"

    return content


def process_content_file(file_path: Path, attestations_by_content: dict) -> Optional[dict]:
    """Process a single content JSON file."""
    with open(file_path, "r", encoding="utf-8") as f:
        content = json.load(f)

    content_id = content.get("id")
    if not content_id:
        print(f"Warning: No ID in {file_path}")
        return None

    attestations = attestations_by_content.get(content_id, [])
    updated_content = add_trust_fields_to_content(content, attestations)

    with open(file_path, "w", encoding="utf-8") as f:
        json.dump(updated_content, f, indent=2, ensure_ascii=False)

    return updated_content


def update_content_index(content_dir: Path, attestations_by_content: dict):
    """Update content/index.json with trust summaries."""
    index_path = content_dir / "index.json"

    if not index_path.exists():
        print(f"Warning: {index_path} not found")
        return

    with open(index_path, "r", encoding="utf-8") as f:
        index = json.load(f)

    for node in index.get("nodes", []):
        content_id = node.get("id")
        if content_id:
            attestations = attestations_by_content.get(content_id, [])
            node["reach"] = get_effective_reach(attestations)
            node["trustScore"] = round(calculate_trust_score(attestations), 2)
            node["hasAttestations"] = len(attestations) > 0

    index["lastUpdated"] = datetime.utcnow().isoformat() + "Z"

    with open(index_path, "w", encoding="utf-8") as f:
        json.dump(index, f, indent=2, ensure_ascii=False)

    print(f"Updated {index_path}")


def main():
    print("Loading attestations...")
    attestations_by_content = load_attestations()
    print(f"Found attestations for {len(attestations_by_content)} content nodes")

    print("\nProcessing content files...")
    content_files = list(CONTENT_DIR.glob("*.json"))
    content_files = [f for f in content_files if f.name != "index.json"]

    processed = 0
    for file_path in content_files:
        result = process_content_file(file_path, attestations_by_content)
        if result:
            processed += 1
            has_att = "✓" if result.get("activeAttestationIds") else "·"
            print(f"  {has_att} {file_path.name[:50]:<50} reach={result.get('reach'):<12} score={result.get('trustScore')}")

    print(f"\nProcessed {processed} content files")

    print("\nUpdating content index...")
    update_content_index(CONTENT_DIR, attestations_by_content)

    print("\nDone!")


if __name__ == "__main__":
    main()
