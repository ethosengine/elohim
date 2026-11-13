"""Read-only edit context; retire when native sense owns poll/reconcile/paydown judgement."""
from pathlib import Path, PurePosixPath
import re
import shlex

from _lib.dependabot_harvest import clusters
from _lib.dependabot_paydown import DEPENDENCY_FILES, read_ledger, remediation


def context(root, event):
    data = event.get("tool_input", {})
    cwd = Path(event.get("cwd") or root)
    if not cwd.is_absolute():
        cwd = root / cwd
    paths = []
    if event.get("tool_name") in {"Edit", "Write"}:
        paths = [data.get("file_path", "")]
    elif event.get("tool_name") == "Bash":
        # Advisory lexical hints only: never execute or inspect shell effects.
        # Follow literal cd commands for the common `cd crate && ... Cargo.lock`.
        for segment in re.split(r"&&|\|\||[;\n]", data.get("command", "")):
            lexer = shlex.shlex(segment, posix=True, punctuation_chars="<>()|")
            lexer.whitespace_split = True
            words = list(lexer)
            if len(words) == 2 and words[0] == "cd":
                directory = Path(words[1])
                cwd = directory if directory.is_absolute() else cwd / directory
            for word in words:
                path = Path(word.split("=", 1)[-1])
                if path.name in DEPENDENCY_FILES:
                    paths.append(str(path if path.is_absolute() else cwd / path))
    directories = set()
    for value in paths:
        path = Path(value)
        if path.name not in DEPENDENCY_FILES:
            continue
        path = path if path.is_absolute() else cwd / path
        try:
            directories.add(path.resolve().relative_to(root.resolve()).parent.as_posix())
        except ValueError:
            continue
    if not directories:
        return None
    ledger = root / ".claude/data/dependabot-findings.jsonl"
    if not ledger.exists():
        return None
    rows = [r for r in read_ledger(ledger) if str(PurePosixPath(r["manifest_path"]).parent) in directories]
    groups = sorted(clusters(rows), key=lambda g: (-g["severities"]["critical"], -g["count"], g["package"]))
    lines = [f'dependabot: {g["package"]} ({g["ecosystem"]}) · {g["count"]} open · '
             f'{", ".join(g["manifests"])} · patched {g["first_patched"] or "unavailable"} → {remediation(g)}'
             for g in groups[:8]]
    if len(groups) > 8:
        lines.append(f"… {len(groups) - 8} more → dependabot-harvest.py --clusters")
    return "\n".join(lines) or None
