#!/usr/bin/env python3
"""
Story-coverage audit — derived projection over `genesis/data/stories/*.md`
frontmatter and `genesis/a2o/features/**/*.feature` file inventory.

## What this surfaces

The storyteller authors Tier-1 narrative anchors that compose with the test
harness — each story declares one canonical `feature:` (the slug whose passing
proves the story's experience is real) plus `adjacent_features[]` (features the
narrative touches without anchoring). This script audits the **coverage
relationship** between the feature corpus on disk and the story corpus:

- **Orphan features** — features that exist on disk but appear in **zero**
  stories (neither canonical nor adjacent). Storyteller-authoring candidates.
- **Adjacent-only features** — features that appear in at least one story but
  only as `adjacent_features`. No story claims them canonically — they could
  be promoted (a new story written that anchors them).
- **Stories with coverage** — per-story summary of canonical + adjacent counts.

Cross-references each orphan/adjacent-only feature against
`.claude/memory-kit/delivery-status-distribution.json` to surface
`delivery_status` from `/deliver`'s manifest, where known. (Features the
bridge has not yet judged are reported `null`.)

## Storage classification (load-bearing — per plan `stateless-seeking-forest.md`)

Both outputs are **LOCAL OPERATOR-TIER ARTIFACTS**, not P2P substrate.

| Artifact | Class | Writer | Source of truth |
|---|---|---|---|
| `.claude/memory-kit/story-coverage-audit.json` | Derived projection (regenerable) | this script only | story frontmatter + features-on-disk |
| `.claude/memory-kit/<date>/story-coverage-audit.md` | Dated human-readable report | this script only | same JSON |

Same tier as `.claude/memory-kit/claude-md-drift.json` and the existing
`delivery-status-distribution.json`. Single-writer; idempotent across runs;
not DHT-bound; not a notarized truth. The P2P-design-gate's intent (catch
peer-divergent shapes before they ship) does not apply: these are derived
projections of source-of-truth that lives elsewhere (frontmatter + files).

## Authority boundary

This script READS frontmatter + filesystem; it never writes to stories,
features, or any P2P-substrate path. `/deliver` is the only authority for
`active.*` / `stable` / `regression` states; this script surfaces orphan
features as candidates the storyteller can pick up.

## Idempotency

Re-running the script in the same day produces zero diff in the JSON outside
the `generated_at` timestamp. Rotation: before write, copies current JSON to
`.prev.json` so the next run can compute orphan-count delta.

Outputs:
  - `.claude/memory-kit/story-coverage-audit.json` (rolling state)
  - `.claude/memory-kit/<YYYY-MM-DD>/story-coverage-audit.md` (report)
"""
from __future__ import annotations

import json
import re
import sys
from datetime import date, datetime, timezone
from pathlib import Path

# Bootstrap: locate .claude/scripts/_lib via repo_root_from_file
_here = Path(__file__).resolve()
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir() and (_here / ".git").exists():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        break
    _here = _here.parent
from _lib import paths as _paths  # noqa: E402
from _lib import frontmatter as _fm  # noqa: E402

REPO_ROOT = _paths.repo_root_from_file(__file__)
REPORTS_ROOT = _paths.reports_root(REPO_ROOT)
TODAY = date.today()
TODAY_DIR = _paths.reports_dir_for_today(REPO_ROOT, TODAY)

STATE_JSON = REPORTS_ROOT / "story-coverage-audit.json"
PREV_JSON = REPORTS_ROOT / "story-coverage-audit.prev.json"
DATED_MD = TODAY_DIR / "story-coverage-audit.md"

STORIES_DIR = REPO_ROOT / "genesis" / "data" / "stories"
FEATURES_DIR = REPO_ROOT / "genesis" / "a2o" / "features"
DELIVERY_DIST_JSON = REPORTS_ROOT / "delivery-status-distribution.json"

# Non-story scaffolding files in genesis/data/stories/
STORY_SCAFFOLDING = {"CONVENTIONS.md", "INDEX.md", "README.md"}

