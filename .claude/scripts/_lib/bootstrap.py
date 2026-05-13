"""Self-locating bootstrap so any .claude/* script can import _lib.

The harness sets CLAUDE_PROJECT_DIR but some scripts run outside that env
(direct CLI, CI, etc.). This module walks up from the caller's file until
it finds `.claude/scripts/_lib` and adds the parent to sys.path.

Usage at top of any script:

    from pathlib import Path
    import sys
    here = Path(__file__).resolve()
    for _ in range(8):
        if (here / ".claude" / "scripts" / "_lib").is_dir():
            sys.path.insert(0, str(here / ".claude" / "scripts"))
            break
        here = here.parent
    from _lib import paths  # now resolvable

We keep this snippet inline rather than wrapping it in a helper because
the helper itself would be unreachable until _lib is on the path. The
snippet is 6 lines; tolerable boilerplate.
"""
