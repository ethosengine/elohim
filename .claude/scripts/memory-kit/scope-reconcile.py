#!/usr/bin/env python3
"""scope-reconcile.py — move artifacts live<->held/ so the plate reconciles to the available compute.

A doc declares `requires_env: [<capability>]` in frontmatter. cluster-state.yaml declares what's available.
When a doc's requires_env is NOT a subset of available, it is git-mv'd to a parallel `held/` tree OUTSIDE
the planner/runner scan path (structurally sequestered — the compile-enforced level); when its capability
returns, it moves back. The tree IS the plate: a git mv broadcasts the scope change atomically to every
consumer (planner, audit, the next agent). Safe now that cites resolve by slug (semantic-computable-links),
so a move never breaks an inbound cite (it reads HELD-CITE, not DEAD-CITE).

Idempotent. Default = dry-run; --apply does the git mv + drops a STOP CLAUDE.md into each held/ tree.
Spec: genesis/docs/superpowers/specs/2026-06-02-scope-tree-reconciler-design.md.
"""
from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

_here = Path(__file__).resolve()
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        break
    _here = _here.parent
from _lib import env_scope as _es  # noqa: E402
from _lib import frontmatter as _fm  # noqa: E402
from _lib import paths as _paths  # noqa: E402
from _lib import cluster_state as _cs  # noqa: E402

ROOT = _paths.repo_root_from_file(__file__)
CLUSTER_STATE = ROOT / "genesis" / "manifests" / "cluster-state.yaml"
GAP_DIR = ROOT / ".claude" / "memory-kit" / "gap-items"
# The deployments arm — humans declare nodeTypes (requirement side); cluster-state resources declare
# provides_node_types (availability side); `suspended` is DERIVED, never hand-written. Derived flags carry
# a provenance marker so the reconciler only ever touches its OWN flags (an operator-manual suspension
# without the marker survives every cascade) — same discipline as generated cites.
DEPLOYMENTS = ROOT / "genesis" / "orchestrator" / "data" / "deployments.json"
_PROVENANCE = "scope-reconcile:"

# (live_dir, held_dir) — held_dir is OUTSIDE the live scan/glob path so a held doc is structurally invisible
# to the planner (placement-audit scans specs/+plans/) and the runner (cucumber globs features/**).
ZONES = [
    ("genesis/docs/superpowers/specs", "genesis/docs/superpowers/held/specs"),
    ("genesis/docs/superpowers/plans", "genesis/docs/superpowers/held/plans"),
    ("genesis/docs/plans", "genesis/docs/held/plans"),
    ("genesis/a2o/features", "genesis/a2o/held/features"),
]

STOP = """# ⛔ STOP — held artifacts (require unavailable hardware)

The docs under this `held/` tree declare a `requires_env:` capability that is **not currently available**
(see `genesis/manifests/cluster-state.yaml`). They are sequestered here, OUTSIDE the planner/runner scan
path, so they don't false-fail or consume planning focus. This is **held, NOT deleted or regressed** —
partial availability is the steady state.

- Do NOT edit or "fix" these as if broken. They are correct; the hardware is absent.
- Inbound `cites:` to these resolve as **HELD-CITE** (content-addressed; not dead) — do not delete the link.
- They move back automatically when their capability returns: `scope-reconcile.py --apply`.

Managed by `.claude/scripts/memory-kit/scope-reconcile.py`. The mover, not a human, files things here.
"""


def _parse_cluster() -> tuple:
    """(all_resource_names, available_names) from cluster-state.yaml. Delegates to the canonical
    `_lib.cluster_state` parser (agentic-context-tooling-consolidation-queue.md item 6) — this and
    placement-audit's `load_cluster_state` / focus-baseline's `_cluster_detail` were three
    independent hand-rolled regex walks of the same block, one of them unscoped; one parser now
    backs all three so the budget reader and the mover can never disagree."""
    state = _cs.load(CLUSTER_STATE)
    return state.all_names(), state.available_names()


def available_caps() -> set:
    """Capabilities that are available==true (degraded/false are NOT available)."""
    return _parse_cluster()[1]


def known_resources() -> set:
    """ALL resource names in cluster-state (the env-availability vocabulary). A requires_env cap NOT in
    this set — e.g. an a2o `@requires:doorway` test-setup tag — is not a hardware-availability concern and
    does NOT trigger a held-move; only cluster-state-tracked capabilities gate scope."""
    return _parse_cluster()[0]