# Vision-axis hint: weight features by their pillar path. Stories drive learner
# experience first, so lamad/auth/qahal weigh heaviest; pure-infra features
# (delivery transport, resilience plumbing) weigh lighter even when they have
# many scenarios. Used for the "top-leverage orphan" ranking only.
VISION_AXIS_WEIGHTS = {
    "lamad": 3.0,         # learning — central learner experience
    "auth": 2.5,          # imagodei — identity, account management
    "qahal": 2.5,         # community, governance, attestation
    "content": 2.0,       # content pipeline + stewardship narratives
    "browser": 1.5,       # cross-cutting browser-mode delivery
    "federation": 1.5,    # federation surface (doorway-side)
    "delivery": 1.0,      # transport/delivery substrate
    "resilience": 1.0,    # infra plumbing — important but not narrative-shaped
}
DEFAULT_VISION_WEIGHT = 1.0  # for unknown axes (don't down-weight; just don't up-weight)

_SCENARIO_RE = re.compile(r"^\s*Scenario(?:\s+Outline)?\s*:", re.MULTILINE)

# Strip trailing inline YAML comment ("key: value  # ...") that the shared
# `_lib.frontmatter` regex captures into the value. Same shape as the helper
# in delivery-status-distribution.py — kept inline here to avoid extracting
# until ≥3 callers share it (per `project_shared_lib_pattern`).
_INLINE_COMMENT_RE = re.compile(r'(?<!\\)\s+#.*$')


def _clean(val) -> str:
    """Strip trailing inline YAML comment and surrounding quotes from a
    frontmatter scalar. Returns "" for non-strings."""
    if not isinstance(val, str):
        return ""
    cleaned = _INLINE_COMMENT_RE.sub('', val).strip()
    return cleaned.strip('"').strip("'").strip()


def _vision_axis_for(feature_rel_path: str) -> str:
    """Extract the vision axis (pillar dir) from a feature path.

    Example: `genesis/a2o/features/lamad/learning-journey.feature` → `lamad`.
    Single-segment features land in `_root` (no pillar)."""
    # Path is repo-relative; we want the segment immediately under
    # genesis/a2o/features/. If the feature lives deeper than that, the
    # next segment is the pillar.
    parts = feature_rel_path.split("/")
    if len(parts) >= 5 and parts[0] == "genesis" and parts[2] == "features":
        return parts[3]
    return "_root"


def _scenario_count(feature_path: Path) -> int:
    """Count `Scenario:` and `Scenario Outline:` lines in a feature file.

    Conservative — comments and tagged scenarios both count once. Errors
    return 0 (the feature file is unreadable; we still report it on disk)."""
    try:
        text = feature_path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return 0
    return len(_SCENARIO_RE.findall(text))


def _feature_inventory() -> dict[str, dict]:
    """Walk genesis/a2o/features/**/*.feature. Build a map:
        feature_rel_path → {
          slug, axis, scenario_count, path_obj (resolved relative to repo)
        }
    Slugs are the bare filename without `.feature`."""
    inventory: dict[str, dict] = {}
    if not FEATURES_DIR.is_dir():
        return inventory
    for p in sorted(FEATURES_DIR.rglob("*.feature")):
        rel = str(p.relative_to(REPO_ROOT))
        slug = p.stem
        inventory[rel] = {
            "feature_path": rel,
            "feature_slug": slug,
            "vision_axis_hint": _vision_axis_for(rel),
            "scenario_count": _scenario_count(p),
        }
    return inventory


