#!/usr/bin/env python3
"""saga-status.py — emit the resiliency-saga headline from the ten dataplane chapters.

The resiliency-saga (genesis/a2o/features/dataplane/resiliency-saga/NN-*.feature) narrates
data surviving node loss as ten ordered chapters, each tagged `@concern:saga-NN-<name>`. This
script joins three deterministic, read-only sources into ONE line every session can open with
(sibling to habits-status.py's delivery-habits headline — same posture: never throw, degrade on
any missing input, no network):

  1. CHAPTERS — the feature files themselves (always present; the ordering ground truth).
  2. FLOWS (optional) — .eprfs/status/flows.jsonl, the per-checkout EPR-REA sidecar. `epr flow
     project` mints one `a2o:scenario-green` Commitment per chapter; `epr flow fulfill` turns a
     green/red sprint-report into Produce (fulfilling)/Dismiss (regression) FlowEvents.
  3. REPORT (optional) — genesis/a2o/reports/sprint-report-dataplane.json, the latest dataplane
     validation run's summary.byConcern rollup (gitignored; may be absent locally — CI-only
     until `genesis/scripts/jenkins-sync.sh` pulls it down).

Usage:
  saga-status.py            one line for the SessionStart context block
  saga-status.py --full     table: every chapter, concern, flows-state, report-verdict, final

Deterministic given its inputs; degrades gracefully (missing flows.jsonl / report / unparseable
recipes.yaml / malformed feature tags are all NORMAL, not errors). Exit 0 always.

--- Flows association notes (read this before touching the matching logic) ---

A Commitment's identity never carries which recipe minted it (`resiliency-saga` and the broader
`elohim-dev-pipeline` recipe both glob over these same feature paths, and the resulting Commitment
atoms DEDUPE to the same CID — verified empirically against an isolated fixture repo, 2026-07-25;
`classified_as` is `["a2o:scenario-green", <repo-relative-path>]`, nothing more). So chapters are
matched to commitments purely by path, never by recipe id — see docs/eprfs-walk-recipe-id backlog.

A Commitment's top-level `cid` renders as a base32 MULTIBASE STRING (e.g. `bafyrei...`); a
FlowEvent's `fulfills`/`resource` fields render the SAME kind of CID as a raw JSON BYTE ARRAY
(`[1, 113, 18, 32, ...]` — CIDv1, varint codec, multihash). Per this repo's convention (see
`.claude/scripts/_lib/cite_graph.py`'s `fingerprint_text` docstring: "Python NEVER encodes a
CID"), this script only ever DECODES the base32 string form back to bytes for an equality
compare — it never encodes/derives a CID from file content. `_decode_cid_b32` below is that one
decode, verified byte-for-byte against a real `epr flow fulfill` run in an isolated fixture repo
(commitment cid decoded == the `fulfills[0]` byte array emitted for the SAME commitment).

Dismiss events carry an EMPTY `fulfills` (fulfill.rs: "Dismiss events carry an empty fulfills, so
only a prior Produce fulfillment ever discharges a commitment"). To associate a Dismiss with its
chapter anyway, this script learns the chapter's `resource` value (the feature file's body CID,
in byte-array form) from any Produce event that fulfills its commitment, then matches Dismiss
events by `resource` equality — verified: a Dismiss's `resource` is byte-identical to the
`resource` of the Produce event fulfilling the SAME commitment (fulfill.rs computes both from the
same `body_cid_of_file(commit_path)` call). A chapter that regresses WITHOUT ever having been
fulfilled first has no learned resource to match against — by construction this can't produce a
false "regressed" (the `fulfilled` gate already requires a prior Produce), so the gap is inert,
not silently wrong.
"""
from __future__ import annotations

import base64
import json
import re
import sys
from pathlib import Path

SCRIPT = Path(__file__).resolve()


def find_repo_root() -> Path:
    """Walk up from this script looking for a directory that looks like the repo root
    (has .git AND genesis/); fall back to the fixed depth habits-status.py also assumes
    (.claude/scripts/<this file> -> repo root is two parents up)."""
    for anc in [SCRIPT] + list(SCRIPT.parents):
        if (anc / ".git").exists() and (anc / "genesis").is_dir():
            return anc
    return SCRIPT.parents[2]


