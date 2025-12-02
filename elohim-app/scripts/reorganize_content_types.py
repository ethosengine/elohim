#!/usr/bin/env python3
"""
Reorganize non-organization content from organizations directories
into proper content type directories (video, audio, books, articles, documents)
"""

import os
import shutil
from pathlib import Path

# Mapping of files to move: source_path -> (epic, content_type, new_name)
moves = {
    # VIDEO - all in governance
    'docs/governance/organizations/climate_town': ('governance', 'video', 'climate_town'),
    'docs/governance/organizations/not_just_bikes': ('governance', 'video', 'not_just_bikes'),
    'docs/governance/organizations/the_permaculture_principles': ('governance', 'video', 'the_permaculture_principles'),
    'docs/governance/organizations/transitiontowns': ('governance', 'video', 'transitiontowns'),

    # AUDIO
    'docs/governance/organizations/future_thinkers_daniel_schmachtenberger_the_existential_game': ('governance', 'audio', 'future_thinkers_daniel_schmachtenberger_the_existential_game'),
    'docs/value_scanner/organizations/home_initiative_for_digital_public_infrastructure': ('value_scanner', 'audio', 'home_initiative_for_digital_public_infrastructure'),

    # BOOKS
    'docs/governance/organizations/finite_and_infinite_games': ('governance', 'books', 'finite_and_infinite_games'),
    'docs/governance/organizations/future_scouts_international_learning_program_social_app_and_handbook': ('governance', 'books', 'future_scouts_international_learning_program_social_app_and_handbook'),
    'docs/governance/organizations/the_collapse_of_complex_societies': ('governance', 'books', 'the_collapse_of_complex_societies'),
    'docs/public_observer/organizations/the_ministry_for_the_future': ('public_observer', 'books', 'the_ministry_for_the_future'),

    # ARTICLES
    'docs/social_medium/organizations/new__public': ('social_medium', 'articles', 'new__public'),

    # DOCUMENTS
    'docs/value_scanner/organizations/artificial_intelligence_values_and_alignment': ('value_scanner', 'documents', 'artificial_intelligence_values_and_alignment'),
}

def main():
    base_path = Path('/home/user/elohim')

    # Track operations
    epics_content_types = set()
    moved_files = []

    for source_rel, (epic, content_type, name) in moves.items():
        source_path = base_path / source_rel
        target_dir = base_path / 'docs' / epic / content_type / name

        if not source_path.exists():
            print(f"⚠️  Source not found: {source_path}")
            continue

        # Create target directory
        target_dir.mkdir(parents=True, exist_ok=True)
        epics_content_types.add((epic, content_type))

        # Move all contents
        for item in source_path.iterdir():
            target_item = target_dir / item.name
            print(f"Moving: {item.relative_to(base_path)} -> {target_item.relative_to(base_path)}")
            shutil.move(str(item), str(target_item))

        # Remove empty source directory
        source_path.rmdir()
        moved_files.append((source_rel, f'docs/{epic}/{content_type}/{name}'))

    # Summary
    print("\n" + "="*60)
    print("SUMMARY")
    print("="*60)

    print(f"\nMoved {len(moved_files)} items:")
    for src, dst in moved_files:
        print(f"  {src}")
        print(f"    -> {dst}")

    print(f"\nContent type directories created:")
    for epic, content_type in sorted(epics_content_types):
        print(f"  {epic}/{content_type}/")

    # Count by content type
    by_type = {}
    for _, (epic, content_type, _) in moves.items():
        key = content_type
        by_type[key] = by_type.get(key, 0) + 1

    print(f"\nBreakdown by content type:")
    for content_type, count in sorted(by_type.items()):
        print(f"  {content_type}: {count}")

if __name__ == '__main__':
    main()