def _resolve_to_feature_path(token: str, inventory: dict[str, dict]) -> str | None:
    """Resolve a story-frontmatter feature reference to a canonical feature
    inventory key.

    Stories use two reference shapes (per CONVENTIONS.md + observed data):
    - **Slug form** (e.g., `stewarded-device-sync`) — typical of the `feature:`
      triple field
    - **Pillar-prefixed path** (e.g., `lamad/learning-journey.feature`) — typical
      of `adjacent_features[]`

    We try exact-suffix match against inventory keys. Returns the matching
    feature_path, or None if the reference is dangling (no feature on disk)."""
    if not token:
        return None
    t = token.strip().strip("'\"")
    # Add .feature suffix if not present, for slug form
    candidate_basename = t if t.endswith(".feature") else f"{t}.feature"
    # Exact suffix match
    for inv_path in inventory.keys():
        if inv_path.endswith("/" + candidate_basename):
            return inv_path
        # Bare-slug match against the slug (no path component)
        if "/" not in t and inv_path.endswith("/" + t + ".feature"):
            return inv_path
    return None


def _scan_stories() -> list[dict]:
    """Walk genesis/data/stories/*.md. For each story, read frontmatter and
    extract the canonical feature + adjacent_features list. Returns a list
    of story-dicts."""
    rows: list[dict] = []
    if not STORIES_DIR.is_dir():
        return rows
    for md in sorted(STORIES_DIR.glob("*.md")):
        if md.name in STORY_SCAFFOLDING:
            continue
        try:
            fm = _fm.parse_file(md)
        except Exception as e:
            rows.append({
                "slug": md.stem,
                "path": str(md.relative_to(REPO_ROOT)),
                "canonical_feature_ref": "",
                "adjacent_feature_refs": [],
                "frontmatter_anomaly": f"parse_error: {e}",
            })
            continue

        slug = _clean(fm.get("slug")) or md.stem
        author_status = _clean(fm.get("status"))
        canonical = _clean(fm.get("feature"))
        # adjacent_features: a list per CONVENTIONS.md
        adjacent_raw = fm.fields.get("adjacent_features", [])
        if isinstance(adjacent_raw, list):
            adjacent = [c for c in (_clean(a) for a in adjacent_raw) if c]
        elif isinstance(adjacent_raw, str) and adjacent_raw.strip():
            cleaned = _clean(adjacent_raw)
            adjacent = [cleaned] if cleaned else []
        else:
            adjacent = []

        # Legacy frontmatter (pre-triple schema): some retired stories carry
        # `covers_features[]` instead of `feature:` + `adjacent_features[]`.
        # Surface both forms so legacy retired stories still count as coverage.
        covers_raw = fm.fields.get("covers_features", [])
        if isinstance(covers_raw, list):
            covers = [c for c in (_clean(x) for x in covers_raw) if c]
        elif isinstance(covers_raw, str) and covers_raw.strip():
            cleaned = _clean(covers_raw)
            covers = [cleaned] if cleaned else []
        else:
            covers = []

        rows.append({
            "slug": slug,
            "path": str(md.relative_to(REPO_ROOT)),
            "author_status": author_status,
            "canonical_feature_ref": canonical,
            "adjacent_feature_refs": adjacent,
            "covers_feature_refs_legacy": covers,
        })
    return rows


def _delivery_status_lookup() -> dict[str, str]:
    """Read `.claude/memory-kit/delivery-status-distribution.json` if present
    and build a feature-slug → delivery_status lookup.

    delivery-status-distribution.json's `stories[]` carries `declared_feature`
    (slug form) + the story's own `delivery_status`. It does NOT carry
    feature-level verdicts directly — those live in `.claude/deliver/manifest.json`.

    For this audit's purposes — surfacing 'is this orphan feature already on
    /deliver's radar?' — we approximate via the story aggregate: if a feature
    slug appears as the `declared_feature` of any story, we copy that story's
    delivery_status. Otherwise we return null.

    This is imperfect (orphan features by definition aren't anchored by any
    story), but it surfaces the slugs that ARE anchored — which lets the
    storyteller spot stories that anchor undelivered features."""
    lookup: dict[str, str] = {}
    if not DELIVERY_DIST_JSON.is_file():
        return lookup
    try:
        data = json.loads(DELIVERY_DIST_JSON.read_text(encoding="utf-8"))
    except Exception:
        return lookup
    for s in data.get("stories", []):
        feat = (s.get("declared_feature") or "").strip()
        ds = s.get("delivery_status")
        if feat and ds:
            lookup[feat] = ds
    return lookup


