#!/usr/bin/env python3
"""
Test ID & Accessibility Audit Hook

Runs after Edit/Write on Angular component files to check:
1. Whether interactive elements have `data-testid` attributes (page object model)
2. Whether interactive elements have accessible labels (a11y)

Silent when clean on both counts. Nudges gently when gaps exist.

Hook Type: PostToolUse
Matcher: Edit|Write
"""
import json
import os
import re
import sys
from pathlib import Path


# Interactive element patterns (regex)
# Each finds an HTML element that a user or test would interact with.
INTERACTIVE_PATTERNS = [
    # Input elements: <input ...>
    r'<input\b[^>]*>',
    # Button elements: <button ...>...</button> (just the opening tag)
    r'<button\b[^>]*>',
    # Select elements
    r'<select\b[^>]*>',
    # Textarea elements
    r'<textarea\b[^>]*>',
    # Anchor with routerLink (Angular navigation)
    r'<a\b[^>]*\[routerLink\][^>]*>',
]

# Elements with click handlers that aren't already buttons/inputs
# Only match non-button/input elements with (click)= binding
CLICK_HANDLER_PATTERN = r'<(?!button|input|select|textarea|a\b)(\w+)\b[^>]*\(click\)[^>]*>'

# data-testid attribute pattern
TESTID_ATTR = re.compile(r'data-testid\s*=\s*["\']([^"\']+)["\']')

# Match data-testid on the same element (within the tag)
TESTID_IN_TAG = re.compile(r'data-testid\s*=')

# Accessible labeling patterns (on the tag itself)
ARIA_LABEL = re.compile(r'(?:aria-label|aria-labelledby|\[attr\.aria-label\]|\[attr\.aria-labelledby\])\s*=')

# Button class names that suggest icon-only (no visible text content)
ICON_BUTTON_HINTS = re.compile(r'class\s*=\s*["\'][^"\']*\b(icon|close|dismiss|toggle|back|menu|collapse|expand|delete|remove|clear|copy|drag|sort|arrow|chevron|hamburger|nav-toggle)\b', re.IGNORECASE)


def has_label_for(element_id: str, html: str) -> bool:
    """Check if a <label for="id"> exists in the template for this element."""
    if not element_id:
        return False
    return bool(re.search(rf'<label\b[^>]*\bfor\s*=\s*["\']{re.escape(element_id)}["\']', html))


def check_a11y(tag_text: str, tag_type: str, element_id: str | None, html: str) -> str | None:
    """
    Check if an interactive element has an accessible label.
    Returns a short issue description, or None if accessible.

    Strategy:
    - Inputs/select/textarea: need <label for="id"> OR aria-label
    - Buttons: only flag if they appear to be icon-only (no text content)
      and lack aria-label. Text-content buttons are assumed accessible.
    - Custom (click) elements: need role + aria-label
    """
    has_aria = bool(ARIA_LABEL.search(tag_text))

    if tag_type in ('input', 'select', 'textarea'):
        # Has aria-label → accessible
        if has_aria:
            return None
        # Has id with a matching <label for> → accessible
        if element_id and has_label_for(element_id, html):
            return None
        # Hidden inputs don't need labels
        if re.search(r'type\s*=\s*["\']hidden["\']', tag_text):
            return None
        # Checkbox/radio are almost always wrapped in a <label> element
        # (implicit association). We can't reliably detect wrapping from regex,
        # so skip these — they're accessible by convention.
        if re.search(r'type\s*=\s*["\'](checkbox|radio)["\']', tag_text):
            return None
        # Other inputs with no id and no aria-label
        if not element_id and not has_aria:
            return 'no label or aria-label'
        # Has id but no matching <label for> — might be wrapped in a <label> tag
        # which is valid but less common. Flag gently.
        if element_id and not has_label_for(element_id, html):
            return f'no <label for="{element_id}"> found'
        return None

    if tag_type == 'button':
        # Only flag icon-only buttons without aria-label.
        # Buttons with text content between tags are accessible but we can't
        # see the content from the opening tag alone. We use class hints.
        if has_aria:
            return None
        if ICON_BUTTON_HINTS.search(tag_text):
            return 'icon-only button, needs aria-label'
        # Also flag buttons that contain only SVG (common pattern: <button><svg>...</svg></button>)
        # We detect this by checking if aria-label is missing AND class suggests no text
        # Skip — we can't reliably detect from opening tag alone. Be conservative.
        return None

    if tag_type == 'a':
        # Links usually have text content. Only flag if they also lack aria-label
        # and have class hints suggesting icon-only.
        if has_aria:
            return None
        if ICON_BUTTON_HINTS.search(tag_text):
            return 'icon-only link, needs aria-label'
        return None

    # Custom elements with (click) — these always need role + aria-label
    if not has_aria:
        has_role = bool(re.search(r'\brole\s*=', tag_text))
        if not has_role:
            return 'needs role and aria-label'
        return 'needs aria-label'

    return None


