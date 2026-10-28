#!/usr/bin/env python3
"""
Absolutes guard — landing signal (the POST half of `absolutes-guard@1`).

PostToolUse hook (matcher: Edit|Write). The PRE half is the `.epr-meta` compose-gate: the
absolutes-guard policy (realized by epr:validator-absolutes-guard, Python-only) fires a `dispatch`
— an advisory background review, never an ask or a deny — when a write NET-NEW introduces an
uncited absolute (always / never / forever / guaranteed / no one can …) into trust-surface prose.
This hook is the POST half, built as the sovereignty guard's (`sovereignty-guard-signal.py`) twin:
when the absolutes LAND, it tells the author each one with its line and the two honest repairs,
logs the landing, and folds it onto a drift measure, so the RULE itself surfaces for review once
landings accumulate.

WHY (operator, 2026-10-15): "'always' is a serious word — it needs to really mean something." The
protocol is graduated, governed, revocable and plural: reach narrows, gates re-admit, stewards act
under a StewardshipGrant for people who hold their rights through others, and trust lags delivery.
An absolute is legitimate when it names the invariant its code ENFORCES.

Detection is the SHARED classifier `_lib.absolutes` over the frame atom `frame-absolute-promise`
(the lexicon and markers are data, read by both halves), so the ledger and the gate can never
disagree on what an absolute is. Net-new only: an Edit is scored on its delta (the pre-edit text
reconstructed from old_string→new_string); a Write has no reliable prior post-hoc and is scored
whole (conservative). Prose only, trust surfaces only — see `_lib/absolutes.py`.

Ledger:   .claude/data/absolutes-guard.jsonl   (one line per landing)
          {ts, path, tool, kind, net_new, hits[{line, col, phrase, text, citation, resolves}],
           frame_ref, classification_cid: null, source: "python", verdict}.
          `classification_cid` stays null: the native host does not run this validator
          (governance-validators.json declares it `python`), and no second DAG-CBOR encoder is
          minted in Python (ruling R-C4 amended).
Drift:    a fold via `epr flow note --kind observation --measure absolutes-landings@1`; the
          running total the escalation reads is DERIVED from the folds by
          absolutes-landings-ceiling@1 (`derive: count-since-reset`), read back with
          `_observation.bound_count` — paid per landing, never per edit. Drain:
          `epr flow note --kind observation --measure absolutes-landings-reset@1 --subject . --value 1`.

Budget 2 s. It never blocks and never fails the tool: any error degrades to silence.

Hook Type: PostToolUse   Matcher: Edit|Write
"""
from __future__ import annotations

import fcntl
import json
import os
import sys
from datetime import datetime, timezone
from pathlib import Path

_here = Path(__file__).resolve()
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        break
    _here = _here.parent
from _lib import absolutes  # noqa: E402  (shared classifier — the frame atom is the source of truth)
from _lib import frame_atoms  # noqa: E402
sys.path.insert(0, str(Path(__file__).resolve().parent))
import _observation as _obs  # noqa: E402  (structured-observation emitter)

_ESCALATE_AT = 5  # landings before the message asks for a rule/corpus drift review
LEDGER_REL = ".claude/data/absolutes-guard.jsonl"
MEASURE = "absolutes-landings@1"
BOUND = "absolutes-landings-ceiling"
RULE_ID = "absolutes-guard"
RETIRE_WHEN = "when the native frame host owns prose extraction and landing observations"


def active_rule_version(repo: Path, rule_id: str) -> str:
    """`<rule_id>@<highest non-superseded version>` — the sovereignty hook's cheap line scan."""
    try:
        best, seen_id, superseded, cur = 0, False, False, 0
        for raw in (repo / ".claude" / "epr-meta" / "policies.yaml").read_text().splitlines():
            line = raw.strip()
            if line.startswith("- id:"):
                if seen_id and not superseded:
                    best = max(best, cur)
                seen_id, superseded, cur = line.split(":", 1)[1].strip() == rule_id, False, 0
            elif seen_id and line.startswith("version:"):
                cur = int(line.split(":", 1)[1].strip() or 0)
            elif seen_id and line.startswith("status:"):
                superseded = line.split(":", 1)[1].strip() == "superseded"
        if seen_id and not superseded:
            best = max(best, cur)
        return f"{rule_id}@{best or 1}"
    except (OSError, ValueError):
        return f"{rule_id}@1"