ROOT = find_repo_root()
SAGA_DIR = ROOT / "genesis" / "a2o" / "features" / "dataplane" / "resiliency-saga"
FLOWS_PATH = ROOT / ".eprfs" / "status" / "flows.jsonl"
REPORT_PATH = ROOT / "genesis" / "a2o" / "reports" / "sprint-report-dataplane.json"

CONCERN_RE = re.compile(r"@concern:(saga-[a-z0-9-]+)")
CHAPTER_NUM_RE = re.compile(r"^(\d{2})-(.+)$")
SAGA_CONCERN_NAME_RE = re.compile(r"^saga-\d{2}-(.+)$")

STATE_ORDER = ("green", "emit-due", "pending-env", "regressed", "red", "unprojected")


# ---------------------------------------------------------------------------
# CID decode (decode only — never encode; see module docstring)
# ---------------------------------------------------------------------------

def _decode_cid_b32(cid_str: str) -> tuple | None:
    """Decode a multibase-base32-lower CID string (`b...`) to its raw byte tuple, the
    same representation FlowEvent `fulfills`/`resource` arrays use. Returns None on any
    malformed input — never raises."""
    try:
        if not cid_str or not cid_str.startswith("b"):
            return None
        payload = cid_str[1:].upper()
        pad = "=" * (-len(payload) % 8)
        return tuple(base64.b32decode(payload + pad))
    except Exception:
        return None


# ---------------------------------------------------------------------------
# Chapters — the ground truth, always available
# ---------------------------------------------------------------------------

class Chapter:
    __slots__ = ("num", "path", "rel_path", "concern", "name")

    def __init__(self, num, path, rel_path, concern, name):
        self.num = num
        self.path = path
        self.rel_path = rel_path
        self.concern = concern
        self.name = name


def load_chapters() -> list:
    chapters = []
    if not SAGA_DIR.is_dir():
        return chapters
    try:
        files = sorted(SAGA_DIR.glob("[0-9][0-9]-*.feature"))
    except Exception:
        return chapters
    for f in files:
        m = CHAPTER_NUM_RE.match(f.stem)
        num = m.group(1) if m else "??"
        try:
            text = f.read_text(encoding="utf-8", errors="replace")
        except OSError:
            text = ""
        cm = CONCERN_RE.search(text)
        concern = cm.group(1) if cm else None
        name = None
        if concern:
            nm = SAGA_CONCERN_NAME_RE.match(concern)
            name = nm.group(1) if nm else concern
        if not name:
            name = m.group(2) if m else f.stem
        try:
            rel_path = str(f.relative_to(ROOT))
        except ValueError:
            rel_path = str(f)
        chapters.append(Chapter(num, f, rel_path, concern, name))
    return chapters


# ---------------------------------------------------------------------------
# Flows sidecar — optional
# ---------------------------------------------------------------------------

def load_flows() -> list:
    """Every parsed line from flows.jsonl, or [] if absent/unreadable. Never raises."""
    if not FLOWS_PATH.exists():
        return []
    lines = []
    try:
        with FLOWS_PATH.open(encoding="utf-8", errors="replace") as fh:
            for line in fh:
                line = line.strip()
                if not line:
                    continue
                try:
                    lines.append(json.loads(line))
                except (json.JSONDecodeError, ValueError):
                    continue
    except OSError:
        return []
    return lines


def index_commitments(records: list) -> dict:
    """{ repo-relative feature path -> decoded commitment cid (byte tuple) } for every
    Active `a2o:scenario-green` commitment. Ambiguous duplicate paths keep the first
    seen (never throws)."""
    out = {}
    for row in records:
        try:
            rec = row.get("record", {})
            if rec.get("kind") != "commitment":
                continue
            if str(rec.get("state", "")).lower() != "active":
                continue
            classified = (rec.get("resourceSpec") or {}).get("classifiedAs") or []
            if not classified or classified[0] != "a2o:scenario-green":
                continue
            path = classified[1] if len(classified) > 1 else None
            if not path:
                continue
            decoded = _decode_cid_b32(row.get("cid", ""))
            if decoded is None:
                continue
            out.setdefault(path, decoded)
        except Exception:
            continue
    return out


def _as_tuple(value):
    try:
        return tuple(value)
    except Exception:
        return None