def is_component_file(file_path: str) -> bool:
    """Check if this is an Angular component template or inline-template file."""
    if file_path.endswith('.component.html'):
        return True
    if file_path.endswith('.component.ts'):
        return True
    return False


def extract_template_from_ts(content: str) -> str | None:
    """Extract inline template from a .component.ts file."""
    # Match template: `...` (backtick-delimited)
    match = re.search(r'template\s*:\s*`(.*?)`', content, re.DOTALL)
    if match:
        return match.group(1)
    return None


def find_interactive_elements(html: str) -> list[dict]:
    """Find interactive elements and check for data-testid and a11y."""
    elements = []

    for pattern in INTERACTIVE_PATTERNS:
        for match in re.finditer(pattern, html):
            tag_text = match.group(0)
            has_testid = bool(TESTID_IN_TAG.search(tag_text))

            # Extract a short description
            tag_type = re.match(r'<(\w+)', tag_text).group(1) if re.match(r'<(\w+)', tag_text) else '?'

            # Try to get id or name for context
            id_match = re.search(r'\bid\s*=\s*["\']([^"\']+)["\']', tag_text)
            name_match = re.search(r'\bname\s*=\s*["\']([^"\']+)["\']', tag_text)
            type_match = re.search(r'\btype\s*=\s*["\']([^"\']+)["\']', tag_text)
            testid_match = TESTID_ATTR.search(tag_text)

            element_id = id_match.group(1) if id_match else None

            label = tag_type
            if id_match:
                label = f'{tag_type}#{element_id}'
            elif name_match:
                label = f'{tag_type}[name={name_match.group(1)}]'
            elif type_match:
                label = f'{tag_type}[type={type_match.group(1)}]'

            a11y_issue = check_a11y(tag_text, tag_type, element_id, html)

            elements.append({
                'label': label,
                'has_testid': has_testid,
                'testid': testid_match.group(1) if testid_match else None,
                'a11y_issue': a11y_issue,
            })

    # Also check for click handlers on non-standard elements
    for match in re.finditer(CLICK_HANDLER_PATTERN, html):
        tag_text = match.group(0)
        has_testid = bool(TESTID_IN_TAG.search(tag_text))
        tag_type = match.group(1)
        testid_match = TESTID_ATTR.search(tag_text)

        a11y_issue = check_a11y(tag_text, tag_type, None, html)

        elements.append({
            'label': f'{tag_type}[(click)]',
            'has_testid': has_testid,
            'testid': testid_match.group(1) if testid_match else None,
            'a11y_issue': a11y_issue,
        })

    return elements


def get_component_name(file_path: str) -> str:
    """Extract component name from file path."""
    basename = os.path.basename(file_path)
    # threshold-login.component.html → threshold-login
    # threshold-login.component.ts → threshold-login
    return basename.replace('.component.html', '').replace('.component.ts', '')


def main():
    try:
        data = json.load(sys.stdin)
        tool_input = data.get('tool_input', {})
        file_path = tool_input.get('file_path', '')

        if not file_path or not is_component_file(file_path):
            sys.exit(0)

        # Read the file content
        if not os.path.exists(file_path):
            sys.exit(0)

        with open(file_path, 'r') as f:
            content = f.read()

        # Extract HTML to scan
        if file_path.endswith('.component.ts'):
            html = extract_template_from_ts(content)
            if not html:
                sys.exit(0)  # No inline template
        else:
            html = content

        # Find interactive elements
        elements = find_interactive_elements(html)

        if not elements:
            sys.exit(0)  # No interactive elements — nothing to report

        modeled = [e for e in elements if e['has_testid']]
        unmodeled = [e for e in elements if not e['has_testid']]
        a11y_issues = [e for e in elements if e.get('a11y_issue')]

        # Silent when fully modeled AND no a11y issues
        if not unmodeled and not a11y_issues:
            sys.exit(0)

        component = get_component_name(file_path)
        total = len(elements)
        covered = len(modeled)

        parts = []

        # TestID report
        if unmodeled:
            parts.append(f'[TestID] {component}: {covered}/{total} interactive elements have data-testid')
            missing = [e['label'] for e in unmodeled[:8]]
            parts.append(f'  Missing: {", ".join(missing)}')
            if len(unmodeled) > 8:
                parts.append(f'  ... and {len(unmodeled) - 8} more')

        # a11y report — separate line, same gentle format
        if a11y_issues:
            parts.append(f'[a11y] {component}: {len(a11y_issues)} element(s) may lack accessible labels')
            for e in a11y_issues[:5]:
                parts.append(f'  {e["label"]}: {e["a11y_issue"]}')
            if len(a11y_issues) > 5:
                parts.append(f'  ... and {len(a11y_issues) - 5} more')

        output = {
            'hookSpecificOutput': {
                'hookEventName': 'PostToolUse',
                'additionalContext': '\n'.join(parts),
            }
        }
        print(json.dumps(output))

    except json.JSONDecodeError:
        sys.exit(0)
    except Exception as e:
        print(f'testid-audit hook error: {e}', file=sys.stderr)
        sys.exit(1)


if __name__ == '__main__':
    main()
