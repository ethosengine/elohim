#!/usr/bin/env python3
"""
Import Foundations for Christian Technology (FCT) content into Lamad format.
Run from elohim-app root: python scripts/import_fct_content.py

This script performs DEEP PARSING to extract meaningful structure:

1. Each FCT Module decomposes into:
   - Module Overview (course-module content node)
   - Learning Objectives (objective content nodes)
   - Bible References (bible-verse content nodes)
   - Stories/Examples (narrative content nodes)
   - Discussion Questions (activity content nodes)
   - Go Deeper Resources (video, book, article content nodes)
   - Application Actions (practice content nodes)

2. Creates rich graph relationships:
   - Bible verses ↔ concepts
   - Videos ↔ topics
   - Books ↔ themes
   - Organizations ↔ domains
   - Shared concepts ↔ Elohim Protocol nodes

3. Creates an FCT learning path with:
   - Chapters (CHT module alignment)
   - Nested steps (module sections)
   - Rich media embeds
   - Checkpoints (discussion activities)

Key Insight: FCT reuses Center for Humane Technology course structure but adds
Christian theological framing. This creates natural overlap with Elohim Protocol's
governance, social medium, and value scanner epics.
"""

import os
import json
import re
import uuid
from pathlib import Path
from datetime import datetime
from typing import Optional

# Paths
FCT_SOURCE_DIR = Path("/projects/elohim/fct")
OUTPUT_DIR = Path("src/assets/lamad-data")
CONTENT_DIR = OUTPUT_DIR / "content"
PATHS_DIR = OUTPUT_DIR / "paths"
GRAPH_DIR = OUTPUT_DIR / "graph"

# Timestamp helpers
def now_iso() -> str:
    return datetime.now().isoformat()

def gen_id(prefix: str) -> str:
    return f"{prefix}-{uuid.uuid4().hex[:8]}"


# =============================================================================
# STANDARDS-ALIGNED HELPER FUNCTIONS
# =============================================================================

def generate_did(source_path: str, node_type: str = "content") -> str:
    """Generate W3C DID from source path."""
    path_part = source_path.replace(".md", "").replace(".feature", "")
    path_part = path_part.replace("/", ":").replace("_", "-").lower()
    path_part = re.sub(r"-+", "-", path_part.strip("-"))
    return f"did:web:elohim.host:{node_type}:{path_part}"


# ActivityPub type mapping
ACTIVITYPUB_TYPE_MAPPING = {
    "epic": "Article",
    "feature": "Article",
    "scenario": "Note",
    "video": "Video",
    "book": "Document",
    "book-chapter": "Document",
    "bible-verse": "Note",
    "course-module": "Article",
    "simulation": "Application",
    "assessment": "Question",
    "concept": "Page",
    "organization": "Organization",
    "podcast": "AudioObject",
    "article": "Article",
}


def infer_activitypub_type(content_type: str) -> str:
    """Map ContentType to ActivityStreams vocabulary."""
    return ACTIVITYPUB_TYPE_MAPPING.get(content_type, "Page")


def get_git_timestamps(file_path: Path) -> dict:
    """Extract creation and modification dates from git history."""
    import subprocess

    try:
        # Get first commit (creation)
        created = subprocess.check_output(
            ["git", "log", "--diff-filter=A", "--format=%aI", "--", str(file_path)],
            cwd=str(Path.cwd()),
            stderr=subprocess.DEVNULL
        ).decode().strip().split('\n')[0]

        # Get last commit (modification)
        modified = subprocess.check_output(
            ["git", "log", "-1", "--format=%aI", "--", str(file_path)],
            cwd=str(Path.cwd()),
            stderr=subprocess.DEVNULL
        ).decode().strip()

        return {
            "created": created if created else datetime.now().isoformat(),
            "modified": modified if modified else datetime.now().isoformat()
        }
    except Exception:
        # Fallback to file system timestamps
        now = datetime.now().isoformat()
        try:
            mtime = datetime.fromtimestamp(file_path.stat().st_mtime).isoformat()
            return {"created": mtime, "modified": mtime}
        except:
            return {"created": now, "modified": now}


def generate_open_graph_metadata(
    title: str,
    description: str,
    node_id: str,
    content_type: str,
    frontmatter: dict,
    timestamps: dict
) -> dict:
    """Generate Open Graph metadata for social sharing."""
    og = {
        "ogTitle": title,
        "ogDescription": description[:200] if description else title,
        "ogType": "article" if content_type in ["epic", "feature", "scenario", "course-module", "article"] else "website",
        "ogUrl": f"https://elohim-protocol.org/content/{node_id}",
        "ogSiteName": "Elohim Protocol - Lamad Learning Platform",
    }

    # Add timestamps for articles
    if content_type in ["epic", "feature", "scenario", "concept", "course-module", "article"]:
        og["articlePublishedTime"] = timestamps.get("created")
        og["articleModifiedTime"] = timestamps.get("modified")

    # Placeholder image (UI devs can override)
    og["ogImage"] = f"https://elohim-protocol.org/assets/images/og-defaults/{content_type}.jpg"
    og["ogImageAlt"] = f"{title} - Elohim Protocol"

    return og


# Schema.org type mapping
SCHEMA_TYPE_MAPPING = {
    "epic": "Article",
    "feature": "Article",
    "video": "VideoObject",
    "book": "Book",
    "book-chapter": "Chapter",
    "organization": "Organization",
    "assessment": "Quiz",
    "course-module": "LearningResource",
    "bible-verse": "CreativeWork",
    "podcast": "PodcastEpisode",
    "article": "Article",
}


def generate_linked_data(
    node_id: str,
    did: str,
    content_type: str,
    title: str,
    description: str,
    timestamps: dict,
    frontmatter: dict
) -> dict:
    """Generate JSON-LD for semantic web compliance."""
    schema_type = SCHEMA_TYPE_MAPPING.get(content_type, "CreativeWork")

    linked_data = {
        "@context": "https://schema.org/",
        "@type": schema_type,
        "@id": f"https://elohim-protocol.org/content/{node_id}",
        "identifier": did,
        "name": title,
        "description": description,
        "dateCreated": timestamps.get("created"),
        "dateModified": timestamps.get("modified"),
        "publisher": {
            "@type": "Organization",
            "@id": "https://elohim-protocol.org",
            "name": "Elohim Protocol"
        }
    }

    # Add author if available
    if frontmatter.get("author_name") or frontmatter.get("author"):
        author_name = frontmatter.get("author_name") or frontmatter.get("author")
        linked_data["author"] = {
            "@type": "Person",
            "name": author_name
        }

    return linked_data


def add_standards_fields(node: dict, source_file: Optional[Path] = None) -> dict:
    """Add standards-aligned fields to an existing content node.

    This helper function augments nodes created elsewhere in the script
    with DID, ActivityPub, Open Graph, and JSON-LD fields.
    """
    node_id = node.get("id", "unknown")
    content_type = node.get("contentType", "concept")
    title = node.get("title", "Untitled")

    # Get description - handle both string and dict content
    description = node.get("description", "")
    if not description and isinstance(node.get("content"), dict):
        description = node.get("content", {}).get("introduction", "")[:500]
    if not description:
        description = title

    # Generate DID from source path or node ID
    source_path = node.get("sourcePath", node_id)
    node_did = generate_did(source_path, "content")

    # Get timestamps - use git if source file provided, otherwise use existing or now
    if source_file and source_file.exists():
        timestamps = get_git_timestamps(source_file)
    else:
        timestamps = {
            "created": node.get("createdAt", now_iso()),
            "modified": node.get("updatedAt", now_iso())
        }

    # Generate standards metadata
    frontmatter = node.get("metadata", {})
    og_metadata = generate_open_graph_metadata(
        title, description, node_id, content_type, frontmatter, timestamps
    )
    linked_data = generate_linked_data(
        node_id, node_did, content_type, title, description, timestamps, frontmatter
    )
    activitypub_type = infer_activitypub_type(content_type)

    # Add standards fields to node
    node["did"] = node_did
    node["activityPubType"] = activitypub_type
    node["openGraphMetadata"] = og_metadata
    node["linkedData"] = linked_data
    node["createdAt"] = timestamps["created"]
    node["updatedAt"] = timestamps["modified"]

    return node


# =============================================================================
# SHARED CONCEPTS - Content that bridges FCT and Elohim Protocol
# =============================================================================

