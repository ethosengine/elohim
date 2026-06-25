#!/usr/bin/env python3
"""Project the repo-root .epr-meta `ci-trigger:` leg into the flat .ci-ignore.
Default: write <repo>/.ci-ignore. --verify: exit 1 if the on-disk file is stale (CI/pre-push gate).
Reuses .claude/scripts/_lib (epr_meta.find_repo_root + ci_trigger). Fail-open by design: callers
guard on `command -v python3`, so absence of this script never blocks a push."""
import sys
from pathlib import Path

_here = Path(__file__).resolve()
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        break
    _here = _here.parent

from _lib import epr_meta, ci_trigger  # noqa: E402


def main(argv: list[str]) -> int:
    repo_root = epr_meta.find_repo_root(Path.cwd())
    fresh, rendered = ci_trigger.verify(repo_root)
    if "--verify" in argv:
        if fresh:
            return 0
        print("ERROR: .ci-ignore is stale relative to the root .epr-meta ci-trigger leg.\n"
              "  Run: python3 .claude/scripts/ci-ignore-projector.py && git add .ci-ignore",
              file=sys.stderr)
        return 1
    (repo_root / ".ci-ignore").write_text(rendered)
    print(f"wrote {repo_root / '.ci-ignore'} ({rendered.count(chr(10))} lines)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
