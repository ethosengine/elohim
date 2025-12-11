#!/usr/bin/env python3
"""
Import human profiles into Lamad format - aligned with TypeScript models.
Run from elohim-app root: python scripts/import_humans.py

This script reads the consolidated humans.json and generates data that aligns
with the TypeScript interfaces in:
- human-node.model.ts (HumanNode, HumanRelationship, RelationshipType)
- human-consent.model.ts (HumanConsent, IntimacyLevel, ConsentState)
- content-node.model.ts (ContentNode for organization/community entities)

Output:
1. HumanNode JSON files (one per human)
2. HumanRelationship JSON files (relationship edges)
3. HumanConsent JSON files (consent records for relationships)
4. Organization ContentNodes
5. Community ContentNodes
6. Graph overview and summary files
"""

import json
import uuid
import math
from pathlib import Path
from datetime import datetime
from typing import Optional

# Paths
HUMANS_SOURCE = Path("/projects/elohim/data/humans/humans.json")
OUTPUT_DIR = Path("src/assets/lamad-data")
CONTENT_DIR = OUTPUT_DIR / "content"
GRAPH_DIR = OUTPUT_DIR / "graph"

# Timestamp helpers
def now_iso() -> str:
    return datetime.utcnow().isoformat() + "Z"

def gen_id(prefix: str) -> str:
    return f"{prefix}-{uuid.uuid4().hex[:8]}"


# =============================================================================
# TYPE MAPPINGS (aligned with TypeScript)
# =============================================================================

# From human-node.model.ts
RELATIONSHIP_LAYER_MAP = {
    "spouse": "family",
    "parent": "family",
    "child": "family",
    "sibling": "family",
    "grandparent": "family",
    "grandchild": "family",
    "extended_family": "family",
    "guardian": "family",
    "neighbor": "neighborhood",
    "community_member": "community",
    "local_friend": "community",
    "coworker": "workplace",
    "manager": "workplace",
    "direct_report": "workplace",
    "mentor": "workplace",
    "mentee": "workplace",
    "business_partner": "workplace",
    "congregation_member": "affinity_network",
    "interest_group_member": "affinity_network",
    "learning_partner": "affinity_network",
    "network_connection": "affinity_network",
    "friend": "community",
    "acquaintance": "community",
    "other": "community",
}

# From human-node.model.ts - RELATIONSHIP_DEFAULT_INTIMACY
RELATIONSHIP_DEFAULT_INTIMACY = {
    "spouse": "intimate",
    "parent": "intimate",
    "child": "intimate",
    "sibling": "trusted",
    "grandparent": "trusted",
    "grandchild": "trusted",
    "extended_family": "connection",
    "guardian": "intimate",
    "neighbor": "connection",
    "community_member": "connection",
    "local_friend": "trusted",
    "coworker": "connection",
    "manager": "connection",
    "direct_report": "connection",
    "mentor": "trusted",
    "mentee": "trusted",
    "business_partner": "trusted",
    "congregation_member": "connection",
    "interest_group_member": "connection",
    "learning_partner": "trusted",
    "network_connection": "recognition",
    "friend": "trusted",
    "acquaintance": "recognition",
    "other": "recognition",
}

# IntimacyLevel ordering (from human-consent.model.ts)
INTIMACY_LEVELS = ["recognition", "connection", "trusted", "intimate"]

# HumanReach values (from human-node.model.ts)
HUMAN_REACH_VALUES = ["hidden", "network", "community", "public"]


# =============================================================================
# HUMAN NODE GENERATION (aligned with HumanNode interface)
# =============================================================================

