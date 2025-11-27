#!/usr/bin/env python3
"""
Generate Lamad mock data from docs directory.
Run from elohim-app root: python scripts/generate_lamad_data.py

Parses /src/assets/docs/ content and outputs to /src/assets/lamad-data/
"""

import os
import json
import re
import yaml
from pathlib import Path
from datetime import datetime
from typing import Optional

# Paths (relative to elohim-app root)
DOCS_DIR = Path("src/assets/docs")
OUTPUT_DIR = Path("src/assets/lamad-data")
MANIFEST_PATH = DOCS_DIR / "manifest.json"

# Content type mapping from manifest types to Lamad ContentTypes
TYPE_MAPPING = {
    "epic": "epic",
    "feature": "feature",
    "scenario": "scenario",
    "user_type": "concept",  # User types are conceptual content
    "book": "book-chapter",
    "organization": "organization",
    "audio": "video",  # Audio treated as video type
    "video": "video",
    "document": "concept",
    "article": "concept",
}

# Epic definitions for the learning path
EPICS = {
    "governance": {
        "id": "governance-epic",
        "title": "AI Governance",
        "path": "governance/epic.md",
        "description": "Constitutional oversight, appeals, and democratic AI governance"
    },
    "value_scanner": {
        "id": "value-scanner-epic",
        "title": "Value Scanner",
        "path": "value_scanner/epic.md",
        "description": "Supporting caregivers and recognizing invisible work"
    },
    "public_observer": {
        "id": "public-observer-epic",
        "title": "Public Observer",
        "path": "public_observer/epic.md",
        "description": "Civic participation and public oversight"
    },
    "autonomous_entity": {
        "id": "autonomous-entity-epic",
        "title": "Autonomous Entity",
        "path": "autonomous_entity/epic.md",
        "description": "Transforming workplace ownership and governance"
    },
    "social_medium": {
        "id": "social-medium-epic",
        "title": "Social Medium",
        "path": "social_medium/epic.md",
        "description": "Building healthier digital communication spaces"
    },
    "economic_coordination": {
        "id": "economic-coordination-epic",
        "title": "Economic Coordination",
        "path": "economic_coordination/epic.md",
        "description": "REA-based value flows, creator recognition, and network economics"
    },
}


def generate_id(source_path: str, prefix: str = "") -> str:
    """Generate a deterministic ID from file path."""
    # Remove file extension and convert to kebab-case
    path_part = source_path.replace(".md", "").replace(".feature", "")
    path_part = path_part.replace("/", "-").replace("_", "-")
    path_part = re.sub(r"-+", "-", path_part)  # Collapse multiple dashes
    path_part = path_part.strip("-").lower()

    if prefix:
        return f"{prefix}-{path_part}"
    return path_part


def parse_yaml_frontmatter(content: str) -> tuple[dict, str]:
    """Extract YAML frontmatter from markdown content."""
    frontmatter = {}
    body = content

    if content.startswith("---"):
        parts = content.split("---", 2)
        if len(parts) >= 3:
            try:
                frontmatter = yaml.safe_load(parts[1]) or {}
                body = parts[2].strip()
            except yaml.YAMLError:
                pass

    return frontmatter, body


def extract_title(content: str, frontmatter: dict) -> str:
    """Extract title from frontmatter or first H1."""
    if "title" in frontmatter:
        return frontmatter["title"]

    # Look for first H1
    match = re.search(r"^#\s+\*?\*?(.+?)\*?\*?\s*$", content, re.MULTILINE)
    if match:
        # Clean up any markdown formatting
        title = match.group(1).strip()
        title = re.sub(r"\*\*(.+?)\*\*", r"\1", title)  # Remove bold
        return title

    return "Untitled"


def extract_description(content: str, frontmatter: dict) -> str:
    """Extract description from frontmatter or first paragraph."""
    if "description" in frontmatter:
        return frontmatter["description"]

    # Get first paragraph after H1
    lines = content.split("\n")
    in_content = False
    paragraph_lines = []

    for line in lines:
        stripped = line.strip()
        if stripped.startswith("#"):
            in_content = True
            continue
        if in_content and stripped:
            if stripped.startswith("#") or stripped.startswith("---"):
                break
            paragraph_lines.append(stripped)
            if len(" ".join(paragraph_lines)) > 200:
                break
        elif in_content and not stripped and paragraph_lines:
            break

    description = " ".join(paragraph_lines)[:500]
    return description if description else "No description available"