# These are conceptual bridges between the two learning paths
# When a learner encounters these in one path, their familiarity carries to the other
SHARED_CONCEPTS = {
    "attention-economy": {
        "elohim_nodes": ["social-medium-epic"],
        "fct_relevance": "The race for human attention - central theme in CHT and FCT Module 1, 3, 5",
        "concept_label": "Attention Economy & Sacred Attention"
    },
    "systems-thinking": {
        "elohim_nodes": ["governance-epic", "public-observer-epic"],
        "fct_relevance": "FCT Module 2 - Systems Thinking, Complex vs Complicated",
        "concept_label": "Systems Thinking"
    },
    "metacrisis": {
        "elohim_nodes": ["manifesto", "governance-epic"],
        "fct_relevance": "FCT Module 3 - Problem Well Stated, Exponential Technology",
        "concept_label": "Metacrisis & Polycrisis"
    },
    "trust-evolution": {
        "elohim_nodes": ["governance-epic", "social-medium-epic"],
        "fct_relevance": "FCT Module 4 - Evolution of Trust, Game Theory",
        "concept_label": "Trust & Coordination"
    },
    "human-dignity": {
        "elohim_nodes": ["manifesto", "social-medium-epic", "value-scanner-epic"],
        "fct_relevance": "FCT Module 5 - Respecting Human Nature, Sacred Attention",
        "concept_label": "Human Dignity & Imago Dei"
    },
    "externalities": {
        "elohim_nodes": ["economic-coordination-epic", "public-observer-epic"],
        "fct_relevance": "FCT Module 6 - Harmful Consequences, Tragedy of Commons",
        "concept_label": "Externalities & Commons"
    },
    "regeneration": {
        "elohim_nodes": ["autonomous-entity-epic", "economic-coordination-epic"],
        "fct_relevance": "FCT Module 7 - Regeneration and Reconciliation",
        "concept_label": "Regenerative Systems"
    },
    "values-centered": {
        "elohim_nodes": ["value-scanner-epic", "manifesto"],
        "fct_relevance": "FCT Module 8, 9 - Centering Values, Love, Fruits of Spirit",
        "concept_label": "Values-Centered Design"
    },
    "shared-understanding": {
        "elohim_nodes": ["governance-epic", "social-medium-epic"],
        "fct_relevance": "FCT Module 10, 11 - Sensemaking, Collaboration",
        "concept_label": "Collective Sensemaking"
    },
    "justice-fairness": {
        "elohim_nodes": ["governance-epic", "autonomous-entity-epic"],
        "fct_relevance": "FCT Module 12 - Supporting Fairness and Justice",
        "concept_label": "Justice & Fairness"
    },
    "human-flourishing": {
        "elohim_nodes": ["manifesto", "value-scanner-epic"],
        "fct_relevance": "FCT Module 13 - Helping People Thrive",
        "concept_label": "Human Flourishing"
    }
}


# =============================================================================
# FCT MODULE STRUCTURE
# =============================================================================

# Map FCT modules to CHT course structure and Elohim Protocol connections
FCT_MODULES = [
    {
        "filename": "Module 1 - The Church Dilemma.md",
        "id": "fct-module-01-church-dilemma",
        "title": "The Church Dilemma",
        "chapter": "foundations",
        "order": 0,
        "cht_module": "Introduction",
        "shared_concepts": ["attention-economy", "metacrisis"],
        "elohim_connections": ["manifesto", "social-medium-epic"]
    },
    {
        "filename": "Module 2 - Systems Thinking.md",
        "id": "fct-module-02-systems-thinking",
        "title": "Systems Thinking",
        "chapter": "foundations",
        "order": 1,
        "cht_module": "Module 1",
        "shared_concepts": ["systems-thinking"],
        "elohim_connections": ["governance-epic"]
    },
    {
        "filename": "Module 3 - A Problem Well Stated - Exponential Techonolgy, Our Divided Attention.md",
        "id": "fct-module-03-problem-well-stated",
        "title": "A Problem Well Stated",
        "chapter": "foundations",
        "order": 2,
        "cht_module": "Module 1",
        "shared_concepts": ["metacrisis", "attention-economy"],
        "elohim_connections": ["manifesto", "social-medium-epic"]
    },
    {
        "filename": "Module 4 - Setting the Stage -__The Need for Scaling Wisdom, __Acting as a Christian.md",
        "id": "fct-module-04-scaling-wisdom",
        "title": "Setting the Stage: Scaling Wisdom",
        "chapter": "setting-stage",
        "order": 3,
        "cht_module": "Module 1",
        "shared_concepts": ["trust-evolution", "systems-thinking"],
        "elohim_connections": ["governance-epic", "social-medium-epic"]
    },
    {
        "filename": "Module 5 - Respecting Human Nature - Our Attention is Sacred.md",
        "id": "fct-module-05-respecting-human-nature",
        "title": "Respecting Human Nature",
        "chapter": "human-nature",
        "order": 4,
        "cht_module": "Module 2",
        "shared_concepts": ["human-dignity", "attention-economy"],
        "elohim_connections": ["manifesto", "social-medium-epic"]
    },
    {
        "filename": "Module 6 - Minimizing Harmful Consequences - Externalities, Tragedy of the Commons, and  Arms Races.md",
        "id": "fct-module-06-harmful-consequences",
        "title": "Minimizing Harmful Consequences",
        "chapter": "human-nature",
        "order": 5,
        "cht_module": "Module 3",
        "shared_concepts": ["externalities"],
        "elohim_connections": ["economic-coordination-epic", "public-observer-epic"]
    },
    {
        "filename": "Module 7 - Regeneration and Reconciliation - Changing the paradim of church success..md",
        "id": "fct-module-07-regeneration",
        "title": "Regeneration and Reconciliation",
        "chapter": "values",
        "order": 6,
        "cht_module": "Module 3",
        "shared_concepts": ["regeneration"],
        "elohim_connections": ["autonomous-entity-epic"]
    },
    {
        "filename": "Module 8 - Centering Values - Part 1, Centering the greatest value, Love.md",
        "id": "fct-module-08-centering-love",
        "title": "Centering Values: Love",
        "chapter": "values",
        "order": 7,
        "cht_module": "Module 4",
        "shared_concepts": ["values-centered"],
        "elohim_connections": ["manifesto", "value-scanner-epic"]
    },
    {
        "filename": "Module 9 - Centering Values - Metrics are Not Neutral, Centering the Fruits of the Spirit.md",
        "id": "fct-module-09-fruits-of-spirit",
        "title": "Centering Values: Fruits of the Spirit",
        "chapter": "values",
        "order": 8,
        "cht_module": "Module 4",
        "shared_concepts": ["values-centered"],
        "elohim_connections": ["manifesto", "value-scanner-epic"]
    },
    {
        "filename": "Module 10 - Creating Shared Understanding - Recognize Distortions.md",
        "id": "fct-module-10-recognize-distortions",
        "title": "Creating Shared Understanding: Recognize Distortions",
        "chapter": "understanding",
        "order": 9,
        "cht_module": "Module 5",
        "shared_concepts": ["shared-understanding"],
        "elohim_connections": ["governance-epic", "social-medium-epic"]
    },
    {
        "filename": "Module 11 - Create Shared Understanding - Collaboration and Sensemaking.md",
        "id": "fct-module-11-collaboration-sensemaking",
        "title": "Creating Shared Understanding: Collaboration",
        "chapter": "understanding",
        "order": 10,
        "cht_module": "Module 5",
        "shared_concepts": ["shared-understanding"],
        "elohim_connections": ["governance-epic", "public-observer-epic"]
    },
    {
        "filename": "Module 12 - Supporting Fairness and Justice.md",
        "id": "fct-module-12-fairness-justice",
        "title": "Supporting Fairness and Justice",
        "chapter": "action",
        "order": 11,
        "cht_module": "Module 6",
        "shared_concepts": ["justice-fairness"],
        "elohim_connections": ["governance-epic", "autonomous-entity-epic"]
    },
    {
        "filename": "Module 13 - Helping People Thrive - A Christian Definition .md",
        "id": "fct-module-13-helping-thrive",
        "title": "Helping People Thrive",
        "chapter": "action",
        "order": 12,
        "cht_module": "Module 7",
        "shared_concepts": ["human-flourishing"],
        "elohim_connections": ["manifesto", "value-scanner-epic"]
    },
    {
        "filename": "Module 14 - Ready To Act_ Embodying Christ in the Digital Age.md",
        "id": "fct-module-14-ready-to-act",
        "title": "Ready To Act: Embodying Christ in the Digital Age",
        "chapter": "action",
        "order": 13,
        "cht_module": "Module 8",
        "shared_concepts": ["human-flourishing", "values-centered"],
        "elohim_connections": ["manifesto"]
    },
    {
        "filename": "Module 15 - Digital Discipleship Retrospective.md",
        "id": "fct-module-15-retrospective",
        "title": "Digital Discipleship Retrospective",
        "chapter": "action",
        "order": 14,
        "cht_module": "Conclusion",
        "shared_concepts": ["human-flourishing"],
        "elohim_connections": ["manifesto"]
    }
]