def requires_env(doc: Path) -> list:
    """Parse requires_env, handling the minimal frontmatter parser's quirks: a block list (already a list),
    an INLINE list `[a, b]` or `[]` (read as a string), or a scalar. Strips inline comments + quotes.
    Delegates to `_lib.env_scope.parse_requires_env` (agentic-context-tooling-consolidation-queue.md
    item 7 — this parse + the `@requires:` regex were extracted to `_lib.env_scope` but the call sites
    here never switched)."""
    if doc.suffix == ".feature":
        return _feature_requires(doc)  # a2o gherkin has no frontmatter — read @requires:<cap> tags
    v = _fm.parse_file(doc).fields.get("requires_env")
    return _es.parse_requires_env(v)


def _feature_requires(doc: Path) -> list:
    """a2o .feature files declare env via `@requires:<cap>` gherkin tags on the feature-level tag line(s)
    (above the first `Feature:` keyword), NOT frontmatter. Delegates to `_lib.env_scope.requires_tags`
    for the tag regex itself (item 7)."""
    caps = []
    for ln in doc.read_text(encoding="utf-8", errors="replace").splitlines():
        stripped = ln.lstrip()
        if stripped.startswith("Feature:"):
            break
        if stripped.startswith("#"):  # gherkin comment — prose mentioning a tag is not a tag
            continue
        caps += _es.requires_tags(ln)
    return caps


def _git_mv(src: Path, dst: Path, apply: bool) -> bool:
    """Move src->dst. REFUSE if dst already exists (a live/held name collision — clobbering would destroy
    content; one of the two is stale and the operator must resolve it). Returns False on conflict/skip."""
    if dst.exists():
        print(f"  ⚠ CONFLICT  {src.relative_to(ROOT)} -> {dst.relative_to(ROOT)} (dst occupied — SKIPPED, operator triage)")
        return False
    if not apply:
        return True
    dst.parent.mkdir(parents=True, exist_ok=True)
    r = subprocess.run(["git", "mv", str(src), str(dst)], cwd=str(ROOT), capture_output=True, text=True)
    if r.returncode != 0:  # not git-tracked → plain move, then `git add` so git-as-ledger still holds
        src.rename(dst)
        subprocess.run(["git", "add", str(dst)], cwd=str(ROOT), capture_output=True, text=True)
    return True


def _ensure_stop(held_root: Path, apply: bool):
    f = held_root / "CLAUDE.md"
    if apply and not f.exists():
        f.parent.mkdir(parents=True, exist_ok=True)
        f.write_text(STOP, encoding="utf-8")


def _requires_split(doc: Path, known: set) -> tuple:
    """(relevant, unknown) — requires_env caps split into cluster-TRACKED vs caps matching no known resource
    name. An unknown cap is a VOCAB DRIFT (e.g. a doc says `harbor` but cluster-state calls it
    `harbor-registry`); it is surfaced as a warning and treated CONSERVATIVELY — it blocks held->live escape,
    because a capability you don't track might be the real blocker hiding under a wrong name."""
    rel, unk = [], []
    for c in requires_env(doc):
        (rel if c in known else unk).append(c)
    return rel, unk


def _gap_index() -> dict:
    """basename -> decomposed gap-items record. Keyed by the doc's FILENAME (stable across a held<->live move,
    unlike the recorded `doc` path). Lets the hold decision be GAP-granular: a doc is held whole only if every
    gap is blocked; a doc with any household-testable gap belongs on the plate (honors iroh ≠ shem)."""
    idx = {}
    if GAP_DIR.is_dir():
        for gf in GAP_DIR.glob("*.json"):
            try:
                rec = json.loads(gf.read_text(encoding="utf-8"))
            except (ValueError, OSError):
                continue
            if not isinstance(rec, dict):
                # A bare items-array (wrapper lost at write) must not kill the
                # SessionStart scope gate — skip it; placement-audit owns the repair nudge.
                continue
            rd = rec.get("doc", "")
            if rd:
                idx[Path(rd).name] = rec
    return idx


