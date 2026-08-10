"""concern_routes.py — resolve a finding to the @concern address it threatens.

The algedonic address book for the dev-plane sentinels (algedonic slice-1,
spec: algedonic-feedback-signal). Pure, stdlib-only, deterministic. A None
return is honest absence — never guess an address.
"""
import re

_CONCERN_TAG = re.compile(r"@concern:([a-z0-9][a-z0-9-]*)")
_ACTIVE_BLOCK = re.compile(
    r"^\s*-\s+id:.*?(?=^\s*-\s+id:|\Z)", re.M | re.S
)

def active_concern(habits_text: str):
    """First @concern tag inside the first `active: true` habit block."""
    for block in _ACTIVE_BLOCK.findall(habits_text or ""):
        if re.search(r"^\s*active:\s*true\s*$", block, re.M):
            m = _CONCERN_TAG.search(block)
            if m:
                return m.group(1)
    return None

def route(cls: str, context: dict):
    """Deterministic class→concern routing. context keys (all optional):
    concern (explicit, wins) · active_concern (fallback for measure-shaped classes)."""
    if context.get("concern"):
        return context["concern"]
    if cls in ("ci-no-measure",):
        return context.get("active_concern")
    return None
