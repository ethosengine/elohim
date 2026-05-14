#!/usr/bin/env python3
"""
Delivery-status distribution audit — derived projection over story + backlog
frontmatter, regenerable, ephemeral.

## Dual-field reading (stories vs backlog)

The unified delivery-status gradient is one vocabulary used by two artifact
types with different field names:

- **Stories** (`genesis/data/stories/*.md`) — two-axis frontmatter:
  - `status:` is the **author-status axis** (draft / canonical / superseded / retired)
  - `delivery_status:` is the **delivery-status axis** (the gradient enum)
- **Backlog** (`genesis/data/timeline/backlog/*.md`) — single-axis frontmatter:
  - `status:` carries the **delivery-status gradient directly** (per Timeline
    CONVENTIONS.md Run #2 extension, 2026-05-14). Backlog entries have no
    separate `delivery_status:` field; the unified gradient and author-status
    collapse into one field here.

This script reads the appropriate field per artifact type. Legacy backlog
values (`proposed`/`ready`/`in-progress`/`completed`/`closed`) are translated
through the Timeline CONVENTIONS.md migration map; unknown values are flagged
as anomalies.

## Storage classification (per plan `stateless-seeking-forest.md`)

The two outputs are LOCAL OPERATOR-TIER ARTIFACTS, not P2P substrate entities.

| Artifact | Class | Writer | Reader(s) | Source of truth |
|---|---|---|---|---|
| `.claude/memory-kit/delivery-status-distribution.json` | Derived projection (regenerable) | this script only (single-writer) | `/deliver` (pickup queue), memory-ceremony Wave 0/1 | Story + backlog frontmatter; this JSON is a snapshot of them |
| `.claude/memory-kit/<date>/delivery-status-distribution.md` | Dated human-readable report | this script only | Human operator + memory-ceremony Wave 0/1 | Same JSON projection above |

Neither is DHT-bound. Neither requires entity-class classification (A/A2/B/B2/C).
Neither has wire contracts, sync messages, or peer-replication shape. They are
the same tier as existing `.claude/memory-kit/claude-md-drift.json` (accumulator
state) and the dated audit reports in `.claude/memory-kit/<date>/`. The P2P
design gate's intent ("catch tables, models, routes, sync messages before they
become peer-divergent") does not apply: these are single-writer or
single-source-projection artifacts that the harness cleans up routinely.

## Consumers

- `/deliver` reads the JSON's `pickup_queue_for_deliver` array to find work
- Memory-ceremony Wave 0/1 reads the dated MD for distribution + diff vs prior run

## Authority boundary

This script READS frontmatter; it never writes to story/backlog frontmatter.
`/deliver` is the only authority for `active.*`/`stable`/`regression` states;
this script merely surfaces what frontmatter currently declares. The
`pickup_queue_for_deliver` is a recommendation, not a verdict.

## Idempotency

Running twice in the same day produces zero diff in the MD report's diff
section. Rotation: before write, copies current JSON to `.prev.json` so the
next run can produce a state-diff section.

## Exit code

Always 0 — observation, not gating.

Outputs:
  - `.claude/memory-kit/delivery-status-distribution.json` (rolling state)
  - `.claude/memory-kit/<YYYY-MM-DD>/delivery-status-distribution.md` (report)
"""
from __future__ import annotations

import json
import os
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

STATE_JSON = REPORTS_ROOT / "delivery-status-distribution.json"
PREV_JSON = REPORTS_ROOT / "delivery-status-distribution.prev.json"
DATED_MD = TODAY_DIR / "delivery-status-distribution.md"

STORIES_DIR = REPO_ROOT / "genesis" / "data" / "stories"
BACKLOG_DIR = REPO_ROOT / "genesis" / "data" / "timeline" / "backlog"
FEATURES_DIR = REPO_ROOT / "genesis" / "a2o" / "features"

# Canonical gradient order — most-delivered → least-delivered; regression sideways at end
GRADIENT_ORDER = [
    "stable",
    "active.latest-stable",
    "active.beta",
    "active.alpha",
    "wip",
    "refined",
    "backlog",
    "envisioned",
    "unknown",
    "undelivered",
    "regression",
]