def create_human_node(human_data: dict, timestamp: str) -> dict:
    """
    Create a HumanNode aligned with human-node.model.ts HumanNode interface.

    Required fields per interface:
    - id: string
    - displayName: string
    - isPseudonymous: boolean
    - createdAt: string
    - updatedAt: string
    - profileReach: HumanReach
    - acceptingConnections: boolean
    """
    human_id = human_data["id"]
    display_name = human_data.get("displayName", "Anonymous")
    bio = human_data.get("bio", "")
    profile_reach = human_data.get("profileReach", "network")
    category = human_data.get("category", "community")

    # Build HumanNode (matches TypeScript interface)
    node = {
        # Required fields
        "id": human_id,
        "displayName": display_name,
        "isPseudonymous": human_data.get("isPseudonymous", False),
        "createdAt": timestamp,
        "updatedAt": timestamp,
        "profileReach": profile_reach,
        "acceptingConnections": human_data.get("acceptingConnections", True),

        # Optional fields
        "bio": bio if bio else None,
        "avatarUrl": f"assets/avatars/{human_id}.jpg",  # Placeholder
        "connectionMessage": human_data.get("connectionMessage"),
    }

    # Primary location (GeographicContext)
    if human_data.get("location"):
        loc = human_data["location"]
        node["primaryLocation"] = {
            "layer": loc.get("layer", "neighborhood"),
            "displayName": loc.get("name", ""),
            "reach": "community",
            "type": loc.get("layer", "neighborhood")
        }

    # Organization IDs (just the IDs, not full objects)
    if human_data.get("organizations"):
        node["organizationIds"] = [org["id"] for org in human_data["organizations"]]

    # Community IDs
    if human_data.get("communities"):
        node["communityIds"] = human_data["communities"]

    # Affinity group IDs (from affinities - these become affinity group references)
    if human_data.get("affinities"):
        # Convert affinity strings to affinity group IDs
        node["affinityGroupIds"] = [f"affinity-{a}" for a in human_data["affinities"]]
        # Also store as public affinity node IDs for learning context
        node["publicAffinityNodeIds"] = [f"concept-{a}" for a in human_data["affinities"]]

    # Attestation IDs
    if human_data.get("attestations"):
        node["publicAttestationIds"] = human_data["attestations"]

    # Relationship IDs (will be populated after relationships are created)
    node["relationshipIds"] = []

    # Trusted connection count (will be computed)
    node["trustedConnectionCount"] = 0

    # Remove None values for cleaner JSON
    node = {k: v for k, v in node.items() if v is not None}

    return node


def create_human_node_metadata(human_data: dict) -> dict:
    """
    Create additional metadata for the human (not part of HumanNode interface
    but useful for our prototype).
    """
    return {
        "category": human_data.get("category", "community"),
        "governanceLayers": infer_governance_layers(human_data),
        "ageCategory": human_data.get("ageCategory"),
        "guardianIds": human_data.get("guardianIds"),
        "hasGuardian": bool(human_data.get("guardianIds")),
        "accessibilityNeeds": human_data.get("accessibilityNeeds"),
        "languagePreferences": human_data.get("languagePreferences"),
        "flags": human_data.get("flags"),
        "hasFlags": bool(human_data.get("flags")),
        "claimedAttestations": human_data.get("claimedAttestations"),
        "foreignCredentials": human_data.get("foreignCredentials"),
        "designNotes": human_data.get("notes"),
    }


def infer_governance_layers(human_data: dict) -> list:
    """Infer which governance layers this human participates in."""
    layers = set()

    if human_data.get("location"):
        layer = human_data["location"].get("layer", "")
        if layer:
            layers.add(layer)

    if human_data.get("organizations"):
        layers.add("workplace")

    if human_data.get("communities"):
        layers.add("community")

    if human_data.get("guardianIds"):
        layers.add("family")

    return list(layers)


# =============================================================================
# HUMAN RELATIONSHIP GENERATION (aligned with HumanRelationship interface)
# =============================================================================

