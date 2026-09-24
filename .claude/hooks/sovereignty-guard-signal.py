#!/usr/bin/env python3
"""
Sovereignty ontology guard — dismissal-aggregation signal (the closed governance loop).

PostToolUse hook (matcher: Edit|Write). The PRE half is the `.epr-meta` compose-gate: the
sovereignty-ontology-guard policy (realized by epr:validator-sovereignty-ontology-guard) fires
when a write NET-NEW introduces apex-assertion sovereignty framing into a governed doc — class
`dispatch` since @3: advisory, a background storyteller review, never an ask or a deny. This hook
is the POST half: when apex framing LANDS (the review is advisory, or the doc sits outside the
cascade), it logs the firing and
aggregates the landings, so the RULE itself surfaces for evaluation/drift-review. This is the
flag→aggregate→surface loop the skill marks reserved in the engine (override-counting is not wired),
built natively as a companion signal instead of faked against an unwired key. The protocol exercising
its own governance over the bytes of this repo.

Detection reuses the SHARED classifier in _lib.frame_atoms (the frame atom both governance hosts
read) so the ledger and the gate can never disagree on what "apex" means. Net-new only: an Edit is scored on the
delta (pre-edit reconstructed from old_string→new_string), so cleaning/maintenance is never logged.

Ledger:   .claude/data/sovereignty-guard.jsonl        (one line per landing)
          {ts, path, tool, net_new, phrases, frame_ref, classification_cid,
           classification_scope, source, verdict, nonce}.
          `frame_ref` is the frame atom's CID (stdlib, `frame_atoms`). `classification_cid` is
          minted ONLY by the native evaluator (ruling R-C4: no second DAG-CBOR encoder in
          Python): `epr govern --new --content-stdin` over the landed bytes. `--new` because the
          native host reads the prior from disk, which post-landing IS the landed file, so the
          classification is of the landed document against an empty prior.
          OFF THE CRITICAL PATH (ruling R-C8): the hook's budget is 2 s and `epr govern` costs
          ~1.3 s, so the parent writes the row with `classification_cid: null, source: pending`
          and a DETACHED child (`--classify`) rewrites that row: `source: native` with the CID,
          or `source: python-degraded` when the evaluator does not run. A reader treats
          `pending` like `python-degraded`.
          `classification_scope` is always `"document"` (ruling R-C14, review W1): `net_new` and
          `phrases` describe the EDIT's delta, but `classification_cid` classifies the WHOLE
          landed document against an empty prior, so a reader must not read the CID as a
          classification of the delta. (A prior-content channel for the native evaluator is
          backlog: genesis/data/timeline/backlog/native-govern-prior-channel.md.)
          `nonce` is 16 random hex digits minted per landing and handed to the classify child,
          which rewrites the pending row BY NONCE (review W4): two landings of one path within
          one second share `(path, ts)` and would otherwise fill each other's rows.
Probe:    the Family-2 SHADOW PROBE (ruling R-C4 amended, plan task C9). A `.md` Write/Edit whose
          net-new bytes reach the atom's `probe_min_net_new_bytes`, that NO keyword matched, and
          that is not testimony (the ontology atom's `testimony_exempt`) spawns a detached child
          (`--probe`): `flow memory index fold --max-files 1 --scope <rel>` → `recall open` → ONE
          `flow memory recall search --provider semantic` for the atom's phrases, scoped to the
          landed file's directory. A score on the landed file at/above `cosine_floor_permille /
          1000` is an ABSTAIN — a row carrying `probe{cosine, floor, verdict, producer, method,
          fold_lag}` and a fold on `frame-probe-abstain@1` — never an accusation, and never
          printed: the author does not see it. The child gives up at 30 s (ruling R-C10: it
          is detached, so the author never waits; the chain measured 11.3 s under load). The
          recall session `frame-probe-<sid>` is opened once per hook session: when its
          continuation (`.eprfs/status/recall/frame-probe-<sid>/continuation.json`, the state
          `recall open` writes) is already on disk, the probe goes straight to the search.
Drift:    ONE thing — a fold via `epr flow note --kind observation --measure
          sovereignty-landings@1`. The private JSON tally this hook kept under
          `.claude/memory-kit/` was deleted with the kit at station six round (b)
          (2026-09-11). The running total the escalation message reads is DERIVED from the
          folds by sovereignty-landings-ceiling@1's `derive: count-since-reset` and read back
          with `_observation.bound_count` — paid per landing, never per edit.

Hook Type: PostToolUse   Matcher: Edit|Write
"""
from __future__ import annotations

