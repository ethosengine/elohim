#!/usr/bin/env python3
"""
Seed Data Validation Hook

Validates JSON files written to genesis/data/lamad/ directories
to ensure they conform to expected schemas before seeding.

Hook Type: PreToolUse
Matcher: Write
"""
import json
import sys
import os
import re
from pathlib import Path
from typing import Optional

# Seed data directories to validate
SEED_DATA_PATHS = [
    "genesis/data/lamad/",
    "data/lamad/",
]

# Required fields for content nodes
CONTENT_REQUIRED_FIELDS = ["id", "title"]

# Valid content types
VALID_CONTENT_TYPES = [
    "article", "quiz", "assessment", "video", "audio", "image",
    "interactive", "simulation", "course", "path", "module",
    "lesson", "exercise", "reference", "glossary", "case-study",
    "scenario", "role", "epic", "readme", "overview", "graph",
    "attestation", "badge", "extension", "agent", "audience",
]

# Valid content formats
VALID_CONTENT_FORMATS = [
    "markdown", "html", "json", "yaml", "text", "binary",
    "perseus", "gherkin", "html5-app", "iframe", "graph-viz",
]

# Fields that should be strings
STRING_FIELDS = ["id", "title", "description", "contentType", "contentFormat"]

# Fields that should be arrays
ARRAY_FIELDS = ["tags", "relatedNodeIds", "prerequisites", "conceptIds"]


def is_seed_data_file(file_path: str) -> bool:
    """Check if the file is in a seed data directory."""
    for seed_path in SEED_DATA_PATHS:
        if seed_path in file_path:
            return True
    return False


def get_directory_type(file_path: str) -> Optional[str]:
    """Determine the type of seed data based on directory."""
    if "/content/" in file_path:
        return "content"
    elif "/paths/" in file_path:
        return "path"
    elif "/assessments/" in file_path:
        return "assessment"
    elif "/perseus/" in file_path:
        return "perseus"
    elif "/graph/" in file_path:
        return "graph"
    elif "/agents/" in file_path:
        return "agent"
    elif "/audiences/" in file_path:
        return "audience"
    elif "/attestations/" in file_path:
        return "attestation"
    elif "/extensions/" in file_path:
        return "extension"
    elif "/governance/" in file_path:
        return "governance"
    elif "/knowledge-maps/" in file_path:
        return "knowledge-map"
    elif "/meta/" in file_path:
        return "meta"
    return None


def validate_content_node(data: dict, file_path: str) -> list[str]:
    """Validate a content node structure."""
    errors = []
    warnings = []

    # Check required fields
    for field in CONTENT_REQUIRED_FIELDS:
        if field not in data:
            errors.append(f"Missing required field: '{field}'")
        elif not data[field]:
            errors.append(f"Field '{field}' is empty")

    # Check string fields are strings
    for field in STRING_FIELDS:
        if field in data and data[field] is not None:
            if not isinstance(data[field], str):
                errors.append(f"Field '{field}' should be a string, got {type(data[field]).__name__}")

    # Check array fields are arrays
    for field in ARRAY_FIELDS:
        if field in data and data[field] is not None:
            if not isinstance(data[field], list):
                errors.append(f"Field '{field}' should be an array, got {type(data[field]).__name__}")

    # Validate contentType if present
    if "contentType" in data and data["contentType"]:
        ct = data["contentType"].lower()
        if ct not in VALID_CONTENT_TYPES:
            warnings.append(f"Unrecognized contentType: '{data['contentType']}' (valid: {', '.join(VALID_CONTENT_TYPES[:5])}...)")

    # Validate contentFormat if present
    if "contentFormat" in data and data["contentFormat"]:
        cf = data["contentFormat"].lower()
        if cf not in VALID_CONTENT_FORMATS:
            warnings.append(f"Unrecognized contentFormat: '{data['contentFormat']}' (valid: {', '.join(VALID_CONTENT_FORMATS[:5])}...)")

    # Check id matches filename convention
    if "id" in data and data["id"]:
        filename = Path(file_path).stem
        if data["id"] != filename and filename != "index":
            warnings.append(f"ID '{data['id']}' doesn't match filename '{filename}'")

    return errors + warnings