def create_human_relationship(
    rel_data: dict,
    rel_index: int,
    consent_id: str,
    timestamp: str
) -> dict:
    """
    Create a HumanRelationship aligned with human-node.model.ts interface.

    Required fields per interface:
    - id: string
    - sourceHumanId: string
    - targetHumanId: string
    - type: RelationshipType
    - consentId: string
    - consentState: ConsentState
    - intimacyLevel: IntimacyLevel
    - establishedAt: string
    - isReciprocated: boolean
    """
    source_id = rel_data["source"]
    target_id = rel_data["target"]
    rel_type = rel_data.get("type", "acquaintance")
    intimacy = rel_data.get("intimacy", RELATIONSHIP_DEFAULT_INTIMACY.get(rel_type, "connection"))
    context = rel_data.get("context")

    relationship = {
        # Required fields
        "id": f"rel-{rel_index:04d}",
        "sourceHumanId": source_id,
        "targetHumanId": target_id,
        "type": rel_type,
        "consentId": consent_id,
        "consentState": "accepted",  # Assumed for demo data
        "intimacyLevel": intimacy,
        "establishedAt": timestamp,
        "isReciprocated": True,  # Assumed for demo data

        # Optional fields
        "lastInteractionAt": timestamp,
    }

    # Context fields
    if context:
        if context.startswith("org-"):
            relationship["organizationId"] = context
        elif context.startswith("community-"):
            relationship["communityId"] = context
        elif context.startswith("affinity-"):
            relationship["affinityGroupId"] = context

    return relationship


def create_human_consent(
    rel_data: dict,
    consent_index: int,
    timestamp: str
) -> dict:
    """
    Create a HumanConsent aligned with human-consent.model.ts interface.

    Required fields per interface:
    - id: string
    - initiatorId: string
    - participantId: string
    - intimacyLevel: IntimacyLevel
    - consentState: ConsentState
    - createdAt: string
    - updatedAt: string
    - stateHistory: ConsentStateChange[]
    """
    source_id = rel_data["source"]
    target_id = rel_data["target"]
    rel_type = rel_data.get("type", "acquaintance")
    intimacy = rel_data.get("intimacy", RELATIONSHIP_DEFAULT_INTIMACY.get(rel_type, "connection"))

    consent = {
        # Required fields
        "id": f"consent-{consent_index:04d}",
        "initiatorId": source_id,
        "participantId": target_id,
        "intimacyLevel": intimacy,
        "consentState": "accepted",
        "createdAt": timestamp,
        "updatedAt": timestamp,
        "consentedAt": timestamp,

        # State history (required array)
        "stateHistory": [
            {
                "fromState": "pending",
                "toState": "accepted",
                "fromLevel": None,
                "toLevel": intimacy,
                "timestamp": timestamp,
                "initiatedBy": "participant",
                "reason": "Demo data - auto-accepted"
            }
        ],
    }

    # For intimate relationships, add attestation reference
    if intimacy == "intimate":
        consent["requiredAttestationType"] = get_attestation_type_for_relationship(rel_type)

    return consent


def get_attestation_type_for_relationship(rel_type: str) -> str:
    """Map relationship type to attestation type (from human-consent.model.ts)."""
    mapping = {
        "spouse": "relationship:marriage",
        "parent": "relationship:parent_child",
        "child": "relationship:parent_child",
        "guardian": "relationship:parent_child",
        "sibling": "relationship:sibling",
        "mentor": "relationship:mentorship",
        "mentee": "relationship:mentorship",
        "business_partner": "relationship:business_partner",
    }
    return mapping.get(rel_type, "relationship:custom")


# =============================================================================
# ORGANIZATION & COMMUNITY NODE GENERATION (as ContentNodes)
# =============================================================================

def extract_organizations(humans: list) -> list:
    """Extract unique organizations from human profiles."""
    orgs = {}
    for human in humans:
        for org in human.get("organizations", []):
            org_id = org.get("id", "")
            if org_id and org_id not in orgs:
                orgs[org_id] = {
                    "id": org_id,
                    "name": org.get("name", org_id),
                    "members": []
                }
            if org_id:
                orgs[org_id]["members"].append({
                    "humanId": human["id"],
                    "role": org.get("role", "member")
                })
    return list(orgs.values())