# Chapter definitions for the FCT learning path
FCT_CHAPTERS = {
    "foundations": {
        "id": "fct-chapter-foundations",
        "title": "Chapter 1: Foundations",
        "description": "Understanding the crisis and developing systems thinking",
        "order": 0,
        "estimatedDuration": "2-3 hours"
    },
    "setting-stage": {
        "id": "fct-chapter-setting-stage",
        "title": "Chapter 2: Setting the Stage",
        "description": "Scaling wisdom and building trust in the digital age",
        "order": 1,
        "estimatedDuration": "45-60 minutes"
    },
    "human-nature": {
        "id": "fct-chapter-human-nature",
        "title": "Chapter 3: Respecting Human Nature",
        "description": "Sacred attention and minimizing harmful consequences",
        "order": 2,
        "estimatedDuration": "1.5-2 hours"
    },
    "values": {
        "id": "fct-chapter-values",
        "title": "Chapter 4: Centering Values",
        "description": "Love, regeneration, and the fruits of the Spirit",
        "order": 3,
        "estimatedDuration": "2-3 hours"
    },
    "understanding": {
        "id": "fct-chapter-understanding",
        "title": "Chapter 5: Creating Shared Understanding",
        "description": "Collaboration, sensemaking, and recognizing distortions",
        "order": 4,
        "estimatedDuration": "1.5-2 hours"
    },
    "action": {
        "id": "fct-chapter-action",
        "title": "Chapter 6: Ready to Act",
        "description": "Fairness, flourishing, and embodying Christ",
        "order": 5,
        "estimatedDuration": "2-3 hours"
    }
}


# =============================================================================
# PARSING FUNCTIONS - DEEP STRUCTURE EXTRACTION
# =============================================================================

def parse_fct_module_deep(filepath: Path, module_info: dict) -> dict:
    """
    Parse an FCT markdown module with DEEP structure extraction.

    Returns a dictionary containing:
    - module_node: The main module content node
    - bible_nodes: Extracted Bible verse content nodes
    - video_nodes: Extracted video content nodes
    - book_nodes: Extracted book/article content nodes
    - activity_nodes: Discussion questions and activities
    - relationships: Graph relationships between extracted nodes
    """
    content = filepath.read_text(encoding="utf-8")

    # Parse sections from markdown
    sections = parse_markdown_sections(content)

    # Extract rich components
    bible_verses = parse_bible_references(
        sections.get("bible_reference(s)", ""),
        module_info
    )

    learning_objectives = parse_learning_objectives(
        sections.get("foundations_of_christians_and_technology_learning_objectives", ""),
        sections.get("foundations_of_humane_tech_concepts_review", "")
    )

    # Parse all media from Go Deeper and Homework sections
    media_result = parse_go_deeper_media(
        sections.get("other_media_/_go_deeper_/_class_maps", ""),
        sections.get("class_homework", ""),
        module_info
    )

    # Legacy compatibility
    videos = media_result

    discussion = parse_discussion_activity(
        sections.get("discussion_question_or_activity", ""),
        module_info
    )

    story = parse_story_section(
        sections.get("story", ""),
        module_info
    )

    application = parse_application_section(
        sections.get("application", ""),
        module_info
    )

    # Build the main module node
    module_node = {
        "id": module_info["id"],
        "contentType": "course-module",
        "title": f"FCT {module_info['order'] + 1}: {module_info['title']}",
        "description": create_module_description(sections),
        "content": {
            "introduction": sections.get("introduction", ""),
            "conclusion": sections.get("conclusion", ""),
            "rawMarkdown": content
        },
        "contentFormat": "structured-json",
        "sourcePath": f"fct/{module_info['filename']}",
        "tags": build_module_tags(module_info, learning_objectives),
        "relatedNodeIds": module_info.get("elohim_connections", []),
        "children": [],  # Will be populated with sub-node IDs
        "metadata": {
            "category": "fct",
            "courseId": "foundations-christian-technology",
            "moduleNumber": module_info["order"] + 1,
            "chapter": module_info["chapter"],
            "chtModule": module_info["cht_module"],
            "sharedConcepts": module_info.get("shared_concepts", []),
            "sectionCount": len(sections),
            "hasVideo": len(videos["nodes"]) > 0,
            "hasBibleReferences": len(bible_verses["nodes"]) > 0,
            "hasDiscussion": discussion is not None,
            "estimatedDuration": estimate_module_duration(sections)
        },
        "learningObjectives": learning_objectives["christian"],
        "chtConcepts": learning_objectives["cht"],
        "richMedia": {
            "videoCount": len(videos["nodes"]),
            "bookCount": len(videos.get("books", [])),
            "bibleVerseCount": len(bible_verses["nodes"])
        },
        "createdAt": now_iso(),
        "updatedAt": now_iso()
    }

    # Collect all child node IDs
    child_ids = []
    for bv in bible_verses["nodes"]:
        child_ids.append(bv["id"])
    for vid in videos["nodes"]:
        child_ids.append(vid["id"])
    if discussion:
        child_ids.append(discussion["id"])
    if story:
        child_ids.append(story["id"])
    if application:
        child_ids.append(application["id"])

    module_node["children"] = child_ids

    # Build relationships
    relationships = build_module_relationships(
        module_info,
        bible_verses["nodes"],
        videos["nodes"],
        discussion,
        story
    )

    return {
        "module_node": module_node,
        "bible_nodes": bible_verses["nodes"],
        "video_nodes": media_result.get("video_nodes", []) + media_result.get("movie_nodes", []),
        "book_nodes": media_result.get("book_nodes", []),
        "podcast_nodes": media_result.get("podcast_nodes", []),
        "article_nodes": media_result.get("article_nodes", []),
        "report_nodes": media_result.get("report_nodes", []),
        "contributor_nodes": media_result.get("contributor_nodes", []),
        "organization_nodes": media_result.get("organization_nodes", []),
        "activity_node": discussion,
        "story_node": story,
        "application_node": application,
        "relationships": relationships + media_result.get("relationships", [])
    }


def parse_markdown_sections(content: str) -> dict:
    """Parse markdown content into named sections."""
    sections = {}
    current_section = "intro"
    current_content = []

    for line in content.split("\n"):
        if line.startswith("### "):
            if current_content:
                sections[current_section] = "\n".join(current_content).strip()
            # Normalize section name
            current_section = line[4:].strip().lower().replace(" ", "_")
            current_content = []
        elif line.startswith("## "):
            # Main title - capture but continue
            if current_content:
                sections[current_section] = "\n".join(current_content).strip()
            sections["title"] = line[3:].strip()
            current_section = "after_title"
            current_content = []
        else:
            current_content.append(line)

    if current_content:
        sections[current_section] = "\n".join(current_content).strip()

    return sections


def parse_bible_references(bible_text: str, module_info: dict) -> dict:
    """
    Parse Bible references into individual verse content nodes.
    Each verse becomes a reusable content node in the graph.
    """
    nodes = []
    relationships = []

    if not bible_text.strip():
        return {"nodes": nodes, "relationships": relationships}

    # Pattern to match Bible verses: "Book Chapter:Verse(s) [optional text]"
    # Examples: "Matthew 22:36-40", "John 13:34-35 (ESV)", "Deuteronomy 11:"
    verse_pattern = r'([1-3]?\s*[A-Z][a-z]+)\s+(\d+):?(\d+)?(?:-(\d+))?\s*(?:\([A-Z]+\))?'

    # Also capture verse with surrounding context
    lines = bible_text.split("\n")
    current_verse = None
    current_context = []

    for line in lines:
        line = line.strip()
        if not line:
            continue

        # Check if line contains a verse reference
        match = re.search(verse_pattern, line)
        if match:
            # Save previous verse if exists
            if current_verse:
                nodes.append(create_bible_verse_node(
                    current_verse,
                    current_context,
                    module_info
                ))

            current_verse = {
                "book": match.group(1).strip(),
                "chapter": match.group(2),
                "verse_start": match.group(3) or "1",
                "verse_end": match.group(4),
                "full_ref": match.group(0)
            }
            current_context = [line]
        elif current_verse:
            # Add to current verse's context
            current_context.append(line)

    # Don't forget last verse
    if current_verse:
        nodes.append(create_bible_verse_node(
            current_verse,
            current_context,
            module_info
        ))

    return {"nodes": nodes, "relationships": relationships}


def create_bible_verse_node(verse_info: dict, context_lines: list, module_info: dict) -> dict:
    """Create a content node for a Bible verse."""
    book = verse_info["book"]
    chapter = verse_info["chapter"]
    verse_start = verse_info["verse_start"]
    verse_end = verse_info.get("verse_end")

    # Create a unique, readable ID
    ref_id = f"{book.lower().replace(' ', '-')}-{chapter}-{verse_start}"
    if verse_end:
        ref_id += f"-{verse_end}"
    node_id = f"fct-bible-{ref_id}"

    # Human readable reference
    human_ref = f"{book} {chapter}:{verse_start}"
    if verse_end:
        human_ref += f"-{verse_end}"

    return {
        "id": node_id,
        "contentType": "bible-verse",
        "title": human_ref,
        "description": f"Scripture reference used in FCT Module {module_info['order'] + 1}",
        "content": {
            "reference": human_ref,
            "book": book,
            "chapter": int(chapter),
            "verseStart": int(verse_start) if verse_start else 1,
            "verseEnd": int(verse_end) if verse_end else None,
            "contextInModule": "\n".join(context_lines),
            "translation": "ESV"  # Default, could be extracted
        },
        "contentFormat": "bible-json",
        "sourcePath": f"fct/{module_info['filename']}",
        "tags": [
            "bible",
            "scripture",
            "fct",
            book.lower().replace(" ", "-"),
            module_info["chapter"]
        ],
        "relatedNodeIds": [module_info["id"]],
        "metadata": {
            "category": "scripture",
            "sourceModule": module_info["id"],
            "thematicContext": module_info["title"]
        },
        "createdAt": now_iso(),
        "updatedAt": now_iso()
    }