import fcntl
import json
import os
import re
import secrets
import subprocess
import sys
import tempfile
import time
from datetime import datetime, timezone
from pathlib import Path

_here = Path(__file__).resolve()
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        break
    _here = _here.parent
from _lib import epr_client  # noqa: E402  (the native evaluator mints the classification CID)
from _lib import frame_atoms  # noqa: E402  (shared classifier — the frame atom is the source of truth)
sys.path.insert(0, str(Path(__file__).resolve().parent))
import _observation as _obs  # noqa: E402  (structured-observation emitter; JSON fallback while absent)


def active_rule_version(repo: Path, rule_id: str) -> str:
    """`<rule_id>@<highest non-superseded version>` from the policy registry.

    The drift ledger records WHICH RULE VERSION its landings were measured against — a review that
    misattributes landings to a superseded row draws the wrong conclusion about whether the rule or
    the corpus is drifting. This was hardcoded to `@1` and went stale the moment the guard bumped to
    `@2` (found by audit, 2026-08-05), so it is derived instead.

    Deliberately a cheap line scan, not a YAML parse: this runs on EVERY Edit|Write under a 2s
    timeout and must never be the reason a hook is slow. Falls back to `@1` if the registry is
    unreadable — telemetry degrades, it never blocks.
    """
    try:
        best, seen_id, superseded = 0, False, False
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

_ESCALATE_AT = 3  # landings before the message asks for a rule/corpus drift review
_SOV_REF = "epr:validator-sovereignty-ontology-guard"
LEDGER_REL = ".claude/data/sovereignty-guard.jsonl"
PENDING = "pending"
PROBE_DEADLINE_S = 30.0         # the shadow probe's whole budget (fold + open + search), R-C10
RECALL_DIR_REL = ".eprfs/status/recall"  # `epr flow memory recall`'s private session records
CLASSIFICATION_SCOPE = "document"  # the CID classifies the landed document, not the delta (R-C14)
PROBE_MEASURE = "frame-probe-abstain@1"


class Deadline:
    """One budget for a child, so its bounded calls cannot sum past it."""

    def __init__(self, seconds: float) -> None:
        self.end = time.monotonic() + seconds

    def left(self) -> float:
        return max(0.0, self.end - time.monotonic())


def native_classification(repo: Path, rel: str, landed: str, frame_ref: str,
                          session: str | None) -> tuple[str | None, str]:
    """`(classification_cid, source)` from the native evaluator over the landed bytes.

    Reuses `epr_client.govern` (the same invocation the PreToolUse resolver makes) and reads the
    verdict whose opaque evidence names THIS frame. `source` is `native` whenever the evaluator
    ran — the CID is then `None` only if no bound rule classified the write here (the guard is
    bound per directory) — and `python-degraded` when it did not run at all.
    """
    payload = epr_client.govern(repo, rel, landed, True, False, session=session)
    if payload is None:
        return None, "python-degraded"
    for verdict in payload.get("verdicts") or []:
        evidence = verdict.get("evidence") if isinstance(verdict, dict) else None
        if isinstance(evidence, dict) and evidence.get("frameRef") == frame_ref:
            cid = evidence.get("classificationCid")
            if isinstance(cid, str) and cid:
                return cid, "native"
    return None, "native"


# ── the ledger (append under a lock; the classify child rewrites one row in place) ───────────
def append_row(repo: Path, row: dict) -> None:
    try:
        led = repo / LEDGER_REL
        led.parent.mkdir(parents=True, exist_ok=True)
        with led.open("a", encoding="utf-8") as fh:
            fcntl.flock(fh, fcntl.LOCK_EX)
            fh.write(json.dumps(row) + "\n")
    except OSError:
        pass