def index_flow_state(records: list, commit_cid) -> dict:
    """For one chapter's commitment cid (byte tuple), return:
      { fulfilled: bool, latest_is_dismiss: bool }
    by scanning every `event` record for Produce events fulfilling this commitment and
    Dismiss events sharing the same (learned) `resource`. See module docstring for the
    resource-matching rationale."""
    produce_ts = []
    dismiss_ts = []
    resource_known = None

    for row in records:
        rec = row.get("record", {})
        if rec.get("kind") != "event":
            continue
        action = rec.get("action")
        occurred_at = rec.get("occurredAt")
        if action == "produce":
            fulfills = rec.get("fulfills") or []
            for f in fulfills:
                if _as_tuple(f) == commit_cid:
                    produce_ts.append(occurred_at)
                    if resource_known is None:
                        resource_known = _as_tuple(rec.get("resource"))
                    break

    if resource_known is not None:
        for row in records:
            rec = row.get("record", {})
            if rec.get("kind") != "event" or rec.get("action") != "dismiss":
                continue
            if _as_tuple(rec.get("resource")) == resource_known:
                dismiss_ts.append(rec.get("occurredAt"))

    fulfilled = len(produce_ts) > 0
    latest_is_dismiss = False
    if fulfilled:
        tagged = [(t, "produce") for t in produce_ts] + [(t, "dismiss") for t in dismiss_ts]
        tagged_sortable = [(_sort_key(t), kind) for (t, kind) in tagged if t]
        if tagged_sortable:
            tagged_sortable.sort(key=lambda x: x[0])
            latest_is_dismiss = tagged_sortable[-1][1] == "dismiss"
    return {"fulfilled": fulfilled, "latest_is_dismiss": latest_is_dismiss}


def _sort_key(occurred_at: str):
    """Best-effort orderable key for an RFC3339 timestamp; falls back to the raw string
    (still monotonic for same-precision UTC 'Z' timestamps) rather than raising."""
    try:
        from datetime import datetime
        return datetime.fromisoformat(str(occurred_at).replace("Z", "+00:00"))
    except Exception:
        return str(occurred_at)


# ---------------------------------------------------------------------------
# Report — optional
# ---------------------------------------------------------------------------

def load_report():
    if not REPORT_PATH.exists():
        return None
    try:
        return json.loads(REPORT_PATH.read_text(encoding="utf-8", errors="replace"))
    except (OSError, json.JSONDecodeError, ValueError):
        return None


def report_verdict(rollup):
    """green / red / pending-env / None (absent or no assertion) — per the formula in
    fulfill.rs: all-green = failed==0 && pending==0 && passed>0; red = failed>0."""
    if not rollup:
        return None
    try:
        passed = int(rollup.get("passed", 0))
        failed = int(rollup.get("failed", 0))
        pending = int(rollup.get("pending", 0))
    except (TypeError, ValueError):
        return None
    if failed > 0:
        return "red"
    if pending > 0:
        return "pending-env"
    if passed > 0:
        return "green"
    return None


# ---------------------------------------------------------------------------
# Combine — per-chapter final state
# ---------------------------------------------------------------------------

def combine_state(has_commitment: bool, fulfilled: bool, latest_is_dismiss: bool, verdict) -> str:
    """Precedence (first match wins) — see module docstring + T6 spec:
      1. no commitment                                  -> unprojected
      2. fulfilled AND (verdict==red OR latest dismiss)  -> regressed
      3. fulfilled AND (verdict==green OR verdict absent) -> green
      4. NOT fulfilled AND verdict==green                -> emit-due
      5. verdict==red                                    -> red
      6. verdict==pending-env                            -> pending-env
      7. verdict absent AND NOT fulfilled                 -> red (unproven)
    """
    if not has_commitment:
        return "unprojected"
    if fulfilled and (verdict == "red" or latest_is_dismiss):
        return "regressed"
    if fulfilled and verdict in ("green", None):
        return "green"
    if (not fulfilled) and verdict == "green":
        return "emit-due"
    if verdict == "red":
        return "red"
    if verdict == "pending-env":
        return "pending-env"
    if verdict is None and not fulfilled:
        return "red"
    return "red"  # defensive: unreached by the 8 (fulfilled x verdict) combos above


# ---------------------------------------------------------------------------
# Assemble
# ---------------------------------------------------------------------------