def _build_state() -> dict:
    inventory = _feature_inventory()
    stories = _scan_stories()
    delivery_lookup = _delivery_status_lookup()

    # Build coverage sets
    canonical_paths: set[str] = set()       # feature_paths anchored by any story
    adjacent_paths: set[str] = set()        # feature_paths touched as adjacent (any story)
    legacy_paths: set[str] = set()          # feature_paths from legacy covers_features
    dangling_refs: list[dict] = []          # (story, ref-token, role) for broken links
    story_summaries: list[dict] = []

    for story in stories:
        canonical_resolved: str | None = None
        if story.get("canonical_feature_ref"):
            ref = story["canonical_feature_ref"]
            resolved = _resolve_to_feature_path(ref, inventory)
            if resolved:
                canonical_paths.add(resolved)
                canonical_resolved = resolved
            else:
                dangling_refs.append({
                    "story_slug": story["slug"],
                    "ref": ref,
                    "role": "canonical",
                })

        adjacent_resolved: list[str] = []
        for ref in story.get("adjacent_feature_refs", []):
            resolved = _resolve_to_feature_path(ref, inventory)
            if resolved:
                adjacent_paths.add(resolved)
                adjacent_resolved.append(resolved)
            else:
                dangling_refs.append({
                    "story_slug": story["slug"],
                    "ref": ref,
                    "role": "adjacent",
                })

        legacy_resolved: list[str] = []
        for ref in story.get("covers_feature_refs_legacy", []):
            resolved = _resolve_to_feature_path(ref, inventory)
            if resolved:
                legacy_paths.add(resolved)
                legacy_resolved.append(resolved)
            else:
                dangling_refs.append({
                    "story_slug": story["slug"],
                    "ref": ref,
                    "role": "covers_legacy",
                })

        story_summaries.append({
            "slug": story["slug"],
            "path": story["path"],
            "author_status": story.get("author_status", ""),
            "canonical_feature_ref": story.get("canonical_feature_ref", ""),
            "canonical_feature_resolved": canonical_resolved,
            "adjacent_count": len(adjacent_resolved),
            "adjacent_resolved": adjacent_resolved,
            "covers_legacy_resolved": legacy_resolved,
            "dangling_ref_count": sum(
                1 for d in dangling_refs if d["story_slug"] == story["slug"]
            ),
        })

    all_referenced = canonical_paths | adjacent_paths | legacy_paths
    adjacent_only_paths = (adjacent_paths | legacy_paths) - canonical_paths
    orphan_paths = set(inventory.keys()) - all_referenced

    def _hydrate(inv_entry: dict) -> dict:
        """Decorate inventory entry with delivery_status hint + leverage score."""
        slug = inv_entry["feature_slug"]
        axis = inv_entry["vision_axis_hint"]
        score = inv_entry["scenario_count"] * VISION_AXIS_WEIGHTS.get(
            axis, DEFAULT_VISION_WEIGHT
        )
        return {
            **inv_entry,
            "delivery_status_from_deliver_manifest": delivery_lookup.get(slug),
            "leverage_score": round(score, 2),
        }

    orphans = sorted(
        (_hydrate(inventory[p]) for p in orphan_paths),
        key=lambda r: (-r["leverage_score"], r["feature_path"]),
    )
    adjacent_only = sorted(
        (_hydrate(inventory[p]) for p in adjacent_only_paths),
        key=lambda r: (-r["leverage_score"], r["feature_path"]),
    )

    return {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "totals": {
            "features_on_disk": len(inventory),
            "features_canonical_in_some_story": len(canonical_paths),
            "features_adjacent_only": len(adjacent_only_paths),
            "features_orphan": len(orphan_paths),
            "stories_total": len(stories),
            "dangling_feature_references": len(dangling_refs),
        },
        "orphans": orphans,
        "adjacent_only": adjacent_only,
        "stories_with_coverage": story_summaries,
        "dangling_references": dangling_refs,
    }