def infer_category(source_path: str) -> str:
    """Infer category from file path."""
    parts = source_path.split("/")
    if len(parts) > 1:
        return parts[0].replace("_", "-")
    return "general"


def extract_tags(frontmatter: dict, source_path: str, content_type: str) -> list[str]:
    """Extract tags from frontmatter and path."""
    tags = []

    # From frontmatter
    if "tags" in frontmatter:
        tags.extend(frontmatter["tags"] if isinstance(frontmatter["tags"], list) else [frontmatter["tags"]])

    # Add content type as tag
    tags.append(content_type)

    # Add category as tag
    category = infer_category(source_path)
    if category != "general":
        tags.append(category)

    # Add epic if present
    if "epic" in frontmatter:
        tags.append(f"epic:{frontmatter['epic']}")

    # Add user_type if present
    if "user_type" in frontmatter:
        tags.append(f"user:{frontmatter['user_type']}")

    return list(set(tags))


def parse_markdown(file_path: Path, manifest_type: str) -> Optional[dict]:
    """Parse a markdown file into ContentNode dict."""
    try:
        content = file_path.read_text(encoding="utf-8")
    except Exception as e:
        print(f"  Error reading {file_path}: {e}")
        return None

    source_path = str(file_path.relative_to(DOCS_DIR))
    frontmatter, body = parse_yaml_frontmatter(content)

    title = extract_title(content, frontmatter)
    description = extract_description(body, frontmatter)
    content_type = TYPE_MAPPING.get(manifest_type, "concept")

    # Build metadata
    metadata = {
        "category": infer_category(source_path),
    }

    # Copy relevant frontmatter fields
    for key in ["epic", "user_type", "archetype_name", "epic_domain",
                "governance_scope", "related_users", "related_epics",
                "primary_epic", "status", "version", "authors", "author"]:
        if key in frontmatter:
            metadata[key] = frontmatter[key]

    # Build related node IDs
    related_ids = []
    if "related_users" in frontmatter and isinstance(frontmatter["related_users"], list):
        for user in frontmatter["related_users"]:
            related_ids.append(f"user-type-{user.replace('_', '-')}")
    if "related_epics" in frontmatter and isinstance(frontmatter["related_epics"], list):
        for epic in frontmatter["related_epics"]:
            related_ids.append(f"{epic.replace('_', '-')}-epic")
    if "epic" in frontmatter:
        related_ids.append(f"{frontmatter['epic'].replace('_', '-')}-epic")

    # Get file modification time
    mtime = datetime.fromtimestamp(file_path.stat().st_mtime).isoformat()

    return {
        "id": generate_id(source_path),
        "contentType": content_type,
        "title": title,
        "description": description,
        "content": content,  # Full markdown content
        "contentFormat": "markdown",
        "tags": extract_tags(frontmatter, source_path, content_type),
        "sourcePath": source_path,
        "relatedNodeIds": related_ids,
        "metadata": metadata,
        "createdAt": mtime,
        "updatedAt": mtime,
    }