def parse_learning_objectives(christian_text: str, cht_text: str) -> dict:
    """Parse learning objectives from both Christian and CHT perspectives."""
    christian = []
    cht = []

    # Parse Christian objectives
    for line in christian_text.split("\n"):
        line = line.strip()
        if re.match(r"^\d+\.", line) or line.startswith("-"):
            obj = re.sub(r"^[\d\.\-\s]+", "", line).strip()
            if obj and len(obj) > 10:
                christian.append(obj)

    # Parse CHT concepts
    for line in cht_text.split("\n"):
        line = line.strip()
        if line.startswith("-") or line.startswith("*"):
            concept = re.sub(r"^[\-\*\s]+", "", line).strip()
            if concept and len(concept) > 5:
                cht.append(concept)

    return {
        "christian": christian[:10],
        "cht": cht[:10]
    }


def parse_go_deeper_media(media_text: str, homework_text: str, module_info: dict) -> dict:
    """
    Parse "Go Deeper" and homework sections into individual media content nodes.
    Creates separate nodes for:
    - Videos (YouTube, Netflix, HBO, etc.)
    - Books (with author attribution)
    - Podcasts
    - Articles
    - Movies/Documentaries
    - Reports/Studies

    Also extracts:
    - ContributorPresence nodes (authors, speakers, filmmakers)
    - Organization nodes (platforms, publishers)
    """
    video_nodes = []
    book_nodes = []
    podcast_nodes = []
    article_nodes = []
    movie_nodes = []
    report_nodes = []
    contributor_nodes = []
    organization_nodes = []
    relationships = []

    combined_text = f"{media_text}\n{homework_text}"

    # Track seen items to avoid duplicates
    seen_titles = set()
    seen_contributors = set()
    seen_organizations = set()

    # Process line by line for better context extraction
    lines = combined_text.split("\n")
    current_category = None

    for line_num, line in enumerate(lines):
        line = line.strip()
        if not line or line.startswith("###"):
            continue

        # Detect category headers
        category_keywords = {
            "church crisis": "church-crisis",
            "poly-crisis": "polycrisis",
            "meta-crisis": "metacrisis",
            "attention economy": "attention-economy",
            "erosion": "truth-erosion",
            "fragmentation": "social-fragmentation",
            "collapse": "institutional-collapse",
            "bias": "algorithmic-bias",
            "cognitive": "cognitive-science",
            "information": "information-literacy",
            "discourse": "public-discourse",
            "knowledge": "knowledge-systems",
            "thinking": "deep-thinking",
            "vulnerabilities": "cognitive-vulnerabilities"
        }

        lower_line = line.lower()
        for keyword, category in category_keywords.items():
            if keyword in lower_line and not line.startswith("*") and not line.startswith("-"):
                current_category = category
                break

        # Skip non-content lines
        if not line.startswith("*") and not line.startswith("-") and not line.startswith('"'):
            # Check if it's a standalone reference line
            if " by " not in line.lower() and "(" not in line:
                continue

        # Clean the line
        clean_line = re.sub(r'^[\*\-\s]+', '', line).strip()
        if not clean_line:
            continue

        # Extract media item from line
        media_item = parse_media_line(clean_line, module_info, current_category)

        if media_item:
            item_key = media_item.get("title", "").lower()[:50]
            if item_key in seen_titles:
                continue
            seen_titles.add(item_key)

            # Route to appropriate list
            content_type = media_item.get("contentType", "")
            if content_type == "video":
                video_nodes.append(media_item)
            elif content_type == "book":
                book_nodes.append(media_item)
            elif content_type == "podcast":
                podcast_nodes.append(media_item)
            elif content_type == "article":
                article_nodes.append(media_item)
            elif content_type in ["movie", "documentary"]:
                movie_nodes.append(media_item)
            elif content_type == "report":
                report_nodes.append(media_item)

            # Extract contributor if present
            contributor = media_item.get("_contributor")
            if contributor and contributor.lower() not in seen_contributors:
                seen_contributors.add(contributor.lower())
                contributor_node = create_contributor_node(contributor, media_item, module_info)
                if contributor_node:
                    contributor_nodes.append(contributor_node)
                    # Create relationship
                    relationships.append({
                        "sourceId": media_item["id"],
                        "targetId": contributor_node["id"],
                        "relationshipType": "created_by",
                        "strength": 1.0
                    })

            # Extract organization if present
            org = media_item.get("_organization")
            if org and org.lower() not in seen_organizations:
                seen_organizations.add(org.lower())
                org_node = create_organization_node(org, media_item, module_info)
                if org_node:
                    organization_nodes.append(org_node)
                    # Create relationship
                    relationships.append({
                        "sourceId": media_item["id"],
                        "targetId": org_node["id"],
                        "relationshipType": "published_by",
                        "strength": 0.9
                    })

    return {
        "nodes": video_nodes + movie_nodes,  # Keep backward compat
        "video_nodes": video_nodes,
        "book_nodes": book_nodes,
        "podcast_nodes": podcast_nodes,
        "article_nodes": article_nodes,
        "movie_nodes": movie_nodes,
        "report_nodes": report_nodes,
        "contributor_nodes": contributor_nodes,
        "organization_nodes": organization_nodes,
        "relationships": relationships,
        "books": [{"title": b.get("title", ""), "author": b.get("_contributor")}
                  for b in book_nodes]  # Legacy format
    }