def append_row(repo: Path, row: dict) -> None:
    try:
        led = repo / LEDGER_REL
        led.parent.mkdir(parents=True, exist_ok=True)
        with led.open("a", encoding="utf-8") as fh:
            fcntl.flock(fh, fcntl.LOCK_EX)
            fh.write(json.dumps(row) + "\n")
    except OSError:
        pass


def main(argv: list[str] | None = None) -> int:
    try:
        data = json.load(sys.stdin)
    except (json.JSONDecodeError, ValueError):
        return 0
    pd = os.environ.get("CLAUDE_PROJECT_DIR")
    ti = data.get("tool_input", {}) or {}
    tool = data.get("tool_name") or ""
    fp = ti.get("file_path") or ""
    if not pd or not fp:
        return 0
    repo = Path(pd).resolve()
    doc = Path(fp) if Path(fp).is_absolute() else repo / fp
    try:
        rel = doc.resolve().relative_to(repo).as_posix()
    except (ValueError, OSError):
        return 0  # outside this repository — not a trust surface we govern
    if absolutes.surface_kind(rel) is None:
        return 0
    try:
        post = doc.read_text(errors="replace")
    except OSError:
        return 0
    if tool == "Edit":
        old, new = ti.get("old_string", ""), ti.get("new_string", "")
        pre = post.replace(new, old) if ti.get("replace_all") else post.replace(new, old, 1)
    else:  # Write — no reliable prior state post-hoc; score the whole file as introduced
        pre = ""
    try:
        found = absolutes.classify(rel, post, pre, repo)
    except (frame_atoms.FrameAtomError, KeyError):
        return 0  # telemetry degrades, it never blocks
    if found is None:
        return 0

    ts = datetime.now(timezone.utc).isoformat(timespec="seconds")
    net_new, hits, frame_ref = found["netNew"], found["hits"], found["frameRef"]
    append_row(repo, {"ts": ts, "path": rel, "tool": tool, "kind": found["kind"],
                      "net_new": net_new, "hits": hits, "frame_ref": frame_ref,
                      "classification_cid": None, "source": "python",
                      "verdict": found["verdict"]})
    _obs.emit(MEASURE, rel, net_new,
              reason="absolutes guard: net-new uncited absolutes landed on a trust surface",
              env={"rule": active_rule_version(repo, RULE_ID), "tool": tool, "frame": frame_ref},
              root=str(repo))
    accumulated = _obs.bound_count(BOUND, root=str(repo), timeout=1)
    total = accumulated if accumulated is not None else 1

    listing = absolutes.describe(hits)
    cites = f"frame {frame_atoms.short_cid(frame_ref)}"
    msg = (f"[absolutes-guard] {net_new} net-new absolute(s) landed in {rel}: {listing}. "
           f"An absolute is a promise; on a trust surface a false one is a trust defect (reach "
           f"narrows, gates re-admit, stewards act under a StewardshipGrant for people who hold "
           f"their rights through others). Two honest repairs: {absolutes.REPAIRS}. ")
    if total >= _ESCALATE_AT:
        msg += (f"{total} landing(s) now aggregated (folds on {MEASURE}; drain with `epr flow note "
                f"--kind observation --measure absolutes-landings-reset@1 --subject . --value 1`) "
                f"— at/over the review threshold. EVALUATE: is the corpus promising what the "
                f"system does not keep, or has the RULE drifted (a lexicon word that keeps "
                f"mis-flagging a legitimate use)? Rewrite the bytes, or refine the lexicon "
                f"(elohim/sdk/schemas/v1/frames/frame-absolute-promise.json) or the rule "
                f"(.claude/epr-meta/policies.yaml → absolutes-guard). {cites}.")
    else:
        msg += f"Landing recorded for drift review. {cites}."
    print(json.dumps({"hookSpecificOutput": {"hookEventName": "PostToolUse",
                                             "additionalContext": msg}}))
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception:  # advisory hook: malformed payloads and telemetry failure cannot fail a tool
        sys.exit(0)
