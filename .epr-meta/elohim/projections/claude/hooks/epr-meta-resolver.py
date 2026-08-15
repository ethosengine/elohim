#!/usr/bin/env python3
# .claude/hooks/epr-meta-resolver.py
"""PreToolUse resolver for the .epr-meta compose-gate. Thin: stdin -> _lib.epr_meta -> verdict JSON.
Fail-open: a guard bug never blocks dev."""

# The intervenor's removal condition (Meadows' shifting-the-burden trap;
# counted by _lib/intervenor_census.py). A condition, never a date.
RETIRE_WHEN = (
    "when the intervenor census (placement-audit --epr-meta) reaches zero live rules — this is "
    "the EVALUATOR for the compose gate, not a rule of its own, so it retires with the last "
    "rule it evaluates and never before. Removing it earlier would silently un-enforce every "
    "manifest at once, which is the fail-open direction this hook already guards against "
    "internally."
)
import hashlib
import json
import sys
import time
from pathlib import Path

try:
    import fcntl
except ImportError:  # pragma: no cover — non-POSIX
    fcntl = None

# --- _lib bootstrap (clone of managed-surface-context.py:26-32) ---
_here = Path(__file__).resolve()
_REPO_ROOT = _here  # best-effort fallback if the loop below never matches
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        _REPO_ROOT = _here
        break
    _here = _here.parent

from _lib import concern_routes  # noqa: E402
from _lib import epr_client  # noqa: E402
from _lib import epr_meta  # noqa: E402
from _lib import store  # noqa: E402

# Re-nudge a given gap-root at most once per ~working session, so the in-flight coverage signal
# INFORMS the agent-with-context without nagging every edit (the minimalism principle, applied to
# the nudge itself). Tunable; the gap self-closes the moment a `covers: subtree` manifest is authored.
_ADVICE_WINDOW = 4 * 3600

# Measure-class findings (the observation tier — never blocks) land here, sentinel-style:
# one line per LIVE fingerprint; a fingerprint PRESENT suppresses re-dispatch; the entry is
# deleted when the finding is drained (file back under ceiling), never parked.
_ARCH_LEDGER_REL = ".claude/data/architecture-findings.jsonl"


# WHICH evaluator produced this process's decision. Set once, after the native
# evaluator is consulted: the native identity when `epr` answered, Python's when
# it did not. A module global rather than a threaded parameter because this is a
# one-shot process with exactly one decision — there is no second value it could
# hold, and six call sites threading an invariant is noise, not clarity.
_EVALUATOR: dict | None = None

# The claimed-actor answer from the native evaluator's `--session` payload, same lifecycle as
# _EVALUATOR: set once from `native.get("actor")` when `epr` answered, left None when it did not
# (no actor plane consulted != unclaimed — that distinction lives in epr_meta.witness).
_ACTOR: dict | None = None


def _witness(root: Path, *, subject, decision: str, cls: str | None, rule_id: str | None = None,
             policy_ref: str | None = None, refer: dict | None = None,
             checks: list | None = None) -> None:
    """Thin wrapper over the shared _lib.epr_meta.witness ledger-append (runtime='claude',
    gate='pre-tool-use') — the SAME helper the git-gate CLI drives, so both runtimes write the
    identical governance-findings.jsonl shape. Best-effort; never raises (witnessing must never
    break the gate — epr_meta.witness already wraps its body in try/except)."""
    epr_meta.witness(root, runtime="claude", gate="pre-tool-use", subject=subject,
                      decision=decision, cls=cls, rule_id=rule_id, policy_ref=policy_ref,
                      refer=refer, checks=checks, evaluator=_EVALUATOR, actor=_ACTOR)


def _advice_debounced(root: Path, key: str) -> bool:
    """True if `key` was advised within _ADVICE_WINDOW; records the advice time otherwise.
    Shares the coverage-nudge store so all epr-meta debounce state lives in one place.
    Read-check-write runs under store.locked_update so concurrent sessions can't lose each
    other's debounce keys (worst case on lock timeout: one extra advisory, never a suppression)."""
    try:
        state_p = root / ".claude/data/epr-meta-advice.json"
        now = time.time()
        hit = {"debounced": False}

        def upd(seen):
            seen = seen if isinstance(seen, dict) else {}
            last = seen.get(key, 0)
            if isinstance(last, (int, float)) and (now - last) < _ADVICE_WINDOW:
                hit["debounced"] = True
                return seen
            seen[key] = now
            return seen

        store.locked_update(state_p, upd, {})
        return hit["debounced"]
    except Exception:  # noqa: BLE001 — store trouble → advise rather than suppress
        return False


