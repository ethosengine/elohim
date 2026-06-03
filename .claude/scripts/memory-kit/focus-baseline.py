#!/usr/bin/env python3
"""focus-baseline.py — the per-subject FOCUS baseline (the READER twin of scope-reconcile, the mover).

Within a subject that's in scope, what is IN FOCUS vs HELD right now, WHY, what OPTIONS exist, and how to
BASELINE or PIVOT — the clear, concise, agent-readable summary the planning + brainstorm/shift flows lacked.

The model mirrors cluster-state itself:
  - NO narrowing active in a subject  ⇒  it is FAIR-GAME — work it freely, exactly like ranging over
    plans/specs out of the box (no focus set = everything in that subject tree is fair-game).
  - A capability going down NARROWS a subject: some features are held (out of the runner glob + agentic
    search), some live features have scenario-level holds. This renders that narrowing so an agent picks
    the in-focus slice, never bails on in-scope work, and never burns effort on out-of-scope artifacts.

Subjects are the a2o feature top-level directory (lamad / delivery / federation / …) — a DELIVERABLE-TARGET
key (path-based), never a vocabulary key (keying by 'shem'/'iroh' would re-introduce the D4 name-collision).
The held/live split + the WHY come from the SAME substrate the mover uses (scope-reconcile + cluster-state),
so the baseline can never disagree with the plate.

Deterministic, cheap, no LLM. `--write` regenerates the co-located baseline `.claude/subject-focus.md`
(refreshed on every `scope-reconcile --apply/--set`, so it never drifts). `--subject <name>` prints one.
Default prints the whole baseline to stdout.

Spec §9: genesis/docs/superpowers/specs/2026-06-02-scope-tree-reconciler-design.md.
"""
from __future__ import annotations

import importlib.util
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent


def _load_scope_reconcile():
    """Import scope-reconcile.py (hyphenated → importlib) to reuse its cluster parse + .feature reader,
    so this baseline can never diverge from the mover's notion of held/available."""
    spec = importlib.util.spec_from_file_location("scope_reconcile", HERE / "scope-reconcile.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


_sr = _load_scope_reconcile()
ROOT = _sr.ROOT
CLUSTER_STATE = _sr.CLUSTER_STATE
A2O_LIVE = ROOT / "genesis" / "a2o" / "features"
A2O_HELD = ROOT / "genesis" / "a2o" / "held" / "features"
_REQUIRES_TAG = re.compile(r"@requires:([a-z0-9][a-z0-9-]*)")


def _cluster_detail() -> tuple[dict, dict]:
    """{name: available_value} and {name: role} from cluster-state.yaml (line-based, like the mover)."""
    avail, roles = {}, {}
    if not CLUSTER_STATE.is_file():
        return avail, roles
    cur, in_res = None, False
    for ln in CLUSTER_STATE.read_text(encoding="utf-8", errors="replace").splitlines():
        if re.match(r"^resources:\s*$", ln):
            in_res = True
            continue
        if not in_res:
            continue
        if re.match(r"^[A-Za-z]", ln):
            break
        m = re.match(r"^  ([A-Za-z0-9_-]+):\s*$", ln)
        if m:
            cur = m.group(1)
            continue
        if cur:
            a = re.match(r"^    available:\s*(\S+)", ln)
            if a:
                avail[cur] = a.group(1)
            r = re.match(r"^    role:\s*(.+?)\s*$", ln)
            if r and cur not in roles:
                roles[cur] = r.group(1).split("#")[0].strip()
    return avail, roles


def _subject_of(feature_path: Path, tree: Path) -> str:
    """The DELIVERABLE-TARGET subject = the first path segment under features/ (e.g. lamad, auth)."""
    rel = feature_path.relative_to(tree)
    return rel.parts[0] if len(rel.parts) > 1 else rel.stem


def _feature_caps(path: Path) -> list[str]:
    """Feature-level @requires:<cap> — the tags ABOVE the first `Feature:` (reuses the mover's rule)."""
    return _sr.requires_env(path)


def _scenario_caps(path: Path) -> list[str]:
    """@requires:<cap> tags BELOW the first `Feature:` (scenario-level) — the runtime-held ones."""
    text, seen_feature, caps = path.read_text(encoding="utf-8", errors="replace"), False, []
    for ln in text.splitlines():
        if ln.lstrip().startswith("Feature:"):
            seen_feature = True
            continue
        if seen_feature:
            caps += _REQUIRES_TAG.findall(ln)
    return caps


def compute():
    """Per-subject focus from the live + held a2o trees ∩ cluster-state. Returns (subjects, avail, roles,
    known). subjects[name] = {in_focus:[paths], mixed:[(path, held_caps, count)], held:[(path, caps)]}."""
    avail_map, roles = _cluster_detail()
    available = {k for k, v in avail_map.items() if v == "true"}
    known = set(avail_map)
    subjects: dict = {}

    def slot(name):
        return subjects.setdefault(name, {"in_focus": [], "mixed": [], "held": []})

    if A2O_LIVE.is_dir():
        for f in sorted(A2O_LIVE.rglob("*.feature")):
            s = slot(_subject_of(f, A2O_LIVE))
            held_scn = [c for c in _scenario_caps(f) if c in known and c not in available]
            if held_scn:
                s["mixed"].append((f, sorted(set(held_scn)), len(held_scn)))
            else:
                s["in_focus"].append(f)
    if A2O_HELD.is_dir():
        for f in sorted(A2O_HELD.rglob("*.feature")):
            s = slot(_subject_of(f, A2O_HELD))
            caps = [c for c in _feature_caps(f) if c in known and c not in available]
            s["held"].append((f, sorted(set(caps)) or ["(an unavailable cap)"]))
    return subjects, available, roles, known


def _why(caps: set, roles: dict) -> str:
    bits = []
    for c in sorted(caps):
        role = roles.get(c, "")
        bits.append(f"{c} unavailable" + (f" — {role}" if role else ""))
    return "; ".join(bits)


def render(write: bool, only_subject: str | None, brief: bool = False) -> str:
    subjects, available, roles, known = compute()
    avail_map, _ = _cluster_detail()
    unavailable = {k: v for k, v in avail_map.items() if v != "true"}

    fair, narrowed = [], []
    for name in sorted(subjects):
        s = subjects[name]
        (narrowed if (s["held"] or s["mixed"]) else fair).append(name)

    out: list[str] = []
    w = out.append
    w("# SUBJECT FOCUS BASELINE   (generated — sibling of subject-routing.yaml; refreshed on every scope flip)")
    w("#")
    w("# No narrowing in a subject ⇒ it is FAIR-GAME: work it freely, like ranging over plans/specs out of")
    w("# the box. A capability going down narrows a subject — its focus, why, and options are shown below.")
    w("# Source of truth: genesis/manifests/cluster-state.yaml + the a2o held/ tree (via scope-reconcile).")
    w("")
    avail_str = ", ".join(sorted(available)) or "(none)"
    unavail_str = ", ".join(f"{k}({v})" for k, v in sorted(unavailable.items())) or "(none — full focus)"
    w(f"substrate: available [{avail_str}]   unavailable [{unavail_str}]")
    w("")

    if not narrowed:
        w("✅ FULL FOCUS — every subject is fair-game (no capability narrowing active). Work anything;")
        w("   pick by vision×readiness as usual. (`scope-reconcile.py --set <cap>=off --apply` narrows the")
        w("   plate when you want a focused slice.)")
        text = "\n".join(out) + "\n"
        if write:
            (ROOT / ".claude" / "subject-focus.md").write_text(text, encoding="utf-8")
        return text

    if not only_subject:
        w(f"FAIR-GAME subjects ({len(fair)}) — no narrowing, everything in focus:")
        w("  " + (" · ".join(fair) or "(none)"))
        w("")
        w(f"NARROWED subjects ({len(narrowed)}) — a capability is down; focus + options per subject:")
        if brief:
            w("  " + (" · ".join(narrowed) or "(none)"))
            w("  → drill in: python3 .claude/scripts/memory-kit/focus-baseline.py --subject <name>")
            return "\n".join(out) + "\n"
        w("")

    targets = [only_subject] if only_subject else narrowed
    for name in targets:
        s = subjects.get(name)
        if not s:
            w(f"### {name} — unknown subject (fair-game if not listed as narrowed)")
            continue
        if not (s["held"] or s["mixed"]):
            w(f"### {name}   ✅ fair-game ({len(s['in_focus'])} in focus, nothing held)")
            w("")
            continue
        held_caps = {c for _, caps in s["held"] for c in caps if c in known}
        held_caps |= {c for _, caps, _ in s["mixed"] for c in caps}
        w(f"### {name}   ⚠ narrowed — {', '.join(sorted(held_caps)) or 'env'} down")
        w(f"  IN FOCUS  : {len(s['in_focus'])} live feature(s), fully testable on available compute")
        for f in s["in_focus"][:6]:
            w(f"              · {f.relative_to(ROOT)}")
        if len(s["in_focus"]) > 6:
            w(f"              · … +{len(s['in_focus']) - 6} more")
        for f, caps, n in s["mixed"]:
            w(f"  MIXED     : {f.relative_to(ROOT).name} — {n} scenario(s) need {','.join(caps)} (runtime-skipped, NOT failed)")
        for f, caps in s["held"]:
            real = [c for c in caps if c in known]
            w(f"  HELD      : {f.relative_to(A2O_HELD)} — needs {','.join(caps)} · returns when {','.join(real) or 'the cap'} available")
        w(f"  WHY       : {_why(held_caps, roles)}")
        w(f"  OPTIONS   : (a) work the in-focus + any household scenarios now"
          f"  (b) expand the plate: scope-reconcile.py --set {sorted(held_caps)[0] if held_caps else '<cap>'}=on --apply"
          f"  (c) pivot to a fair-game subject")
        w(f"  BASELINE/PIVOT: /shift or /brainstorm scoped to {name}'s in-focus slice; to pivot, pick a FAIR-GAME subject above")
        w("")

    text = "\n".join(out) + "\n"
    if write:
        (ROOT / ".claude" / "subject-focus.md").write_text(text, encoding="utf-8")
    return text


if __name__ == "__main__":
    args = sys.argv[1:]
    subj = None
    if "--subject" in args:
        i = args.index("--subject")
        subj = args[i + 1] if i + 1 < len(args) else None
    sys.stdout.write(render(write="--write" in args, only_subject=subj, brief="--brief" in args))