def _scope_verdict(doc: Path, avail: set, known: set, gidx: dict) -> tuple:
    """(verdict, blocking_caps): 'held' = every gap blocked (uniformly out of scope), 'live' = has at least one
    satisfiable gap (mixed/testable → belongs on the plate), 'ambiguous' = no scope info at all.

    The doc's LIVE frontmatter `requires_env` is the inheritance default (read fresh, not from the possibly
    stale cache) — so a doc-level scope change takes effect immediately. The gap cache supplies only the
    per-gap `@requires:` OVERRIDES; a doc with no cache decides purely on the frontmatter default."""
    doc_req = [c for c in requires_env(doc) if c in known]
    rec = gidx.get(doc.name)
    if rec and rec.get("items"):
        blocked_caps, satisfiable = set(), False
        for it in rec["items"]:
            resolved = _es.resolved_requires_env(it.get("requires_env"), doc_req)
            if _es.gap_blocked(resolved, avail, known):
                blocked_caps |= {c for c in resolved if c in known and c not in avail}
            else:
                satisfiable = True
        return ("live" if satisfiable else "held", sorted(blocked_caps))
    if not doc_req:
        return ("ambiguous", [])
    miss = [c for c in doc_req if c not in avail]
    return ("held" if miss else "live", miss)


def compute_drift() -> tuple:
    """Pure scan — what the plate WOULD reconcile to, no side effects. The shared core of reconcile() (the
    mover) and report() (the SessionStart gate). Returns (to_held, to_live, vocab_warnings, held_anomalies).

    GAP-GRANULAR: the held<->live decision is per the scope VERDICT (every-gap-blocked vs has-testable-gap),
    not the doc-level requires_env alone — so a mixed plan (most gaps testable now, only cross-node-assertion
    gaps need an unavailable cap) stays on the plate instead of being held whole. Only cluster-tracked caps
    gate scope (an a2o `@requires:doorway` fixture tag is not a hardware concern). held -> live requires
    AFFIRMATIVE evidence (a 'live' verdict); an 'ambiguous' held doc STAYS held (never auto-published)."""
    known, avail = _parse_cluster()
    gidx = _gap_index()
    to_held, to_live, vocab, held_anomalies = [], [], [], []
    for live_rel, held_rel in ZONES:
        live, held = ROOT / live_rel, ROOT / held_rel
        if live.is_dir():
            for d in sorted(live.rglob("*.md")) + sorted(live.rglob("*.feature")):
                _rel, unk = _requires_split(d, known)
                if d.suffix == ".feature":
                    unk = []  # a2o @requires: mixes hardware + fixture tags — an unknown here is a fixture
                    # precondition (doorway, seeded-content), not hardware-vocab drift.
                if unk:
                    vocab.append((d, unk))
                verdict, caps = _scope_verdict(d, avail, known, gidx)
                if verdict == "held":
                    to_held.append((d, held / d.relative_to(live), caps))
        if held.is_dir():
            for d in sorted(held.rglob("*.md")) + sorted(held.rglob("*.feature")):
                if d.name == "CLAUDE.md":
                    continue
                _rel, unk = _requires_split(d, known)
                if d.suffix == ".feature":
                    unk = []
                if unk:
                    vocab.append((d, unk))
                verdict, caps = _scope_verdict(d, avail, known, gidx)
                if verdict == "live":
                    to_live.append((d, live / d.relative_to(held), caps))
                elif verdict == "ambiguous":
                    held_anomalies.append(d)
    return to_held, to_live, vocab, held_anomalies


# ── deployments.json arm (third arm of the reconciler) ──────────────────────────────────────────────────
# deploy-render + seed + a2o test gates all key off `suspended` (the documented single source of truth for
# "is this human exercise-able"); this arm keeps that flag DERIVED from cluster-state so the two homes
# cannot drift apart again (2026-06-03: 11 shem-only humans stayed declared-deployed for days after shem
# went down → every genesis run hammered a non-resolving conductor with 24 provisioning failures).


def _parse_provides() -> dict:
    """nodeType -> set(resource) from cluster-state `provides_node_types: [a, b]` (4-space key under a
    resource). Delegates to the canonical `_lib.cluster_state` parser (item 6)."""
    return _cs.load(CLUSTER_STATE).provides_map()


def _humans() -> list:
    """The humans list from deployments.json (tolerates top-level array or {humans: [...]})."""
    if not DEPLOYMENTS.is_file():
        return []
    try:
        data = json.loads(DEPLOYMENTS.read_text(encoding="utf-8"))
    except ValueError:
        return []
    return data if isinstance(data, list) else (data.get("humans") or [])