def _load_prev() -> dict | None:
    if PREV_JSON.is_file():
        try:
            return json.loads(PREV_JSON.read_text(encoding="utf-8"))
        except Exception:
            return None
    return None


def _render_md(state: dict, prev: dict | None) -> str:
    lines: list[str] = []
    lines.append(f"# Story-coverage audit — {TODAY.isoformat()}")
    lines.append("")
    lines.append(
        "Coverage projection over `genesis/data/stories/*.md` (frontmatter "
        "`feature:` + `adjacent_features[]`) vs `genesis/a2o/features/**/*.feature` "
        "(filesystem inventory). Single-writer; regenerated by "
        "`story-coverage-audit.py`. See script docstring for storage classification."
    )
    lines.append("")
    lines.append(f"Generated: `{state['generated_at']}`")
    lines.append("")

    t = state["totals"]
    lines.append("## Distribution")
    lines.append("")
    lines.append("| measure | count |")
    lines.append("|---|---:|")
    lines.append(f"| Features on disk | {t['features_on_disk']} |")
    lines.append(f"| Features anchored canonically by ≥1 story | {t['features_canonical_in_some_story']} |")
    lines.append(f"| Features adjacent-only (touched, never anchored) | {t['features_adjacent_only']} |")
    lines.append(f"| **Orphan features (zero coverage)** | **{t['features_orphan']}** |")
    lines.append(f"| Stories total | {t['stories_total']} |")
    lines.append(f"| Dangling feature references (story → missing file) | {t['dangling_feature_references']} |")
    lines.append("")

    # Diff section
    lines.append("## Diff vs prior run")
    lines.append("")
    if prev is None:
        lines.append("_First run — no prior snapshot available._")
        lines.append("")
    else:
        prev_t = prev.get("totals", {})
        diffs: list[str] = []
        for key in ("features_on_disk", "features_canonical_in_some_story",
                    "features_adjacent_only", "features_orphan", "stories_total",
                    "dangling_feature_references"):
            delta = t.get(key, 0) - prev_t.get(key, 0)
            if delta != 0:
                sign = "+" if delta > 0 else ""
                diffs.append(f"| `{key}` | {sign}{delta} |")
        if diffs:
            lines.append("| measure | delta |")
            lines.append("|---|---:|")
            lines.extend(diffs)
            lines.append("")
        else:
            lines.append("_No changes since prior snapshot._")
            lines.append("")

    # Top-10 orphans by leverage
    lines.append("## Top 10 orphan features by leverage")
    lines.append("")
    lines.append(
        "Highest-value storyteller authoring candidates. Leverage = "
        "`scenario_count × vision_axis_weight`. "
        "Lamad/auth/qahal pillars weight up; pure-infra (delivery/resilience) "
        "weight at floor."
    )
    lines.append("")
    if not state["orphans"]:
        lines.append("_No orphan features._")
        lines.append("")
    else:
        lines.append("| rank | feature | scenarios | axis | leverage | /deliver? |")
        lines.append("|---:|---|---:|---|---:|---|")
        for i, o in enumerate(state["orphans"][:10], start=1):
            ds = o.get("delivery_status_from_deliver_manifest") or "—"
            lines.append(
                f"| {i} | `{o['feature_path']}` | {o['scenario_count']} | "
                f"`{o['vision_axis_hint']}` | {o['leverage_score']} | {ds} |"
            )
        lines.append("")

    # Adjacent-only
    lines.append("## Adjacent-only features (no story anchors them canonically)")
    lines.append("")
    if not state["adjacent_only"]:
        lines.append("_No adjacent-only features._")
        lines.append("")
    else:
        lines.append(
            f"**{len(state['adjacent_only'])} features** touched by ≥1 story as "
            "adjacent but never canonical. Candidates for promotion via new "
            "story authoring."
        )
        lines.append("")
        lines.append("| feature | scenarios | axis | leverage |")
        lines.append("|---|---:|---|---:|")
        for o in state["adjacent_only"][:20]:
            lines.append(
                f"| `{o['feature_path']}` | {o['scenario_count']} | "
                f"`{o['vision_axis_hint']}` | {o['leverage_score']} |"
            )
        if len(state["adjacent_only"]) > 20:
            lines.append(f"| _...and {len(state['adjacent_only']) - 20} more_ | | | |")
        lines.append("")

    # Per-story summary
    lines.append("## Per-story coverage summary")
    lines.append("")
    if not state["stories_with_coverage"]:
        lines.append("_No stories on disk._")
        lines.append("")
    else:
        lines.append("| slug | author_status | canonical feature | resolved? | adjacent | dangling |")
        lines.append("|---|---|---|---|---:|---:|")
        for s in state["stories_with_coverage"]:
            resolved_mark = "yes" if s.get("canonical_feature_resolved") else (
                "no" if s.get("canonical_feature_ref") else "n/a"
            )
            canon = s.get("canonical_feature_ref") or "—"
            lines.append(
                f"| `{s['slug']}` | `{s.get('author_status','')}` | "
                f"`{canon}` | {resolved_mark} | {s['adjacent_count']} | "
                f"{s['dangling_ref_count']} |"
            )
        lines.append("")

    # Dangling references
    if state.get("dangling_references"):
        lines.append("## Dangling feature references")
        lines.append("")
        lines.append(
            "Story frontmatter references a feature that does not exist on disk. "
            "Either the feature needs authoring, or the story's reference is "
            "stale. This is the substrate evidence for delivery-debt that the "
            "cartographer should backlog (e.g., `write-<slug>.feature`)."
        )
        lines.append("")
        lines.append("| story | ref | role |")
        lines.append("|---|---|---|")
        for d in state["dangling_references"]:
            lines.append(f"| `{d['story_slug']}` | `{d['ref']}` | `{d['role']}` |")
        lines.append("")

    lines.append("---")
    lines.append("")
    lines.append(
        "_Read-only. The storyteller decides which orphans to anchor with new "
        "stories; the cartographer backlogs feature-authoring tasks for dangling "
        "references. This script never writes to stories or feature files._"
    )
    lines.append("")

    return "\n".join(lines) + "\n"