def parse_gherkin(file_path: Path) -> list[dict]:
    """Parse a Gherkin feature file into ContentNode dicts."""
    try:
        content = file_path.read_text(encoding="utf-8")
    except Exception as e:
        print(f"  Error reading {file_path}: {e}")
        return []

    source_path = str(file_path.relative_to(DOCS_DIR))
    mtime = datetime.fromtimestamp(file_path.stat().st_mtime).isoformat()
    category = infer_category(source_path)

    nodes = []
    lines = content.split("\n")

    # Extract feature-level tags
    feature_tags = []
    feature_title = ""
    feature_description = []
    current_line = 0

    # Parse tags at start
    while current_line < len(lines):
        line = lines[current_line].strip()
        if line.startswith("@"):
            feature_tags.extend([t.strip("@") for t in line.split() if t.startswith("@")])
            current_line += 1
        elif line.startswith("Feature:"):
            feature_title = line.replace("Feature:", "").strip()
            current_line += 1
            break
        else:
            current_line += 1

    # Parse feature description
    while current_line < len(lines):
        line = lines[current_line].strip()
        if line.startswith("@") or line.startswith("Scenario") or line.startswith("Background"):
            break
        if line:
            feature_description.append(line)
        current_line += 1

    # Extract epic from tags
    epic_id = None
    for tag in feature_tags:
        if tag.startswith("epic:"):
            epic_id = tag.split(":")[1]

    # Create feature node
    feature_id = generate_id(source_path, "feature")
    scenario_ids = []

    # Parse scenarios
    while current_line < len(lines):
        line = lines[current_line].strip()

        if line.startswith("@"):
            scenario_tags = [t.strip("@") for t in line.split() if t.startswith("@")]
            current_line += 1

            # Look for scenario
            if current_line < len(lines):
                scenario_line = lines[current_line].strip()
                scenario_match = re.match(r"^(Scenario|Scenario Outline):\s*(.+)$", scenario_line)
                if scenario_match:
                    scenario_type = "scenario"
                    scenario_title = scenario_match.group(2).strip()
                    scenario_id = generate_id(f"{source_path}-{scenario_title}", "scenario")
                    scenario_ids.append(scenario_id)

                    # Collect scenario content
                    scenario_content_lines = [scenario_line]
                    current_line += 1
                    while current_line < len(lines):
                        next_line = lines[current_line]
                        if next_line.strip().startswith("@") or next_line.strip().startswith("Scenario"):
                            break
                        scenario_content_lines.append(next_line)
                        current_line += 1

                    nodes.append({
                        "id": scenario_id,
                        "contentType": "scenario",
                        "title": scenario_title,
                        "description": scenario_title,
                        "content": "\n".join(scenario_content_lines),
                        "contentFormat": "gherkin",
                        "tags": scenario_tags + feature_tags + [category, "scenario"],
                        "sourcePath": source_path,
                        "relatedNodeIds": [feature_id] + ([f"{epic_id.replace('_', '-')}-epic"] if epic_id else []),
                        "metadata": {
                            "category": category,
                            "featureId": feature_id,
                            "epic": epic_id,
                        },
                        "createdAt": mtime,
                        "updatedAt": mtime,
                    })
        elif line.startswith("Scenario"):
            scenario_match = re.match(r"^(Scenario|Scenario Outline):\s*(.+)$", line)
            if scenario_match:
                scenario_title = scenario_match.group(2).strip()
                scenario_id = generate_id(f"{source_path}-{scenario_title}", "scenario")
                scenario_ids.append(scenario_id)

                scenario_content_lines = [line]
                current_line += 1
                while current_line < len(lines):
                    next_line = lines[current_line]
                    if next_line.strip().startswith("@") or next_line.strip().startswith("Scenario"):
                        break
                    scenario_content_lines.append(next_line)
                    current_line += 1

                nodes.append({
                    "id": scenario_id,
                    "contentType": "scenario",
                    "title": scenario_title,
                    "description": scenario_title,
                    "content": "\n".join(scenario_content_lines),
                    "contentFormat": "gherkin",
                    "tags": feature_tags + [category, "scenario"],
                    "sourcePath": source_path,
                    "relatedNodeIds": [feature_id] + ([f"{epic_id.replace('_', '-')}-epic"] if epic_id else []),
                    "metadata": {
                        "category": category,
                        "featureId": feature_id,
                        "epic": epic_id,
                    },
                    "createdAt": mtime,
                    "updatedAt": mtime,
                })
        else:
            current_line += 1

    # Create feature node
    feature_node = {
        "id": feature_id,
        "contentType": "feature",
        "title": feature_title or "Untitled Feature",
        "description": " ".join(feature_description)[:500] if feature_description else feature_title,
        "content": content,
        "contentFormat": "gherkin",
        "tags": feature_tags + [category, "feature"],
        "sourcePath": source_path,
        "relatedNodeIds": scenario_ids + ([f"{epic_id.replace('_', '-')}-epic"] if epic_id else []),
        "metadata": {
            "category": category,
            "epic": epic_id,
            "scenarioCount": len(scenario_ids),
        },
        "createdAt": mtime,
        "updatedAt": mtime,
    }

    return [feature_node] + nodes