ACTIVE_OR_STABLE = {"active.alpha", "active.beta", "active.latest-stable", "stable"}

# Non-story scaffolding files in genesis/data/stories/
STORY_SCAFFOLDING = {"CONVENTIONS.md", "INDEX.md", "README.md"}

# Backlog legacy-status migration map (per Timeline CONVENTIONS.md Run #2 extension,
# 2026-05-14). Pre-extension backlog entries used these enum values; they translate
# into the unified delivery-status gradient. The script applies this translation
# transparently and flags the original value via `legacy_status_translated`.
BACKLOG_LEGACY_TO_GRADIENT = {
    "proposed": "backlog",
    "ready": "refined",
    "in-progress": "wip",
    "in-shift": "wip",
    "completed": "active.alpha",  # /deliver may bump higher post-judgment
    "closed": "stable",
}


def _feature_file_exists(feature: str) -> bool:
    """Check whether a declared feature has a matching .feature file under
    genesis/a2o/features/. The frontmatter `feature:` field is a slug
    (e.g., `stewarded-device-sync`); we search for `<slug>.feature` anywhere
    under the features tree."""
    if not feature or not FEATURES_DIR.is_dir():
        return False
    target = f"{feature}.feature"
    for p in FEATURES_DIR.rglob("*.feature"):
        if p.name == target:
            return True
    return False


_INLINE_COMMENT_RE = re.compile(r'(?<!\\)\s+#.*$')


def _clean(val: str) -> str:
    """Strip trailing inline YAML comment and surrounding quotes.

    The shared `_lib.frontmatter` parser uses a simple `key: value` regex
    that captures everything after the colon. Inline comments (`# ...`) are
    not stripped. We do that here for every scalar we read."""
    if not isinstance(val, str):
        return val
    cleaned = _INLINE_COMMENT_RE.sub('', val).strip()
    # frontmatter.parse already strips one pair of surrounding quotes;
    # strip again in case quotes appear inside the cleaned scalar.
    return cleaned.strip('"').strip("'").strip()


def _scan_dir(directory: Path, scaffolding: set[str], kind: str) -> list[dict]:
    """Scan a directory for *.md files and extract delivery-status state.

    Field resolution differs by `kind`:
    - `kind="story"`: read `delivery_status:` (separate from `status:` author axis)
    - `kind="backlog"`: read `status:` (single axis; backlog has no `delivery_status:`)
      and apply BACKLOG_LEGACY_TO_GRADIENT translation for pre-Run-2 enum values.

    Returns a list of dicts with the canonical schema for the output JSON.
    Files missing a usable status get `delivery_status: "unknown"` in the
    report (but a `frontmatter_anomaly` flag is set)."""
    rows: list[dict] = []
    if not directory.is_dir():
        return rows
    for md in sorted(directory.glob("*.md")):
        if md.name in scaffolding:
            continue
        try:
            fm = _fm.parse_file(md)
        except Exception as e:
            rows.append({
                "slug": md.stem,
                "path": str(md),
                "frontmatter_anomaly": f"parse_error: {e}",
                "author_status": "",
                "delivery_status": "unknown",
                "delivery_status_updated": "",
                "delivery_status_source": "",
                "declared_feature": "",
                "feature_file_exists": False,
            })
            continue

        slug = _clean(fm.get("slug")) or md.stem
        status_field = _clean(fm.get("status"))
        delivery_status_field = _clean(fm.get("delivery_status"))
        delivery_status_updated = _clean(fm.get("delivery_status_updated"))
        delivery_status_source = _clean(fm.get("delivery_status_source"))
        feature = _clean(fm.get("feature"))

        anomaly = ""
        legacy_translated_from = ""

        if kind == "backlog":
            # Backlog: status: IS the delivery-status gradient. There is no
            # separate author-status axis on backlog entries.
            raw = status_field
            author_status = ""  # backlog has no author-status field
            if not raw:
                anomaly = "missing_delivery_status"
                delivery_status = "unknown"
            elif raw in GRADIENT_ORDER:
                delivery_status = raw
            elif raw in BACKLOG_LEGACY_TO_GRADIENT:
                delivery_status = BACKLOG_LEGACY_TO_GRADIENT[raw]
                legacy_translated_from = raw
            else:
                anomaly = f"unknown_delivery_status_value:{raw}"
                delivery_status = "unknown"
        else:
            # Story: read delivery_status: directly; status: is author-status
            author_status = status_field
            delivery_status = delivery_status_field
            if not delivery_status:
                anomaly = "missing_delivery_status"
                delivery_status = "unknown"
            elif delivery_status not in GRADIENT_ORDER:
                anomaly = f"unknown_delivery_status_value:{delivery_status}"

        row = {
            "slug": slug,
            "path": str(md),
            "author_status": author_status,
            "delivery_status": delivery_status,
            "delivery_status_updated": delivery_status_updated,
            "delivery_status_source": delivery_status_source,
            "declared_feature": feature,
            "feature_file_exists": _feature_file_exists(feature) if feature else False,
        }
        if legacy_translated_from:
            row["legacy_status_translated_from"] = legacy_translated_from
        if anomaly:
            row["frontmatter_anomaly"] = anomaly
        rows.append(row)
    return rows