def main() -> int:
    REPORTS_ROOT.mkdir(parents=True, exist_ok=True)
    TODAY_DIR.mkdir(parents=True, exist_ok=True)

    state = _build_state()

    # Rotation: copy current JSON to .prev.json before overwrite
    prev = None
    if STATE_JSON.is_file():
        try:
            prev = json.loads(STATE_JSON.read_text(encoding="utf-8"))
            PREV_JSON.write_text(
                json.dumps(prev, indent=2) + "\n", encoding="utf-8"
            )
        except Exception:
            prev = None

    md = _render_md(state, prev)

    STATE_JSON.write_text(json.dumps(state, indent=2) + "\n", encoding="utf-8")
    DATED_MD.write_text(md, encoding="utf-8")

    print(f"Wrote {STATE_JSON.relative_to(REPO_ROOT)}")
    print(f"Wrote {DATED_MD.relative_to(REPO_ROOT)}")
    print(f"  features_on_disk:                  {state['totals']['features_on_disk']}")
    print(f"  features_canonical_in_some_story:  {state['totals']['features_canonical_in_some_story']}")
    print(f"  features_adjacent_only:            {state['totals']['features_adjacent_only']}")
    print(f"  features_orphan:                   {state['totals']['features_orphan']}")
    print(f"  stories_total:                     {state['totals']['stories_total']}")
    print(f"  dangling_feature_references:       {state['totals']['dangling_feature_references']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