def create_quiz_content_node() -> dict:
    """Create the 'Who Are You?' quiz assessment node."""
    now = datetime.now().isoformat()

    return {
        "id": "quiz-who-are-you",
        "contentType": "assessment",
        "title": "Find Your Path in the Protocol",
        "description": "A brief quiz to help you discover which epic domain resonates with you",
        "content": {
            "passingScore": 0,  # No pass/fail, just recommendations
            "allowRetake": True,
            "showCorrectAnswers": False,
            "questions": [
                {
                    "id": "q1",
                    "type": "multiple-choice",
                    "question": "Which of these activities interests you most?",
                    "options": [
                        "Shaping AI policy and constitutional frameworks",
                        "Supporting caregivers and recognizing invisible work",
                        "Transforming workplace ownership and governance",
                        "Civic participation and public oversight",
                        "Building healthier digital communication spaces"
                    ],
                    "rubric": {
                        "0": "governance",
                        "1": "value_scanner",
                        "2": "autonomous_entity",
                        "3": "public_observer",
                        "4": "social_medium"
                    }
                },
                {
                    "id": "q2",
                    "type": "multiple-choice",
                    "question": "What aspect of the current system concerns you most?",
                    "options": [
                        "AI systems making decisions without democratic oversight",
                        "Unpaid care work being invisible in our economy",
                        "Workers having no voice in their workplaces",
                        "Citizens being disconnected from decision-making",
                        "Social media amplifying division and misinformation"
                    ],
                    "rubric": {
                        "0": "governance",
                        "1": "value_scanner",
                        "2": "autonomous_entity",
                        "3": "public_observer",
                        "4": "social_medium"
                    }
                },
                {
                    "id": "q3",
                    "type": "multiple-choice",
                    "question": "Which role resonates most with how you see yourself?",
                    "options": [
                        "Policy maker, constitutional council member, or researcher",
                        "Caregiver, parent, or community supporter",
                        "Worker, business owner, or economic participant",
                        "Citizen, activist, journalist, or community organizer",
                        "Content creator, moderator, or digital community builder"
                    ],
                    "rubric": {
                        "0": "governance",
                        "1": "value_scanner",
                        "2": "autonomous_entity",
                        "3": "public_observer",
                        "4": "social_medium"
                    }
                },
                {
                    "id": "q4",
                    "type": "multiple-choice",
                    "question": "What kind of impact do you want to have?",
                    "options": [
                        "Ensuring AI serves humanity through proper governance",
                        "Making care work visible and valued in society",
                        "Creating more equitable economic relationships",
                        "Strengthening democratic participation and transparency",
                        "Fostering genuine human connection online"
                    ],
                    "rubric": {
                        "0": "governance",
                        "1": "value_scanner",
                        "2": "autonomous_entity",
                        "3": "public_observer",
                        "4": "social_medium"
                    }
                }
            ],
            "resultMapping": {
                "governance": {
                    "epicId": "governance-epic",
                    "title": "AI Governance",
                    "description": "You're drawn to constitutional oversight and democratic AI governance"
                },
                "value_scanner": {
                    "epicId": "value-scanner-epic",
                    "title": "Value Scanner",
                    "description": "You're passionate about recognizing and valuing care work"
                },
                "autonomous_entity": {
                    "epicId": "autonomous-entity-epic",
                    "title": "Autonomous Entity",
                    "description": "You want to transform workplace ownership and economic relationships"
                },
                "public_observer": {
                    "epicId": "public-observer-epic",
                    "title": "Public Observer",
                    "description": "You're committed to civic participation and public accountability"
                },
                "social_medium": {
                    "epicId": "social-medium-epic",
                    "title": "Social Medium",
                    "description": "You want to build healthier digital communication spaces"
                }
            }
        },
        "contentFormat": "quiz-json",
        "tags": ["assessment", "onboarding", "personalization", "quiz"],
        "sourcePath": "generated/quiz-who-are-you.json",
        "relatedNodeIds": list(EPICS.keys()),
        "metadata": {
            "recommendsEpics": True,
            "quizType": "domain-discovery",
            "category": "onboarding"
        },
        "createdAt": now,
        "updatedAt": now,
    }


