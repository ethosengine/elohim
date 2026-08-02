#!/usr/bin/env python3
"""concern_canon_projection — the SDK projection of the concern canon (plan task P4.6).

Design: `genesis/docs/superpowers/plans/2026-08-02-seam-concern-contract-architecture-plan.md`
(Design surface 7, "SDK inheritance surface"). Generates
`elohim/sdk/schemas/v1/registries/concern-classes.json` — the canon as
`{id, concern, title, statement, version, contentHash, home}` — **from BOTH registry homes**
(`.claude/epr-meta/policies.yaml` enforcement rows + `.claude/epr-meta/concerns.yaml` Precedent
rows), tips only, superseded rows excluded.

WHY THIS LIVES IN `_lib` AND NOT `elohim/sdk/schemas/scripts/` (the house-style question, answered
deliberately). That directory's generators are `.mjs` and read JSON Schema; this one reads the
two `.epr-meta` YAML registries, and the repo already has exactly one authority for parsing them:
`_lib.epr_meta.load_policies` (size/depth guards, contentHash verification, per-row degradation)
and `_lib.seam_census.load_concerns` (its sibling for the second home). A node generator would have
to re-derive "which row is the tip" — the two-pass, superseded-aware walk `seam_census` already
owns — and that second derivation is two truths about one fact, which is the duplication the
canon's own C7 (advertise/serve symmetry) and the two-plane split exist to prevent. So: the
generator composes the LIVE readers, and the SDK receives a plain JSON artifact that needs no
Claude tooling to consume. The consumer side of the contract stays language-neutral; only the
producer reaches into `.claude/`.

CONSUMERS (a registry row with no consumer is silently-unloaded by another name):
  • `crates/seam-contracts/src/canon.rs` — `PolicyPin` consts; its conformance test reads the
    YAML registries DIRECTLY (independent source, anti-mirror), never this projection.
  • the concern x seam matrix generator (plan P4.3) — reads the published projection so an
    external consumer's matrix uses the same input as ours.
  • `concern-canon-changelog.json` (authored, beside this file) — lineage rows this module
    verifies BOTH directions against the registries.

Run:
  python3 .claude/scripts/_lib/concern_canon_projection.py            # --check (default)
  python3 .claude/scripts/_lib/concern_canon_projection.py --write    # regenerate in place
Exit 0 = fresh/valid, 1 = stale or lineage mismatch (the message names the fix command).
"""
from __future__ import annotations

import json
import os
import re
import sys
from pathlib import Path

if __package__ in (None, ""):  # direct-run bootstrap (`python3 .../concern_canon_projection.py`)
    _here = Path(__file__).resolve()
    for _ in range(8):
        if (_here / ".claude" / "scripts" / "_lib").is_dir():
            sys.path.insert(0, str(_here / ".claude" / "scripts"))
            break
        _here = _here.parent

from _lib import epr_meta as em  # noqa: E402
from _lib import seam_census as sc  # noqa: E402

CLASSES_REL = "elohim/sdk/schemas/v1/registries/concern-classes.json"
CHANGELOG_REL = "elohim/sdk/schemas/v1/registries/concern-canon-changelog.json"
POLICIES_REL = ".claude/epr-meta/policies.yaml"
CONCERNS_REL = ".claude/epr-meta/concerns.yaml"

FIX_CMD = "python3 .claude/scripts/_lib/concern_canon_projection.py --write"

# `superseded_by:` is either a bare `<id>@<version>` (same home) or `<file>#<id>@<version>`
# (across homes — the C6a graduation shape). Both resolve to the same (id, version) tip.
_SUPERSEDED_BY = re.compile(r"^(?:(?P<home>[\w.\-]+)#)?(?P<id>[a-z0-9\-]+)@(?P<version>\d+)$")


def _norm(text) -> str:
    """Collapse a YAML folded scalar to one deterministic line.

    A `>` block already folds newlines to spaces, but keeps a trailing newline and can carry
    double-newline paragraph breaks. Byte-comparable output demands one normalization, applied
    once, here.
    """
    return re.sub(r"\s+", " ", str(text or "")).strip()


