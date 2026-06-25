"""The .epr-meta cascade + merge — the reuse of subject_routing's root-first/nearest-wins
ancestor walk, retargeted to the `.epr-meta` manifest with nested-YAML parsing."""
from __future__ import annotations
from pathlib import Path

from _lib import frontmatter as fm

try:
    import yaml  # PyYAML — needed for the nested rule config
except Exception:  # pragma: no cover
    yaml = None

MANIFEST_NAME = ".epr-meta"
MAX_CASCADE_DEPTH = 32


def yaml_available() -> bool:
    return yaml is not None


def load_meta(path: Path) -> dict:
    """Parse one .epr-meta (frontmatter block -> nested dict). {} on any failure (caller fails open)."""
    try:
        block = fm.parse_file(path).raw_block
        if yaml is None:
            return {}
        data = yaml.safe_load(block) or {}
        return data if isinstance(data, dict) else {}
    except Exception:
        return {}


def find_repo_root(start: Path) -> Path:
    here = start.resolve()
    if here.is_file():
        here = here.parent
    for _ in range(12):
        if (here / ".git").exists():
            return here
        if here.parent == here:
            break
        here = here.parent
    return start.resolve().parent if start.is_file() else start.resolve()


def collect_cascade(target: Path) -> list[Path]:
    """Ancestor .epr-meta files from `target`'s dir up to a `root: true` base (or repo root),
    bounded by MAX_CASCADE_DEPTH. Returned ROOT-FIRST so nearest wins on merge."""
    target = target.resolve()
    here = target.parent if (target.is_file() or not target.exists()) else target
    root = find_repo_root(here)
    chain: list[Path] = []
    depth = 0
    while depth < MAX_CASCADE_DEPTH:
        meta = here / MANIFEST_NAME
        if meta.is_file():
            chain.append(meta)
            if load_meta(meta).get("root") is True:
                break
        if here == root or here.parent == here:
            break
        here = here.parent
        depth += 1
    return list(reversed(chain))  # root-first


def merge_rules(chain: list[Path]) -> dict:
    """Merge root-first; nearer overrides on rule `id` / validator `ref`."""
    merged: dict = {"rules": {}, "validators": {}, "sources": []}
    for meta in chain:
        cfg = load_meta(meta)
        merged["sources"].append(str(meta))
        for rule in cfg.get("rules", []) or []:
            rid = rule.get("id")
            if rid:
                merged["rules"][rid] = rule
        for v in cfg.get("validators", []) or []:
            ref = v.get("ref")
            if ref:
                merged["validators"][ref] = v
    return merged