def create_organization_content_node(org_data: dict, timestamp: str) -> dict:
    """Create an organization as a ContentNode (content-node.model.ts)."""
    org_id = org_data["id"]
    name = org_data.get("name", org_id)
    members = org_data.get("members", [])

    return {
        "id": org_id,
        "did": f"did:web:elohim.host:org:{org_id.replace('org-', '')}",
        "contentType": "organization",
        "activityPubType": "Organization",
        "title": name,
        "description": f"Organization with {len(members)} members",
        "content": {
            "name": name,
            "memberCount": len(members),
            "members": members
        },
        "contentFormat": "html",
        "tags": ["organization", "human-graph"],
        "relatedNodeIds": [m["humanId"] for m in members],
        "metadata": {
            "category": "organization",
            "memberCount": len(members)
        },
        "reach": "community",
        "createdAt": timestamp,
        "updatedAt": timestamp
    }


def extract_communities(humans: list) -> list:
    """Extract unique communities from human profiles."""
    communities = {}
    for human in humans:
        for comm_id in human.get("communities", []):
            if comm_id not in communities:
                communities[comm_id] = {
                    "id": comm_id,
                    "name": comm_id.replace("community-", "").replace("-", " ").title(),
                    "members": []
                }
            communities[comm_id]["members"].append(human["id"])
    return list(communities.values())


def create_community_content_node(comm_data: dict, timestamp: str) -> dict:
    """Create a community as a ContentNode (content-node.model.ts)."""
    comm_id = comm_data["id"]
    name = comm_data.get("name", comm_id)
    members = comm_data.get("members", [])

    return {
        "id": comm_id,
        "did": f"did:web:elohim.host:community:{comm_id.replace('community-', '')}",
        "contentType": "organization",  # Communities are a type of organization
        "activityPubType": "Group",
        "title": name,
        "description": f"Community with {len(members)} members",
        "content": {
            "name": name,
            "memberCount": len(members),
            "memberIds": members
        },
        "contentFormat": "html",
        "tags": ["community", "human-graph"],
        "relatedNodeIds": members,
        "metadata": {
            "category": "community",
            "memberCount": len(members)
        },
        "reach": "community",
        "createdAt": timestamp,
        "updatedAt": timestamp
    }


# =============================================================================
# FILE WRITING
# =============================================================================

def write_json_file(path: Path, data: dict):
    """Write JSON file with proper formatting."""
    path.write_text(json.dumps(data, indent=2))


def write_human_nodes(human_nodes: list, metadata_map: dict, timestamp: str):
    """Write human nodes to individual files."""
    humans_dir = CONTENT_DIR / "humans"
    humans_dir.mkdir(parents=True, exist_ok=True)

    for node in human_nodes:
        # Combine node with metadata for storage
        full_node = {
            **node,
            "_metadata": metadata_map.get(node["id"], {}),
            "_nodeType": "HumanNode",
            "_schemaVersion": "1.0.0"
        }
        write_json_file(humans_dir / f"{node['id']}.json", full_node)

    # Write index
    index = {
        "lastUpdated": timestamp,
        "count": len(human_nodes),
        "humans": [
            {
                "id": n["id"],
                "displayName": n["displayName"],
                "profileReach": n["profileReach"],
                "category": metadata_map.get(n["id"], {}).get("category", "community")
            }
            for n in human_nodes
        ]
    }
    write_json_file(humans_dir / "index.json", index)

    return len(human_nodes)


def write_relationships(relationships: list, timestamp: str):
    """Write relationships to graph directory."""
    rels_file = GRAPH_DIR / "human-relationships.json"

    data = {
        "lastUpdated": timestamp,
        "count": len(relationships),
        "relationships": relationships,
        "_schemaVersion": "1.0.0",
        "_interface": "HumanRelationship"
    }
    write_json_file(rels_file, data)

    return len(relationships)


def write_consents(consents: list, timestamp: str):
    """Write consent records to graph directory."""
    consents_file = GRAPH_DIR / "human-consents.json"

    data = {
        "lastUpdated": timestamp,
        "count": len(consents),
        "consents": consents,
        "_schemaVersion": "1.0.0",
        "_interface": "HumanConsent"
    }
    write_json_file(consents_file, data)

    return len(consents)


