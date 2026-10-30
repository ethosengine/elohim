"""Git-native adapter for the .epr-meta compose-gate — the harness-agnostic binding surface.

The 4th adapter over _lib.epr_meta (after the Claude PreToolUse hook, ci-ignore-projector, and
the now-retired placement-audit.py). Governs ANY author's commit/push — Codex, Gemini, a human, or Claude driving git
via Bash — by evaluating the SAME pure rule engine over files derived from git (staged blobs / a
rev-range) rather than Claude tool_input. Collaboration THROUGH the protocol: no agent drives
another; the shared substrate governs every author equally.

Real logic lives HERE (in _lib, tested under __tests__) so .claude/hooks/ stays thin and the
transport can be swapped — exactly what .claude/hooks/.epr-meta declares. Driven by the thin CLI
.claude/scripts/epr-meta-git-gate.py and .husky/{pre-commit,pre-push}.

Governance phase — fail-open: a governance-rule bug never blocks a commit. Restorative (Mishpat,
not punishment): a malformed manifest downgrades deny->ask; editing an .epr-meta is never blocked;
consequence lands on the write (rejectable, fixable), never on the agent.

Formatting phase (`format_phase`, commit-format-ratchet@1) — fail-CLOSED by steward ruling
2026-10-30 ("we should never commit unlinted code"): every failure is a refusing Verdict with a
named cure, never an empty result. See the section at the bottom of this module.
"""
from __future__ import annotations

import subprocess
from pathlib import Path

from _lib import epr_meta

# Mirrors _lib.epr_meta severities; kept local so this adapter imports no private name.
_SEVERITY = {"deny": 3, "ask": 2, "inject": 1, "measure": 0, "dispatch": 0}


def git(*args: str) -> str | None:
    """Run a git plumbing command; return stdout, or None on any failure (fail-open)."""
    try:
        out = subprocess.run(["git", *args], capture_output=True, text=True, check=False)
        return out.stdout if out.returncode == 0 else None
    except Exception:  # noqa: BLE001
        return None


def changed_files(mode: str, rng: str | None, *, runner=git) -> list[tuple[str, str]]:
    """(path, status-letter) for staged files (--staged) or a rev-range. ACMR only — a delete
    can't violate an author-time content rule. `runner` is injectable for tests."""
    if mode == "staged":
        raw = runner("diff", "--cached", "--name-status", "--diff-filter=ACMR")
    else:
        raw = runner("diff", "--name-status", "--diff-filter=ACMR", rng or "HEAD~1..HEAD")
    if not raw:
        return []
    out: list[tuple[str, str]] = []
    for ln in raw.splitlines():
        parts = ln.split("\t")
        if len(parts) < 2:
            continue
        out.append((parts[-1], parts[0][0]))  # rename dst is last; status letter is first char
    return out


def content_of(mode: str, rng: str | None, path: str, *, runner=git) -> str | None:
    """Post-write content of `path`: the staged blob (--staged) or the range head blob."""
    if mode == "staged":
        return runner("show", f":{path}")
    head = rng.split("..")[-1] if rng and ".." in rng else "HEAD"
    return runner("show", f"{head}:{path}")


def head_has_parent(mode: str, rng: str | None, path: str, *, runner=git) -> bool:
    """True if the file's parent dir already exists in the base tree (so a new file in it is NOT a
    new-subdir signal). Base = HEAD for --staged, else the range base."""
    parent = str(Path(path).parent)
    if parent in (".", ""):
        return True  # repo root always exists
    base = "HEAD" if mode == "staged" else (rng.split("..")[0] if rng and ".." in rng else "HEAD")
    listing = runner("ls-tree", base, f"{parent}/")
    return bool(listing and listing.strip())


def build_write(path: str, status: str, content: str | None, head_has_parent: bool) -> dict:
    """PURE: derive the resolver's `write` dict from git facts. `is_new` = added (A);
    `is_new_subdir` = added AND its parent dir absent from the base tree (the git analog of the
    resolver's 'parent dir does not yet exist'). Shape identical to epr-meta-resolver.py:294."""
    is_new = status == "A"
    return {"path": path, "content": content, "is_new": is_new,
            "is_new_subdir": is_new and not head_has_parent}


def verdict_for(path: str, write: dict):
    """Full resolver sequence for one write -> combined Verdict|None. Mirrors
    epr-meta-resolver.py:250-323 minus the Claude-only side-channels. Malformed manifest downgrades
    deny->ask (unless the target IS an .epr-meta — legacy flat OR directory-form
    `.epr-meta/manifest.md` — which is never blocked so the fix is never bricked)."""
    target = Path(path)
    chain = epr_meta.collect_cascade(target)
    if not chain:
        return None
    if not epr_meta.is_manifest_path(target):
        problems = [(m, errs) for m in chain if (errs := epr_meta.check_meta(m))]
        if problems:
            detail = "; ".join(f"{m}: {', '.join(e)}" for m, e in problems)
            return epr_meta.Verdict("ask", f"governance manifest malformed — {detail}. Fix the "
                                    "manifest to restore full governance; proceeding now requires "
                                    "acknowledgement.", "epr-meta:malformed")
    merged = epr_meta.merge_rules(chain)
    root = epr_meta.find_repo_root(target)
    policies, _pol_errs = epr_meta.load_policies(root)
    epr_meta.expand_policies(merged, policies)
    return epr_meta.combine(epr_meta.evaluate(merged, write))