def assemble():
    chapters = load_chapters()
    flow_records = load_flows()
    commitments = index_commitments(flow_records) if flow_records else {}
    report = load_report()
    by_concern = ((report or {}).get("summary") or {}).get("byConcern") or {}

    rows = []
    for ch in chapters:
        commit_cid = commitments.get(ch.rel_path)
        has_commitment = commit_cid is not None
        if has_commitment:
            flow_state = index_flow_state(flow_records, commit_cid)
        else:
            flow_state = {"fulfilled": False, "latest_is_dismiss": False}
        rollup = by_concern.get(ch.concern) if ch.concern else None
        verdict = report_verdict(rollup)
        state = combine_state(has_commitment, flow_state["fulfilled"], flow_state["latest_is_dismiss"], verdict)
        rows.append({
            "chapter": ch,
            "has_commitment": has_commitment,
            "fulfilled": flow_state["fulfilled"],
            "latest_is_dismiss": flow_state["latest_is_dismiss"],
            "verdict": verdict,
            "state": state,
        })
    return rows, report, bool(flow_records)


# ---------------------------------------------------------------------------
# Render
# ---------------------------------------------------------------------------

def headline(rows, report, flows_present) -> str:
    if not rows:
        return ""
    n = len(rows)
    green_rows = [r for r in rows if r["state"] == "green"]
    g = len(green_rows)

    frontier = next((r for r in rows if r["state"] != "green"), None)

    counts = {}
    for r in rows:
        counts[r["state"]] = counts.get(r["state"], 0) + 1

    if frontier is None:
        head = f"SAGA resiliency: {g}/{n} green — all chapters green"
    else:
        ch = frontier["chapter"]
        head = f"SAGA resiliency: {g}/{n} green · frontier: ch{ch.num} {ch.name} ({frontier['state']})"

    extras = []
    for state in ("pending-env", "emit-due", "regressed", "unprojected", "red"):
        k = counts.get(state, 0)
        # Don't double-report the frontier's own bucket count of 1 as noise when it's
        # the only one — still show it; the count is informative even at k=1 for
        # states other than the frontier's, and harmless (if noisy) when it matches.
        if k > 0:
            extras.append(f"{k} {state}")
    line = head
    if extras:
        line += " · " + " · ".join(extras)
    if not report:
        line += " · (no report)"
    return line


def full(rows, report, flows_present) -> str:
    lines = [f"Resiliency saga — {SAGA_DIR}", ""]
    if report:
        lines.append(f"report: runId={report.get('runId', '?')} generatedAt={report.get('generatedAt', '?')}")
    else:
        lines.append(f"report: (absent — {REPORT_PATH} not found locally; run genesis/scripts/jenkins-sync.sh)")
    lines.append(f"flows:  {'present' if flows_present else '(absent — ' + str(FLOWS_PATH) + ' not found)'}")
    lines.append("")

    mark = {
        "green": "✅", "emit-due": "\U0001f7e1", "regressed": "\U0001f7e0",
        "red": "\U0001f534", "pending-env": "⚪", "unprojected": "▫️",
    }
    header = f"{'ch':<4} {'concern':<28} {'flows':<10} {'report':<12} {'final':<12}"
    lines.append(header)
    lines.append("-" * len(header))
    for r in rows:
        ch = r["chapter"]
        flows_state = "unprojected" if not r["has_commitment"] else (
            "regressed" if r["latest_is_dismiss"] and r["fulfilled"] else
            ("fulfilled" if r["fulfilled"] else "unfulfilled")
        )
        report_state = r["verdict"] or "absent"
        m = mark.get(r["state"], "?")
        lines.append(
            f"ch{ch.num:<2} {(ch.concern or '(no @concern tag)'):<28} {flows_state:<10} "
            f"{report_state:<12} {m} {r['state']}"
        )

    lines.append("")
    counts = {}
    for r in rows:
        counts[r["state"]] = counts.get(r["state"], 0) + 1
    n = len(rows)
    g = counts.get("green", 0)
    lines.append(f"{g}/{n} green — " + ", ".join(f"{v} {k}" for k, v in counts.items() if k != "green" and v))
    return "\n".join(lines)


def main() -> int:
    try:
        rows, report, flows_present = assemble()
    except Exception:
        return 0  # never break session start / any caller over a bad input
    try:
        if "--full" in sys.argv:
            out = full(rows, report, flows_present)
        else:
            out = headline(rows, report, flows_present)
        if out:
            print(out)
    except Exception:
        return 0
    return 0


if __name__ == "__main__":
    sys.exit(main())
