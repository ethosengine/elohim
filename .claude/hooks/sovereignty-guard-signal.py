#!/usr/bin/env python3
"""
Sovereignty ontology guard — dismissal-aggregation signal (the closed governance loop).

PostToolUse hook (matcher: Edit|Write). The PRE half is the `.epr-meta` compose-gate: the
sovereignty-ontology-guard policy (realized by epr:validator-sovereignty-ontology-guard) fires an
`ask` when a write NET-NEW introduces apex-assertion sovereignty framing into a governed doc. That
`ask` is a GUARD, not a ban — one keystroke proceeds. This hook is the POST half: when apex framing
LANDS anyway (the guard was dismissed, or the doc sits outside the cascade), it logs the firing and
aggregates the landings, so the RULE itself surfaces for evaluation/drift-review. This is the
flag→aggregate→surface loop the skill marks reserved in the engine (override-counting is not wired),
built natively as a companion signal instead of faked against an unwired key. The protocol exercising
its own governance over the bytes of this repo.

Detection reuses the SHARED detector in _lib.epr_meta (_sov_apex_count / _SOV_APEX_PHRASES) so the
ledger and the gate can never disagree on what "apex" means. Net-new only: an Edit is scored on the
delta (pre-edit reconstructed from old_string→new_string), so cleaning/maintenance is never logged.

Ledger:   .claude/data/sovereignty-guard.jsonl        (one line per landing)
Drift:    .claude/memory-kit/sovereignty-guard-drift.json  (running tally; escalates for rule review)

Hook Type: PostToolUse   Matcher: Edit|Write
"""
from __future__ import annotations

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
from _lib import epr_meta as em  # noqa: E402  (shared detector — single source of truth)

_ESCALATE_AT = 3  # landings before the message asks for a rule/corpus drift review


def _matched_phrases(pre: str, post: str) -> list[str]:
    lp, lpost = pre.lower(), post.lower()
    return sorted({p for p in em._SOV_APEX_PHRASES if lpost.count(p) > lp.count(p)})


def main() -> int:
    try:
        data = json.load(sys.stdin)
    except (json.JSONDecodeError, ValueError):
        return 0
    pd = os.environ.get("CLAUDE_PROJECT_DIR")
    ti = data.get("tool_input", {}) or {}
    tool = data.get("tool_name") or ""
    fp = ti.get("file_path") or ""
    if not pd or not fp or not fp.endswith(".md"):
        return 0
    repo = Path(pd).resolve()
    doc = Path(fp) if Path(fp).is_absolute() else repo / fp
    try:
        rel = str(doc.resolve().relative_to(repo))
    except (ValueError, OSError):
        rel = fp
    try:
        post = doc.read_text(errors="replace")
    except OSError:
        return 0
    if em._SOV_FRAME_MARKER in post.lower():
        return 0  # frame adjudicated for this surface — consistent with the gate

    # Reconstruct the pre-edit content so we score NET-NEW apex framing (introduction, not maintenance).
    if tool == "Edit":
        old, new = ti.get("old_string", ""), ti.get("new_string", "")
        pre = post.replace(new, old) if ti.get("replace_all") else post.replace(new, old, 1)
    else:  # Write — no reliable prior state post-hoc; score the whole file as introduced (conservative)
        pre = ""
    net_new = em._sov_apex_count(post) - em._sov_apex_count(pre)
    if net_new <= 0:
        return 0

    ts = datetime.now(timezone.utc).isoformat(timespec="seconds")
    phrases = _matched_phrases(pre, post)

    # 1) append the landing to the ledger
    try:
        led = repo / ".claude" / "data" / "sovereignty-guard.jsonl"
        led.parent.mkdir(parents=True, exist_ok=True)
        with led.open("a", encoding="utf-8") as fh:
            fh.write(json.dumps({"ts": ts, "path": rel, "tool": tool,
                                 "net_new": net_new, "phrases": phrases}) + "\n")
    except OSError:
        pass

    # 2) aggregate into the drift tally (the signal that flows back to the rule)
    total = net_new
    try:
        dj = repo / ".claude" / "memory-kit" / "sovereignty-guard-drift.json"
        dj.parent.mkdir(parents=True, exist_ok=True)
        state = {}
        if dj.is_file():
            try:
                state = json.loads(dj.read_text())
            except (json.JSONDecodeError, ValueError):
                state = {}
        state["rule"] = "sovereignty-ontology-guard@1"
        state["landings"] = int(state.get("landings", 0)) + net_new
        state["last"] = ts
        by = state.setdefault("by_path", {})
        by[rel] = int(by.get(rel, 0)) + net_new
        state["distinct_paths"] = len(by)
        total = state["landings"]
        dj.write_text(json.dumps(state, indent=2) + "\n")
    except OSError:
        pass

    # 3) surface it. Below threshold: a light note the landing was recorded. At/over: ask for review.
    ph = ", ".join(phrases) or "apex-sovereignty framing"
    if total >= _ESCALATE_AT:
        msg = (f"[sovereignty-guard] apex-sovereignty framing landed in {rel} ({ph}). "
               f"{total} landing(s) now aggregated across {rel!s} and peers "
               f"(.claude/memory-kit/sovereignty-guard-drift.json) — at/over the review threshold. "
               f"EVALUATE: is the corpus drifting toward the crypto self-sovereignty apex the protocol "
               f"rejects, or has the RULE itself drifted (a legitimate adversary/bounded/bridge frame it "
               f"keeps mis-flagging)? Canon: genesis/docs/architecture/stewardship-over-sovereignty.md; "
               f"values-forward.md Stance II.4. Reframe the bytes, or refine the rule "
               f"(.claude/epr-meta/policies.yaml → sovereignty-ontology-guard).")
    else:
        msg = (f"[sovereignty-guard] logged: apex-sovereignty framing ({ph}) landed in {rel}. If this is "
               f"a legitimate frame (adversary / bounded / bridge-legibility), consider a `sovereignty-frame:` "
               f"marker so the guard stays quiet here; otherwise reframe toward stewardship (canon: "
               f"genesis/docs/architecture/stewardship-over-sovereignty.md). Landing recorded for drift review.")
    print(json.dumps({"hookSpecificOutput": {"hookEventName": "PostToolUse", "additionalContext": msg}}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