def canon_rows(repo_root: Path) -> tuple[list[dict], list[str]]:
    """Both homes -> (tip rows in canonical concern order, errors).

    Tip selection mirrors `seam_census.census_data`'s two-pass rule EXACTLY, because it is the
    same question: ACTIVE rows first (policies.yaml wins a tie — an enforcement row is the
    stronger home), superseded rows only as a fallback for a class with no active row anywhere.
    A pin-mismatched row is never projected: an unverifiable content hash confers nothing (C5),
    so it degrades to "this class has no tip" rather than publishing a hash we cannot vouch for.
    """
    policies, perrs = em.load_policies(repo_root)
    concerns, cerrs = sc.load_concerns(repo_root)
    errs = [f"{POLICIES_REL}: {e}" for e in perrs] + [f"{CONCERNS_REL}: {e}" for e in cerrs]

    def _rows(source: dict, home: str, want_superseded: bool):
        for row in source.values():
            if not isinstance(row, dict) or row.get("__pin_mismatch__"):
                continue
            cid = row.get("concern")
            if cid not in set(sc.ALL_CONCERN_IDS):
                continue
            if (str(row.get("status", "")).strip() == "superseded") is not want_superseded:
                continue
            yield cid, home, row

    tip: dict[str, tuple[str, dict]] = {}
    for want_superseded in (False, True):
        for cid, home, row in (list(_rows(policies, "policies.yaml", want_superseded))
                               + list(_rows(concerns, "concerns.yaml", want_superseded))):
            tip.setdefault(cid, (home, row))

    out = []
    for cid in sc.ALL_CONCERN_IDS:
        if cid not in tip:
            errs.append(f"concern class {cid} has NO row in either home — the canon advertises "
                        f"{len(sc.ALL_CONCERN_IDS)} classes and would serve {len(tip)} (C7)")
            continue
        home, row = tip[cid]
        out.append({
            "id": row["id"],
            "concern": cid,
            "title": _norm(row.get("title")),
            "statement": _norm(row.get("statement")),
            "version": row["version"],
            "contentHash": row.get("contentHash") or "",
            "home": home,
        })
    return out, errs


def lineage_edges(repo_root: Path) -> tuple[list[dict], list[str]]:
    """Every supersession edge declared in either home, as `{policy_id, from, to, ...}`.

    This is the changelog's independent source: the changelog is AUTHORED (a `migration_note` is
    a judgment no generator can invent), and `check()` asserts both directions against these
    edges — an unrecorded graduation and a changelog row naming no real edge each fail loud.
    """
    policies, _ = em.load_policies(repo_root)
    concerns, _ = sc.load_concerns(repo_root)
    all_rows = {("policies.yaml", k): v for k, v in policies.items()}
    all_rows.update({("concerns.yaml", k): v for k, v in concerns.items()})

    by_key: dict[tuple[str, int], tuple[str, dict]] = {}
    for (home, _key), row in all_rows.items():
        if isinstance(row, dict) and row.get("id") and isinstance(row.get("version"), int):
            by_key[(row["id"], row["version"])] = (home, row)

    edges, errs = [], []
    for (home, _key), row in sorted(all_rows.items()):
        if not isinstance(row, dict) or not row.get("superseded_by"):
            continue
        if row.get("concern") not in set(sc.ALL_CONCERN_IDS):
            continue  # non-canon policy rows keep their own lineage; not this projection's job
        m = _SUPERSEDED_BY.match(str(row["superseded_by"]).strip())
        if not m:
            errs.append(f"{row['id']}@{row['version']} has unparseable `superseded_by: "
                        f"{row['superseded_by']!r}` (want `<id>@<n>` or `<file>#<id>@<n>`)")
            continue
        to_id, to_ver = m.group("id"), int(m.group("version"))
        target = by_key.get((to_id, to_ver))
        if target is None:
            errs.append(f"{row['id']}@{row['version']} supersedes {to_id}@{to_ver}, which exists "
                        f"in NEITHER home — a dangling lineage pointer (C7)")
            continue
        edges.append({
            "policy_id": row["id"],
            "concern": row.get("concern"),
            "from": row["version"],
            "to": to_ver,
            "from_home": home,
            "to_home": target[0],
            "from_content_hash": row.get("contentHash") or "",
            "to_content_hash": target[1].get("contentHash") or "",
        })
    edges.sort(key=lambda e: (e["policy_id"], e["from"]))
    return edges, errs


def canon_version(repo_root: Path) -> int:
    """`concern-canon-version` as declared by concerns.yaml (the canon's own home)."""
    text = (Path(repo_root) / CONCERNS_REL).read_text(encoding="utf-8", errors="replace")
    m = re.search(r"^concern-canon-version:\s*(\d+)\s*$", text, re.M)
    return int(m.group(1)) if m else 0


def render_classes(repo_root: Path) -> tuple[str, list[str]]:
    """The exact bytes of `concern-classes.json` (2-space JSON, trailing newline)."""
    rows, errs = canon_rows(repo_root)
    doc = {
        "$comment": (
            "GENERATED by .claude/scripts/_lib/concern_canon_projection.py — DO NOT HAND-EDIT. "
            "Source of truth is the two registry homes (.claude/epr-meta/policies.yaml + "
            ".claude/epr-meta/concerns.yaml); this is their published projection for external "
            "peer runtimes. Tips only: a superseded row never appears here, and its lineage "
            "lives in concern-canon-changelog.json. Regenerate with `" + FIX_CMD + "`."
        ),
        "concernCanonVersion": canon_version(repo_root),
        "generatedFrom": [POLICIES_REL, CONCERNS_REL],
        "classes": rows,
    }
    return json.dumps(doc, indent=2, ensure_ascii=False) + "\n", errs


