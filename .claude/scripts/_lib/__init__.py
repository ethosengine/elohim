"""Shared helpers for .claude/scripts and .claude/hooks Python utilities.

Importable from any script by walking up to find `.claude/scripts/_lib`:

    from _lib import bootstrap  # noqa: E402
    bootstrap.ensure_on_path()  # idempotent; safe to call multiple times
    from _lib import paths, frontmatter, store

Or shorter (relies on harness setting CLAUDE_PROJECT_DIR):

    import os, sys
    sys.path.insert(0, os.path.join(os.environ["CLAUDE_PROJECT_DIR"], ".claude", "scripts"))
    from _lib import paths

This package is intentionally small: only extractions where 3+ callers shared
the same pattern. Resist scope creep. If a helper has only 1-2 callers, keep
it inline.
"""