def parse_media_line(line: str, module_info: dict, category: str = None) -> dict:
    """Parse a single line to extract media information."""

    # Common patterns for different media types
    patterns = {
        # "Title" (year type, duration) - e.g., "The Social Dilemma" (2020, Netflix documentary, 1:34:00)
        "titled_media": r'"([^"]+)"\s*\((\d{4})?,?\s*([^,\)]+)?,?\s*([^)]+)?\)',

        # Title by Author (type) - e.g., "Deep Work" by Cal Newport (book)
        "by_author": r'"?([^"]+)"?\s+by\s+([A-Z][a-z]+(?:\s+[A-Z][a-z]+)+)\s*\(?(book|article)?\)?',

        # Title | Speaker | Platform (duration) - e.g., Title | Tristan Harris | TED (18:37)
        "pipe_format": r'([^|]+)\|\s*([^|]+)\|\s*([^(]+)\s*\(?(\d+:\d+)?\)?',

        # Title, Platform, duration - e.g., Title, Youtube Vox 4:31
        "comma_format": r'([^,]+),\s*(YouTube|Youtube|Vox|TED|Netflix|HBO|PBS)\s*,?\s*([^,]+)?,?\s*(\d+:\d+)?',

        # YouTube URL with context
        "youtube_url": r'(https?://(?:www\.)?(?:youtube\.com/watch\?v=|youtu\.be/)([\w-]+))',

        # Podcast pattern
        "podcast": r'([^(]+)\s*\(Podcast\)',

        # Report/Study pattern
        "report": r'"([^"]+)"\s*\(([^)]+(?:study|report|report)[^)]*)\)',
    }

    result = None
    node_id_base = f"fct-media-{module_info['order']:02d}"

    # Try YouTube URL first
    youtube_match = re.search(patterns["youtube_url"], line)
    if youtube_match:
        video_id = youtube_match.group(2) if youtube_match.lastindex >= 2 else youtube_match.group(1).split("/")[-1].split("=")[-1]
        # Extract title from surrounding text
        title_text = line.replace(youtube_match.group(0), "").strip()
        title_text = re.sub(r'^[\*\-\s,]+', '', title_text).strip()
        title_text = re.sub(r'[\*\-\s,]+$', '', title_text).strip()

        # Try to find duration
        duration_match = re.search(r'(\d+:\d+(?::\d+)?)', line)
        duration = duration_match.group(1) if duration_match else None

        # Try to find platform/channel
        platform = "YouTube"
        org = None
        for p in ["TED", "Vox", "CONAN", "TIME", "NYT", "PBS", "ClimateTown", "CPG Grey"]:
            if p.lower() in line.lower():
                org = p
                break

        # Try to find speaker
        speaker = None
        speaker_patterns = [
            r'\|\s*([A-Z][a-z]+\s+[A-Z][a-z]+)\s*\|',
            r'([A-Z][a-z]+\s+[A-Z][a-z]+)\s*[-–]\s',
            r'by\s+([A-Z][a-z]+\s+[A-Z][a-z]+)',
        ]
        for sp in speaker_patterns:
            sm = re.search(sp, title_text)
            if sm:
                speaker = sm.group(1)
                break

        clean_title = title_text[:150] if title_text else f"Video from FCT Module {module_info['order'] + 1}"

        result = {
            "id": f"{node_id_base}-yt-{video_id[:8]}",
            "contentType": "video",
            "title": clean_title,
            "description": f"Recommended video for {module_info['title']}",
            "content": {
                "platform": platform,
                "videoId": video_id,
                "embedUrl": f"https://www.youtube.com/embed/{video_id}",
                "watchUrl": f"https://www.youtube.com/watch?v={video_id}",
                "duration": duration
            },
            "contentFormat": "video-embed",
            "sourcePath": f"fct/{module_info['filename']}",
            "tags": ["video", "fct", "youtube", module_info["chapter"]],
            "relatedNodeIds": [module_info["id"]],
            "metadata": {
                "category": category or "fct-media",
                "sourceModule": module_info["id"],
                "mediaType": "video"
            },
            "_contributor": speaker,
            "_organization": org,
            "createdAt": now_iso(),
            "updatedAt": now_iso()
        }
        return result

    # Try podcast pattern
    podcast_match = re.search(patterns["podcast"], line, re.IGNORECASE)
    if podcast_match:
        title = podcast_match.group(1).strip()
        title = re.sub(r'^[\*\-\s\\]+', '', title)
        title = re.sub(r'[\*\\]+', '', title)  # Remove any remaining asterisks/escapes

        result = {
            "id": f"{node_id_base}-podcast-{slugify(title)[:20]}",
            "contentType": "podcast",
            "title": title,
            "description": f"Podcast recommended in FCT Module {module_info['order'] + 1}",
            "content": {
                "format": "podcast",
                "title": title
            },
            "contentFormat": "audio",
            "sourcePath": f"fct/{module_info['filename']}",
            "tags": ["podcast", "fct", "audio", module_info["chapter"]],
            "relatedNodeIds": [module_info["id"]],
            "metadata": {
                "category": category or "fct-media",
                "sourceModule": module_info["id"],
                "mediaType": "podcast"
            },
            "createdAt": now_iso(),
            "updatedAt": now_iso()
        }
        return result

    # Try book by author pattern
    book_match = re.search(patterns["by_author"], line)
    if book_match:
        title = book_match.group(1).strip().strip('"')
        author = book_match.group(2).strip() if book_match.lastindex >= 2 else None
        media_type = book_match.group(3) if book_match.lastindex >= 3 else "book"

        result = {
            "id": f"{node_id_base}-book-{slugify(title)[:20]}",
            "contentType": "book",
            "title": title,
            "description": f"Book by {author}" if author else f"Book referenced in FCT",
            "content": {
                "title": title,
                "author": author,
                "format": media_type or "book"
            },
            "contentFormat": "book",
            "sourcePath": f"fct/{module_info['filename']}",
            "tags": ["book", "fct", module_info["chapter"]],
            "relatedNodeIds": [module_info["id"]],
            "metadata": {
                "category": category or "fct-media",
                "sourceModule": module_info["id"],
                "mediaType": "book",
                "author": author
            },
            "_contributor": author,
            "createdAt": now_iso(),
            "updatedAt": now_iso()
        }
        return result

    # Try titled media pattern (movies, documentaries)
    titled_match = re.search(patterns["titled_media"], line)
    if titled_match:
        title = titled_match.group(1).strip()
        year = titled_match.group(2) if titled_match.lastindex >= 2 else None
        media_desc = titled_match.group(3).strip() if titled_match.lastindex >= 3 and titled_match.group(3) else ""
        duration = titled_match.group(4) if titled_match.lastindex >= 4 else None

        # Determine type from description
        media_type = "video"
        content_type = "video"
        platform = None

        media_desc_lower = media_desc.lower() if media_desc else ""
        if "documentary" in media_desc_lower or "docu" in media_desc_lower:
            content_type = "documentary"
            media_type = "documentary"
        elif "movie" in media_desc_lower or "film" in media_desc_lower:
            content_type = "movie"
            media_type = "movie"
        elif "study" in media_desc_lower or "report" in media_desc_lower:
            content_type = "report"
            media_type = "report"

        # Extract platform
        for p in ["Netflix", "HBO", "PBS", "Amazon", "Hulu"]:
            if p.lower() in media_desc_lower:
                platform = p
                break

        result = {
            "id": f"{node_id_base}-{content_type}-{slugify(title)[:20]}",
            "contentType": content_type,
            "title": title,
            "description": f"{title} ({year})" if year else title,
            "content": {
                "title": title,
                "year": int(year) if year else None,
                "platform": platform,
                "duration": duration,
                "format": media_type
            },
            "contentFormat": media_type,
            "sourcePath": f"fct/{module_info['filename']}",
            "tags": [content_type, "fct", module_info["chapter"]],
            "relatedNodeIds": [module_info["id"]],
            "metadata": {
                "category": category or "fct-media",
                "sourceModule": module_info["id"],
                "mediaType": media_type,
                "year": year
            },
            "_organization": platform,
            "createdAt": now_iso(),
            "updatedAt": now_iso()
        }
        return result

    # Try to detect articles
    if "article" in line.lower() or "the atlantic" in line.lower():
        # Extract title and author
        title_match = re.search(r'"([^"]+)"', line)
        author_match = re.search(r'by\s+([A-Z][a-z]+\s+[A-Z][a-z]+)', line)

        if title_match:
            title = title_match.group(1)
            author = author_match.group(1) if author_match else None

            result = {
                "id": f"{node_id_base}-article-{slugify(title)[:20]}",
                "contentType": "article",
                "title": title,
                "description": f"Article by {author}" if author else "Article",
                "content": {
                    "title": title,
                    "author": author,
                    "format": "article"
                },
                "contentFormat": "article",
                "sourcePath": f"fct/{module_info['filename']}",
                "tags": ["article", "fct", module_info["chapter"]],
                "relatedNodeIds": [module_info["id"]],
                "metadata": {
                    "category": category or "fct-media",
                    "sourceModule": module_info["id"],
                    "mediaType": "article"
                },
                "_contributor": author,
                "createdAt": now_iso(),
                "updatedAt": now_iso()
            }
            return result

    return None


def slugify(text: str) -> str:
    """Create a URL-safe slug from text."""
    text = text.lower()
    text = re.sub(r'[^\w\s-]', '', text)
    text = re.sub(r'[\s_]+', '-', text)
    text = re.sub(r'-+', '-', text)
    return text.strip('-')


def create_contributor_node(name: str, media_item: dict, module_info: dict) -> dict:
    """Create a ContributorPresence node for an author/speaker/creator."""
    if not name or len(name) < 3:
        return None

    # Skip common false positives
    skip_names = ["the", "and", "for", "with", "from"]
    if name.lower() in skip_names:
        return None

    contributor_id = f"fct-contributor-{slugify(name)}"

    return {
        "id": contributor_id,
        "contentType": "contributor",
        "title": name,
        "description": f"Author, speaker, or creator referenced in FCT",
        "content": {
            "name": name,
            "role": determine_contributor_role(media_item),
            "works": [media_item.get("title", "")]
        },
        "contentFormat": "contributor-json",
        "sourcePath": f"fct/{module_info['filename']}",
        "tags": ["contributor", "fct", "person"],
        "relatedNodeIds": [module_info["id"], media_item["id"]],
        "metadata": {
            "category": "contributor",
            "sourceModule": module_info["id"]
        },
        "createdAt": now_iso(),
        "updatedAt": now_iso()
    }


def determine_contributor_role(media_item: dict) -> str:
    """Determine the role of a contributor based on media type."""
    content_type = media_item.get("contentType", "")
    if content_type == "book":
        return "author"
    elif content_type == "article":
        return "writer"
    elif content_type == "video":
        return "speaker"
    elif content_type == "podcast":
        return "host"
    elif content_type in ["movie", "documentary"]:
        return "filmmaker"
    return "creator"


def create_organization_node(name: str, media_item: dict, module_info: dict) -> dict:
    """Create an Organization node for a platform/publisher."""
    if not name or len(name) < 2:
        return None

    org_id = f"fct-org-{slugify(name)}"

    # Determine organization type
    platform_types = {
        "youtube": "video-platform",
        "netflix": "streaming-service",
        "hbo": "streaming-service",
        "ted": "media-organization",
        "vox": "media-organization",
        "pbs": "broadcaster",
        "nyt": "newspaper",
        "time": "magazine",
        "the atlantic": "magazine",
        "mit": "university",
        "unc": "university",
        "stanford": "university",
        "pen america": "nonprofit"
    }

    org_type = "organization"
    for key, val in platform_types.items():
        if key in name.lower():
            org_type = val
            break

    return {
        "id": org_id,
        "contentType": "organization",
        "title": name,
        "description": f"{org_type.replace('-', ' ').title()} referenced in FCT",
        "content": {
            "name": name,
            "type": org_type
        },
        "contentFormat": "organization-json",
        "sourcePath": f"fct/{module_info['filename']}",
        "tags": ["organization", "fct", org_type],
        "relatedNodeIds": [module_info["id"], media_item["id"]],
        "metadata": {
            "category": "organization",
            "sourceModule": module_info["id"],
            "organizationType": org_type
        },
        "createdAt": now_iso(),
        "updatedAt": now_iso()
    }


