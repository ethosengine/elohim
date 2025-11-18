#!/usr/bin/env python3
"""
Update node_type in frontmatter for all moved content type files
"""

import re
from pathlib import Path

# Map of directories to their node types
content_type_paths = {
    'video': 'video',
    'audio': 'audio',
    'books': 'book',
    'articles': 'article',
    'documents': 'document',
}

def update_node_type(file_path, node_type):
    """Update the node_type in the YAML frontmatter"""
    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()

    # Replace node_type: organization with the correct type
    updated = re.sub(
        r'^node_type: organization$',
        f'node_type: {node_type}',
        content,
        flags=re.MULTILINE
    )

    if updated != content:
        with open(file_path, 'w', encoding='utf-8') as f:
            f.write(updated)
        return True
    return False

def main():
    base_path = Path('/home/user/elohim/docs')
    updated_count = 0

    # Find all README.md files in content type directories
    for content_type_dir, node_type in content_type_paths.items():
        # Search in all epics for this content type
        for readme_path in base_path.rglob(f'*/{content_type_dir}/*/README.md'):
            if update_node_type(readme_path, node_type):
                print(f"Updated: {readme_path.relative_to(base_path.parent)} -> node_type: {node_type}")
                updated_count += 1

    print(f"\nTotal files updated: {updated_count}")

if __name__ == '__main__':
    main()