def deployment_drift() -> list:
    """[(name, action, caps)] — pure derivation, no side effects.
    action ∈ suspend (blocked but declared live) | unsuspend (placeable but our flag still set) |
             adopt (blocked + suspended but provenance missing → normalize ownership) |
             manual-hold (operator-owned suspension the derivation disagrees with → report, never touch) |
             vocab (nodeTypes match no provides_node_types → vocabulary drift, conservative no-op)."""
    provides = _parse_provides()
    _known, avail = _parse_cluster()
    out = []
    for h in _humans():
        name = h.get("name")
        if not name:
            continue
        nts = h.get("nodeTypes") or []
        caps_per_nt = [provides.get(nt, set()) for nt in nts]
        if nts and all(not c for c in caps_per_nt):
            out.append((name, "vocab", nts))
            continue
        placeable = any(c & avail for c in caps_per_nt)
        missing = sorted({cap for c in caps_per_nt for cap in c if cap not in avail})
        susp = bool(h.get("suspended"))
        ours = str(h.get("suspendedBy") or "").startswith(_PROVENANCE)
        if not placeable and not susp:
            out.append((name, "suspend", missing))
        elif not placeable and susp and not ours:
            out.append((name, "adopt", missing))
        elif placeable and susp and ours:
            out.append((name, "unsuspend", []))
        elif placeable and susp and not ours:
            out.append((name, "manual-hold", []))
    return out


def reconcile_deployments(apply: bool) -> list:
    """Print + (with apply) line-edit deployments.json to match the derivation. Line-based so the
    hand-maintained formatting and $comment fields survive; re-validated with json.loads before write
    (an unparseable result is discarded — fail closed)."""
    drift = deployment_drift()
    for name, action, caps in drift:
        if action == "suspend":
            print(f"  ⏸ SUSPEND   {name}  (no nodeType placeable; needs {caps})")
        elif action == "unsuspend":
            print(f"  ▶ UNSUSPEND {name}  (capability returned)")
        elif action == "adopt":
            print(f"  ✦ ADOPT     {name}  (suspended matches derivation; adding provenance marker)")
        elif action == "manual-hold":
            print(f"  ⚠ MANUAL-HOLD {name}  (operator-owned suspension; derivation says placeable — left alone)")
        elif action == "vocab":
            print(f"  ⚠ VOCAB     {name}  nodeTypes {caps} match no provides_node_types in cluster-state")
    actionable = [d for d in drift if d[1] in ("suspend", "unsuspend", "adopt")]
    if not (apply and actionable):
        return drift
    lines = DEPLOYMENTS.read_text(encoding="utf-8").splitlines()
    for name, action, caps in actionable:
        idx = next((i for i, ln in enumerate(lines)
                    if re.match(rf'^\s*"name":\s*"{re.escape(name)}",\s*$', ln)), None)
        if idx is None:
            continue
        indent = re.match(r"^(\s*)", lines[idx]).group(1)
        end = next((k for k in range(idx + 1, len(lines)) if re.match(r'^\s*"name":\s*"', lines[k])), len(lines))
        sus = next((k for k in range(idx, end) if re.match(r'^\s*"suspended":', lines[k])), None)
        prov = next((k for k in range(idx, end) if re.match(r'^\s*"suspendedBy":', lines[k])), None)
        provline = f'{indent}"suspendedBy": "{_PROVENANCE}{",".join(caps)}",'
        if action == "suspend":
            if sus is not None:
                lines[sus] = f'{indent}"suspended": true,'
                if prov is not None:
                    lines[prov] = provline
                else:
                    lines.insert(sus + 1, provline)
            else:
                lines.insert(idx + 1, provline)
                lines.insert(idx + 1, f'{indent}"suspended": true,')
        elif action == "adopt":
            if prov is not None:
                lines[prov] = provline
            elif sus is not None:
                lines.insert(sus + 1, provline)
        elif action == "unsuspend":
            for k in sorted([x for x in (sus, prov) if x is not None], reverse=True):
                del lines[k]
    new = "\n".join(lines) + "\n"
    json.loads(new)  # fail closed: never write an unparseable deployments.json
    DEPLOYMENTS.write_text(new, encoding="utf-8")
    return drift