def create_learning_path(content_nodes: dict) -> dict:
    """Create the main Elohim Protocol learning path."""
    now = datetime.now().isoformat()

    steps = [
        # Step 0: Manifesto
        {
            "order": 0,
            "resourceId": "manifesto",
            "stepTitle": "The Vision",
            "stepNarrative": "Begin with the foundational vision of the Elohim Protocol - digital infrastructure for human flourishing",
            "learningObjectives": [
                "Understand the core vision of decentralized governance",
                "Learn the five epic domains",
                "Identify the principles of sovereign identity and love-centered technology"
            ],
            "optional": False,
            "completionCriteria": ["Read the manifesto"],
            "estimatedTime": "30-45 minutes"
        },
        # Step 1: Who Are You? Quiz
        {
            "order": 1,
            "resourceId": "quiz-who-are-you",
            "stepTitle": "Find Your Path",
            "stepNarrative": "Discover which epic domain aligns with your interests, values, and the impact you want to make",
            "learningObjectives": ["Identify your primary domain of interest"],
            "optional": False,
            "completionCriteria": ["Complete the quiz"],
            "attestationGranted": "protocol-explorer",
            "estimatedTime": "5-10 minutes"
        },
    ]

    # Add epic steps (optional after quiz)
    epic_order = 2
    for key, epic in EPICS.items():
        steps.append({
            "order": epic_order,
            "resourceId": epic["id"],
            "stepTitle": epic["title"],
            "stepNarrative": epic["description"],
            "learningObjectives": [
                f"Understand the {epic['title']} domain",
                "Learn about user types in this domain",
                "Explore scenarios and use cases"
            ],
            "optional": True,  # User picks based on quiz recommendation
            "completionCriteria": [f"Read the {epic['title']} epic overview"],
            "estimatedTime": "15-20 minutes"
        })
        epic_order += 1

    return {
        "id": "elohim-protocol",
        "version": "1.0.0",
        "title": "Elohim Protocol: Living Documentation",
        "description": "Discover the vision and find your place in the Protocol",
        "purpose": "Understand the Elohim Protocol and identify which domain resonates with you",
        "createdBy": "system",
        "contributors": [],
        "createdAt": now,
        "updatedAt": now,
        "steps": steps,
        "tags": ["foundation", "protocol", "onboarding"],
        "difficulty": "beginner",
        "estimatedDuration": "1-2 hours",
        "visibility": "public",
        "attestationsGranted": ["protocol-explorer"]
    }


def create_content_index(nodes: list[dict]) -> dict:
    """Create the content index (metadata only)."""
    now = datetime.now().isoformat()

    index_entries = []
    for node in nodes:
        index_entries.append({
            "id": node["id"],
            "title": node["title"],
            "description": node["description"][:200] if len(node.get("description", "")) > 200 else node.get("description", ""),
            "contentType": node["contentType"],
            "tags": node["tags"]
        })

    return {
        "nodes": index_entries,
        "lastUpdated": now,
        "totalCount": len(index_entries)
    }


def create_path_index(learning_path: dict) -> dict:
    """Create the path index."""
    now = datetime.now().isoformat()

    return {
        "lastUpdated": now,
        "totalCount": 1,
        "paths": [
            {
                "id": learning_path["id"],
                "title": learning_path["title"],
                "description": learning_path["description"],
                "difficulty": learning_path["difficulty"],
                "estimatedDuration": learning_path["estimatedDuration"],
                "stepCount": len(learning_path["steps"]),
                "tags": learning_path["tags"]
            }
        ]
    }