def _compute_totals(rows: list[dict]) -> dict[str, int]:
    """Tally rows by delivery_status. All gradient values are reported,
    even when zero, to keep the schema stable across runs."""
    totals = {state: 0 for state in GRADIENT_ORDER}
    for r in rows:
        ds = r.get("delivery_status", "unknown")
        if ds in totals:
            totals[ds] += 1
        else:
            totals.setdefault(ds, 0)
            totals[ds] += 1
    return totals


def _pickup_queue(stories: list[dict]) -> list[dict]:
    """Pickup-queue criterion (per plan): canonical stories with
    delivery_status NOT IN (active.*, stable). These need `/deliver` attention.

    Regression is intentionally INCLUDED — a regression should be re-delivered
    by /deliver to recover."""
    queue: list[dict] = []
    for s in stories:
        if s.get("author_status") != "canonical":
            continue
        ds = s.get("delivery_status", "unknown")
        if ds in ACTIVE_OR_STABLE:
            continue
        reason_bits: list[str] = [f"delivery_status={ds}"]
        feature = s.get("declared_feature")
        if feature and not s.get("feature_file_exists"):
            reason_bits.append(f"feature_file_missing:{feature}.feature")
        elif feature:
            reason_bits.append(f"feature_file_present:{feature}.feature")
        queue.append({"slug": s["slug"], "reason": " ; ".join(reason_bits)})
    return queue


def _load_prev() -> dict | None:
    if PREV_JSON.is_file():
        try:
            return json.loads(PREV_JSON.read_text(encoding="utf-8"))
        except Exception:
            return None
    return None


def _build_state(stories: list[dict], backlog: list[dict]) -> dict:
    totals = _compute_totals(stories + backlog)
    return {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "totals": totals,
        "stories": stories,
        "backlog": backlog,
        "pickup_queue_for_deliver": _pickup_queue(stories),
    }