def decide(verdicts: list, *, ack: bool) -> tuple[int, list[str]]:
    """PURE: fold per-file combined verdicts into (exit_code, messages). Most-severe wins.
    deny->block(1); ask->block unless ack; inject/measure->advise+allow(0)."""
    msgs: list[str] = []
    worst = None
    for v in verdicts:
        if v is None:
            continue
        if worst is None or _SEVERITY[v.cls] > _SEVERITY[worst.cls]:
            worst = v
        label = "refer" if v.cls == "inject" and v.refer_reason == "stale-evidence" else v.cls
        msgs.append(f"[{label}] {v.reason} (rule `{v.rule_id}`)")
    if worst is None:
        return 0, msgs
    if worst.cls == "deny":
        return 1, msgs
    if worst.cls == "ask":
        if ack:
            return 0, [*msgs, "[.epr-meta] ask-class acknowledged via EPR_META_ACK=1 — allowed."]
        return 1, [*msgs, "[.epr-meta] This write needs acknowledgement. If reviewed and intended, "
                          "re-run with EPR_META_ACK=1 (this gate only), or bypass ALL hooks with "
                          "--no-verify."]
    return 0, msgs  # inject / measure — advisory only, never blocks


def run(mode: str, rng: str | None, *, ack: bool, runner=git) -> tuple[int, list[str]]:
    """Evaluate every changed file through the real cascade; return (exit_code, messages)."""
    verdicts = []
    for path, status in changed_files(mode, rng, runner=runner):
        content = content_of(mode, rng, path, runner=runner)
        parent_exists = head_has_parent(mode, rng, path, runner=runner)
        verdicts.append(verdict_for(path, build_write(path, status, content, parent_exists)))
    return decide(verdicts, ack=ack)


# ── Commit-time formatting ratchet (policy commit-format-ratchet@1) ──────────────────────────────
# Separate from the governance comparison above on purpose: the native evaluator scopes
# `epr:validator-staged-format` to Python (DeclaredElsewhere), so folding a formatting deny into
# the per-file decision the two hosts compare would read as an EVALUATOR DISAGREEMENT. The CLI
# folds this phase's exit into the final code instead, so a native permit never clears it.
#
# Unlike the governance phase, this phase does NOT fail open: a selected file without a completed
# result, an unreadable index, or a concurrent index change refuses (deny, not acknowledgeable);
# a missing tool or an unmeasurable file refuses as `ask` with its install cure (EPR_META_ACK=1
# acknowledges, and the acknowledgement is witnessed). Bypassing all hooks stays git's own
# `--no-verify` / HUSKY=0 — there is no new switch.
from _lib import epr_format  # noqa: E402

FORMAT_GATE_RULE = epr_format.FORMAT_POLICY_ID


def _format_rules_for(target: Path, write: dict, cache: dict, policy_cache: dict):
    """(matched staged-format rules, problem) for one snapshot path through the REAL cascade."""
    key = str(target.parent)
    if key not in cache:
        chain = epr_meta.collect_cascade(target)
        problems = [(m, errs) for m in chain if (errs := epr_meta.check_meta(m))]
        merged = epr_meta.merge_rules(chain)
        bound = [rid for rid, r in merged["rules"].items()
                 if str(r.get("policy", "")).startswith(f"{epr_format.FORMAT_POLICY_ID}@")
                 or r.get("validator") == epr_format.STAGED_FORMAT_VALIDATOR]
        root = epr_meta.find_repo_root(target)
        if str(root) not in policy_cache:
            policy_cache[str(root)] = epr_meta.load_policies(root)
        policies, pol_errs = policy_cache[str(root)]
        exp_errs = epr_meta.expand_policies(merged, policies)
        rules = {rid: r for rid, r in merged["rules"].items()
                 if r.get("validator") == epr_format.STAGED_FORMAT_VALIDATOR}
        problem = None
        if problems:
            problem = "; ".join(f"{m}: {', '.join(e)}" for m, e in problems)
        lost = [rid for rid in bound if rid not in rules]
        if lost:
            problem = (f"formatting binding(s) {lost} did not resolve to the staged-format "
                       f"validator ({'; '.join(pol_errs + exp_errs) or 'unknown cause'})")
        cache[key] = (rules, problem)
    rules, problem = cache[key]
    matched = {rid: r for rid, r in rules.items()
               if epr_meta._matches_when(r.get("when") or {}, write)}  # noqa: SLF001
    return matched, problem