def create_graph_relationships(nodes_by_id: dict) -> dict:
    """
    Create explicit graph relationships for visualization.

    Relationships:
    - CONTAINS: Epic → Features, Feature → Scenarios
    - BELONGS_TO: Reverse of CONTAINS
    - REQUIRES: Prerequisites (manifesto required for all epics)
    - RELATES_TO: Cross-domain connections
    """
    now = datetime.now().isoformat()
    relationships = []
    relationship_id = 0

    # Track which nodes belong to which epics/features
    epic_features = {epic["id"]: [] for epic in EPICS.values()}
    feature_scenarios = {}

    for node_id, node in nodes_by_id.items():
        content_type = node.get("contentType", "")
        metadata = node.get("metadata", {})
        epic_id = metadata.get("epic")

        # Feature belongs to epic
        if content_type == "feature" and epic_id:
            epic_node_id = f"{epic_id.replace('_', '-')}-epic"
            if epic_node_id in epic_features:
                epic_features[epic_node_id].append(node_id)

                # Add CONTAINS relationship
                relationships.append({
                    "id": f"rel-{relationship_id}",
                    "source": epic_node_id,
                    "target": node_id,
                    "type": "CONTAINS",
                    "metadata": {"level": 1}
                })
                relationship_id += 1

        # Scenario belongs to feature
        if content_type == "scenario":
            feature_id = metadata.get("featureId")
            if feature_id:
                if feature_id not in feature_scenarios:
                    feature_scenarios[feature_id] = []
                feature_scenarios[feature_id].append(node_id)

                # Add CONTAINS relationship
                relationships.append({
                    "id": f"rel-{relationship_id}",
                    "source": feature_id,
                    "target": node_id,
                    "type": "CONTAINS",
                    "metadata": {"level": 2}
                })
                relationship_id += 1

    # Add REQUIRES relationships (manifesto required for all epics)
    if "manifesto" in nodes_by_id:
        for epic in EPICS.values():
            relationships.append({
                "id": f"rel-{relationship_id}",
                "source": epic["id"],
                "target": "manifesto",
                "type": "REQUIRES",
                "metadata": {"prerequisite": True}
            })
            relationship_id += 1

    # Add cross-epic RELATES_TO relationships (epics relate to each other)
    epic_ids = [epic["id"] for epic in EPICS.values()]
    for i, epic_id in enumerate(epic_ids):
        # Each epic relates to the next (circular)
        next_epic = epic_ids[(i + 1) % len(epic_ids)]
        relationships.append({
            "id": f"rel-{relationship_id}",
            "source": epic_id,
            "target": next_epic,
            "type": "RELATES_TO",
            "metadata": {"cross_domain": True}
        })
        relationship_id += 1

    return {
        "lastUpdated": now,
        "totalCount": len(relationships),
        "relationships": relationships,
        "summary": {
            "epicCount": len(EPICS),
            "featureCount": sum(len(features) for features in epic_features.values()),
            "scenarioCount": sum(len(scenarios) for scenarios in feature_scenarios.values()),
            "byEpic": {
                epic_id: {
                    "featureCount": len(features),
                    "features": features
                }
                for epic_id, features in epic_features.items()
            }
        }
    }


def create_graph_overview(nodes_by_id: dict) -> dict:
    """
    Create the graph overview for zoom level 0.
    Contains only epics + manifesto with their connections.
    """
    now = datetime.now().isoformat()

    overview_nodes = []

    # Add manifesto node
    if "manifesto" in nodes_by_id:
        manifesto = nodes_by_id["manifesto"]
        overview_nodes.append({
            "id": "manifesto",
            "title": manifesto.get("title", "The Elohim Protocol"),
            "contentType": "concept",
            "description": manifesto.get("description", "")[:200],
            "hasChildren": False,
            "childCount": 0,
            "position": {"x": 0, "y": 0},  # Center position
            "level": 0,
            "isRoot": True
        })

    # Add epic nodes in a circle around manifesto
    import math
    epic_list = list(EPICS.values())
    for i, epic in enumerate(epic_list):
        epic_id = epic["id"]
        angle = (2 * math.pi * i) / len(epic_list)
        radius = 200  # Distance from center

        # Count features for this epic
        feature_count = sum(
            1 for node in nodes_by_id.values()
            if node.get("contentType") == "feature"
            and node.get("metadata", {}).get("epic") == epic_id.replace("-epic", "").replace("-", "_")
        )

        overview_nodes.append({
            "id": epic_id,
            "title": epic["title"],
            "contentType": "epic",
            "description": epic["description"],
            "hasChildren": feature_count > 0,
            "childCount": feature_count,
            "position": {
                "x": radius * math.cos(angle),
                "y": radius * math.sin(angle)
            },
            "level": 0,
            "isRoot": False
        })

    # Create edges for overview
    overview_edges = []

    # Connect manifesto to all epics
    for epic in EPICS.values():
        overview_edges.append({
            "source": "manifesto",
            "target": epic["id"],
            "type": "FOUNDATION"
        })

    # Connect adjacent epics in circle
    for i, epic in enumerate(epic_list):
        next_epic = epic_list[(i + 1) % len(epic_list)]
        overview_edges.append({
            "source": epic["id"],
            "target": next_epic["id"],
            "type": "RELATES_TO"
        })

    return {
        "lastUpdated": now,
        "zoomLevel": 0,
        "nodes": overview_nodes,
        "edges": overview_edges,
        "totalNodes": len(overview_nodes),
        "totalEdges": len(overview_edges)
    }