def _diff_states(prev: dict | None, curr: dict) -> dict:
    """Compute a structured diff between two state dicts.

    Returns:
        {
          "first_run": bool,
          "totals_delta": {state: delta_int} (only states that changed),
          "story_moves": [{slug, from, to}],
          "backlog_moves": [{slug, from, to}],
          "stories_added": [slug],
          "stories_removed": [slug],
          "backlog_added": [slug],
          "backlog_removed": [slug],
        }
    """
    if prev is None:
        return {"first_run": True}

    diff: dict = {
        "first_run": False,
        "totals_delta": {},
        "story_moves": [],
        "backlog_moves": [],
        "stories_added": [],
        "stories_removed": [],
        "backlog_added": [],
        "backlog_removed": [],
    }

    prev_totals = prev.get("totals", {})
    curr_totals = curr.get("totals", {})
    all_states = set(prev_totals) | set(curr_totals)
    for s in all_states:
        delta = curr_totals.get(s, 0) - prev_totals.get(s, 0)
        if delta != 0:
            diff["totals_delta"][s] = delta

    def _state_by_slug(rows: list[dict]) -> dict[str, str]:
        return {r["slug"]: r.get("delivery_status", "unknown") for r in rows}

    prev_stories = _state_by_slug(prev.get("stories", []))
    curr_stories = _state_by_slug(curr.get("stories", []))
    for slug, ds in curr_stories.items():
        if slug not in prev_stories:
            diff["stories_added"].append(slug)
        elif prev_stories[slug] != ds:
            diff["story_moves"].append(
                {"slug": slug, "from": prev_stories[slug], "to": ds}
            )
    for slug in prev_stories:
        if slug not in curr_stories:
            diff["stories_removed"].append(slug)

    prev_backlog = _state_by_slug(prev.get("backlog", []))
    curr_backlog = _state_by_slug(curr.get("backlog", []))
    for slug, ds in curr_backlog.items():
        if slug not in prev_backlog:
            diff["backlog_added"].append(slug)
        elif prev_backlog[slug] != ds:
            diff["backlog_moves"].append(
                {"slug": slug, "from": prev_backlog[slug], "to": ds}
            )
    for slug in prev_backlog:
        if slug not in curr_backlog:
            diff["backlog_removed"].append(slug)

    return diff


def _render_md(state: dict, diff: dict) -> str:
    lines: list[str] = []
    lines.append(f"# Delivery-status distribution — {TODAY.isoformat()}")
    lines.append("")
    lines.append(
        f"Snapshot of `delivery_status:` across `genesis/data/stories/*.md` and "
        f"`genesis/data/timeline/backlog/*.md`. Single-writer projection; "
        f"regenerated by `delivery-status-distribution.py`. See script docstring "
        f"for storage-classification details."
    )
    lines.append("")
    lines.append(f"Generated: `{state['generated_at']}`")
    lines.append("")

    # Distribution table
    lines.append("## Distribution")
    lines.append("")
    lines.append("| state | count |")
    lines.append("|---|---|")
    for st in GRADIENT_ORDER:
        lines.append(f"| `{st}` | {state['totals'].get(st, 0)} |")
    # Catch any extra states not in gradient (anomalies)
    extra = sorted(set(state["totals"]) - set(GRADIENT_ORDER))
    for st in extra:
        lines.append(f"| `{st}` (anomalous) | {state['totals'][st]} |")
    lines.append("")

    # Diff section
    lines.append("## Diff vs prior run")
    lines.append("")
    if diff.get("first_run"):
        lines.append("_First run — no prior snapshot available._")
        lines.append("")
    else:
        td = diff.get("totals_delta", {})
        if not td and not diff.get("story_moves") and not diff.get("backlog_moves") \
                and not diff.get("stories_added") and not diff.get("stories_removed") \
                and not diff.get("backlog_added") and not diff.get("backlog_removed"):
            lines.append("_No changes since prior snapshot._")
            lines.append("")
        else:
            if td:
                lines.append("### Totals delta")
                lines.append("")
                lines.append("| state | delta |")
                lines.append("|---|---|")
                for st in sorted(td):
                    sign = "+" if td[st] > 0 else ""
                    lines.append(f"| `{st}` | {sign}{td[st]} |")
                lines.append("")
            if diff.get("story_moves"):
                lines.append("### Stories that moved states")
                lines.append("")
                for m in diff["story_moves"]:
                    lines.append(f"- `{m['slug']}` : `{m['from']}` → `{m['to']}`")
                lines.append("")
            if diff.get("backlog_moves"):
                lines.append("### Backlog items that moved states")
                lines.append("")
                for m in diff["backlog_moves"]:
                    lines.append(f"- `{m['slug']}` : `{m['from']}` → `{m['to']}`")
                lines.append("")
            if diff.get("stories_added"):
                lines.append("### Stories added")
                lines.append("")
                for s in diff["stories_added"]:
                    lines.append(f"- `{s}`")
                lines.append("")
            if diff.get("stories_removed"):
                lines.append("### Stories removed")
                lines.append("")
                for s in diff["stories_removed"]:
                    lines.append(f"- `{s}`")
                lines.append("")
            if diff.get("backlog_added"):
                lines.append("### Backlog added")
                lines.append("")
                for s in diff["backlog_added"]:
                    lines.append(f"- `{s}`")
                lines.append("")
            if diff.get("backlog_removed"):
                lines.append("### Backlog removed")
                lines.append("")
                for s in diff["backlog_removed"]:
                    lines.append(f"- `{s}`")
                lines.append("")

    # Pickup queue
    lines.append("## Pickup queue for `/deliver`")
    lines.append("")
    queue = state.get("pickup_queue_for_deliver", [])
    if not queue:
        lines.append("_Empty — every canonical story is at `active.*`/`stable`._")
        lines.append("")
    else:
        lines.append(f"Canonical stories with `delivery_status` not in "
                     f"(`active.*`, `stable`). **{len(queue)} item(s).**")
        lines.append("")
        for q in queue:
            lines.append(f"- `{q['slug']}` — {q['reason']}")
        lines.append("")

    # Anomalies
    anomalies: list[tuple[str, str, str]] = []  # (kind, slug, detail)
    for r in state.get("stories", []) + state.get("backlog", []):
        if "frontmatter_anomaly" in r:
            anomalies.append(("frontmatter", r["slug"], r["frontmatter_anomaly"]))
    if anomalies:
        lines.append("## Anomalies")
        lines.append("")
        for kind, slug, detail in anomalies:
            lines.append(f"- [{kind}] `{slug}` — {detail}")
        lines.append("")

    # Story table
    lines.append("## Stories — full table")
    lines.append("")
    lines.append("| slug | author_status | delivery_status | declared_feature | feature_file? |")
    lines.append("|---|---|---|---|---|")
    for r in state.get("stories", []):
        feat = r.get("declared_feature") or ""
        feat_ok = "yes" if r.get("feature_file_exists") else ("no" if feat else "n/a")
        lines.append(
            f"| `{r['slug']}` | `{r.get('author_status','')}` | "
            f"`{r.get('delivery_status','')}` | `{feat}` | {feat_ok} |"
        )
    lines.append("")

    # Backlog table
    lines.append("## Backlog — full table")
    lines.append("")
    lines.append("| slug | author_status | delivery_status |")
    lines.append("|---|---|---|")
    for r in state.get("backlog", []):
        lines.append(
            f"| `{r['slug']}` | `{r.get('author_status','')}` | "
            f"`{r.get('delivery_status','')}` |"
        )
    lines.append("")

    return "\n".join(lines) + "\n"