def extract_video_title_from_context(context: str, video_id: str) -> str:
    """Try to extract a video title from surrounding context."""
    # Look for common patterns like "Title - Channel" or markdown links
    lines = context.split("\n")
    for line in lines:
        # Skip if line is just the URL
        if video_id in line and "http" not in line.lower():
            continue
        # Look for text that looks like a title
        if len(line) > 10 and len(line) < 200:
            # Clean markdown formatting
            title = re.sub(r'[\[\]\(\)\\*]', '', line).strip()
            if title and "http" not in title.lower():
                return title[:150]
    return None


def parse_discussion_activity(discussion_text: str, module_info: dict) -> dict:
    """Parse discussion questions/activities into a content node."""
    if not discussion_text.strip():
        return None

    return {
        "id": f"{module_info['id']}-discussion",
        "contentType": "activity",
        "title": f"Discussion: {module_info['title']}",
        "description": "Group discussion and reflection activity",
        "content": {
            "instructions": discussion_text,
            "activityType": "discussion",
            "groupSize": "small-group",
            "estimatedTime": "15-20 minutes"
        },
        "contentFormat": "activity-json",
        "sourcePath": f"fct/{module_info['filename']}",
        "tags": ["activity", "discussion", "fct", module_info["chapter"]],
        "relatedNodeIds": [module_info["id"]],
        "metadata": {
            "category": "fct-activity",
            "sourceModule": module_info["id"],
            "activityType": "discussion"
        },
        "createdAt": now_iso(),
        "updatedAt": now_iso()
    }


def parse_story_section(story_text: str, module_info: dict) -> dict:
    """Parse the Story section into a narrative content node."""
    if not story_text.strip():
        return None

    # Try to extract a title from the story
    lines = story_text.strip().split("\n")
    title = None
    for line in lines[:3]:
        if line.strip() and len(line.strip()) < 100:
            title = line.strip()
            break

    return {
        "id": f"{module_info['id']}-story",
        "contentType": "narrative",
        "title": title or f"Story: {module_info['title']}",
        "description": "Biblical story or example illustrating module concepts",
        "content": {
            "narrative": story_text,
            "narrativeType": "biblical-example"
        },
        "contentFormat": "markdown",
        "sourcePath": f"fct/{module_info['filename']}",
        "tags": ["story", "narrative", "fct", "biblical-example", module_info["chapter"]],
        "relatedNodeIds": [module_info["id"]],
        "metadata": {
            "category": "fct-narrative",
            "sourceModule": module_info["id"]
        },
        "createdAt": now_iso(),
        "updatedAt": now_iso()
    }


def parse_application_section(application_text: str, module_info: dict) -> dict:
    """Parse the Application section into a practice content node."""
    if not application_text.strip():
        return None

    return {
        "id": f"{module_info['id']}-application",
        "contentType": "practice",
        "title": f"Application: {module_info['title']}",
        "description": "Practical application and action steps",
        "content": {
            "instructions": application_text,
            "practiceType": "application"
        },
        "contentFormat": "markdown",
        "sourcePath": f"fct/{module_info['filename']}",
        "tags": ["application", "practice", "fct", module_info["chapter"]],
        "relatedNodeIds": [module_info["id"]],
        "metadata": {
            "category": "fct-practice",
            "sourceModule": module_info["id"]
        },
        "createdAt": now_iso(),
        "updatedAt": now_iso()
    }


def create_module_description(sections: dict) -> str:
    """Create a module description from available sections."""
    intro = sections.get("introduction", "")
    if intro:
        # Take first paragraph or 500 chars
        paragraphs = intro.split("\n\n")
        if paragraphs:
            return paragraphs[0][:500]
    return sections.get("title", "")[:500]


def build_module_tags(module_info: dict, learning_objectives: dict) -> list:
    """Build comprehensive tags for a module."""
    tags = [
        "fct",
        "christian-technology",
        "humane-tech",
        f"cht-{module_info['cht_module'].lower().replace(' ', '-')}",
        module_info["chapter"]
    ]

    # Add shared concepts
    tags.extend(module_info.get("shared_concepts", []))

    # Extract key themes from objectives
    theme_keywords = ["attention", "wisdom", "love", "systems", "values",
                      "trust", "justice", "fairness", "flourishing"]
    for obj in learning_objectives.get("christian", []):
        for keyword in theme_keywords:
            if keyword.lower() in obj.lower() and keyword not in tags:
                tags.append(keyword)

    return tags[:15]  # Limit tags


def estimate_module_duration(sections: dict) -> str:
    """Estimate reading/completion time for a module."""
    total_words = 0
    for content in sections.values():
        if isinstance(content, str):
            total_words += len(content.split())

    # Rough estimate: 200 words per minute reading, plus discussion time
    reading_minutes = total_words // 200
    total_minutes = reading_minutes + 15  # Add time for reflection/discussion

    if total_minutes < 30:
        return "20-30 minutes"
    elif total_minutes < 60:
        return "45-60 minutes"
    elif total_minutes < 90:
        return "1-1.5 hours"
    else:
        return "1.5-2 hours"


def build_module_relationships(module_info: dict, bible_nodes: list,
                               video_nodes: list, discussion_node: dict,
                               story_node: dict) -> list:
    """Build graph relationships for module components."""
    relationships = []
    module_id = module_info["id"]

    # Module contains bible verses
    for bv in bible_nodes:
        relationships.append({
            "sourceId": module_id,
            "targetId": bv["id"],
            "relationshipType": "contains",
            "strength": 1.0,
            "metadata": {"contentType": "bible-verse"}
        })

    # Module contains videos
    for vid in video_nodes:
        relationships.append({
            "sourceId": module_id,
            "targetId": vid["id"],
            "relationshipType": "contains",
            "strength": 1.0,
            "metadata": {"contentType": "video"}
        })

    # Module contains discussion
    if discussion_node:
        relationships.append({
            "sourceId": module_id,
            "targetId": discussion_node["id"],
            "relationshipType": "contains",
            "strength": 1.0,
            "metadata": {"contentType": "activity"}
        })

    # Module contains story
    if story_node:
        relationships.append({
            "sourceId": module_id,
            "targetId": story_node["id"],
            "relationshipType": "contains",
            "strength": 1.0,
            "metadata": {"contentType": "narrative"}
        })

    # Connections to Elohim Protocol nodes
    for elohim_node in module_info.get("elohim_connections", []):
        relationships.append({
            "sourceId": module_id,
            "targetId": elohim_node,
            "relationshipType": "conceptually_related",
            "strength": 0.8,
            "metadata": {
                "bridge": "fct-elohim",
                "sharedConcepts": module_info.get("shared_concepts", [])
            }
        })

    return relationships


# Legacy compatibility - wrap the deep parser
def parse_fct_module(filepath: Path, module_info: dict) -> dict:
    """Parse FCT module - wrapper for backwards compatibility."""
    result = parse_fct_module_deep(filepath, module_info)
    return result["module_node"]


# =============================================================================
# LEARNING PATH GENERATION
# =============================================================================

def create_fct_learning_path(modules: list[dict]) -> dict:
    """Create the FCT learning path with chapters."""

    # Group modules by chapter
    chapters_data = {}
    for module in modules:
        chapter_key = module["chapter"]
        if chapter_key not in chapters_data:
            chapters_data[chapter_key] = []
        chapters_data[chapter_key].append(module)

    # Build chapter structures
    chapters = []
    for chapter_key, chapter_info in FCT_CHAPTERS.items():
        chapter_modules = chapters_data.get(chapter_key, [])

        steps = []
        for module in sorted(chapter_modules, key=lambda m: m["order"]):
            step = {
                "order": module["order"],
                "resourceId": module["id"],
                "stepTitle": module["title"],
                "stepNarrative": f"CHT Module: {module['cht_module']} - Christian perspective on technology ethics",
                "learningObjectives": [
                    f"Understand {module['title'].lower()} from a Christian perspective",
                    f"Connect to CHT {module['cht_module']} foundations"
                ],
                "optional": False,
                "completionCriteria": [f"Read {module['title']}"],
                "estimatedTime": "30-45 minutes"
            }

            # Add shared concepts as learning context
            if module.get("shared_concepts"):
                step["sharedConcepts"] = module["shared_concepts"]

            steps.append(step)

        chapter = {
            "id": chapter_info["id"],
            "title": chapter_info["title"],
            "description": chapter_info["description"],
            "order": chapter_info["order"],
            "estimatedDuration": chapter_info["estimatedDuration"],
            "steps": steps
        }
        chapters.append(chapter)

    # Sort chapters by order
    chapters.sort(key=lambda c: c["order"])

    # Build flat steps for backward compatibility
    flat_steps = []
    for module in sorted(modules, key=lambda m: m["order"]):
        flat_steps.append({
            "order": module["order"],
            "resourceId": module["id"],
            "stepTitle": module["title"],
            "stepNarrative": f"CHT Module: {module['cht_module']}",
            "learningObjectives": [f"Understand {module['title'].lower()}"],
            "optional": False,
            "completionCriteria": [f"Complete {module['title']}"],
            "estimatedTime": "30-45 minutes"
        })

    # Create the path
    path = {
        "id": "foundations-christian-technology",
        "version": "1.0.0",
        "title": "Foundations for Christian Technology",
        "description": "A course equipping Christians to navigate and shape the digital age with wisdom, integrating Center for Humane Technology principles with biblical foundations.",
        "purpose": "Upgrade the collective wisdom of the church to improve awareness and decision-making in a world facing emergent 'godlike' technology",
        "createdBy": "fct-import",
        "contributors": ["Matthew Dowell", "Center for Humane Technology"],
        "createdAt": now_iso(),
        "updatedAt": now_iso(),
        "pathType": "expedition",  # Long-form deep dive
        "chapters": chapters,
        "steps": flat_steps,
        "tags": [
            "fct",
            "christian-technology",
            "humane-tech",
            "ethics",
            "discipleship",
            "church"
        ],
        "difficulty": "intermediate",
        "estimatedDuration": "10-15 hours",
        "visibility": "public",
        "prerequisitePaths": [],  # No prerequisites - entry point
        "attestationsGranted": ["fct-foundations", "digital-discipleship"],
        "metadata": {
            "sourceUrl": "https://humanetech.com/course",
            "christianAdaptation": True,
            "chtModuleCount": 8,
            "fctModuleCount": 15
        }
    }

    return path