def write_content_nodes(nodes: list, node_type: str, timestamp: str):
    """Write organization/community content nodes."""
    for node in nodes:
        write_json_file(CONTENT_DIR / f"{node['id']}.json", node)

    return len(nodes)


def update_content_index(
    human_nodes: list,
    org_nodes: list,
    comm_nodes: list,
    timestamp: str
):
    """Update the main content index."""
    content_index_file = CONTENT_DIR / "index.json"

    if content_index_file.exists():
        with open(content_index_file) as f:
            content_index = json.load(f)
    else:
        content_index = {"lastUpdated": timestamp, "totalCount": 0, "nodes": []}

    existing_ids = {n["id"] for n in content_index["nodes"]}

    # Add organization and community nodes to content index
    all_nodes = org_nodes + comm_nodes
    added = 0
    for node in all_nodes:
        if node["id"] not in existing_ids:
            content_index["nodes"].append({
                "id": node["id"],
                "title": node["title"],
                "description": node.get("description", "")[:200],
                "contentType": node["contentType"],
                "tags": node.get("tags", [])
            })
            added += 1

    content_index["lastUpdated"] = timestamp
    content_index["totalCount"] = len(content_index["nodes"])
    write_json_file(content_index_file, content_index)

    return added


# =============================================================================
# GRAPH OVERVIEW GENERATION
# =============================================================================

def create_human_graph_overview(
    human_nodes: list,
    relationships: list,
    metadata_map: dict,
    timestamp: str
):
    """Create visualization-ready graph overview."""

    # Category positions for visualization
    category_positions = {
        "core-family": {"x": 0, "y": 0, "radius": 100},
        "workplace": {"x": 250, "y": 0, "radius": 80},
        "community": {"x": 125, "y": 200, "radius": 100},
        "affinity": {"x": -125, "y": 200, "radius": 120},
        "local-economy": {"x": -250, "y": 0, "radius": 150},
        "newcomer": {"x": 0, "y": -200, "radius": 80},
        "visitor": {"x": -200, "y": -150, "radius": 50},
        "red-team": {"x": 200, "y": -150, "radius": 100},
        "edge-case": {"x": 0, "y": 300, "radius": 50}
    }

    # Group humans by category
    by_category = {}
    for node in human_nodes:
        cat = metadata_map.get(node["id"], {}).get("category", "community")
        if cat not in by_category:
            by_category[cat] = []
        by_category[cat].append(node)

    # Position nodes
    overview_nodes = []
    for cat, cat_humans in by_category.items():
        pos = category_positions.get(cat, {"x": 0, "y": 0, "radius": 100})
        for i, node in enumerate(cat_humans):
            angle = (2 * math.pi * i) / max(len(cat_humans), 1)
            x = pos["x"] + pos["radius"] * math.cos(angle) * (0.5 + 0.5 * (i % 2))
            y = pos["y"] + pos["radius"] * math.sin(angle) * (0.5 + 0.5 * (i % 2))

            overview_nodes.append({
                "id": node["id"],
                "displayName": node["displayName"],
                "category": cat,
                "profileReach": node["profileReach"],
                "position": {"x": x, "y": y},
                "connectionCount": len(node.get("relationshipIds", []))
            })

    # Create edges for visualization
    overview_edges = []
    for rel in relationships:
        overview_edges.append({
            "id": rel["id"],
            "source": rel["sourceHumanId"],
            "target": rel["targetHumanId"],
            "type": rel["type"],
            "intimacyLevel": rel["intimacyLevel"]
        })

    overview = {
        "lastUpdated": timestamp,
        "graphType": "human",
        "description": "Human relationship graph",
        "nodes": overview_nodes,
        "edges": overview_edges,
        "totalNodes": len(overview_nodes),
        "totalEdges": len(overview_edges),
        "categories": list(by_category.keys()),
        "categoryCounts": {cat: len(h) for cat, h in by_category.items()}
    }

    write_json_file(GRAPH_DIR / "human-overview.json", overview)
    return overview


