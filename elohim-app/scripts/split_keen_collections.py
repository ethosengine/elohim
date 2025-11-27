#!/usr/bin/env python3
"""
Script to split keen.json into separate JSON collections by content type.
Categories: organizations (websites), apps, audio, video, images, documents, books, articles
"""

import json
import re
from urllib.parse import urlparse
from pathlib import Path

def categorize_gem(gem):
    """Determine the content type of a gem based on its metalink data."""
    if not gem.get('metalink'):
        return 'organizations'

    metalink = gem['metalink']
    url = metalink.get('url', '').lower()
    title = metalink.get('title', '').lower()
    description = metalink.get('description', '').lower()
    publisher = metalink.get('publisher', '').lower()

    # Check for video content
    video_patterns = [
        'youtube.com', 'youtu.be', 'vimeo.com', 'video',
        'watch?v=', '/watch', 'ted.com/talks'
    ]
    if any(pattern in url for pattern in video_patterns):
        return 'video'
    if 'video' in title or 'video' in description:
        return 'video'

    # Check for audio content
    audio_patterns = [
        'podcast', 'spotify.com', 'soundcloud.com', 'audio',
        'apple.com/podcast', 'listen', 'radio'
    ]
    if any(pattern in url for pattern in audio_patterns):
        return 'audio'
    if 'podcast' in title or 'podcast' in description or 'audio' in title:
        return 'audio'

    # Check for books
    book_patterns = [
        'book', 'isbn', 'amazon.com/dp', 'goodreads.com',
        'books.google', 'read online', 'ebook'
    ]
    if any(pattern in url for pattern in book_patterns):
        return 'books'
    if 'book' in title or 'author' in description:
        return 'books'

    # Check for apps
    app_patterns = [
        'app.', 'apps.', 'play.google.com', 'apps.apple.com',
        'chrome.google.com/webstore', 'addons.mozilla.org',
        'download', 'install'
    ]
    if any(pattern in url for pattern in app_patterns):
        return 'apps'
    if 'app' in title or 'software' in title or 'tool' in title:
        # But exclude if it's clearly a website/organization
        if not any(x in url for x in ['.org', '.com/about', '/mission']):
            return 'apps'

    # Check for images
    image_patterns = [
        '.jpg', '.jpeg', '.png', '.gif', '.svg', '.webp',
        'imgur.com', 'flickr.com', 'image', 'photo', 'gallery'
    ]
    if any(pattern in url for pattern in image_patterns):
        return 'images'
    if 'image' in title or 'photo' in title or 'gallery' in title:
        return 'images'

    # Check for documents/PDFs
    doc_patterns = [
        '.pdf', '.doc', '.docx', 'document', 'whitepaper',
        'paper', 'docs.google.com', 'drive.google.com'
    ]
    if any(pattern in url for pattern in doc_patterns):
        return 'documents'
    if 'whitepaper' in title or 'document' in title or 'paper' in description:
        return 'documents'

    # Check for articles/blog posts
    article_patterns = [
        'article', 'blog', 'post', 'medium.com', 'substack.com',
        '/news/', '/blog/', '/article/', 'press', 'story'
    ]
    if any(pattern in url for pattern in article_patterns):
        return 'articles'
    if 'article' in title or 'blog' in title or 'post' in title:
        return 'articles'

    # Default to organizations (websites)
    return 'organizations'

def main():
    # Read the keen.json file
    keen_path = Path('/home/user/elohim/docs/keen.json')
    with open(keen_path, 'r', encoding='utf-8') as f:
        keen_data = json.load(f)

    # Initialize collections
    collections = {
        'organizations': [],
        'apps': [],
        'audio': [],
        'video': [],
        'images': [],
        'documents': [],
        'books': [],
        'articles': []
    }

    # Process all sections and gems
    total_gems = 0
    for section in keen_data.get('sections', []):
        for gem in section.get('gems', []):
            total_gems += 1
            category = categorize_gem(gem)

            # Add section context to gem
            gem_with_context = {
                'gemId': gem.get('gemId'),
                'section': {
                    'sectionId': section.get('sectionId'),
                    'title': section.get('title'),
                    'description': section.get('description')
                },
                'text': gem.get('text'),
                'metalink': gem.get('metalink'),
                'tags': gem.get('tags', []),
                'contributor': gem.get('contributor'),
                'tipImage': gem.get('tipImage')
            }

            collections[category].append(gem_with_context)

    # Create output directory
    output_dir = Path('/home/user/elohim/docs/keen_collections')
    output_dir.mkdir(exist_ok=True)

    # Write separate JSON files for each category
    stats = {}
    for category, gems in collections.items():
        if gems:  # Only create file if there are gems in this category
            output_path = output_dir / f'{category}.json'

            # Create a structured output with metadata
            output_data = {
                'source': 'keen.json',
                'category': category,
                'count': len(gems),
                'sourceKeenId': keen_data.get('keenId'),
                'sourceTitle': keen_data.get('title'),
                'gems': gems
            }

            with open(output_path, 'w', encoding='utf-8') as f:
                json.dump(output_data, f, indent=2, ensure_ascii=False)

            stats[category] = len(gems)
            print(f"Created {category}.json with {len(gems)} gems")

    # Print summary
    print(f"\n=== Summary ===")
    print(f"Total gems processed: {total_gems}")
    for category in sorted(stats.keys()):
        print(f"  {category}: {stats[category]}")

    # Create a summary JSON
    summary = {
        'total_gems': total_gems,
        'categories': stats,
        'source_file': 'docs/keen.json',
        'output_directory': 'docs/keen_collections/'
    }

    summary_path = output_dir / '_summary.json'
    with open(summary_path, 'w', encoding='utf-8') as f:
        json.dump(summary, f, indent=2)

    print(f"\nCreated summary at {summary_path}")

if __name__ == '__main__':
    main()