def _is_pending(row, rel: str, ts: str, nonce: str) -> bool:
    """THE pending row a child was spawned for: its own nonce, not merely its (path, second)."""
    return (isinstance(row, dict) and row.get("source") == PENDING and bool(nonce)
            and row.get("nonce") == nonce and row.get("path") == rel and row.get("ts") == ts)


def rewrite_pending_row(repo: Path, rel: str, ts: str, nonce: str, fill) -> dict | None:
    """Rewrite the `pending` row carrying `nonce` (for `rel`, `ts`) with `fill(row)`'s fields, in
    place and under the ledger lock (a concurrent append waits, and is never lost to a replace).
    Returns the row as it was, or None when there is no such row."""
    led = repo / LEDGER_REL
    try:
        with led.open("r+", encoding="utf-8") as fh:
            fcntl.flock(fh, fcntl.LOCK_EX)
            lines = fh.read().splitlines(keepends=True)
            for i in range(len(lines) - 1, -1, -1):
                try:
                    row = json.loads(lines[i])
                except ValueError:
                    continue
                if _is_pending(row, rel, ts, nonce):
                    was = dict(row)
                    row.update(fill(was))
                    lines[i] = json.dumps(row) + "\n"
                    fh.seek(0)
                    fh.write("".join(lines))
                    fh.truncate()
                    return was
    except OSError:
        pass
    return None


# ── detached children ───────────────────────────────────────────────────────────────────────
def spawn_child(repo: Path, args: list[str], content: str | None = None) -> None:
    """ONE detached child of this hook: its own session, no inherited stdout/stderr (the harness
    reads the hook's stdout to EOF, so an inherited pipe would make the author wait), the landed
    bytes on stdin from an unlinked temp file. Never waited on."""
    try:
        with tempfile.TemporaryFile() as feed:
            if content is not None:
                feed.write(content.encode("utf-8"))
                feed.seek(0)
            subprocess.Popen(
                [sys.executable, str(Path(__file__).resolve()), *args],
                cwd=str(repo), stdin=feed, stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL, start_new_session=True, close_fds=True,
            )
    except (OSError, ValueError, subprocess.SubprocessError):
        pass


def classify_child(args: list[str], stdin) -> int:
    """`--classify <rel> <session> <ts> <nonce>`: mint the native classification for the row the
    parent wrote as `pending` with that nonce, over the landed bytes on stdin (rulings R-C8,
    R-C14). Without a nonce there is no row it may claim, so it does nothing."""
    if len(args) < 4:
        return 0
    rel, session, ts, nonce = args[0], args[1] or None, args[2], args[3]
    pd = os.environ.get("CLAUDE_PROJECT_DIR")
    if not pd or not nonce:
        return 0
    repo = Path(pd).resolve()
    landed = stdin.read()
    frame_ref = _pending_frame_ref(repo, rel, ts, nonce)
    if frame_ref is None:
        return 0
    cid, source = native_classification(repo, rel, landed, frame_ref, session)
    rewrite_pending_row(repo, rel, ts, nonce,
                        lambda _row: {"classification_cid": cid, "source": source})
    return 0


def _pending_frame_ref(repo: Path, rel: str, ts: str, nonce: str) -> str | None:
    """The frame ref of the pending row carrying `nonce`, read under the ledger lock (shared), so
    a concurrent append or rewrite is never read half-written (review W4)."""
    try:
        with (repo / LEDGER_REL).open("r", encoding="utf-8") as fh:
            fcntl.flock(fh, fcntl.LOCK_SH)
            lines = fh.read().splitlines()
    except OSError:
        return None
    for line in reversed(lines):
        try:
            row = json.loads(line)
        except ValueError:
            continue
        if _is_pending(row, rel, ts, nonce):
            ref = row.get("frame_ref")
            return ref if isinstance(ref, str) else None
    return None


