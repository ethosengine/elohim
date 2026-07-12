#!/usr/bin/env python3
"""
Post-edit EPR link suggestion hook for Claude Code.
Detects <a> tags, markdown links, and routerLink attributes that could
become epr: content links — the protocol's native addressing format.

Only outputs if it finds convertible links. Suggestions are informational,
not blocking.

Hook Type: PostToolUse
Matcher: Edit|Write
"""

import json
import os
import re
import sys


def extract_content_slug(url: str) -> str | None:
    """Extract a content slug from a URL if it looks like internal content.

    Returns the slug (e.g. 'manifesto') or None if the URL is external,
    an anchor, or already uses epr: format.
    """
    if not url:
        return None

    # Skip external, anchor-only, already-epr, mailto, tel
    if url.startswith(('http://', 'https://', '#', 'epr:', 'mailto:', 'tel:')):
        return None

    # /lamad/resource/{slug} → slug
    m = re.match(r'/lamad/resource/([a-z0-9-]+)', url)
    if m:
        return m.group(1)

    # /content/{slug} → slug
    m = re.match(r'/content/([a-z0-9-]+)', url)
    if m:
        return m.group(1)

    # Relative path like ./fair-exchange or fair-exchange.md (no slashes in slug)
    m = re.match(r'\.?/?([a-z][a-z0-9-]+)(?:\.md)?$', url)
    if m and '/' not in m.group(1):
        return m.group(1)

    return None


def scan_html(lines: list[str]) -> list[str]:
    """Scan HTML template lines for convertible links."""
    suggestions = []

    for i, line in enumerate(lines, 1):
        # Skip lines that already use epr: or the EPR link component
        if 'epr:' in line or 'app-epr-link' in line:
            continue

        # Detect <a href="..."> with internal URLs
        for m in re.finditer(r'<a\s[^>]*href="([^"]+)"', line):
            href = m.group(1)
            slug = extract_content_slug(href)
            if slug:
                suggestions.append(
                    f'  L{i}: href="{href}" \u2192 consider epr:{slug}'
                )

        # Detect routerLink="/lamad/resource/..."
        for m in re.finditer(r'routerLink="(/lamad/resource/([^"]+))"', line):
            route, slug = m.group(1), m.group(2)
            suggestions.append(
                f'  L{i}: routerLink="{route}" \u2192 consider '
                f'<app-epr-link epr="epr:{slug}">'
            )

    return suggestions


def scan_markdown(lines: list[str]) -> list[str]:
    """Scan markdown lines for convertible links."""
    suggestions = []

    for i, line in enumerate(lines, 1):
        # Skip lines that already use epr:
        if 'epr:' in line:
            continue

        # Skip fenced code blocks
        if line.strip().startswith('```'):
            continue

        # Detect [text](url) where url could become epr:
        for m in re.finditer(r'\[([^\]]+)\]\(([^)]+)\)', line):
            text, url = m.group(1), m.group(2)
            slug = extract_content_slug(url)
            if slug:
                suggestions.append(
                    f'  L{i}: [{text}]({url}) \u2192 consider [{text}](epr:{slug})'
                )

    return suggestions


def main():
    try:
        data = json.load(sys.stdin)

        tool_input = data.get('tool_input', {})
        file_path = tool_input.get('file_path', '')

        if not file_path:
            sys.exit(0)

        # Only check HTML templates and markdown files
        ext = os.path.splitext(file_path)[1]
        if ext not in ('.html', '.md'):
            sys.exit(0)

        # Skip files outside the main content areas
        if not any(d in file_path for d in (
            '/elohim-app/', '/genesis/', '/doorway-app/'
        )):
            sys.exit(0)

        try:
            with open(file_path) as f:
                lines = f.readlines()
        except (FileNotFoundError, PermissionError):
            sys.exit(0)

        # Route to scanner by file type
        if ext == '.html':
            suggestions = scan_html(lines)
        elif ext == '.md':
            suggestions = scan_markdown(lines)
        else:
            suggestions = []

        if suggestions:
            basename = os.path.basename(file_path)
            tip = ('  Tip: Use <app-epr-link epr="epr:id"> in templates, '
                   '[text](epr:id) in markdown')
            msg = (f'[EPR] {basename}:\n'
                   + '\n'.join(suggestions[:8])
                   + '\n' + tip)
            result = {
                'hookSpecificOutput': {
                    'hookEventName': 'PostToolUse',
                    'additionalContext': msg,
                }
            }
            print(json.dumps(result))

        sys.exit(0)

    except json.JSONDecodeError:
        sys.exit(0)
    except Exception as e:
        print(f'epr-link-check hook error: {e}', file=sys.stderr)
        sys.exit(1)


if __name__ == '__main__':
    main()
