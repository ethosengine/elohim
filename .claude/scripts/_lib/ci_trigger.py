"""ci-trigger: the build-time signal in .epr-meta — collected from the repo-root manifest and
projected into the flat .ci-ignore. Orthogonal to the author-time rule engine (the resolver never
reads it: epr_meta.merge_rules merges only rules/validators). Reuses epr_meta for cascade I/O."""
from __future__ import annotations
from pathlib import Path

from _lib import epr_meta

GENERATED_HEADER = """\
# GENERATED — DO NOT EDIT.
# Projected from the `ci-trigger:` leg of the repo-root .epr-meta by
# .claude/scripts/ci-ignore-projector.py. To change what CI ignores, edit the
# root .epr-meta's ci-trigger.ignore and regenerate (the pre-push freshness gate enforces this).
#
# Pattern grammar (gitignore-flavored, parsed by genesis/orchestrator/ci-ignore.mjs):
#   foo/             subtree prefix match
#   path/to/file     exact path match (anchored)
#   CLAUDE.md        basename match anywhere
#
# .ci-ignore is a LOCAL/pre-push optimization — CI relies on manifest source-globs, not this file.
"""


def collect_ci_trigger(repo_root: Path) -> list[str]:
    """Ordered, de-duplicated ignore-pattern set from the repo-root .epr-meta ci-trigger.ignore.
    v1 reads the root manifest verbatim (the inline-now source of truth); per-subtree discovery +
    dir-qualification is the deferred decentralization refinement (P6 spec §7). Fail-open: any
    parse failure yields [] (load_meta already returns {} on failure)."""
    cfg = epr_meta.load_meta(repo_root / epr_meta.MANIFEST_NAME)
    ct = cfg.get("ci-trigger") or {}
    out: list[str] = []
    for p in ct.get("ignore") or []:
        if isinstance(p, str) and p and p not in out:
            out.append(p)
    return out


def render_ci_ignore(patterns: list[str]) -> str:
    """Deterministic flat .ci-ignore text: GENERATED header + one pattern per line."""
    body = "".join(f"{p}\n" for p in patterns)
    return f"{GENERATED_HEADER}\n{body}"


def verify(repo_root: Path) -> tuple[bool, str]:
    """(is_fresh, rendered): the on-disk .ci-ignore vs the freshly-projected text (byte comparison)."""
    rendered = render_ci_ignore(collect_ci_trigger(repo_root))
    ci_ignore = repo_root / ".ci-ignore"
    current = ci_ignore.read_text() if ci_ignore.is_file() else ""
    return (current == rendered, rendered)