# =============================================================================
# GRAPH RELATIONSHIPS
# =============================================================================

def create_fct_relationships(modules: list[dict]) -> list[dict]:
    """Create graph relationships connecting FCT to Elohim Protocol content."""
    relationships = []

    # 1. Module-to-Elohim connections
    for module in modules:
        for elohim_node in module.get("elohim_connections", []):
            relationships.append({
                "sourceId": module["id"],
                "targetId": elohim_node,
                "relationshipType": "conceptually_related",
                "strength": 0.8,
                "metadata": {
                    "bridge": "fct-elohim",
                    "sharedConcepts": module.get("shared_concepts", [])
                }
            })

    # 2. Shared concept relationships
    for concept_id, concept_info in SHARED_CONCEPTS.items():
        # Find FCT modules with this concept
        fct_modules_with_concept = [
            m for m in modules
            if concept_id in m.get("shared_concepts", [])
        ]

        # Create bidirectional relationships
        for fct_module in fct_modules_with_concept:
            for elohim_node in concept_info["elohim_nodes"]:
                # Only add if not already connected
                existing = any(
                    r["sourceId"] == fct_module["id"] and r["targetId"] == elohim_node
                    for r in relationships
                )
                if not existing:
                    relationships.append({
                        "sourceId": fct_module["id"],
                        "targetId": elohim_node,
                        "relationshipType": "shared_concept",
                        "strength": 0.9,  # Strong conceptual relationship
                        "metadata": {
                            "concept": concept_id,
                            "conceptLabel": concept_info["concept_label"]
                        }
                    })

    # 3. Sequential relationships within FCT
    for i, module in enumerate(modules[:-1]):
        next_module = modules[i + 1]
        relationships.append({
            "sourceId": module["id"],
            "targetId": next_module["id"],
            "relationshipType": "precedes",
            "strength": 1.0,
            "metadata": {"course": "fct"}
        })

    return relationships


# =============================================================================
# MAIN
# =============================================================================

