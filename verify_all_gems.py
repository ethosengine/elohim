#!/usr/bin/env python3
"""
Verify all gems from keen_collections are in the docs
"""

import json
from pathlib import Path
import subprocess

def get_gem_ids_from_collections():
    """Get all gem IDs from the collection JSON files"""
    collections_dir = Path('/home/user/elohim/docs/keen_collections')
    all_gem_ids = set()

    for json_file in collections_dir.glob('*.json'):
        if json_file.name == '_summary.json':
            continue

        with open(json_file, 'r') as f:
            data = json.load(f)
            for gem in data.get('gems', []):
                all_gem_ids.add(gem['gemId'])

    return all_gem_ids

def find_gem_in_docs(gem_id):
    """Search for a gem ID in the docs"""
    result = subprocess.run(
        ['grep', '-r', '--include=*.md', gem_id, '/home/user/elohim/docs'],
        capture_output=True,
        text=True
    )
    return result.returncode == 0

def main():
    collection_gem_ids = get_gem_ids_from_collections()
    print(f"Total gems in collections: {len(collection_gem_ids)}")

    missing = []
    found = []

    for gem_id in sorted(collection_gem_ids):
        if find_gem_in_docs(gem_id):
            found.append(gem_id)
        else:
            missing.append(gem_id)

    print(f"Found in docs: {len(found)}")
    print(f"Missing from docs: {len(missing)}")

    if missing:
        print("\nMissing gem IDs:")
        for gem_id in missing:
            print(f"  - {gem_id}")
        return False
    else:
        print("\n✓ All gems from collections are present in docs!")
        return True

if __name__ == '__main__':
    success = main()
    exit(0 if success else 1)