def _bound_ref(policy_ref: str | None, merged: dict, root: Path, rule_id: str) -> str | None:
    """`<policy-id>@<version>` plus the manifest id when resolvable — the nearest cascade source
    (`merged["sources"]`, root-first) that actually DECLARES `rule_id`, walked nearest-first so a
    local override wins over an inherited ancestor binding. `<policy_ref>#<manifest-rel-path>`
    when a declaring manifest is found on disk; the bare policy_ref otherwise (still honest, just
    less specific). None only when there is no policy_ref at all (an inline, unpolicied measure
    rule) — never a guess."""
    if not policy_ref:
        return None
    for src in reversed(merged.get("sources") or []):  # nearest-first
        try:
            cfg = epr_meta.load_meta(Path(src))
        except Exception:  # noqa: BLE001 — resolvable is best-effort, never fatal
            continue
        if any(isinstance(r, dict) and r.get("id") == rule_id for r in (cfg.get("rules") or [])):
            try:
                manifest_rel = str(Path(src).resolve().relative_to(root))
            except (ValueError, OSError):
                manifest_rel = str(src)
            return f"{policy_ref}#{manifest_rel}"
    return policy_ref


def _handle_measures(measures, merged, root: Path, fp_path: str) -> list[str]:
    """The measure side-channel: file each hard-ceiling verdict as a fingerprinted architecture
    finding (flag→agent→canon→stasis, the deprecation-sentinel pattern). NEW fingerprint →
    ledger append + dispatch directive; already-filed → debounced one-line citation."""
    notes: list[str] = []
    try:
        rel = str(Path(fp_path).resolve().relative_to(root))
    except (ValueError, OSError):
        rel = fp_path
    for v in measures:
        try:
            fpid = hashlib.sha256(f"{v.rule_id}|{rel}".encode()).hexdigest()[:12]
            ledger = root / _ARCH_LEDGER_REL
            rule = merged["rules"].get(v.rule_id, {})
            agent = (rule.get("measure") or {}).get("dispatch-agent", "general-purpose")
            # Read-check-append under an flock on the ledger fd itself, so concurrent sessions
            # can't both see "not filed" and duplicate the row ("one line per LIVE fingerprint").
            # Non-blocking with bounded retry — a hook never waits on a lock; on timeout we skip
            # (the next edit of the still-over-ceiling file retries).
            ledger.parent.mkdir(parents=True, exist_ok=True)
            is_new = None  # None = lock not acquired
            with open(ledger, "a+", encoding="utf-8", errors="replace") as fh:
                locked = fcntl is None
                if not locked:
                    deadline = time.monotonic() + 0.5
                    while time.monotonic() < deadline:
                        try:
                            fcntl.flock(fh.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
                            locked = True
                            break
                        except OSError:
                            time.sleep(0.02)
                if locked:
                    fh.seek(0)
                    known = set()
                    for ln in fh.read().splitlines():
                        try:
                            known.add(json.loads(ln).get("fp"))
                        except Exception:  # noqa: BLE001
                            continue
                    is_new = fpid not in known
                    if is_new:
                        entry = {"fp": fpid, "rule": v.rule_id, "policy": rule.get("policy-ref"),
                                 "path": rel, "detail": v.reason[:300],
                                 "first_seen": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
                                 "status": "open"}
                        # Typed-evidence graduation (algedonic phase-1 Task 4): additive fields
                        # matching the wire schemas' evidence block — never inputs to `fpid` above,
                        # so existing ledger rows keep their fingerprints byte-for-byte. `stock`/
                        # `limit` come structurally off the Verdict (never parsed from `detail`
                        # prose); `bound_ref` names the policy + (when resolvable) the declaring
                        # manifest; `concern` is explicit-param-only — honest absence omits the key
                        # rather than guessing (concern_routes.route, slice-1 Task 1).
                        evidence = getattr(v, "evidence", None) or {}
                        if isinstance(evidence.get("stock"), int):
                            entry["stock"] = evidence["stock"]
                        if isinstance(evidence.get("limit"), int):
                            entry["limit"] = evidence["limit"]
                        bound_ref = _bound_ref(rule.get("policy-ref"), merged, root, v.rule_id)
                        if bound_ref:
                            entry["bound_ref"] = bound_ref
                        concern = concern_routes.route(
                            "measure", {"concern": (rule.get("measure") or {}).get("concern")})
                        if concern:
                            entry["concern"] = concern
                        fh.write(json.dumps(entry, ensure_ascii=False) + "\n")
            if is_new is None:
                continue  # lock contention — another session owns this moment; next edit retries
            if not is_new:
                if not _advice_debounced(root, f"arch-fp:{fpid}"):
                    notes.append(
                        f"[.epr-meta measure] `{rel}` remains over its hard LoC ceiling — already "
                        f"filed as architecture finding {fpid} ({_ARCH_LEDGER_REL}, status open). "
                        f"Drain via its backlog plan; do not re-file.")
                continue
            notes.append(
                f"[.epr-meta measure] NEW architecture finding {fpid}: {v.reason} Captured to "
                f"{_ARCH_LEDGER_REL}. DISPATCH NOW (do not derail the current task): launch the "
                f"`{agent}` agent via the Agent tool with run_in_background: true and the prompt "
                f"'Architecture finding {fpid}: `{rel}` is at/over its LoC hard ceiling. Scope its "
                f"module seams and canonicalize a modularization plan into "
                f"genesis/data/timeline/backlog/ (timeline-CONVENTIONS-conformant, "
                f"status-carrying) — plan the restructuring as bounded work; do NOT refactor "
                f"inline. Honor the refactor-safety notes in the policy why "
                f"(registry: .claude/epr-meta/policies.yaml).' If that agent type is unavailable "
                f"this session, use general-purpose with the same prompt. Then continue your "
                f"current task.")
        except Exception:  # noqa: BLE001 — observation tier must never break the gate
            continue
    return notes


def _handle_dispatches(dispatches, merged, fp_path: str) -> list[str]:
    """The dispatch side-channel: build the sentinel-idiom directive text per fired dispatch
    verdict (deprecation-sentinel.py:487-509's dispatch-a-background-agent phrasing, verbatim
    shape) from the rule's `parameters` (dispatch-agent / dispatch-prompt). Pure text assembly —
    witnessing is the caller's job (main(), which has `root`)."""
    notes: list[str] = []
    for v in dispatches:
        rule = merged["rules"].get(v.rule_id, {})
        params = rule.get("parameters") if isinstance(rule.get("parameters"), dict) else {}
        agent = params.get("dispatch-agent", "general-purpose")
        prompt = params.get("dispatch-prompt", "review this change")
        notes.append(
            f"[epr-meta] dispatch rule `{v.rule_id}` fired — DISPATCH NOW (do not derail the "
            f"current task): launch the `{agent}` agent via the Agent tool with "
            f"run_in_background: true and the prompt: {prompt} (subject: {fp_path})")
    return notes


def _emit_deny(reason: str):
    print(json.dumps({"hookSpecificOutput": {
        "hookEventName": "PreToolUse", "permissionDecision": "deny",
        "permissionDecisionReason": reason}}))
    sys.exit(0)


def _emit_ask(reason: str):
    print(json.dumps({"hookSpecificOutput": {
        "hookEventName": "PreToolUse", "permissionDecision": "ask",
        "permissionDecisionReason": reason}}))
    sys.exit(0)


def _emit_advise(text: str):
    print(json.dumps({"hookSpecificOutput": {
        "hookEventName": "PreToolUse", "additionalContext": text}}))
    sys.exit(0)


def _coverage_nudge(target: Path):
    """The IN-FLIGHT coverage signal: if `target` sits inside an UNCLAIMED substantial region, return a
    debounced advisory pointing the agent — who has the live context RIGHT NOW — at the gap-root to
    govern. This is the prevention-shaped, context-rich remediation path (vs a blind external sweep):
    the best `.epr-meta` is written by whoever is already intimately in the directory. None if the
    region is already owned, not substantial, or recently advised. Fail-open: any error → no nudge."""
    try:
        root = epr_meta.find_repo_root(target)
        cfg = epr_meta.governance_cfg(root)
        adv = epr_meta.coverage_advice(
            target, repo_root=root, min_files=cfg["min_files"], min_subdirs=cfg["min_subdirs"],
            min_exts=cfg["min_exts"], exclude_globs=cfg["exclude_globs"])
        if not adv:
            return None
        gap = adv["gap_root"]
        state_p = root / ".claude/data/epr-meta-advice.json"
        seen = store.load_json(state_p, {}) or {}
        now = time.time()
        last = seen.get(gap, 0)
        if isinstance(last, (int, float)) and (now - last) < _ADVICE_WINDOW:
            return None  # debounced — already nudged for this region this session
        seen[gap] = now
        store.save_json(state_p, seen)
        return (f"[.epr-meta coverage] You're working in `{gap}` — a substantial directory with no "
                f"governance ownership (no `covers: subtree` .epr-meta above it). You have the live "
                f"context NOW: consider authoring `{gap}/.epr-meta` declaring `covers: subtree` — a real "
                f"rule if there's a recurring, mechanizable drift here, else rules-free with a `why:` "
                f"recording the considered 'no edit-time gate needed' decision. See the "
                f"`elohim-epr-metafile` skill. (In-flight remediation; full queue: "
                f"`placement-audit.py --epr-meta`.)")
    except Exception:  # noqa: BLE001 — the nudge is advisory; never let it block or crash the hook
        return None


def main():
    raw = sys.stdin.read()
    try:
        data = json.loads(raw)
    except Exception:
        _witness(_REPO_ROOT, subject=None, decision="permit", cls=None,
                 refer={"layer": "operator", "reason": "gate-exception"},
                 checks=["malformed stdin JSON — failing open"])
        sys.exit(0)  # malformed stdin -> fail open, never silently (polarity law)

    session_id = data.get("session_id")

    tool = data.get("tool_name", "")
    if tool not in ("Write", "Edit"):
        sys.exit(0)
    ti = data.get("tool_input", {}) or {}
    fp = ti.get("file_path", "")
    if not fp:
        sys.exit(0)

    target = Path(fp)
    is_new = (tool == "Write") and not target.exists()
    # No separate dir-create event in Claude Code: a Write whose parent dir does not yet exist
    # is the new-subdir signal (drives no-new-subdirs / require-sibling).
    is_new_subdir = (tool == "Write") and not target.parent.exists()

    # Resolve the content the rules will see.
    #   Write: the payload carries the full post-write content.
    #   Edit:  synthesize the POST-edit content — read on-disk, apply old_string->new_string
    #          (respecting replace_all) — so content-triggered rules (contains-any / validator)
    #          evaluate what the edit INTRODUCES, not the stale pre-edit pre-image (which both
    #          misses a newly-added match AND lets a match the edit REMOVES keep blocking).
    if tool == "Edit":
        if target.exists():
            try:
                disk = target.read_text(errors="replace")
            except Exception:
                disk = None
            if disk is not None:
                old = ti.get("old_string", "")
                new = ti.get("new_string", "")
                if not old or old not in disk:
                    # old_string absent from disk -> the Edit itself will fail; nothing to gate.
                    sys.exit(0)
                content = disk.replace(old, new) if ti.get("replace_all") else disk.replace(old, new, 1)
            else:
                content = None  # unreadable -> content rules see nothing (the coverage nudge still runs)
        else:
            # Edit on a not-yet-existing file (the Edit will fail): leave content unresolved so
            # content rules stay silent, but keep the hook alive for the coverage nudge / cascade.
            content = None
    else:  # Write
        content = ti.get("content")
        if content is None and target.exists():
            # Defensive: a Write lacking an explicit content field on an existing file.
            try:
                content = target.read_text(errors="replace")
            except Exception:
                content = None

    # IN-FLIGHT COVERAGE SIGNAL — independent of the rule cascade. Authoring an .epr-meta is itself
    # never nudged (you're already governing). Computed up-front so it can fire even when there is no
    # cascade at all (the most common gap: a wholly-ungoverned substantial region).
    cov_nudge = None if epr_meta.is_manifest_path(target) else _coverage_nudge(target)

    chain = epr_meta.collect_cascade(target)
    if not chain:
        if cov_nudge:
            _emit_advise(cov_nudge)  # the empty-chair: "no governance here" → deliver the signal
        sys.exit(0)

    if not epr_meta.yaml_available():
        _emit_advise("[.epr-meta] PyYAML unavailable — compose-gate rules NOT enforced for this "
                     "write. Install PyYAML to re-enable the gate. (failing open, not silent.)")

    advisories = []
    if cov_nudge:  # a governed-but-unclaimed region: the nudge rides along with any rule advisories
        advisories.append(cov_nudge)

    metas = [(m, epr_meta.load_meta(m)) for m in chain]
    # Recursion-guard surface 6.2: a governed subtree whose cascade reached the repo/depth bound
    # without a `root: true` constitutional base is a misconfiguration. v1 ADVISES (fail-open
    # friendly); the spec's stricter `deny` is a hardening once the root is repo-wide.
    if not any(cfg.get("root") is True for _, cfg in metas):
        advisories.append("[.epr-meta] no `root: true` constitutional base in this cascade — "
                          "add one (this subtree's governance has no anchor).")

    root = epr_meta.find_repo_root(target)
    write = {"path": fp, "content": content, "is_new": is_new,
             "is_new_subdir": is_new_subdir}

    # Soft-ceiling injects re-fire on EVERY edit of an over-soft file — debounce per (rule, path)
    # so the nudge informs once per working session instead of nagging each keystroke. Injected
    # into resolve_write (which stays pure otherwise) as the `verdict_filter` extension point.
    def _soft_filter(v, merged) -> bool:
        r = merged["rules"].get(v.rule_id, {})
        return (v.cls == "inject" and "measure" in r
                and _advice_debounced(root, f"loc-soft:{v.rule_id}:{fp}"))

    # The governance correspondence spine: the SAME pure decision path the golden-vector parity
    # runner drives (cascade -> malformed check -> merge+expand policies -> evaluate -> combine ->
    # decision mapping). This resolver is thin BY CONSTRUCTION — it owns stdin/stdout, witnessing,
    # the coverage nudge, the soft-debounce filter, and the two Claude-side side-channels
    # (arch-ledger for measure, the sentinel-agent directive for dispatch); the decision itself
    # lives in _lib.epr_meta.
    result = epr_meta.resolve_write(target, write, root, verdict_filter=_soft_filter)
    advisories += result["advisories"]
    merged = result.get("merged")

    # ── the native evaluator is the DECISION AUTHORITY ────────────────────────
    # Both hosts evaluate; `epr` decides. Running only one would delete the
    # dispute path, and a dispute record is the artifact that makes the plane
    # trustworthy — agreement is the trivial half of the claim.
    #
    # Authority applies to the DECISION, not to the advice. When the two agree
    # (the overwhelmingly common case, measured at 129/129 decisions across
    # every governed subtree) Python's class/rule/reason are kept, because three
    # validators are scoped `python` by declaration and their `inject`
    # advisories exist nowhere else. Single decision authority must not quietly
    # become less advice. Only a genuine dispute takes Rust's attribution too —
    # its decision, its reasons.
    global _EVALUATOR, _ACTOR
    native = epr_client.govern(root, fp, content, is_new, is_new_subdir, session=session_id)
    if native is None:
        _EVALUATOR = epr_meta.evaluator_identity()
        # Announce the missing second opinion only where there was a DECISION to
        # cross-check. A clean allow — no rule fired, nothing witnessed — is
        # silent by the polarity law, and nobody disputes a verdict that does not
        # exist; banner-ing it would break that floor and put a notice on
        # essentially every governed write. Debounced on top, so even a real
        # degradation informs once a session instead of nagging per keystroke.
        if result.get("witnessed") and not _advice_debounced(root, "native-evaluator-degraded"):
            advisories.append(epr_client.DEGRADED_NOTICE)
    else:
        _EVALUATOR = native.get("evaluator")
        _ACTOR = native.get("actor")
        dispute = epr_client.disagreement(result, native)
        if dispute:
            # The dispute record names BOTH evaluators by content address, so a
            # third party can fetch each build and re-derive who was right.
            _witness(root, subject=fp, decision=native.get("decision"),
                     cls=native.get("winningClass"), rule_id=native.get("ruleId"),
                     refer={"layer": "operator", "reason": "evaluator-disagreement"},
                     checks=[f"EVALUATOR DISAGREEMENT: {dispute}",
                             f"disputed: {epr_meta.evaluator_identity()}",
                             f"authority: {native.get('evaluator')}"])
            advisories.append(
                f"[.epr-meta] EVALUATOR DISAGREEMENT — {dispute}. The native evaluator (`epr`) "
                f"is the decision authority and its verdict stands; the dispute is recorded to "
                f"the governance ledger by name. This is a governance-plane defect, not a "
                f"content problem: the two hosts must derive the same decision from the same "
                f"manifest, and one of them is wrong.")
            result = {**result,
                      "decision": native.get("decision"),
                      "cls": native.get("winningClass"),
                      "rule_id": native.get("ruleId"),
                      "reason": native.get("reason") or result.get("reason"),
                      "refer": ({"layer": "operator", "reason": native["referReason"]}
                                if native.get("referReason") else result.get("refer"))}

    # Measure side-channel (observation tier — never blocks): hard-ceiling verdicts become
    # fingerprinted architecture findings + a dispatch directive, sentinel-style. Independent of
    # the overall decision (fires even when a deny/ask elsewhere wins) — mirrors pre-slice
    # behavior. Each fired measure is witnessed regardless of the overall winning class.
    if result["measures"]:
        advisories += _handle_measures(result["measures"], merged, root, fp)
        for v in result["measures"]:
            rule = merged["rules"].get(v.rule_id, {}) if merged else {}
            _witness(root, subject=fp, decision="permit", cls="measure", rule_id=v.rule_id,
                     policy_ref=rule.get("policy-ref"), checks=[v.reason])

    # Dispatch side-channel (never blocks, class dispatch): the NEW capability this slice adds —
    # pre-slice, a fired `class: dispatch` verdict was silently dropped (never messaged, never
    # witnessed). Independent of the overall decision, same as measures — witnessed even when a
    # deny/ask elsewhere wins; the directive text only reaches the model when dispatch itself is
    # the winning (permitting) outcome, since a deny/ask response has no room for it.
    if result["dispatches"]:
        for v in result["dispatches"]:
            rule = merged["rules"].get(v.rule_id, {}) if merged else {}
            params = rule.get("parameters") if isinstance(rule.get("parameters"), dict) else {}
            _witness(root, subject=fp, decision="permit", cls="dispatch", rule_id=v.rule_id,
                     policy_ref=rule.get("policy-ref"),
                     checks=[f"dispatch-agent={params.get('dispatch-agent', '?')}"])
        if result["cls"] == "dispatch":
            advisories += _handle_dispatches(result["dispatches"], merged, fp)

    decision = result["decision"]
    cls = result["cls"]
    reason = result["reason"]

    if decision == "refuse":  # deny — unchanged UX
        _witness(root, subject=fp, decision="refuse", cls="deny", rule_id=result["rule_id"],
                 policy_ref=(merged["rules"].get(result["rule_id"], {}).get("policy-ref")
                             if merged and result["rule_id"] else None),
                 checks=[reason])
        _emit_deny(reason)

    if decision == "refer":  # ask, OR an unresolvable-validator / malformed-manifest routing
        _witness(root, subject=fp, decision="refer", cls=cls, rule_id=result["rule_id"],
                 policy_ref=(merged["rules"].get(result["rule_id"], {}).get("policy-ref")
                             if merged and result["rule_id"] else None),
                 refer=result["refer"], checks=[reason])
        _emit_ask(reason)

    # decision == "permit": inject (advisory) / dispatch / measure / clean-allow-with-advisories.
    if cls == "inject":
        _witness(root, subject=fp, decision="permit", cls="inject", rule_id=result["rule_id"],
                 policy_ref=(merged["rules"].get(result["rule_id"], {}).get("policy-ref")
                             if merged and result["rule_id"] else None),
                 checks=[reason])
        _emit_advise(" ".join([reason, *advisories]))

    if advisories:
        _emit_advise(" ".join(advisories))
    sys.exit(0)  # silent allow — no rule fired, nothing to witness (the polarity floor)


if __name__ == "__main__":
    try:
        main()
    except SystemExit:
        raise
    except Exception as e:  # fail-open: internal error, NOT a deny
        try:
            _witness(_REPO_ROOT, subject=None, decision="permit", cls=None,
                     refer={"layer": "operator", "reason": "gate-exception"},
                     checks=[f"internal error: {e}"])
        except Exception:  # noqa: BLE001 — witnessing must never break the gate
            pass
        print(f"epr-meta-resolver internal error: {e}", file=sys.stderr)
        sys.exit(1)