def create_epic_detail(epic_key: str, nodes_by_id: dict) -> dict:
    """
    Create detail view for a specific epic (zoom level 1).
    Contains the epic and its features.
    """
    now = datetime.now().isoformat()

    epic_info = EPICS.get(epic_key)
    if not epic_info:
        return None

    epic_id = epic_info["id"]
    nodes = []
    edges = []

    # Add epic node at center
    nodes.append({
        "id": epic_id,
        "title": epic_info["title"],
        "contentType": "epic",
        "description": epic_info["description"],
        "hasChildren": True,
        "childCount": 0,  # Will be updated
        "position": {"x": 0, "y": 0},
        "level": 1,
        "isParent": True
    })

    # Find all features for this epic
    import math
    features = []
    for node_id, node in nodes_by_id.items():
        if (node.get("contentType") == "feature" and
            node.get("metadata", {}).get("epic") == epic_key):
            features.append(node)

    # Position features in a circle around epic
    for i, feature in enumerate(features):
        angle = (2 * math.pi * i) / max(len(features), 1)
        radius = 150

        # Count scenarios for this feature
        scenario_count = sum(
            1 for n in nodes_by_id.values()
            if n.get("metadata", {}).get("featureId") == feature["id"]
        )

        nodes.append({
            "id": feature["id"],
            "title": feature.get("title", "Untitled"),
            "contentType": "feature",
            "description": feature.get("description", "")[:200],
            "hasChildren": scenario_count > 0,
            "childCount": scenario_count,
            "position": {
                "x": radius * math.cos(angle),
                "y": radius * math.sin(angle)
            },
            "level": 1,
            "isParent": False
        })

        edges.append({
            "source": epic_id,
            "target": feature["id"],
            "type": "CONTAINS"
        })

    # Update epic child count
    nodes[0]["childCount"] = len(features)

    return {
        "lastUpdated": now,
        "zoomLevel": 1,
        "parentId": epic_id,
        "epicKey": epic_key,
        "nodes": nodes,
        "edges": edges,
        "totalNodes": len(nodes),
        "totalEdges": len(edges)
    }