def validate_path_node(data: dict, file_path: str) -> list[str]:
    """Validate a learning path structure."""
    errors = []

    # Paths need id and title
    if "id" not in data:
        errors.append("Missing required field: 'id'")
    if "title" not in data:
        errors.append("Missing required field: 'title'")

    # Check for modules/sections structure
    if "modules" in data:
        if not isinstance(data["modules"], list):
            errors.append("'modules' should be an array")
        else:
            for i, module in enumerate(data["modules"]):
                if isinstance(module, dict):
                    if "conceptIds" in module and not isinstance(module["conceptIds"], list):
                        errors.append(f"modules[{i}].conceptIds should be an array")

    return errors


def validate_perseus_item(data: dict, file_path: str) -> list[str]:
    """Validate Perseus quiz format."""
    errors = []

    # Perseus items need specific structure
    if "question" not in data and "content" not in data:
        errors.append("Perseus items need 'question' or 'content' field")

    if "widgets" in data and not isinstance(data["widgets"], dict):
        errors.append("'widgets' should be an object")

    return errors


def validate_graph_data(data: dict, file_path: str) -> list[str]:
    """Validate graph visualization data."""
    errors = []

    # Graphs need nodes and edges/links
    if "nodes" not in data and "elements" not in data:
        # Could be a simple id/title reference, just check basics
        if "id" not in data:
            errors.append("Graph data needs 'nodes' array or 'id' field")

    if "nodes" in data and not isinstance(data["nodes"], list):
        errors.append("'nodes' should be an array")

    if "edges" in data and not isinstance(data["edges"], list):
        errors.append("'edges' should be an array")

    return errors


def validate_json_content(content: str, file_path: str) -> tuple[list[str], list[str]]:
    """Validate JSON content and return (errors, warnings)."""
    errors = []
    warnings = []

    # Try to parse JSON
    try:
        data = json.loads(content)
    except json.JSONDecodeError as e:
        return [f"Invalid JSON: {e}"], []

    if not isinstance(data, dict):
        # Could be an array (index files), that's ok
        if isinstance(data, list):
            return [], []
        return ["JSON root should be an object or array"], []

    # Determine validation type based on directory
    dir_type = get_directory_type(file_path)

    if dir_type == "content":
        issues = validate_content_node(data, file_path)
    elif dir_type == "path":
        issues = validate_path_node(data, file_path)
    elif dir_type == "perseus":
        issues = validate_perseus_item(data, file_path)
    elif dir_type == "graph":
        issues = validate_graph_data(data, file_path)
    elif dir_type in ["assessment", "agent", "audience", "attestation", "extension", "governance", "knowledge-map"]:
        # These all follow content node structure
        issues = validate_content_node(data, file_path)
    else:
        # Unknown type, just check for basic structure
        issues = validate_content_node(data, file_path)

    # Separate errors from warnings
    for issue in issues:
        if issue.startswith("Missing") or issue.startswith("Invalid") or "should be" in issue:
            errors.append(issue)
        else:
            warnings.append(issue)

    return errors, warnings


def main():
    try:
        # Read hook input from stdin
        data = json.load(sys.stdin)

        tool_name = data.get('tool_name', '')
        if tool_name != 'Write':
            sys.exit(0)

        tool_input = data.get('tool_input', {})
        file_path = tool_input.get('file_path', '')
        content = tool_input.get('content', '')

        if not file_path or not content:
            sys.exit(0)

        # Only validate JSON files in seed data directories
        if not file_path.endswith('.json'):
            sys.exit(0)

        if not is_seed_data_file(file_path):
            sys.exit(0)

        # Get relative path for cleaner output
        project_dir = os.environ.get('CLAUDE_PROJECT_DIR', '/projects/elohim')
        try:
            rel_path = os.path.relpath(file_path, project_dir)
        except ValueError:
            rel_path = file_path

        # Validate the content
        errors, warnings = validate_json_content(content, file_path)

        if errors:
            # Block the write with error details
            error_msg = f"SEED DATA VALIDATION FAILED for '{rel_path}':\n"
            error_msg += "\n".join(f"  - {e}" for e in errors)
            if warnings:
                error_msg += "\n\nWarnings:\n"
                error_msg += "\n".join(f"  - {w}" for w in warnings)

            output = {
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "permissionDecision": "deny",
                    "permissionDecisionReason": error_msg
                }
            }
            print(json.dumps(output))
            sys.exit(0)

        if warnings:
            # Allow but add context about warnings
            output = {
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "additionalContext": f"SEED DATA WARNINGS for '{rel_path}':\n" + "\n".join(f"  - {w}" for w in warnings)
                }
            }
            print(json.dumps(output))

        sys.exit(0)

    except json.JSONDecodeError:
        sys.exit(0)
    except Exception as e:
        print(f"seed-data-validation hook error: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