def reconcile(apply: bool) -> int:
    _known, avail = _parse_cluster()
    to_held, to_live, vocab, held_anomalies = compute_drift()
    for src, dst, caps in to_held:
        print(f"  → HELD   {src.relative_to(ROOT)}  (every gap blocked; needs {caps or 'an unavailable cap'})")
        _git_mv(src, dst, apply)
    for src, dst, caps in to_live:
        if caps:
            print(f"  ← LIVE   {dst.relative_to(ROOT)}  (mixed-scope: has household-testable gaps; some still need {caps})")
        else:
            print(f"  ← LIVE   {dst.relative_to(ROOT)}  (capability available again)")
        _git_mv(src, dst, apply)
    # STOP markers + orphan cleanup: a zone with real (non-CLAUDE.md) held docs gets the STOP; an emptied
    # zone has its orphan STOP removed (git rm) and its now-empty subdirs pruned.
    for _lr, held_rel in ZONES:
        held = ROOT / held_rel
        if not held.is_dir():
            continue
        real = [p for p in (list(held.rglob("*.md")) + list(held.rglob("*.feature"))) if p.name != "CLAUDE.md"]
        zone_root = held.parent if held.name in ("specs", "plans", "features") else held
        if real:
            _ensure_stop(zone_root, apply)
        elif apply:
            stop = zone_root / "CLAUDE.md"
            if stop.exists():
                subprocess.run(["git", "rm", "-q", "--ignore-unmatch", str(stop)], cwd=str(ROOT), capture_output=True, text=True)
                if stop.exists():
                    stop.unlink()
            for dd in sorted(held.rglob("*"), reverse=True):
                if dd.is_dir() and not any(dd.iterdir()):
                    dd.rmdir()
    for d in held_anomalies:
        print(f"  ⚠ HELD-STAYS  {d.relative_to(ROOT)}  (no parseable requires_env — cannot confirm safe to publish; operator triage)")
    for d, unk in vocab:
        print(f"  ⚠ VOCAB  {d.relative_to(ROOT)}  requires_env {unk} unknown to cluster-state (reconcile the name)")
    dep = reconcile_deployments(apply)
    dep_actionable = [x for x in dep if x[1] in ("suspend", "unsuspend", "adopt")]
    if apply:
        # Keep the co-located focus baseline (.claude/subject-focus.md) in lockstep with the plate, so
        # a fresh agent reads an always-current "what's in focus vs held" without running anything.
        subprocess.run(
            [sys.executable, str(Path(__file__).resolve().parent / "focus-baseline.py"), "--write"],
            cwd=str(ROOT), capture_output=True,
        )
    verb = "APPLIED" if apply else "DRY-RUN"
    print(f"scope-reconcile [{verb}]: available={sorted(avail) or '(none)'}")
    print(f"  → held: {len(to_held)}   ← live: {len(to_live)}   ⏸ deployments: {len(dep_actionable)}")
    if not apply and (to_held or to_live or dep_actionable):
        print("  (re-run with --apply to git mv + reconcile deployment flags)")
    return 0


def report() -> int:
    """One-line scope-drift gate for the SessionStart headline (mirrors cleanup-pressure.py --status). Says
    whether the plate is ALIGNED with the substrate, or N docs are ready to EXPAND onto the plate (a
    capability returned) / need to be HELD (a capability was lost) — the symmetric inverse of how held-ward
    sequestration is already surfaced. The move stays one command; this just makes the readiness impossible
    to miss, so stories + architecture follow the substrate without anyone remembering to check."""
    to_held, to_live, vocab, _ = compute_drift()
    parts = []
    if to_live:
        # 'live' covers both a returned capability and a mixed plan with household-testable gaps — either way
        # it belongs back on the plate. Don't imply the cap returned (a mixed doc's caps are still blocked).
        parts.append(f"{len(to_live)} to return to plate")
    if to_held:
        caps = sorted({c for _, _, r in to_held for c in r if c})
        capnote = f" ({','.join(caps)})" if caps else ""
        parts.append(f"{len(to_held)} to hold{capnote}")
    dep = deployment_drift()
    dep_actionable = [x for x in dep if x[1] in ("suspend", "unsuspend", "adopt")]
    if dep_actionable:
        caps = sorted({c for _, a, r in dep_actionable for c in r if c})
        capnote = f" ({','.join(caps)})" if caps else ""
        parts.append(f"{len(dep_actionable)} deployment flag(s){capnote}")
    vocab_note = ""
    if vocab:
        caps = sorted({c for _, u in vocab for c in u})
        vocab_note = f"  ⚠ unknown-cap: {','.join(caps)} (vocab drift vs cluster-state)"
    dep_vocab = [x for x in dep if x[1] == "vocab"]
    if dep_vocab:
        vocab_note += f"  ⚠ nodeType-vocab: {','.join(sorted({n for n, _, _ in dep_vocab}))}"
    if not parts:
        print(f"scope: aligned ✅  (plate matches substrate){vocab_note}")
    else:
        print(f"scope: ⚠ {' · '.join(parts)}  →  scope-reconcile.py --apply{vocab_note}")
    return 0