def main():
    print("=" * 60)
    print("FCT Content Import to Lamad Format")
    print("=" * 60)
    print("\nDEEP PARSING MODE: Extracting rich structure from each module")
    print("  - Bible verses -> individual content nodes")
    print("  - Videos -> video content nodes")
    print("  - Discussion activities -> activity nodes")
    print("  - Stories -> narrative nodes")
    print("  - Applications -> practice nodes")
    print("=" * 60)

    # Ensure directories exist
    CONTENT_DIR.mkdir(parents=True, exist_ok=True)
    PATHS_DIR.mkdir(parents=True, exist_ok=True)
    GRAPH_DIR.mkdir(parents=True, exist_ok=True)

    # Collectors for all extracted content
    all_module_nodes = []
    all_bible_nodes = []
    all_video_nodes = []
    all_book_nodes = []
    all_podcast_nodes = []
    all_article_nodes = []
    all_report_nodes = []
    all_contributor_nodes = []
    all_organization_nodes = []
    all_activity_nodes = []
    all_story_nodes = []
    all_application_nodes = []
    all_relationships = []
    parsed_modules = []

    # Track seen IDs to avoid duplicates across modules
    seen_contributor_ids = set()
    seen_organization_ids = set()

    # 1. Parse FCT modules with DEEP extraction
    print("\n1. Deep Parsing FCT Modules...")

    for module_info in FCT_MODULES:
        filepath = FCT_SOURCE_DIR / module_info["filename"]
        if filepath.exists():
            # Use deep parser
            result = parse_fct_module_deep(filepath, module_info)

            # Collect all extracted content
            all_module_nodes.append(result["module_node"])
            all_bible_nodes.extend(result["bible_nodes"])
            all_video_nodes.extend(result["video_nodes"])
            all_book_nodes.extend(result.get("book_nodes", []))
            all_podcast_nodes.extend(result.get("podcast_nodes", []))
            all_article_nodes.extend(result.get("article_nodes", []))
            all_report_nodes.extend(result.get("report_nodes", []))

            # Deduplicate contributors across modules
            for contrib in result.get("contributor_nodes", []):
                if contrib["id"] not in seen_contributor_ids:
                    seen_contributor_ids.add(contrib["id"])
                    all_contributor_nodes.append(contrib)

            # Deduplicate organizations across modules
            for org in result.get("organization_nodes", []):
                if org["id"] not in seen_organization_ids:
                    seen_organization_ids.add(org["id"])
                    all_organization_nodes.append(org)

            if result["activity_node"]:
                all_activity_nodes.append(result["activity_node"])
            if result["story_node"]:
                all_story_nodes.append(result["story_node"])
            if result["application_node"]:
                all_application_nodes.append(result["application_node"])
            all_relationships.extend(result["relationships"])

            parsed_modules.append(module_info)

            # Print extraction summary for this module
            media_count = (len(result['video_nodes']) + len(result.get('book_nodes', [])) +
                          len(result.get('podcast_nodes', [])) + len(result.get('article_nodes', [])))
            print(f"   ✓ {module_info['title']}")
            print(f"      Bible: {len(result['bible_nodes'])} | Media: {media_count} | "
                  f"Contributors: {len(result.get('contributor_nodes', []))} | "
                  f"Orgs: {len(result.get('organization_nodes', []))}")
        else:
            print(f"   ✗ Missing: {module_info['filename']}")

    # Summary
    print(f"\n   EXTRACTION SUMMARY:")
    print(f"   - Modules: {len(all_module_nodes)}")
    print(f"   - Bible verses: {len(all_bible_nodes)}")
    print(f"   - Videos/Movies: {len(all_video_nodes)}")
    print(f"   - Books: {len(all_book_nodes)}")
    print(f"   - Podcasts: {len(all_podcast_nodes)}")
    print(f"   - Articles: {len(all_article_nodes)}")
    print(f"   - Contributors: {len(all_contributor_nodes)}")
    print(f"   - Organizations: {len(all_organization_nodes)}")
    print(f"   - Activities: {len(all_activity_nodes)}")
    print(f"   - Stories: {len(all_story_nodes)}")
    print(f"   - Applications: {len(all_application_nodes)}")
    print(f"   - Relationships: {len(all_relationships)}")

    # 2. Write ALL content nodes (modules + extracted children + media + contributors + orgs)
    print("\n2. Writing Content Nodes...")

    def write_nodes(nodes, label, source_file=None):
        for node in nodes:
            # Add standards-aligned fields before writing
            node = add_standards_fields(node, source_file)
            node_file = CONTENT_DIR / f"{node['id']}.json"
            node_file.write_text(json.dumps(node, indent=2))
        if nodes:
            print(f"   ✓ {label}: {len(nodes)} (with standards fields)")

    write_nodes(all_module_nodes, "Modules")
    write_nodes(all_bible_nodes, "Bible verses")
    write_nodes(all_video_nodes, "Videos/Movies")
    write_nodes(all_book_nodes, "Books")
    write_nodes(all_podcast_nodes, "Podcasts")
    write_nodes(all_article_nodes, "Articles")
    write_nodes(all_contributor_nodes, "Contributors")
    write_nodes(all_organization_nodes, "Organizations")
    write_nodes(all_activity_nodes, "Activities")
    write_nodes(all_story_nodes, "Stories")
    write_nodes(all_application_nodes, "Applications")

    # Total content nodes created
    total_nodes = (len(all_module_nodes) + len(all_bible_nodes) + len(all_video_nodes) +
                   len(all_book_nodes) + len(all_podcast_nodes) + len(all_article_nodes) +
                   len(all_contributor_nodes) + len(all_organization_nodes) +
                   len(all_activity_nodes) + len(all_story_nodes) + len(all_application_nodes))
    print(f"   ─────────────────────")
    print(f"   TOTAL: {total_nodes} content nodes created")

    # 3. Create FCT learning path
    print("\n3. Creating FCT Learning Path...")
    path = create_fct_learning_path(parsed_modules)
    path_file = PATHS_DIR / f"{path['id']}.json"
    path_file.write_text(json.dumps(path, indent=2))
    print(f"   Created: {path['id']} with {len(path['chapters'])} chapters")

    # 4. Update path index
    print("\n4. Updating Path Index...")
    path_index_file = PATHS_DIR / "index.json"
    if path_index_file.exists():
        with open(path_index_file) as f:
            path_index = json.load(f)
    else:
        path_index = {"lastUpdated": now_iso(), "totalCount": 0, "paths": []}

    # Check if FCT path already exists
    existing_ids = {p["id"] for p in path_index["paths"]}
    if path["id"] not in existing_ids:
        path_index["paths"].append({
            "id": path["id"],
            "title": path["title"],
            "description": path["description"],
            "difficulty": path["difficulty"],
            "estimatedDuration": path["estimatedDuration"],
            "stepCount": len(path["steps"]),
            "chapterCount": len(path["chapters"]),
            "pathType": path["pathType"],
            "tags": path["tags"],
            "category": "fct"
        })
        path_index["lastUpdated"] = now_iso()
        path_index["totalCount"] = len(path_index["paths"])
        path_index_file.write_text(json.dumps(path_index, indent=2))
        print(f"   Added to index: {path['id']}")
    else:
        print(f"   Already in index: {path['id']}")

    # 5. Update content index with ALL extracted nodes
    print("\n5. Updating Content Index...")
    content_index_file = CONTENT_DIR / "index.json"
    if content_index_file.exists():
        with open(content_index_file) as f:
            content_index = json.load(f)
    else:
        content_index = {"lastUpdated": now_iso(), "totalCount": 0, "nodes": []}

    existing_node_ids = {n["id"] for n in content_index["nodes"]}

    # Combine all nodes for indexing
    all_nodes = (all_module_nodes + all_bible_nodes + all_video_nodes +
                 all_book_nodes + all_podcast_nodes + all_article_nodes +
                 all_contributor_nodes + all_organization_nodes +
                 all_activity_nodes + all_story_nodes + all_application_nodes)

    added_count = 0
    for node in all_nodes:
        if node["id"] not in existing_node_ids:
            # Get description safely
            description = node.get("description", "")
            if isinstance(description, dict):
                description = str(description)[:200]
            else:
                description = str(description)[:200]

            content_index["nodes"].append({
                "id": node["id"],
                "title": node["title"],
                "description": description,
                "contentType": node["contentType"],
                "tags": node.get("tags", []),
                "category": node.get("metadata", {}).get("category", "fct")
            })
            added_count += 1

    content_index["lastUpdated"] = now_iso()
    content_index["totalCount"] = len(content_index["nodes"])
    content_index_file.write_text(json.dumps(content_index, indent=2))
    print(f"   Added {added_count} nodes to content index (total: {content_index['totalCount']})")

    # 6. Create graph relationships
    print("\n6. Creating Graph Relationships...")

    # Combine extracted relationships with cross-path relationships
    cross_path_relationships = create_fct_relationships(parsed_modules)
    combined_relationships = all_relationships + cross_path_relationships

    # Load existing relationships
    relationships_file = GRAPH_DIR / "relationships.json"
    if relationships_file.exists():
        with open(relationships_file) as f:
            existing_rels = json.load(f)
    else:
        existing_rels = {"lastUpdated": now_iso(), "totalCount": 0, "relationships": []}

    # Handle both old format (source/target/type) and new format (sourceId/targetId/relationshipType)
    existing_pairs = set()
    for r in existing_rels["relationships"]:
        source = r.get("sourceId") or r.get("source")
        target = r.get("targetId") or r.get("target")
        rel_type = r.get("relationshipType") or r.get("type")
        if source and target and rel_type:
            existing_pairs.add((source, target, rel_type))

    # Convert new relationships to the existing format (source/target/type)
    new_rels = []
    rel_counter = existing_rels["totalCount"]
    for rel in combined_relationships:
        source = rel.get("sourceId") or rel.get("source")
        target = rel.get("targetId") or rel.get("target")
        rel_type = rel.get("relationshipType") or rel.get("type")

        key = (source, target, rel_type)
        if key not in existing_pairs:
            # Use the existing format for consistency
            new_rel = {
                "id": f"rel-{rel_counter}",
                "source": source,
                "target": target,
                "type": rel_type.upper() if rel_type else "RELATED",
                "metadata": rel.get("metadata", {})
            }
            new_rels.append(new_rel)
            existing_pairs.add(key)
            rel_counter += 1

    existing_rels["relationships"].extend(new_rels)
    existing_rels["lastUpdated"] = now_iso()
    existing_rels["totalCount"] = len(existing_rels["relationships"])
    relationships_file.write_text(json.dumps(existing_rels, indent=2))
    print(f"   Added {len(new_rels)} new relationships")
    print(f"   Total relationships: {existing_rels['totalCount']}")

    # 7. Create shared concepts summary
    print("\n7. Creating Shared Concepts Summary...")
    concepts_summary = {
        "lastUpdated": now_iso(),
        "description": "Conceptual bridges between FCT and Elohim Protocol",
        "concepts": []
    }

    for concept_id, concept_info in SHARED_CONCEPTS.items():
        fct_modules = [
            m["id"] for m in parsed_modules
            if concept_id in m.get("shared_concepts", [])
        ]
        concepts_summary["concepts"].append({
            "id": concept_id,
            "label": concept_info["concept_label"],
            "fctRelevance": concept_info["fct_relevance"],
            "elohimNodes": concept_info["elohim_nodes"],
            "fctModules": fct_modules
        })

    concepts_file = CONTENT_DIR / "fct-shared-concepts.json"
    concepts_file.write_text(json.dumps(concepts_summary, indent=2))
    print(f"   Created shared concepts summary: {len(concepts_summary['concepts'])} concepts")

    # Summary
    print("\n" + "=" * 60)
    print("IMPORT COMPLETE!")
    print("=" * 60)

    print(f"\n📚 CONTENT CREATED:")
    print(f"  Modules:       {len(all_module_nodes):>4}")
    print(f"  Bible Verses:  {len(all_bible_nodes):>4}")
    print(f"  Videos/Movies: {len(all_video_nodes):>4}")
    print(f"  Books:         {len(all_book_nodes):>4}")
    print(f"  Podcasts:      {len(all_podcast_nodes):>4}")
    print(f"  Articles:      {len(all_article_nodes):>4}")
    print(f"  Activities:    {len(all_activity_nodes):>4}")
    print(f"  Stories:       {len(all_story_nodes):>4}")
    print(f"  Applications:  {len(all_application_nodes):>4}")
    print(f"  ─────────────────────")
    print(f"  TOTAL CONTENT: {total_nodes:>4}")

    print(f"\n👥 CONTRIBUTORS & ORGANIZATIONS:")
    print(f"  Contributors:  {len(all_contributor_nodes):>4} (authors, speakers, filmmakers)")
    print(f"  Organizations: {len(all_organization_nodes):>4} (platforms, publishers)")

    print(f"\n🗺️  LEARNING PATH:")
    print(f"  - {path['title']}")
    print(f"  - {len(path['chapters'])} chapters")
    print(f"  - {path['estimatedDuration']} total duration")

    print(f"\n🔗 GRAPH RELATIONSHIPS:")
    print(f"  - {len(new_rels)} new relationships added")
    print(f"  - {existing_rels['totalCount']} total in graph")

    print(f"\n🌉 SHARED CONCEPT BRIDGES (FCT ↔ Elohim Protocol):")
    for concept_id, concept_info in SHARED_CONCEPTS.items():
        print(f"  • {concept_info['concept_label']}")

    print(f"\n📊 HIERARCHY DEMONSTRATED:")
    print(f"  Course (FCT)")
    print(f"   └── Chapters ({len(path['chapters'])})")
    for ch in path.get("chapters", [])[:3]:
        print(f"        └── {ch['title']}")
        for step in ch.get("steps", [])[:2]:
            print(f"             └── {step.get('stepTitle', 'Step')}")
            # Show children of first module
            if step.get("resourceId") and step == ch.get("steps", [])[0]:
                for mod in all_module_nodes[:1]:
                    if mod["id"] == step["resourceId"]:
                        for child_id in mod.get("children", [])[:3]:
                            print(f"                  └── {child_id}")
                        if len(mod.get("children", [])) > 3:
                            print(f"                  └── ... +{len(mod['children'])-3} more")

    print("\n" + "=" * 60)
    print("Run the script from elohim-app root:")
    print("  python scripts/import_fct_content.py")
    print("=" * 60)


if __name__ == "__main__":
    main()