# ── the Family-2 shadow probe ───────────────────────────────────────────────────────────────
def net_new_bytes(pre: str, post: str) -> int:
    """Bytes of the lines this write introduced — the classifier's own net-new (line-delta)
    notion, so re-ordering or cleaning existing prose never counts."""
    remaining: dict[str, int] = {}
    for line in pre.split("\n"):
        remaining[line] = remaining.get(line, 0) + 1
    total = 0
    for line in post.split("\n"):
        if remaining.get(line):
            remaining[line] -= 1
        else:
            total += len(line.encode("utf-8")) + 1
    return total


_FRONTMATTER = re.compile(r"\A---[ \t]*\n(.*?)\n---[ \t]*(?:\n|\Z)", re.S)


def is_testimony(rel: str, post: str, ontology: dict) -> bool:
    """The ontology atom's `testimony_exempt`: a path prefix, or `<frontmatter_key>: true`."""
    exempt = ontology.get("testimony_exempt") or {}
    if any(rel.startswith(prefix) for prefix in exempt.get("path_prefixes") or []):
        return True
    key = exempt.get("frontmatter_key")
    match = _FRONTMATTER.match(post)
    if not key or not match:
        return False
    for line in match.group(1).splitlines():
        name, sep, value = line.partition(":")
        if sep and name.strip() == key:
            return value.split("#", 1)[0].strip().strip("\"'").lower() == "true"
    return False


def _run_json(binary: str, repo: Path, args: list[str], timeout: float) -> dict | None:
    """One bounded `epr` call whose stdout is a JSON object, or None (a timeout kills it)."""
    if timeout <= 0:
        return None
    try:
        done = subprocess.run([binary, *args, "--root", str(repo)], capture_output=True,
                              text=True, timeout=timeout, cwd=str(repo),
                              stdin=subprocess.DEVNULL)
        if done.returncode != 0:
            return None
        value = json.loads(done.stdout)
        return value if isinstance(value, dict) else None
    except (OSError, ValueError, subprocess.SubprocessError):
        return None


def _session_label(session: str) -> str:
    return re.sub(r"[^A-Za-z0-9_-]", "_", session or "none")[:64]


def recall_session_open(repo: Path, label: str) -> bool:
    """Whether `recall open` already hydrated this session: it writes the session's state to
    `.eprfs/status/recall/<session>/continuation.json` (`Execution::open`), creating the directory
    first — so an empty or absent continuation is an open that never completed, not a session."""
    try:
        return (repo / RECALL_DIR_REL / label / "continuation.json").stat().st_size > 0
    except OSError:
        return False