def check(repo_root: Path) -> list[str]:
    """Freshness + lineage conformance. Returns problems (empty = green)."""
    repo_root = Path(repo_root)
    problems: list[str] = []

    want, errs = render_classes(repo_root)
    problems.extend(errs)
    path = repo_root / CLASSES_REL
    if not path.is_file():
        problems.append(f"{CLASSES_REL} is MISSING — run `{FIX_CMD}`")
    else:
        have = path.read_text(encoding="utf-8")
        if have != want:
            problems.append(
                f"{CLASSES_REL} is STALE ({len(have)}B on disk vs {len(want)}B regenerated) — the "
                f"canon moved and its published projection did not. Run `{FIX_CMD}`."
            )

    edges, ledgs = lineage_edges(repo_root)
    problems.extend(ledgs)
    cpath = repo_root / CHANGELOG_REL
    if not cpath.is_file():
        problems.append(f"{CHANGELOG_REL} is MISSING — the changelog is AUTHORED (a "
                        f"migration_note is a judgment, not a derivation); create it with one "
                        f"row per supersession edge: {[e['policy_id'] for e in edges]}")
        return problems
    try:
        changelog = json.loads(cpath.read_text(encoding="utf-8"))
        changes = changelog.get("changes") or []
    except (OSError, json.JSONDecodeError) as e:
        problems.append(f"{CHANGELOG_REL} unreadable/invalid JSON: {e}")
        return problems

    # BOTH directions (C7 applied to the canon's own lineage surface).
    have_keys = {(c.get("policy_id"), c.get("from"), c.get("to")) for c in changes}
    for e in edges:
        key = (e["policy_id"], e["from"], e["to"])
        if key not in have_keys:
            problems.append(
                f"supersession {e['policy_id']}@{e['from']} -> @{e['to']} "
                f"({e['from_home']} -> {e['to_home']}) has NO row in {CHANGELOG_REL}. An external "
                f"consensus diffing pins would see the pin move with no migration note — that is "
                f"the cross-repo cascade failing silently. Add the row (from_content_hash "
                f"{e['from_content_hash']}, to_content_hash {e['to_content_hash']})."
            )
    edge_keys = {(e["policy_id"], e["from"], e["to"]) for e in edges}
    for c in changes:
        key = (c.get("policy_id"), c.get("from"), c.get("to"))
        if key not in edge_keys:
            problems.append(
                f"{CHANGELOG_REL} row {key} names a supersession the registries do not declare — "
                f"the changelog advertises a migration that never happened (C7)."
            )
            continue
        edge = next(e for e in edges if (e["policy_id"], e["from"], e["to"]) == key)
        for field in ("from_content_hash", "to_content_hash"):
            if c.get(field) and c[field] != edge[field]:
                problems.append(
                    f"{CHANGELOG_REL} row {key} pins {field}={c[field]} but the registry row "
                    f"hashes to {edge[field]} — a stale changelog hash points an external "
                    f"consumer at bytes that no longer exist."
                )
        for field in ("migration_note", "affected_contract"):
            if not str(c.get(field) or "").strip():
                problems.append(f"{CHANGELOG_REL} row {key} has an empty `{field}` — an empty "
                                f"migration note is the row not existing, wearing a costume.")
    return problems


def write(repo_root: Path) -> tuple[Path, list[str]]:
    repo_root = Path(repo_root)
    text, errs = render_classes(repo_root)
    path = repo_root / CLASSES_REL
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(path.suffix + ".tmp")
    tmp.write_text(text, encoding="utf-8")
    os.replace(tmp, path)
    return path, errs


def _repo_root() -> Path:
    here = Path(__file__).resolve()
    for _ in range(8):
        if (here / ".claude" / "epr-meta").is_dir():
            return here
        here = here.parent
    return Path(os.environ.get("CLAUDE_PROJECT_DIR", "."))


def main(argv: list[str]) -> int:
    root = _repo_root()
    if "--write" in argv:
        path, errs = write(root)
        for e in errs:
            print(f"  ⚠ {e}")
        rows, _ = canon_rows(root)
        print(f"wrote {path.relative_to(root)} — {len(rows)} concern classes "
              f"(canon version {canon_version(root)})")
        problems = check(root)
        for p in problems:
            print(f"  ⛔ {p}")
        return 1 if problems else 0
    problems = check(root)
    if problems:
        print(f"concern-canon projection: {len(problems)} problem(s)")
        for p in problems:
            print(f"  ⛔ {p}")
        return 1
    rows, _ = canon_rows(root)
    print(f"concern-canon projection: fresh ✅ ({len(rows)} classes, "
          f"canon version {canon_version(root)}, both homes)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