def main() -> int:
    REPORTS_ROOT.mkdir(parents=True, exist_ok=True)
    TODAY_DIR.mkdir(parents=True, exist_ok=True)

    stories = _scan_dir(STORIES_DIR, STORY_SCAFFOLDING, kind="story")
    backlog = _scan_dir(BACKLOG_DIR, set(), kind="backlog")

    state = _build_state(stories, backlog)

    # Rotation: copy current JSON to .prev.json before overwrite
    prev = _load_prev()
    if STATE_JSON.is_file():
        # The current STATE_JSON becomes the prev for the NEXT run; load it now
        # so the CURRENT run's diff is against the previous snapshot.
        try:
            current_on_disk = json.loads(STATE_JSON.read_text(encoding="utf-8"))
            # Promote current-on-disk to prev for this run's diff
            prev = current_on_disk
            PREV_JSON.write_text(
                json.dumps(current_on_disk, indent=2) + "\n", encoding="utf-8"
            )
        except Exception:
            pass

    diff = _diff_states(prev, state)
    md = _render_md(state, diff)

    STATE_JSON.write_text(json.dumps(state, indent=2) + "\n", encoding="utf-8")
    DATED_MD.write_text(md, encoding="utf-8")

    print(f"Wrote {STATE_JSON.relative_to(REPO_ROOT)}")
    print(f"Wrote {DATED_MD.relative_to(REPO_ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