def probe_child(args: list[str]) -> int:
    """`--probe <rel> <session> [<tool>]`: fold → open → ONE semantic search scoped to the landed
    file's directory; abstain when the landed file itself scores at/above the atom's floor."""
    if len(args) < 2:
        return 0
    rel, session = args[0], args[1]
    tool = args[2] if len(args) > 2 else ""
    deadline = Deadline(PROBE_DEADLINE_S)
    pd = os.environ.get("CLAUDE_PROJECT_DIR")
    binary = epr_client.resolve_binary()
    if not pd or not binary:
        return 0
    repo = Path(pd).resolve()
    try:
        atom, frame_ref = frame_atoms.load_frames()[_SOV_REF]
    except (frame_atoms.FrameAtomError, KeyError):
        return 0
    signal = atom["recall_signal"]
    floor = signal["cosine_floor_permille"] / 1000
    query = " / ".join(signal["phrases"])
    scope = str(Path(rel).parent) or "."
    label = f"frame-probe-{_session_label(session)}"

    # The fold first, scoped to the landed file itself (`--scope` matches the exact path), so the
    # bytes the search reads are the bytes that landed: an unscoped `--max-files 1` folds the
    # first file the plan lists as behind, which is almost never this one. Its answer does not
    # matter (a busy fold is another session's fold, and the search reports its own lag).
    try:
        subprocess.run([binary, "flow", "memory", "index", "fold", "--max-files", "1",
                        "--scope", rel, "--root", str(repo)], capture_output=True, cwd=str(repo),
                       stdin=subprocess.DEVNULL, timeout=max(0.01, deadline.left()))
    except (OSError, subprocess.SubprocessError):
        pass
    # One open per hook session (ruling R-C10, review M3): the open cost 4.3-5.9 s under load and
    # a session's later probes reuse the continuation it wrote.
    if not recall_session_open(repo, label) and _run_json(
            binary, repo, ["flow", "memory", "recall", "open", "--session", label, "--need", query,
                           "--json"], deadline.left()) is None:
        return 0
    answer = _run_json(binary, repo, ["flow", "memory", "recall", "search", "--provider",
                                      "semantic", "--query", query, "--search-scope", scope,
                                      "--session", label, "--json"], deadline.left())
    best = None
    for cand in ((answer or {}).get("retrieval") or {}).get("candidates") or []:
        if not isinstance(cand, dict) or cand.get("path") != rel:
            continue
        score, producer, method = cand.get("score"), cand.get("producer"), cand.get("method")
        if not isinstance(score, (int, float)) or isinstance(score, bool):
            continue
        if not isinstance(producer, str) or not producer or not isinstance(method, str) or not method:
            continue  # a candidate that cannot say who produced it under which method is not evidence
        if best is None or score > best["score"]:
            best = cand
    if best is None or float(best["score"]) < floor or deadline.left() <= 0:
        return 0
    lag = best.get("fold_lag")
    append_row(repo, {
        "ts": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "path": rel, "tool": tool, "net_new": 0, "phrases": [], "frame_ref": frame_ref,
        "classification_cid": None, "classification_scope": CLASSIFICATION_SCOPE,
        "source": "semantic-probe", "verdict": "abstain",
        "probe": {"cosine": float(best["score"]), "floor": floor, "verdict": "abstain",
                  "producer": best["producer"], "method": best["method"],
                  "fold_lag": lag if isinstance(lag, int) and not isinstance(lag, bool) else None},
    })
    _obs.emit(PROBE_MEASURE, rel, 1,
              reason="frame probe: keyword-silent write resembles the apex-sovereignty frame — abstain",
              env={"frame": frame_ref, "cosine": f"{float(best['score']):.3f}"}, root=str(repo))
    return 0


def maybe_probe(repo: Path, rel: str, tool: str, pre: str, post: str, session: str) -> None:
    """Spawn the shadow probe for a keyword-silent `.md` Write/Edit past the byte floor that is
    not testimony. Any unreadable atom means no probe — telemetry degrades, it never blocks."""
    if tool not in ("Write", "Edit"):
        return
    try:
        atom, _ref = frame_atoms.load_frames()[_SOV_REF]
        ontology = frame_atoms.load_ontology()
    except (frame_atoms.FrameAtomError, KeyError):
        return
    if net_new_bytes(pre, post) < atom["recall_signal"]["probe_min_net_new_bytes"]:
        return
    if is_testimony(rel, post, ontology):
        return
    spawn_child(repo, ["--probe", rel, session, tool])