def create_human_graph_summary(
    human_nodes: list,
    relationships: list,
    consents: list,
    org_nodes: list,
    comm_nodes: list,
    metadata_map: dict,
    categories: dict,
    timestamp: str
):
    """Create summary statistics for human graph."""

    # Count by category
    by_category = {}
    for node in human_nodes:
        cat = metadata_map.get(node["id"], {}).get("category", "community")
        by_category[cat] = by_category.get(cat, 0) + 1

    # Count by relationship type
    by_rel_type = {}
    for rel in relationships:
        t = rel["type"]
        by_rel_type[t] = by_rel_type.get(t, 0) + 1

    # Count by intimacy
    by_intimacy = {}
    for rel in relationships:
        i = rel["intimacyLevel"]
        by_intimacy[i] = by_intimacy.get(i, 0) + 1

    # Special cases
    minors = len([n for n in human_nodes
                  if metadata_map.get(n["id"], {}).get("ageCategory") == "minor"])
    unguarded = len([n for n in human_nodes
                     if metadata_map.get(n["id"], {}).get("ageCategory") == "minor"
                     and not metadata_map.get(n["id"], {}).get("hasGuardian")])
    flagged = len([n for n in human_nodes
                   if metadata_map.get(n["id"], {}).get("hasFlags")])
    pseudonymous = len([n for n in human_nodes if n.get("isPseudonymous")])

    summary = {
        "lastUpdated": timestamp,
        "description": "Human graph layer statistics",
        "totals": {
            "humans": len(human_nodes),
            "relationships": len(relationships),
            "consents": len(consents),
            "organizations": len(org_nodes),
            "communities": len(comm_nodes)
        },
        "byCategory": {
            cat: {
                "count": count,
                "description": categories.get(cat, {}).get("description", "")
            }
            for cat, count in by_category.items()
        },
        "byRelationshipType": by_rel_type,
        "byIntimacyLevel": by_intimacy,
        "specialCases": {
            "minors": minors,
            "unguardedMinors": unguarded,
            "flaggedUsers": flagged,
            "pseudonymous": pseudonymous
        }
    }

    write_json_file(GRAPH_DIR / "human-summary.json", summary)
    return summary


# =============================================================================
# MAIN
# =============================================================================