def format_phase(repo: Path, *, git: "epr_format.Git | None" = None,
                 scratch: Path | None = None) -> tuple[list[tuple[str, object]], list[str], dict]:
    """Judge the STAGED index against every staged-format binding in its own committed cascade.
    Returns ([(path, Verdict)], notices, stats). Never raises: every failure is a Verdict."""
    repo = Path(repo)
    git = git or epr_format.Git(repo)
    verdicts: list[tuple[str, object]] = []
    notices: list[str] = []
    stats: dict = {"selected": 0, "checked": 0}

    def refuse(cls: str, reason: str, refer: str):
        verdicts.append(("(staged index)", epr_meta.Verdict(cls, reason,
                                                             f"{FORMAT_GATE_RULE}:{refer}", refer)))

    try:
        with epr_meta.parse_memo(), epr_format.StagedFormatter(repo, git, scratch=scratch) as sf:
            changes = epr_format.staged_changes(git, sf.head)
            if not changes:
                sf.verify_unchanged()
                return verdicts, notices, stats
            candidates = [c for c in changes
                          if Path(c.path).suffix.lower() in epr_format.FORMAT_CANDIDATE_EXTS]
            if candidates and not epr_meta.yaml_available():
                refuse("ask", "PyYAML is unavailable, so the formatting policy cannot be resolved "
                              f"for {len(candidates)} staged file(s). Cure: {epr_format.CURE_PYTHON}",
                       "environment")
                return verdicts, notices, stats
            if any(c.path.endswith(".rs") for c in candidates) and epr_format.tomllib is None:
                refuse("ask", "Python tomllib is unavailable, so Cargo editions cannot be "
                              f"resolved. Cure: {epr_format.CURE_PYTHON}", "environment")
                return verdicts, notices, stats
            sf.prefetch_closure(changes)
            rescans = sf.contract_rescans(changes)
            if rescans:
                sf.prefetch_closure(rescans)
                changes = changes + rescans
            # The governance closure, from the index: every ancestor manifest + the registry.
            closure = [epr_format.POLICY_REGISTRY_REL]
            for c in changes:
                for d in epr_format.ancestors(c.path):
                    closure.extend(epr_format.join(d, n) for n in epr_format.MANIFEST_CANDIDATES)
            sf.index.prefetch(closure)
            for rel in dict.fromkeys(closure):
                if sf.index.read(rel) is not None:
                    sf.index.materialize(rel)
            cache: dict = {}
            policy_cache: dict = {}
            selected, rules_by_path = [], {}
            for c in changes:
                write = build_write(c.path, "A" if c.is_new else "M", None, True)
                rules, problem = _format_rules_for(sf.index.path(c.path), write, cache,
                                                   policy_cache)
                if problem:
                    refuse("ask", f"the formatting policy for {c.path} cannot be resolved from the "
                                  f"staged manifests: {problem}. Fix the manifest or registry.",
                           "policy-unresolved")
                    return verdicts, notices, stats
                if rules:
                    selected.append(c)
                    rules_by_path[c.path] = rules
            stats["selected"] = len(selected)
            if not selected:
                sf.verify_unchanged()
                return verdicts, notices, stats
            try:
                results = sf.judge(selected)
            except epr_format.MissingTool as e:
                refuse("ask", f"{e} — {len(selected)} governed staged file(s) cannot be checked. "
                              f"Cure: {e.cure}", "missing-tool")
                return verdicts, notices, stats
            sf.verify_unchanged()
            stats["rust_pool"] = epr_format.RUST_POOL
            stats["prettier_workers"] = sf.prettier.launches
            stats["git_calls"] = git.calls
            for c in selected:
                res = results.get(c.path)
                if res is None:
                    raise epr_format.EnforcementError(
                        f"{c.path} was selected by the formatting policy but no result came back")
                if res.state not in ("outside", "ignored"):
                    stats["checked"] += 1
                write = {"path": c.path, "content": None, "is_new": c.is_new,
                         "is_new_subdir": False, "staged_format": res.as_dict()}
                v = epr_meta.combine(epr_meta.evaluate({"rules": rules_by_path[c.path]}, write))
                if v is not None:
                    verdicts.append((c.path, v))
                elif res.state == "debt":
                    notices.append(f"[notice] {res.reason}. ({res.cure})")
    except epr_format.EnforcementError as e:
        refuse("deny", f"formatting gate could not complete: {e}", "enforcement")
    except epr_format.GitError as e:
        refuse("deny", f"formatting gate could not read the index: {e}", "enforcement")
    except Exception as e:  # noqa: BLE001 — an engine bug refuses; it never permits silently
        refuse("deny", f"formatting gate internal error: {type(e).__name__}: {e}. Report it; to "
                       f"commit anyway bypass all hooks with --no-verify", "enforcement")
    return verdicts, notices, stats