# ── developer flip control ──────────────────────────────────────────────────────────────────────────────
# shem is a RUNTIME capability, but developers need deliberate control to flip it on/off (not wait for a CI
# probe), and the two signal homes must stay coherent: cluster-state.yaml (durable, planning-layer) and
# ELOHIM_REMOTE_COMPUTE_STATUS (runtime, a2o/CI). The runtime signal is DERIVED from cluster-state here so they
# cannot disagree — flip the durable home, eval the derived export, and both layers + the held tree follow.
_STATES = {"on": "true", "available": "true", "true": "true",
           "off": "false", "unavailable": "false", "false": "false", "degraded": "degraded"}
# the cluster resource whose availability drives the runtime signal ELOHIM_REMOTE_COMPUTE_STATUS (the remote node)
REMOTE_COMPUTE_RESOURCE = "shem"


def _flip_resource(resource: str, value: str) -> bool:
    """Set <resource>.available = <value> in cluster-state.yaml (the durable home). Line-based, matching the
    lenient parser. Returns True iff a line changed."""
    if not CLUSTER_STATE.is_file():
        return False
    lines = CLUSTER_STATE.read_text(encoding="utf-8").splitlines()
    in_block, changed = False, False
    for i, ln in enumerate(lines):
        if re.match(rf"^  {re.escape(resource)}:\s*$", ln):
            in_block = True
            continue
        if in_block:
            m = re.match(r"^(    available:\s*)(\S+)(.*)$", ln)
            if m:
                if m.group(2) != value:
                    lines[i] = f"{m.group(1)}{value}{m.group(3)}"
                    changed = True
                break
            if re.match(r"^  [A-Za-z]", ln):  # next resource — no available: line in this block
                break
    if changed:
        CLUSTER_STATE.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return changed


def derive_remote_compute_status() -> str:
    """The runtime signal DERIVED from cluster-state — so the durable home and the runtime home cannot
    disagree. 'available' iff the remote-compute resource (shem) is available."""
    return "available" if REMOTE_COMPUTE_RESOURCE in _parse_cluster()[1] else "unavailable"


def env_line() -> int:
    """Emit the runtime export derived from cluster-state. Usage: eval "$(scope-reconcile.py --env)"."""
    print(f"export ELOHIM_REMOTE_COMPUTE_STATUS={derive_remote_compute_status()}")
    return 0


def set_state(arg: str, apply: bool) -> int:
    """--set <resource>=<on|off|degraded> — the developer flip. Edits the durable home, shows the coherent
    runtime export, then reconciles the held tree (dry-run unless --apply)."""
    if "=" not in (arg or ""):
        print("usage: --set <resource>=<on|off|degraded>", file=sys.stderr)
        return 2
    res, st = (x.strip() for x in arg.split("=", 1))
    st = st.lower()
    if st not in _STATES:
        print(f"--set: unknown state '{st}' (use on|off|degraded)", file=sys.stderr)
        return 2
    known = known_resources()
    if res not in known:
        print(f"--set: unknown resource '{res}' (known: {sorted(known)})", file=sys.stderr)
        return 2
    changed = _flip_resource(res, _STATES[st])
    print(f"cluster-state: {res}.available = {_STATES[st]}" + ("" if changed else "  (already set)"))
    if res == REMOTE_COMPUTE_RESOURCE:
        print(f'runtime ⇒  eval "$(scope-reconcile.py --env)"   # export ELOHIM_REMOTE_COMPUTE_STATUS={derive_remote_compute_status()}')
    print()
    return reconcile(apply)


if __name__ == "__main__":
    args = sys.argv[1:]
    if "--env" in args:
        raise SystemExit(env_line())
    if "--set" in args:
        i = args.index("--set")
        raise SystemExit(set_state(args[i + 1] if i + 1 < len(args) else "", "--apply" in args))
    if "--report" in args:
        raise SystemExit(report())
    raise SystemExit(reconcile("--apply" in args))
