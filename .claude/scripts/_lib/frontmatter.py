"""Minimal YAML-frontmatter parser — handles the subset our memory entries use.

We avoid importing PyYAML to keep _lib pure-stdlib. Memory frontmatter is
restricted to simple key: value scalars (name, description, type, status, ...)
plus an occasional list. The parser handles:

  - key: value           # scalar
  - key: |               # block scalar (not commonly used; reads first line)
  - list of strings under `related:` (used by spec frontmatter)

For richer YAML, callers should use PyYAML directly.
"""
from __future__ import annotations

import re
from dataclasses import dataclass, field
from typing import Iterable

DELIMITER = "---"


@dataclass
class Frontmatter:
    raw_block: str = ""
    fields: dict[str, str | list[str]] = field(default_factory=dict)
    body: str = ""

    def get(self, key: str, default: str = "") -> str:
        v = self.fields.get(key, default)
        return v if isinstance(v, str) else default


_KV_RE = re.compile(r"^([a-zA-Z0-9_-]+)\s*:\s*(.*)$")


def parse(text: str) -> Frontmatter:
    """Parse a markdown document with optional `---`-delimited frontmatter.

    Returns a Frontmatter with empty `fields` if no frontmatter present.
    """
    lines = text.splitlines()
    if not lines or lines[0].strip() != DELIMITER:
        return Frontmatter(body=text)

    end_idx: int | None = None
    for i, line in enumerate(lines[1:], start=1):
        if line.strip() == DELIMITER:
            end_idx = i
            break
    if end_idx is None:
        return Frontmatter(body=text)

    block_lines = lines[1:end_idx]
    body_lines = lines[end_idx + 1:]

    fm = Frontmatter(raw_block="\n".join(block_lines), body="\n".join(body_lines))
    pending_list_key: str | None = None
    for line in block_lines:
        stripped = line.rstrip()
        if not stripped:
            pending_list_key = None
            continue
        # List continuation
        if pending_list_key and stripped.lstrip().startswith("- "):
            item = stripped.lstrip()[2:].strip()
            existing = fm.fields.get(pending_list_key)
            if isinstance(existing, list):
                existing.append(item)
            else:
                fm.fields[pending_list_key] = [item]
            continue
        m = _KV_RE.match(stripped)
        if not m:
            pending_list_key = None
            continue
        key, val = m.group(1), m.group(2).strip()
        if val == "":
            # Could be a list opening
            fm.fields[key] = []
            pending_list_key = key
        elif val == "|":
            # Block scalar — not commonly used; skip
            pending_list_key = None
        else:
            fm.fields[key] = val.strip('"').strip("'")
            pending_list_key = None
    return fm


def parse_file(path) -> Frontmatter:
    """Convenience: parse frontmatter from a file path (str or Path)."""
    from pathlib import Path
    p = Path(path)
    return parse(p.read_text(encoding="utf-8", errors="replace"))
