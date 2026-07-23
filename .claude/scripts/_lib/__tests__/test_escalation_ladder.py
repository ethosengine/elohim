#!/usr/bin/env python3
"""Escalation-ladder validator regression — the fenced-manifest hole (2026-07-23).

The charter validator originally `safe_load`-ed raw manifest content; every real
`.epr-meta` is `---`-fenced, so the validator ABSTAINED on all of them and would
never have fired in production. `_frontmatter_or_whole` (mirroring the Rust
`frontmatter_or_whole` in repository_validators.rs) closes it. These probes keep
it closed — and pin the deliberation-provenance convention (`deliberated-*` counts
as ratified alongside legacy `operator-*`).
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from _lib import epr_meta  # noqa: E402

V = epr_meta.REFERENCE_VALIDATORS["epr:validator-escalation-ladder"]
ROOT = str(epr_meta.find_repo_root(Path(__file__)))


def fenced(body: str) -> str:
    return "---\nepr-meta-version: 1\nid: t\n" + body + "---\nbody prose\n"


def write(content: str) -> dict:
    return {"path": f"{ROOT}/xtest/.epr-meta", "is_new": True, "prior_content": None,
            "content": content}


CASES = [
    ("fenced unbacked deny flags", write(fenced(
        "rules:\n  - id: sneaky\n    class: deny\n    when: {write: '*.md'}\n    why: t\n")), True),
    ("fenced unbacked ask flags", write(fenced(
        "rules:\n  - id: sneaky\n    class: ask\n    when: {write: '*.md'}\n    why: t\n")), True),
    ("fenced measure passes (agent self-grant tier)", write(fenced(
        "rules:\n  - id: obs\n    class: measure\n    when: {write: '*.md'}\n    why: t\n")), False),
    ("fenced dispatch passes (agent self-grant tier)", write(fenced(
        "rules:\n  - id: d\n    class: dispatch\n    when: {write: '*.md'}\n    why: t\n")), False),
    ("deliberated-provenance pin counts as backed", write(fenced(
        "rules:\n  - id: bound\n    policy: governance-escalation-ladder@1\n")), False),
    ("bare-YAML unbacked deny still flags", write(
        "rules:\n  - id: sneaky\n    class: deny\n    why: t\n"), True),
]


def main() -> int:
    failures = []
    for name, w, expect in CASES:
        got = V(w)
        mark = "✅" if got is expect else "❌"
        print(f"  {mark} {name}: {got} (expect {expect})")
        if got is not expect:
            failures.append(name)
    if failures:
        print(f"\n  {len(failures)} FAILURE(S): {failures}")
        return 1
    print(f"\n  {len(CASES)} ladder assertions passed ✅")
    return 0


if __name__ == "__main__":
    sys.exit(main())