def main(argv: list[str] | None = None) -> int:
    argv = sys.argv[1:] if argv is None else argv
    if argv[:1] == ["--classify"]:
        return classify_child(argv[1:], sys.stdin)
    if argv[:1] == ["--probe"]:
        return probe_child(argv[1:])
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
    # Reconstruct the pre-edit content so we score NET-NEW apex framing (introduction, not maintenance).
    if tool == "Edit":
        old, new = ti.get("old_string", ""), ti.get("new_string", "")
        pre = post.replace(new, old) if ti.get("replace_all") else post.replace(new, old, 1)
    else:  # Write — no reliable prior state post-hoc; score the whole file as introduced (conservative)
        pre = ""
    session = str(data.get("session_id") or "")
    try:
        found = frame_atoms.classify(
            {"content": post, "prior_content": pre, "is_new": False, "path": rel}, _SOV_REF)
    except frame_atoms.FrameAtomError:
        return 0  # telemetry degrades, it never blocks
    if found is None:
        # Keyword-silent: the only road left is the shadow probe, detached and never printed.
        maybe_probe(repo, rel, tool, pre, post, session)
        return 0
    if found["verdict"] == "legitimate":
        return 0  # a frame adjudicated for this surface — consistent with the gate
    net_new = found["netNew"]
    frame_ref = found["frameRef"]

    ts = datetime.now(timezone.utc).isoformat(timespec="seconds")
    phrases = [p for p in found["matchedRecallSignal"]
               if not p.endswith(":") and p != frame_atoms.UNSCANNED_TAIL]

    # 1) append the landing to the ledger as `pending`, then hand the native classification to a
    #    detached child that rewrites this row (ruling R-C8 — govern costs more than the budget).
    #    The nonce names THIS row for its child (review W4); the scope says the CID the child
    #    mints classifies the whole landed document, not this edit's delta (review W1).
    nonce = secrets.token_hex(8)
    append_row(repo, {"ts": ts, "path": rel, "tool": tool,
                      "net_new": net_new, "phrases": phrases,
                      "frame_ref": frame_ref,
                      "classification_cid": None,
                      "classification_scope": CLASSIFICATION_SCOPE,
                      "source": PENDING, "verdict": found["verdict"], "nonce": nonce})
    spawn_child(repo, ["--classify", rel, session, ts, nonce], content=post)

    # 2) aggregate into the drift tally (the signal that flows back to the rule).
    # The bound lives in .claude/epr-meta (sovereignty-landings@1) and the fold is the ONLY
    # home as of station six round (b): the private JSON tally this hook kept was deleted with
    # the kit. The accumulated number the message escalates on is read straight back out of the
    # fold plane by sovereignty-landings-ceiling@1's `derive: count-since-reset`.
    rule_version = active_rule_version(repo, "sovereignty-ontology-guard")
    _obs.emit("sovereignty-landings@1", rel, net_new,
              reason="sovereignty guard: apex-sovereignty framing landed after the ask",
              env={"rule": rule_version, "tool": tool, "frame": frame_ref}, root=str(repo))
    # Read the accumulation back rather than keeping one. Paid per LANDING, not per edit —
    # this branch is only reached when apex-sovereignty framing actually landed. `None` means
    # the count could not be taken (no binary, no verb, a bad read); the message then reports
    # THIS edit's landings and says the total is unavailable, instead of printing a number
    # nobody measured.
    accumulated = _obs.bound_count("sovereignty-landings-ceiling", root=str(repo))
    total = accumulated if accumulated is not None else net_new

    # 3) surface it. Below threshold: a light note the landing was recorded. At/over: ask for review.
    ph = ", ".join(phrases) or "apex-sovereignty framing"
    cites = f"frame {frame_atoms.short_cid(frame_ref)} · classification {PENDING}"
    if total >= _ESCALATE_AT:
        msg = (f"[sovereignty-guard] apex-sovereignty framing landed in {rel} ({ph}). "
               f"{total} landing(s) now aggregated across {rel!s} and peers "
               f"(folds on sovereignty-landings@1; drain with `epr flow note --kind observation "
               f"--measure sovereignty-landings-reset@1 --subject . --value 1`) — at/over the "
               f"review threshold. "
               f"EVALUATE: is the corpus drifting toward the crypto self-sovereignty apex the protocol "
               f"rejects, or has the RULE itself drifted (a legitimate adversary/bounded/bridge frame it "
               f"keeps mis-flagging)? Canon: genesis/docs/architecture/stewardship-over-sovereignty.md; "
               f"values-forward.md Stance II.4. Reframe the bytes, or refine the rule "
               f"(.claude/epr-meta/policies.yaml → sovereignty-ontology-guard). {cites}.")
    else:
        msg = (f"[sovereignty-guard] logged: apex-sovereignty framing ({ph}) landed in {rel}. If this is "
               f"a legitimate frame (adversary / bounded / bridge-legibility), consider a `sovereignty-frame:` "
               f"marker so the guard stays quiet here; otherwise reframe toward stewardship (canon: "
               f"genesis/docs/architecture/stewardship-over-sovereignty.md). Landing recorded for drift review. "
               f"{cites}.")
    print(json.dumps({"hookSpecificOutput": {"hookEventName": "PostToolUse", "additionalContext": msg}}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