def main():
    print("=" * 60)
    print("Human Import - Aligned with TypeScript Models")
    print("=" * 60)
    print("\nGenerating data aligned with:")
    print("  - human-node.model.ts (HumanNode, HumanRelationship)")
    print("  - human-consent.model.ts (HumanConsent)")
    print("  - content-node.model.ts (Organization, Community)")
    print("=" * 60)

    timestamp = now_iso()

    # Ensure directories exist
    CONTENT_DIR.mkdir(parents=True, exist_ok=True)
    GRAPH_DIR.mkdir(parents=True, exist_ok=True)

    # Load source data
    print(f"\n1. Loading humans from {HUMANS_SOURCE}...")
    with open(HUMANS_SOURCE) as f:
        humans_data = json.load(f)

    humans = humans_data.get("humans", [])
    relationships_data = humans_data.get("relationships", [])
    categories = humans_data.get("categories", {})

    print(f"   Loaded {len(humans)} humans, {len(relationships_data)} relationships")

    # 2. Create HumanNodes
    print("\n2. Creating HumanNodes (human-node.model.ts)...")
    human_nodes = []
    metadata_map = {}
    for human in humans:
        node = create_human_node(human, timestamp)
        metadata = create_human_node_metadata(human)
        human_nodes.append(node)
        metadata_map[node["id"]] = metadata

    print(f"   Created {len(human_nodes)} HumanNodes")

    # 3. Create HumanRelationships and HumanConsents
    print("\n3. Creating HumanRelationships and HumanConsents...")
    relationships = []
    consents = []

    for idx, rel_data in enumerate(relationships_data):
        consent_id = f"consent-{idx:04d}"
        relationship = create_human_relationship(rel_data, idx, consent_id, timestamp)
        consent = create_human_consent(rel_data, idx, timestamp)

        relationships.append(relationship)
        consents.append(consent)

        # Update relationship IDs on human nodes
        source_id = rel_data["source"]
        target_id = rel_data["target"]
        for node in human_nodes:
            if node["id"] == source_id or node["id"] == target_id:
                if "relationshipIds" not in node:
                    node["relationshipIds"] = []
                node["relationshipIds"].append(relationship["id"])

    # Count trusted connections
    for node in human_nodes:
        trusted_count = 0
        for rel in relationships:
            if (rel["sourceHumanId"] == node["id"] or rel["targetHumanId"] == node["id"]):
                if rel["intimacyLevel"] in ["trusted", "intimate"]:
                    trusted_count += 1
        node["trustedConnectionCount"] = trusted_count

    print(f"   Created {len(relationships)} HumanRelationships")
    print(f"   Created {len(consents)} HumanConsents")

    # 4. Extract organizations and communities
    print("\n4. Creating Organization and Community ContentNodes...")
    orgs = extract_organizations(humans)
    org_nodes = [create_organization_content_node(o, timestamp) for o in orgs]

    communities = extract_communities(humans)
    comm_nodes = [create_community_content_node(c, timestamp) for c in communities]

    print(f"   Created {len(org_nodes)} Organizations")
    print(f"   Created {len(comm_nodes)} Communities")

    # 5. Write all files
    print("\n5. Writing files...")

    humans_written = write_human_nodes(human_nodes, metadata_map, timestamp)
    print(f"   - HumanNodes: {humans_written} files")

    rels_written = write_relationships(relationships, timestamp)
    print(f"   - HumanRelationships: {rels_written} in human-relationships.json")

    consents_written = write_consents(consents, timestamp)
    print(f"   - HumanConsents: {consents_written} in human-consents.json")

    orgs_written = write_content_nodes(org_nodes, "organization", timestamp)
    print(f"   - Organizations: {orgs_written} files")

    comms_written = write_content_nodes(comm_nodes, "community", timestamp)
    print(f"   - Communities: {comms_written} files")

    added_to_index = update_content_index(human_nodes, org_nodes, comm_nodes, timestamp)
    print(f"   - Added {added_to_index} to content index")

    # 6. Create graph overviews
    print("\n6. Creating graph overview and summary...")
    overview = create_human_graph_overview(human_nodes, relationships, metadata_map, timestamp)
    print(f"   - human-overview.json ({overview['totalNodes']} nodes, {overview['totalEdges']} edges)")

    summary = create_human_graph_summary(
        human_nodes, relationships, consents, org_nodes, comm_nodes,
        metadata_map, categories, timestamp
    )
    print(f"   - human-summary.json")

    # Summary
    print("\n" + "=" * 60)
    print("IMPORT COMPLETE!")
    print("=" * 60)

    print(f"\n📊 TOTALS:")
    print(f"   HumanNodes:        {len(human_nodes):>4}")
    print(f"   HumanRelationships: {len(relationships):>3}")
    print(f"   HumanConsents:     {len(consents):>4}")
    print(f"   Organizations:     {len(org_nodes):>4}")
    print(f"   Communities:       {len(comm_nodes):>4}")

    print(f"\n🔗 BY INTIMACY:")
    for level in INTIMACY_LEVELS:
        count = summary["byIntimacyLevel"].get(level, 0)
        if count > 0:
            print(f"   {level:<12}: {count:>3}")

    print(f"\n📁 OUTPUT:")
    print(f"   {CONTENT_DIR}/humans/     - HumanNode files")
    print(f"   {GRAPH_DIR}/human-relationships.json")
    print(f"   {GRAPH_DIR}/human-consents.json")
    print(f"   {GRAPH_DIR}/human-overview.json")
    print(f"   {GRAPH_DIR}/human-summary.json")

    print("\n" + "=" * 60)


if __name__ == "__main__":
    main()