def main():
    print("=" * 60)
    print("Lamad Data Generator")
    print("=" * 60)

    # Ensure we're in the right directory
    if not DOCS_DIR.exists():
        print(f"Error: {DOCS_DIR} not found. Run from elohim-app root.")
        return

    # Load manifest
    print(f"\n1. Loading manifest from {MANIFEST_PATH}...")
    try:
        manifest = json.loads(MANIFEST_PATH.read_text())
    except Exception as e:
        print(f"Error loading manifest: {e}")
        return

    files = manifest.get("files", [])
    print(f"   Found {len(files)} files in manifest")

    # Create output directories
    print(f"\n2. Creating output directories...")
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    (OUTPUT_DIR / "paths").mkdir(exist_ok=True)
    (OUTPUT_DIR / "content").mkdir(exist_ok=True)
    print(f"   Created {OUTPUT_DIR}")

    # Parse all content files
    print(f"\n3. Parsing content files...")
    all_nodes = []
    nodes_by_id = {}

    for file_entry in files:
        file_path = DOCS_DIR / file_entry["path"]
        file_type = file_entry.get("type", "article")

        if not file_path.exists():
            print(f"   Skipping missing: {file_entry['path']}")
            continue

        if file_entry["path"].endswith(".feature"):
            # Parse Gherkin
            nodes = parse_gherkin(file_path)
            for node in nodes:
                if node["id"] not in nodes_by_id:
                    nodes_by_id[node["id"]] = node
                    all_nodes.append(node)
        elif file_entry["path"].endswith(".md"):
            # Parse Markdown
            node = parse_markdown(file_path, file_type)
            if node and node["id"] not in nodes_by_id:
                nodes_by_id[node["id"]] = node
                all_nodes.append(node)

    print(f"   Parsed {len(all_nodes)} content nodes")

    # Count by type
    type_counts = {}
    for node in all_nodes:
        ct = node["contentType"]
        type_counts[ct] = type_counts.get(ct, 0) + 1
    print(f"   Types: {type_counts}")

    # Add quiz content node
    print(f"\n4. Creating quiz assessment...")
    quiz_node = create_quiz_content_node()
    all_nodes.append(quiz_node)
    nodes_by_id[quiz_node["id"]] = quiz_node
    print(f"   Created: {quiz_node['id']}")

    # Create learning path
    print(f"\n5. Creating learning path...")
    learning_path = create_learning_path(nodes_by_id)
    print(f"   Created: {learning_path['id']} with {len(learning_path['steps'])} steps")

    # Create indexes
    print(f"\n6. Creating indexes...")
    content_index = create_content_index(all_nodes)
    path_index = create_path_index(learning_path)
    print(f"   Content index: {content_index['totalCount']} entries")
    print(f"   Path index: {path_index['totalCount']} paths")

    # Create graph data for visualization
    print(f"\n7. Creating graph data...")
    (OUTPUT_DIR / "graph").mkdir(exist_ok=True)

    # Generate relationships
    graph_relationships = create_graph_relationships(nodes_by_id)
    print(f"   Generated {graph_relationships['totalCount']} relationships")

    # Generate overview (zoom level 0)
    graph_overview = create_graph_overview(nodes_by_id)
    print(f"   Generated overview with {graph_overview['totalNodes']} nodes")

    # Generate epic details (zoom level 1)
    epic_details = {}
    for epic_key in EPICS.keys():
        detail = create_epic_detail(epic_key, nodes_by_id)
        if detail:
            epic_details[epic_key] = detail
            print(f"   Generated {epic_key} detail with {detail['totalNodes']} nodes")

    # Write output files
    print(f"\n8. Writing output files...")

    # Write content index
    index_path = OUTPUT_DIR / "content" / "index.json"
    index_path.write_text(json.dumps(content_index, indent=2))
    print(f"   Wrote: {index_path}")

    # Write individual content nodes
    for node in all_nodes:
        node_path = OUTPUT_DIR / "content" / f"{node['id']}.json"
        node_path.write_text(json.dumps(node, indent=2))
    print(f"   Wrote {len(all_nodes)} content files to {OUTPUT_DIR / 'content'}")

    # Write path index
    path_index_path = OUTPUT_DIR / "paths" / "index.json"
    path_index_path.write_text(json.dumps(path_index, indent=2))
    print(f"   Wrote: {path_index_path}")

    # Write learning path
    learning_path_path = OUTPUT_DIR / "paths" / f"{learning_path['id']}.json"
    learning_path_path.write_text(json.dumps(learning_path, indent=2))
    print(f"   Wrote: {learning_path_path}")

    # Write graph data
    graph_rel_path = OUTPUT_DIR / "graph" / "relationships.json"
    graph_rel_path.write_text(json.dumps(graph_relationships, indent=2))
    print(f"   Wrote: {graph_rel_path}")

    graph_overview_path = OUTPUT_DIR / "graph" / "overview.json"
    graph_overview_path.write_text(json.dumps(graph_overview, indent=2))
    print(f"   Wrote: {graph_overview_path}")

    for epic_key, detail in epic_details.items():
        detail_path = OUTPUT_DIR / "graph" / f"epic-{epic_key}.json"
        detail_path.write_text(json.dumps(detail, indent=2))
    print(f"   Wrote {len(epic_details)} epic detail files to {OUTPUT_DIR / 'graph'}")

    print(f"\n" + "=" * 60)
    print("Generation complete!")
    print(f"Output: {OUTPUT_DIR}")
    print(f"Graph data: {OUTPUT_DIR / 'graph'}")
    print("=" * 60)


if __name__ == "__main__":
    main()
