#!/usr/bin/env python3
"""
placement-audit.py — DETERMINISTIC audit of the doc + memory surfaces against
genesis/docs/PLACEMENT.md. No LLM. Three modes:

  (default)      scoreboard: lifecycle state, tech-debt, agent pressure, STRUCTURAL
                 EQUILIBRIUM (anti-dump: pressure-dir occupancy + runaway detection),
                 equilibrium verdict.
  --ledger       the BUDGET: every file → its position + state + next-action, instantly
                 auditable. Writes the full per-file ledger to .claude/memory-kit/state-ledger.json.
  --focus        the planner's currently-testable surface from cluster-state.yaml
                 (BLOCKED-BY-ENV held, not regressed); flip a resource → scope cascades.
                 [--cluster-state PATH]

Honest by design: prints its own caveats. 'verified' = evidence DECLARED, not CI-green
(actual green is the verification gate's job — ci-investigator).
"""
from __future__ import annotations

import json
import re
import sys
import time
from pathlib import Path

FLAGS = [a for a in sys.argv[1:] if a.startswith("--")]
ARGS = [a for a in sys.argv[1:] if not a.startswith("--")]
AS_JSON = "--json" in FLAGS
FOCUS = "--focus" in FLAGS
LEDGER = "--ledger" in FLAGS
HEADLINE = "--headline" in FLAGS
COVERAGE = "--coverage" in FLAGS
STASIS = "--stasis" in FLAGS
EPR_META = "--epr-meta" in FLAGS
CS_OVERRIDE = None
if "--cluster-state" in sys.argv[1:]:
    i = sys.argv.index("--cluster-state")
    if i + 1 < len(sys.argv):
        CS_OVERRIDE = sys.argv[i + 1]
        ARGS = [a for a in ARGS if a != CS_OVERRIDE]
# _lib bootstrap — env_scope is the gap-granular substrate-scope resolver (shared with decompose/scope-reconcile);
# cluster_state is the canonical cluster-state.yaml parser (shared with scope-reconcile/focus-baseline).
_here = Path(__file__).resolve()
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        break
    _here = _here.parent
from _lib import env_scope as _es  # noqa: E402
from _lib import paths as _paths  # noqa: E402
from _lib import cluster_state as _cs  # noqa: E402

ROOT = Path(ARGS[0]).resolve() if ARGS else _paths.repo_root_from_file(__file__)

SURFACES = {
    "ACTIVE:specs": "genesis/docs/superpowers/specs",
    "ACTIVE:plans": "genesis/docs/superpowers/plans",
    "ACTIVE:plans(legacy)": "genesis/docs/plans",
    "CANONICAL": "genesis/docs/content/elohim-protocol/architecture",
    "HISTORY": "genesis/docs/content/elohim-protocol/history",
    "NOTES": "genesis/docs/superpowers/notes",
    "RESEARCH": "genesis/docs/research",
}
ACTIVE_HOMES = {"ACTIVE:specs", "ACTIVE:plans", "ACTIVE:plans(legacy)"}
MEMORY_DIR = ".claude/memory"
STATE_ROOT = ROOT / "genesis/docs/_state"          # pressure dirs (should trend EMPTY)
RETIRED_DIR = ROOT / "genesis/docs/content/elohim-protocol/history/_retired"
DOCS_ROOT = ROOT / "genesis/docs"
LEDGER_OUT = ROOT / ".claude/memory-kit/state-ledger.json"
CLUSTER_STATE = Path(CS_OVERRIDE).resolve() if CS_OVERRIDE else (ROOT / "genesis/manifests/cluster-state.yaml")

LANDED_WORDS = {"landed", "stable", "done", "complete", "completed", "shipped", "accepted",
                "latest-stable"}
CLAIMED_WORDS = {"claimed-not-verified"}
REGRESSED_WORDS = {"regressed", "regression"}
DEAD_WORDS = {"superseded", "abandoned", "cancelled", "canceled", "deprecated", "retired"}
ACTIVE_WORDS = {"draft", "design", "brainstorm", "proposal", "proposed", "in-flight",
                "inflight", "wip", "vision", "approved", "accepted-draft", "active"}
# PLACEMENT.md §16: CANONICAL = "living cross-cutting truth (gospel-tier)" with tier: architecture.
# A canonical seed's status leads with one of these descriptive permanence words ("Living document",
# "Architecture pattern/principle") — it is the SETTLED steady state, not pressure. Recognized only
# when the doc's home is CANONICAL (a "living" status on an ACTIVE-home plan would still be a smell).
LIVING_WORDS = {"living", "architecture", "principle", "pattern"}
LINK_KEYS = ("canonical", "distills", "cites", "verified_by", "informed-by", "informs",
             "supersedes", "superseded-by", "supersedes-or-conforms-to",
             # a declared parent/related/spec IS a real anchor — a doc that names its lineage
             # is not orphaned; recognizing these keys removes a deterministic false-orphan class.
             "parent", "related", "spec", "source-spec", "roadmap")
MD_LINK_RE = re.compile(r"\]\(([^)\s]+?\.md)[^)]*\)")
MD_STATUS_RE = re.compile(r"^\*\*Status:\*\*\s*(.+?)\s*$", re.M)

# ---- STASIS = the loop's true "done". A composite STASIS SCORE benchmarks against an ideal of 1.0;
# "at stasis" = score within the ±MARGIN band (>= 1 - MARGIN). Each dimension is a COVERAGE RATIO
# (achieved / ideal, benchmark 1.0); BYTE dims pass at <= budget*(1+MARGIN); HARD dims (a dump is a
# dump) gate the score. The margin is tuned to the agentic developers' context capacity (a surface
# they can hold without burning the window).
STASIS_MARGIN = 0.15          # ±15% band; benchmark = 1.0 → at stasis when score >= 0.85
MEMORY_MD_BUDGET = 24000      # bytes — MEMORY.md (always-loaded index). The harness truncates the
                              # load at ~24.4KB, so the prior 46000 enforced 1.9× the real cap —
                              # aligned 2026-07-02 with memory-index-projector.py HARD_BUDGET_BYTES.
CLAUDE_MD_BUDGET = 12000      # bytes — a gate an agent should hold without burning context
# A LINK (every edge in the graph) = a PATH + a 1-2 sentence plain-text EXPLAINER of what's at the end.
# Bare paths don't count: the traceability dimensions require the explainer, so a reader spends context
# only on relevant links. History records (path + gotcha sentence) are the template.


def parse_front_matter(text: str) -> dict:
    fm: dict = {}
    if not text.startswith("---"):
        return fm
    end = text.find("\n---", 3)
    if end == -1:
        return fm
    cur = None
    for raw in text[3:end].splitlines():
        line = raw.rstrip()
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        m = re.match(r"^([A-Za-z0-9_\-]+):\s*(.*)$", line)
        if m and not line.startswith((" ", "\t")):
            cur = m.group(1).strip()
            val = m.group(2).strip()
            if val.startswith("["):
                inner = val[1:val.index("]")] if "]" in val else val[1:]
                fm[cur] = [x.split("#")[0].strip().strip("'\"") for x in inner.split(",") if x.strip()]
            elif val:
                fm[cur] = val.split("#")[0].strip().strip("'\"")
            else:
                fm[cur] = []
        else:
            li = re.match(r"^\s*-\s+(.*)$", line)
            if li and cur is not None and isinstance(fm.get(cur), list):
                fm[cur].append(li.group(1).split("#")[0].strip().strip("'\""))
    return fm


def requires_env_of(fm):
    v = fm.get("requires_env") or fm.get("requires-env") or []
    return v if isinstance(v, list) else [v]


def status_bucket(raw: str):
    w = (raw or "").strip().lower().split()[0] if raw else ""
    w = w.strip(":*").strip()
    if w in LANDED_WORDS:
        return "LANDED"
    if w in CLAIMED_WORDS:
        return "CLAIMED"
    if w in REGRESSED_WORDS:
        return "REGRESSED"
    if w in DEAD_WORDS:
        return "DEAD"
    if w in ACTIVE_WORDS:
        return "ACTIVE"
    if w in LIVING_WORDS:
        return "LIVING"
    return "UNKNOWN" if w else "NONE"


def load_cluster_state(path: Path):
    """(avail: {name -> raw available scalar}, updated: str). Delegates to the canonical
    `_lib.cluster_state` parser (agentic-context-tooling-consolidation-queue.md item 6) — this
    used to be an UNSCOPED hand-rolled regex walk with no `resources:` block-boundary check, so a
    decoy block BEFORE or AFTER `resources:` was folded in as a phantom resource.
    `avail` holds only resources that DECLARE `available:` (decision 2 in `_lib/cluster_state.py`'s
    docstring — a role-only block is not a runtime claim and must not gate the budget); this is
    also the `known` set the gap ledger passes to `env_scope.gap_blocked`.
    Pinned through THIS call site by `_lib/__tests__/cluster_state_test.py`."""
    state = _cs.load(path)
    return state.available_map(), state.updated


def load_manifest():
    """Read the context-coverage manifest (.claude/memory-kit/context-coverage.yaml) — the tuning
    surface. Returns {targets:{k:{value}}, dimensions:{k:{weight}}, exclusions:[...], hard:[...]}.
    Absent → empty (code defaults apply). The `why:` methodology blocks are for humans; ignored here."""
    p = ROOT / ".claude/memory-kit/context-coverage.yaml"
    cfg = {"targets": {}, "dimensions": {}, "exclusions": [], "hard": []}
    if not p.exists():
        return cfg
    section, key = None, None
    for raw in p.read_text().splitlines():
        if not raw.strip() or raw.lstrip().startswith("#"):
            continue
        indent = len(raw) - len(raw.lstrip(" "))
        s = raw.strip()
        if indent == 0:
            # strip any inline comment BEFORE the colon — `dimensions:   # …` must resolve to
            # `dimensions`, else the section never matches and its weights/exclusions are silently
            # dropped (DIMENSION_WEIGHTS == {} → every weight falls back to 1.0, defeating tuning).
            section, key = s.split("#", 1)[0].strip().rstrip(":"), None
        elif section in ("targets", "dimensions") and indent == 2 and s.endswith(":"):
            key = s[:-1].strip()
            cfg[section].setdefault(key, {})
        elif section in ("targets", "dimensions") and indent >= 4 and key:
            k, _, v = s.partition(":")
            if k.strip() in ("value", "weight"):
                try:
                    cfg[section][key][k.strip()] = float(v.strip().strip("'\""))
                except Exception:  # noqa: BLE001
                    pass
        elif section in ("exclusions", "hard") and s.startswith("- "):
            cfg[section].append(s[2:].strip().strip("'\""))
    return cfg


def _mfval(targets, k, default):
    return targets.get(k, {}).get("value", default)


# Override the deterministic-default constants FROM THE MANIFEST (the tuning surface). Edit the
# manifest, not this file, to retune — and the `why:` block beside each value explains the choice.
_MF = load_manifest()
STASIS_MARGIN = _mfval(_MF["targets"], "margin", STASIS_MARGIN)
MEMORY_MD_BUDGET = int(_mfval(_MF["targets"], "memory_md_budget", MEMORY_MD_BUDGET))
CLAUDE_MD_BUDGET = int(_mfval(_MF["targets"], "claude_md_budget", CLAUDE_MD_BUDGET))
DIMENSION_WEIGHTS = {k: v.get("weight", 1.0) for k, v in _MF["dimensions"].items()}


def scan_docs():
    """Every doc across the surfaces + the _state pressure dirs → records."""
    homes = dict(SURFACES)
    if STATE_ROOT.is_dir():
        for sub in sorted(STATE_ROOT.iterdir()):
            if sub.is_dir():
                homes[f"_state:{sub.name}"] = str(sub.relative_to(ROOT))
    basenames = set()
    for rel in homes.values():
        d = ROOT / rel
        if d.is_dir():
            for f in d.glob("*.md"):
                if f.name not in ("INDEX.md", "CLAUDE.md", "README.md"):
                    basenames.add(f.name)
    docs, link_targets, status_vals = [], set(), {}
    for home, rel in homes.items():
        d = ROOT / rel
        if not d.is_dir():
            continue
        for f in sorted(d.glob("*.md")):
            if f.name in ("INDEX.md", "CLAUDE.md", "README.md"):
                continue
            text = f.read_text(errors="replace")
            fm = parse_front_matter(text)
            raw = fm.get("status") or ""
            md = MD_STATUS_RE.search(text)
            if not raw and md:
                raw = md.group(1)
            bucket = status_bucket(raw if isinstance(raw, str) else "")
            if raw and isinstance(raw, str):
                status_vals[raw.strip()] = status_vals.get(raw.strip(), 0) + 1
            out = set(MD_LINK_RE.findall(text))
            for k in LINK_KEYS:
                v = fm.get(k)
                out.update(v if isinstance(v, list) else ([v] if isinstance(v, str) else []))
            outb = {Path(x).name for x in out if x.endswith(".md")}
            link_targets.update(b for b in outb if b in basenames and b != f.name)
            docs.append({"home": home, "name": f.name, "path": str(f.relative_to(ROOT)),
                         "bucket": bucket, "status": raw if isinstance(raw, str) else "",
                         "out": outb, "verified": bool(fm.get("verified_by") or fm.get("landed_commit")),
                         "requires_env": requires_env_of(fm),
                         "traces": fm.get("traces") if isinstance(fm.get("traces"), list) else [],
                         "has_canonical_link": ("canonical" in fm or any("/architecture/" in x for x in out))})
    for r in docs:
        r["inbound"] = r["name"] in link_targets
    return docs, status_vals


def scan_memory():
    d = ROOT / MEMORY_DIR
    out = []
    if d.is_dir():
        for f in sorted(d.glob("*.md")):
            if f.name == "MEMORY.md":
                continue
            fm = parse_front_matter(f.read_text(errors="replace"))
            out.append({"path": str(f.relative_to(ROOT)), "linked": bool(fm.get("cites")),
                        "requires_env": requires_env_of(fm)})
    return out


# ── STRAY-DOC CENSUS — the REMEDIAL leg (2026-06-26) ─────────────────────────
# The compose-gate (.epr-meta) is PREVENTIVE: it makes a new doc born governed, but it never
# reaches back to relocate a draft already parked outside a home, nor heals a redirect stub a
# prior move left behind. cleanup-scan and every other sweep only ever looked INSIDE the
# registered SURFACES — so the unscoped gap (repo-root *.md and the genesis/docs root, above
# the surface subdirs) was the one place residue could pile up invisibly. This census closes
# that gap; like the pressure dirs, it must trend to 0. Two zones, conventional docs allowed.
STRAY_ZONES = [("<root>", ""), ("genesis/docs", "genesis/docs")]
# Structural/conventional docs that legitimately live at a scanned root → never stray.
# Matched CASE-INSENSITIVELY: this repo uses a lowercase `claude.md` directory-doc pattern (a
# routing-referenced, non-auto-loaded nav doc — see _PROCESS_MARKERS in subject_routing.py, which
# lower()s before matching) alongside the uppercase gospel `CLAUDE.md`; both are legitimate.
# ROADMAP.md is deliberately ABSENT: this repo homes the roadmap under genesis/data/timeline/
# roadmap/ (the doc-lifecycle design's P0 retires the root stub), so a root ROADMAP.md is a
# REDIRECT-STUB to surface, not a home. A future intentional root redirect can be allow-listed.
STRAY_ALLOW = {
    "": {"README.md", "CLAUDE.md", "AGENTS.md", "GEMINI.md", "LICENSE.md",
         "CONTRIBUTING.md", "SECURITY.md", "CHANGELOG.md", "CODE_OF_CONDUCT.md"},
    "genesis/docs": {"PLACEMENT.md", "CLAUDE.md", "README.md"},
}
STUB_RE = re.compile(r"\b(moved|now lives|relocated|see instead|split into|"
                     r"preserved in git history|generated|do not edit|projected from)\b", re.I)


def scan_stray():
    """The unscoped gap: *.md parked at the repo root or the genesis/docs root — outside every
    registered SURFACE and not a conventional/structural doc. Each is classified REDIRECT-STUB
    (a short 'moved/see/generated' pointer — a half-cleanup breadcrumb) or STRAY-DRAFT (an
    ungoverned working draft that still owes a governed home). Residue → 0."""
    out = []
    for label, rel in STRAY_ZONES:
        d = (ROOT / rel) if rel else ROOT
        if not d.is_dir():
            continue
        allow = {a.lower() for a in STRAY_ALLOW.get(rel, set())}
        for f in sorted(d.glob("*.md")):
            if f.name.startswith(".") or f.name.lower() in allow:
                continue
            try:
                if not f.is_file():   # a dir named *.md, or a broken symlink — never crash the headline
                    continue
                text = f.read_text(errors="replace")
            except OSError:           # unreadable/permission/race — degrade like the sibling gates, don't blank SessionStart
                continue
            body = "\n".join(ln for ln in text.splitlines()
                             if ln.strip() and not ln.lstrip().startswith("#"))
            nlines = text.count("\n") + 1
            kind = "REDIRECT-STUB" if (nlines <= 25 and STUB_RE.search(body)) else "STRAY-DRAFT"
            out.append({"path": str(f.relative_to(ROOT)), "zone": label, "kind": kind})
    return out


def budget_state(rec, available):
    """A doc's instantly-auditable STATE + the next action to advance it.

    The directory a doc physically sits in IS its state (the state-machine-gen
    invariant): a doc in a `_state/<state>/` pressure dir is classified by its
    home, which is authoritative over frontmatter.
    """
    home = rec.get("home", "")
    if home == "_state:regression":
        return "REGRESSED", "fix and re-verify on an available env"
    if home == "_state:blockers":
        return "BLOCKED-BY-ENV", "restore env (cluster-state.yaml)"
    if home == "_state:unverified":
        return "CLAIMED-ONLY", "VERIFY (ci-investigator)"
    if home == "_state:needs-triage":
        return "NEEDS-TRIAGE", "classify: add status + place in graph"
    missing = [e for e in rec["requires_env"] if e not in available]
    if missing:
        return "BLOCKED-BY-ENV", f"restore env: {','.join(missing)} (cluster-state.yaml)"
    b = rec["bucket"]
    if b == "NONE":
        return "NEEDS-TRIAGE", "classify: add status + place in graph"
    if b == "DEAD":
        return "SUPERSEDED", "distill → history, retire body"
    if b == "REGRESSED":
        return "REGRESSED", "fix and re-verify on an available env"
    if b == "CLAIMED":
        return "CLAIMED-ONLY", "VERIFY (ci-investigator)"
    if b == "LANDED":
        # CANONICAL / HISTORY are the settled destinations (PLACEMENT.md): an `accepted`/`landed` status
        # there is the steady state, not a deliverable awaiting CI. Only ACTIVE-home landed-without-evidence
        # is the over-claim that needs the verification gate.
        if home in ("CANONICAL", "HISTORY"):
            return "SETTLED", "settled canon/history — no verification queue"
        return ("VERIFIED-STABLE", "retire-eligible → history") if rec["verified"] else ("CLAIMED-ONLY", "VERIFY (ci-investigator)")
    if b == "ACTIVE":
        return "ACTIVE", "in-flight"
    if b == "LIVING":
        # Canonical living-truth seed (PLACEMENT.md §16). Settled iff it sits in its canonical home;
        # a "living"-status doc loitering in an ACTIVE plan/spec dir is a real placement smell.
        if home == "CANONICAL":
            return "SETTLED", "living canonical seed — no verification queue"
        return "UNKNOWN-STATUS", "living status outside canonical home — re-place or re-status"
    return "UNKNOWN-STATUS", "normalize the status string"


# ----- modes -----------------------------------------------------------------
def iter_env_requirers():
    for rel in list(SURFACES.values()) + [MEMORY_DIR]:
        d = ROOT / rel
        if d.is_dir():
            for f in sorted(d.glob("*.md")):
                if f.name in ("INDEX.md", "CLAUDE.md"):
                    continue
                envs = requires_env_of(parse_front_matter(f.read_text(errors="replace")))
                if envs:
                    yield f, envs


def focus_mode():
    avail, updated = load_cluster_state(CLUSTER_STATE)
    available = {k for k, v in avail.items() if v == "true"}
    unavailable = {k: v for k, v in avail.items() if v != "true"}
    in_scope, blocked = [], []
    for f, envs in iter_env_requirers():
        miss = [e for e in envs if e not in available]
        (blocked if miss else in_scope).append((f, envs, miss))
    print(f"FOCUS — currently-testable surface   (cluster-state: {CLUSTER_STATE.name} @ {updated})")
    print("=" * 72)
    print("  AVAILABLE   : " + (", ".join(sorted(available)) or "(none)"))
    print("  UNAVAILABLE : " + (", ".join(f"{k}({v})" for k, v in sorted(unavailable.items())) or "(none)"))
    print(f"\n  IN SCOPE for refinement-stability ({len(in_scope)}):")
    for f, e, _ in in_scope:
        print(f"    ✅ {f.relative_to(ROOT)}   requires_env: {e}")
    if not in_scope:
        print("    (none declared)")
    print(f"\n  BLOCKED-BY-ENV ({len(blocked)}) — HELD, out of scope now, NOT regressed:")
    for f, e, miss in blocked:
        print(f"    ⛔ {f.relative_to(ROOT)}   needs {e} → missing {miss}")
    if not blocked:
        print("    (none)")
    print("\n  NOTE: this lists only .md docs that DECLARE requires_env. The a2o TEST SURFACE (per-subject")
    print("  in-focus vs held features/scenarios) is the generated baseline → .claude/subject-focus.md")
    print("    drill in: python3 .claude/scripts/memory-kit/focus-baseline.py [--subject <name>]")
    print("\n  → Edit cluster-state.yaml and re-run: scope cascades immediately.")
    return 0


def _ledger_doc_req(rec):
    """The doc-level requires_env default for a gap-cache record — read from the doc's LIVE frontmatter
    (resolving its current path; the cached `doc` may be stale after a held<->live move), so the budget can't
    drift from reality. Falls back to the cached doc_requires_env if the doc can't be located."""
    ref = rec.get("doc", "")
    p = (ROOT / ref) if ref else None
    if ref and (not p or not p.is_file()):
        matches = list(DOCS_ROOT.rglob(Path(ref).name))
        p = matches[0] if matches else None
    if p and p.is_file():
        try:
            return _es.parse_requires_env(parse_front_matter(p.read_text(encoding="utf-8", errors="replace")).get("requires_env"))
        except OSError:
            pass
    return rec.get("doc_requires_env", [])


def ledger_mode():
    avail, _ = load_cluster_state(CLUSTER_STATE)
    available = {k for k, v in avail.items() if v == "true"}
    docs, _ = scan_docs()
    mem = scan_memory()
    rows = []
    for d in docs:
        st, nxt = budget_state(d, available)
        rows.append({"path": d["path"], "position": d["home"], "state": st, "next": nxt})
    for m in mem:
        miss = [e for e in m["requires_env"] if e not in available]
        if miss:
            rows.append({"path": m["path"], "position": "MEMORY", "state": "BLOCKED-BY-ENV", "next": f"restore {','.join(miss)}"})
        elif m["linked"]:
            rows.append({"path": m["path"], "position": "MEMORY", "state": "LINKED", "next": "—"})
        else:
            rows.append({"path": m["path"], "position": "MEMORY", "state": "MEM-UNLINKED", "next": "add cites: to a system"})

    LEDGER_OUT.parent.mkdir(parents=True, exist_ok=True)
    LEDGER_OUT.write_text(json.dumps({"total_files": len(rows), "rows": rows}, indent=1, sort_keys=True))
    if AS_JSON:
        print(json.dumps({"total_files": len(rows), "rows": rows}, indent=1, sort_keys=True))
        return 0

    bal = {}
    for r in rows:
        bal[r["state"]] = bal.get(r["state"], 0) + 1
    settled = {"VERIFIED-STABLE", "ACTIVE", "LINKED", "CANONICAL"}
    # BLOCKED-BY-ENV is HELD (Cascade: NONE, out-of-current-scope per PLACEMENT.md) —
    # not the doc's fault, so it is NOT counted as actionable pressure.
    pressure_order = ["NEEDS-TRIAGE", "MEM-UNLINKED", "CLAIMED-ONLY", "REGRESSED",
                      "SUPERSEDED", "UNKNOWN-STATUS"]
    total = len(rows)
    print(f"BUDGET LEDGER — every file accounted (position + state)   total files: {total}")
    print("=" * 72)
    print("  BALANCE (sums to total — nothing hides):")
    for st in sorted(bal, key=lambda s: (-bal[s], s)):
        mark = "·" if st in settled else "▶"
        print(f"    {mark} {st:<16} {bal[st]:>4}  ({100*bal[st]//total:>2}%)")
    need = sum(bal.get(s, 0) for s in pressure_order)
    held = bal.get("BLOCKED-BY-ENV", 0)
    print(f"  {'-'*4}\n    PRESSURE (needs action): {need}   HELD (not pressure): {held}   SETTLED: {total-need-held}")

    print("\n  PRESSURE QUEUE (what to do, by state):")
    for st in pressure_order:
        items = [r for r in rows if r["state"] == st]
        if not items:
            continue
        print(f"    ▶ {st} ({len(items)}) — {items[0]['next']}")
        for r in items[:4]:
            print(f"        {r['position']:<18} {r['path']}")
        if len(items) > 4:
            print(f"        … +{len(items)-4} more")
    # decomposed gap-items (from decompose.py) — the implementation budget
    gap_dir = ROOT / ".claude/memory-kit/gap-items"
    gfiles = sorted(gap_dir.glob("*.json")) if gap_dir.is_dir() else []
    if gfiles:
        # GAP-GRANULAR scope (honors iroh ≠ shem): each gap resolves requires_env (own override, else the
        # doc-level default), and is BLOCKED-BY-ENV iff a cluster-tracked required cap is unavailable —
        # regardless of whether its parent doc sits in held/. A blocked gap is OUT of the active budget; a
        # household-testable gap inside a mixed/held doc stays pickable. See _lib/env_scope.py.
        known = set(avail.keys())
        g_open = g_claim = g_blocked = 0
        rows_g = []
        for gf in gfiles:
            try:
                rec = json.loads(gf.read_text())
            except Exception:  # noqa: BLE001
                continue
            doc_req = _ledger_doc_req(rec)
            o = c = b = 0
            for it in rec.get("items", []):
                resolved = _es.resolved_requires_env(it.get("requires_env"), doc_req)
                if _es.gap_blocked(resolved, available, known):
                    b += 1
                elif it.get("state") == "OPEN":
                    o += 1
                elif it.get("state") == "CLAIMED":
                    c += 1
            g_open += o
            g_claim += c
            g_blocked += b
            if o or c or b:
                rows_g.append((rec.get("doc", gf.stem), o, c, b))
        bl = f", {g_blocked} BLOCKED-BY-ENV (held — gap needs an unavailable cap)" if g_blocked else ""
        print(f"\n  DECOMPOSED GAPS ({len(rows_g)} docs with items, {len(gfiles)} files scanned): "
              f"{g_open} OPEN to implement, {g_claim} CLAIMED to verify (checked ≠ done){bl}")
        for doc, o, c, b in rows_g[:8]:
            held = f" / {b:>2} held" if b else ""
            print(f"      {o:>3} open / {c:>3} claimed{held}   {doc}")
        if len(rows_g) > 8:
            print(f"      … +{len(rows_g)-8} more")

    print(f"\n  Full per-file ledger written: {LEDGER_OUT.relative_to(ROOT)}  (--json for stdout)")
    return 0


def equilibrium_section():
    """Structural anti-dump: pressure-dir occupancy + runaway detection."""
    print(f"\n{'='*72}\nSTRUCTURAL EQUILIBRIUM (anti-dump)\n{'='*72}")
    pressure_occ = {}
    if STATE_ROOT.is_dir():
        for sub in sorted(STATE_ROOT.iterdir()):
            if sub.is_dir():
                pressure_occ[sub.name] = len([f for f in sub.glob("*.md") if f.name not in ("CLAUDE.md", "README.md")])
    print("  PRESSURE DIRS (equilibrium = 0 docs each):")
    for name, n in pressure_occ.items():
        print(f"    {name:<14} {n:>4} docs   " + ("✅ empty (stable)" if n == 0 else "⚠ PRESSURE"))
    if not pressure_occ:
        print("    (none generated yet — run state-machine-gen.py)")
    # runaway detection
    depths, biggest = [], (None, 0)
    if DOCS_ROOT.is_dir():
        for d in sorted(DOCS_ROOT.rglob("*")):
            # scope-tree held/ docs are sequestered out of the scan path — they
            # must not count toward the structural anti-dump / runaway measure.
            if d.is_dir() and "held" not in d.relative_to(DOCS_ROOT).parts:
                depth = len(d.relative_to(DOCS_ROOT).parts)
                depths.append((depth, d))
                n = len([f for f in d.glob("*.md")])
                if n > biggest[1] or (n == biggest[1] and (biggest[0] is None or str(d) < str(biggest[0]))):
                    biggest = (d, n)
    maxdepth = max((x[0] for x in depths), default=0)
    deep = [str(d.relative_to(ROOT)) for dp, d in depths if dp >= 5]
    retired_n = len(list(RETIRED_DIR.glob("*.md"))) if RETIRED_DIR.is_dir() else 0
    print("  RUNAWAY RISK:")
    print(f"    deepest doc dir depth: {maxdepth}   " + (f"⚠ deep dirs: {deep[:3]}" if deep else "✅ shallow"))
    if biggest[0]:
        print(f"    largest single dir: {biggest[0].relative_to(ROOT)} ({biggest[1]} docs)  — watch for sprawl")
    print(f"    archive (_retired): {retired_n} files   " + ("✅ no dump" if retired_n == 0 else "— verify all are verified-stable"))
    return all(n == 0 for n in pressure_occ.values()) if pressure_occ else False


def decompose_coverage(surface_docs):
    """How many ACTIVE specs/plans have actually been REVIEWED (decomposed)?

    This is the un-captured workload — the backlog the loop must drain. Without it the
    loop could declare 'stasis' while 160 plans sit un-reviewed; with it, the loop keeps
    dispatching until coverage is complete, so effort is proportional to the REAL backlog.
      captured     = a gap-items/<slug>.json exists with extracted items
      needs_agent  = decompose ran but found no structure (prose spec) → an agent must extract
      undecomposed = never decomposed at all
    """
    gap_dir = ROOT / ".claude/memory-kit/gap-items"
    active = [d for d in surface_docs if d["home"] in ACTIVE_HOMES]
    captured, needs_agent, undecomposed, un_list = 0, 0, 0, []
    for d in active:
        p = Path(d["path"])
        gf = gap_dir / f"{p.parent.name}__{p.stem}.json"
        if not gf.exists():
            undecomposed += 1
            un_list.append((d["path"], "undecomposed"))
            continue
        try:
            rec = json.loads(gf.read_text())
        except Exception:  # noqa: BLE001
            rec = {}
        if rec.get("method") == "none" or not rec.get("items"):
            needs_agent += 1
            un_list.append((d["path"], "needs-agent"))
        else:
            captured += 1
    return {"active": len(active), "captured": captured, "needs_agent": needs_agent,
            "undecomposed": undecomposed, "uncaptured": undecomposed + needs_agent, "un_list": un_list}


def coverage_mode():
    docs, _ = scan_docs()
    surface = [d for d in docs if not d["home"].startswith("_state")]
    cov = decompose_coverage(surface)
    if AS_JSON:
        print(json.dumps({"active": cov["active"], "captured": cov["captured"],
                          "needs_agent": cov["needs_agent"], "undecomposed": cov["undecomposed"],
                          "uncaptured": cov["uncaptured"],
                          "uncaptured_docs": [{"path": p, "why": w} for p, w in cov["un_list"]]}, indent=1))
        return 0
    print("DECOMPOSE-COVERAGE — the un-reviewed / un-captured workload")
    print("=" * 72)
    print(f"  active specs+plans          : {cov['active']}")
    print(f"  ✅ decomposed (captured)     : {cov['captured']}")
    print(f"  ▶ un-decomposed (never reviewed): {cov['undecomposed']}")
    print(f"  ▶ needs-agent (prose, no structure): {cov['needs_agent']}")
    print(f"  {'-'*4}")
    print(f"  UN-CAPTURED WORKLOAD        : {cov['uncaptured']}   "
          f"(the loop dispatches until this hits 0 → effort ∝ real backlog)")
    print("\n  the loop's queue (next un-captured):")
    for p, w in cov["un_list"][:14]:
        print(f"    [{w:<12}] {p}")
    if len(cov["un_list"]) > 14:
        print(f"    … +{len(cov['un_list'])-14} more")
    return 0


def trace_coverage(active_docs):
    """% of active specs/plans carrying >=1 EXPLAINED trace link: a `traces:` entry of the form
    '<path> — <1-2 sentence explainer>' (em-dash / `--` / `|` separates path from explainer). Bare
    paths do NOT count (the link rule). Starts near 0 until explained traces roll out; the loop drives it up."""
    if not active_docs:
        return 1.0
    explained = 0
    for d in active_docs:
        for t in (d.get("traces") or []):
            if isinstance(t, str) and any(sep in t for sep in (" — ", " -- ", " | ")):
                explained += 1
                break
    return explained / len(active_docs)


def _ratio(num, den):
    return 1.0 if not den else num / den


def stasis_mode():
    docs, _ = scan_docs()
    surface = [d for d in docs if not d["home"].startswith("_state")]
    active = [d for d in surface if d["home"] in ACTIVE_HOMES]
    mem = scan_memory()
    cov = decompose_coverage(surface)
    stated = sum(1 for d in active if d["bucket"] != "NONE")
    nonorphan = sum(1 for d in active if d["inbound"] or d["out"])
    hist = [d for d in surface if d["home"] == "HISTORY"]
    hist_ok = sum(1 for d in hist if d["has_canonical_link"])
    mem_linked = sum(1 for m in mem if m["linked"])
    cmds = []
    for pat in ("CLAUDE.md", "genesis/**/CLAUDE.md", ".claude/**/CLAUDE.md"):
        cmds += [p for p in ROOT.glob(pat) if "worktrees" not in p.parts and "node_modules" not in p.parts]
    cmds = sorted(set(cmds))
    cmd_ok = sum(1 for p in cmds if p.stat().st_size <= CLAUDE_MD_BUDGET)
    mmd = ROOT / MEMORY_DIR / "MEMORY.md"
    mmd_bytes = mmd.stat().st_size if mmd.exists() else 0
    dumps = 0
    if RETIRED_DIR.is_dir():
        for f in RETIRED_DIR.glob("*.md"):
            fm = parse_front_matter(f.read_text(errors="replace"))
            if not (fm.get("verified_by") or fm.get("verification") or fm.get("landed_commit")):
                dumps += 1
    pressure_docs = 0
    if STATE_ROOT.is_dir():
        for sub in STATE_ROOT.iterdir():
            if sub.is_dir():
                pressure_docs += len([f for f in sub.glob("*.md") if f.name not in ("CLAUDE.md", "README.md")])

    measured = [
        ("capture: specs/plans decomposed", _ratio(cov["captured"], cov["active"])),
        ("status: docs carry a state", _ratio(stated, len(active))),
        ("well-formed: docs linked (non-orphan)", _ratio(nonorphan, len(active))),
        ("memory: entries cite a system", _ratio(mem_linked, len(mem))),
        ("CLAUDE.md right-sized (<= budget)", _ratio(cmd_ok, len(cmds))),
        ("history: bidirectional canonical link", _ratio(hist_ok, len(hist)) if hist else 1.0),
        ("traceability: docs carry explained trace links", trace_coverage(active)),
        ("MEMORY.md within byte budget", 1.0 if mmd_bytes <= MEMORY_MD_BUDGET * (1 + STASIS_MARGIN) else MEMORY_MD_BUDGET / max(mmd_bytes, 1)),
        ("epr-meta: substantial dirs owned by an .epr-meta", epr_meta_coverage_data()["ratio"]),
    ]
    hard = [("_retired dumps == 0", dumps == 0), ("pressure dirs empty", pressure_docs == 0)]
    unmeasured = [
        "code-FILE-level traceability (.ts/.rs → story/scenario) — doc-level traces measured above; per-code-file convention not yet wired",
        "gap-items → testing-infrastructure tagging (NOT wired)",
        "CLAUDE.md recursive coverage: every dir needing a gate has one (run claude-md-audit.py)",
    ]
    thr = 1 - STASIS_MARGIN
    # weighted by the manifest's per-dimension weights (KEYS aligned with `measured` order)
    KEYS = ["capture", "status", "well_formed", "memory_linked", "claude_md_rightsized",
            "history_bidirectional", "traceability", "memory_md_budget", "epr_meta_coverage"]
    weights = [DIMENSION_WEIGHTS.get(k, 1.0) for k in KEYS]
    wsum = sum(weights) or 1.0
    score = sum(r * w for (_, r), w in zip(measured, weights)) / wsum
    hard_ok = all(ok for _, ok in hard)
    at_stasis = score >= thr and hard_ok

    if AS_JSON:
        # The composite as a DECLARED measure. It is a weighted mean of coverage ratios, so
        # unlike drift_score it is dimensionally AND commensurably coherent — every term is a
        # ratio of the same kind of thing (covered ÷ total). What it still cannot claim is a
        # tight interval: the weights are a tuning choice, and averaging ratios over different
        # denominators is a known aggregation trap (a dimension covering 4 items and one
        # covering 400 contribute equally at equal weight). So the band is the spread of the
        # contributing dimensions — honest about the disagreement it is averaging away — and the
        # claim is `modelled` because a human chose the weights.
        rs = [r for _, r in measured]
        payload = {"benchmark": 1.0, "margin": STASIS_MARGIN, "threshold": round(thr, 3),
                   "score": round(score, 3), "hard_ok": hard_ok, "at_stasis": at_stasis,
                   "dimensions": {n: round(r, 3) for n, r in measured},
                   "unmeasured": unmeasured}
        try:
            from _lib import signal_measure as _sm
            payload["measure"] = _sm.measure(
                round(score, 3), "ratio", claim="modelled",
                interval={"lo": round(min(rs), 3), "hi": round(max(rs), 3)} if rs else None,
                basis=(f"weighted mean of {len(measured)} coverage ratios (weights from "
                       f"context-coverage.yaml); interval is the spread across contributing "
                       f"dimensions, NOT a sampling band. {len(unmeasured)} dimensions are "
                       f"UNMEASURED and excluded entirely — full stasis cannot be claimed while "
                       f"they are unwired, so this score is an upper bound on what is known"))
        except Exception:  # noqa: BLE001 — the scoreboard never dies on its own declaration
            pass
        print(json.dumps(payload, indent=1))
        return 0

    print(f"STASIS — composite score vs benchmark 1.0, ±{int(STASIS_MARGIN*100)}% band (at stasis ≥ {thr:.2f})")
    print("=" * 72)
    for name, r in measured:
        print(f"  {'✅' if r >= thr else '❌'} {name:<40} {r*100:5.1f}%")
    for name, ok in hard:
        print(f"  {'✅' if ok else '❌'} {name:<40} {'pass' if ok else 'FAIL — gates the score'}")
    print("  " + "-" * 40)
    print(f"  STASIS SCORE (measured): {score:.3f} / 1.000   →  "
          f"{'✅ AT STASIS' if at_stasis else f'❌ NOT at stasis (need ≥ {thr:.2f})'}")
    print(f"\n  ⏳ UNMEASURED — full stasis CANNOT be claimed until these are wired (checked ≠ done):")
    for u in unmeasured:
        print(f"    ? {u}")
    return 0


def decompose_due_count():
    """The BACK fire point's tripwire count: ACTIVE-home docs that carry a terminal
    status (landed | superseded | abandoned) yet still live plan-shaped — i.e. they
    are PAST-DUE to decompose to zero residue. Read from the placement-drift accumulator
    (.claude/memory-kit/placement-drift.json) populated by the placement-drift-signal.py
    PostToolUse hook. Absent/malformed store → 0 (graceful: the hook may not have fired yet).
    Stale entries whose doc no longer carries a terminal status (or no longer exists) are
    skipped, so the count reflects current reality, not historical signal."""
    p = ROOT / ".claude/memory-kit/placement-drift.json"
    try:
        store = json.loads(p.read_text())
    except (OSError, ValueError):
        return 0
    due = store.get("due") if isinstance(store, dict) else None
    if not isinstance(due, dict):
        return 0
    terminal = {"superseded", "abandoned", "cancelled", "canceled", "deprecated", "retired",
                "landed", "stable", "done", "complete", "completed", "shipped", "accepted",
                "latest-stable"}
    n = 0
    for rel in due:
        f = ROOT / rel
        if not f.is_file():
            continue
        fm = parse_front_matter(f.read_text(errors="replace"))
        raw = fm.get("status") or ""
        md = MD_STATUS_RE.search(f.read_text(errors="replace"))
        if not raw and md:
            raw = md.group(1)
        w = (raw or "").strip().lower().split()
        if w and w[0].strip(":*\"'") in terminal:
            n += 1
    return n


def _date_of_doc(path: Path):
    """Best 'last refreshed' date for a living doc: frontmatter `updated:` wins, then
    `created:`, then filesystem mtime. Returns a YYYY-MM-DD string or None. Honest: the
    MAP/roadmap are uncommitted living docs, so frontmatter/mtime beats git for currency."""
    if not path.is_file():
        return None
    try:
        fm = parse_front_matter(path.read_text(errors="replace"))
    except OSError:
        fm = {}
    for k in ("updated", "created", "date"):
        v = fm.get(k)
        if isinstance(v, str) and v.strip():
            return v.strip()[:10]
    try:
        return time.strftime("%Y-%m-%d", time.gmtime(path.stat().st_mtime))
    except OSError:
        return None


def map_currency_line():
    """The LEGIBILITY/PATH + PRIORITIZATION/ROADMAP currency readout for the headline.

    path:    N architecture seed(s) changed since the MAP walk was last refreshed,
             read from the map-currency-drift.json accumulator (populated by the
             map-drift-signal.py PostToolUse hook). The accumulator resets to 0 when
             MAP.md itself is edited, so N is the in-flight staleness of the WALK vs
             the seeds (territory). Absent/malformed store → 0 (graceful: hook may not
             have fired yet).
    roadmap: when the roadmap synthesis was last refreshed (the PRIORITIZATION axis).
             Sourced from the canonical roadmap artifact's date (frontmatter/mtime)."""
    store_p = ROOT / ".claude/memory-kit/map-currency-drift.json"
    n, last_refresh = 0, None
    try:
        store = json.loads(store_p.read_text())
        if isinstance(store, dict):
            changed = store.get("changed")
            if isinstance(changed, dict):
                # only count seeds that still exist on disk (graceful self-correction)
                n = sum(1 for rel in changed if (ROOT / rel).is_file())
            lr = store.get("last_map_refresh")
            if isinstance(lr, str) and lr:
                last_refresh = lr[:10]
    except (OSError, ValueError):
        pass
    # MAP.md date (the walk) — prefer the accumulator's recorded refresh, else the doc itself
    map_date = last_refresh or _date_of_doc(
        ROOT / "genesis/docs/content/elohim-protocol/architecture/MAP.md")
    # roadmap date (the prioritization synthesis) — the canonical, maintained roadmap
    # artifact the commands send developers to (the PATH axis tracks canonical MAP.md;
    # the PRIORITIZATION axis tracks this canonical artifact, not the cartographer's
    # scratch synthesis at .claude/memory-kit/Q2-emerging-roadmap.md it draws from).
    roadmap_date = _date_of_doc(
        ROOT / "genesis/data/timeline/roadmap/vision-readiness-sprint-roadmap.md")
    since = f" since MAP update ({map_date})" if map_date else " since MAP update"
    return (f"  path: {n} seed{'' if n == 1 else 's'} changed{since}"
            f"   |   roadmap: refreshed {roadmap_date or '—'}"
            f"   ({'walk current ✅' if n == 0 else '⚠ MAP.md may be stale vs seeds → refresh the walk'})")


# ── .epr-meta governance coverage — the directory-level meta-behavior dimension. Reuses the
# subtree-coverage walk in _lib/epr_meta.py (a `covers: subtree` manifest is fully responsible for
# its subtree → the walk terminates there; a substantial unclaimed dir is a GAP). The downward dual
# of claude-md-audit's missing-CLAUDE.md census: that asks "does every complex dir front-load init
# context?", this asks "is every substantial region OWNED by a self-responsible .epr-meta?". ──
def epr_meta_coverage_data() -> dict:
    """Run the subtree-coverage census with the repo's tuned thresholds + submodule/yaml exclusions —
    the SINGLE config source `_lib/epr_meta.governance_cfg`, shared with the in-flight resolver hook so
    the descending census and the ascending nudge can never disagree about which dirs are governable."""
    from _lib import epr_meta as _em  # _lib is on sys.path via the bootstrap above
    c = _em.governance_cfg(ROOT)
    return _em.subtree_coverage(ROOT, min_files=c["min_files"], min_subdirs=c["min_subdirs"],
                                min_exts=c["min_exts"], exclude_globs=c["exclude_globs"])


def epr_meta_measure_data() -> dict:
    """The measure-rule census (LoC ceilings): the batch dual of the resolver's per-edit measure
    verdict, from the same policy registry + cascade semantics. {"hard": rows, "soft": rows}."""
    from _lib import epr_meta as _em
    c = _em.governance_cfg(ROOT)
    return _em.measure_census(ROOT, exclude_globs=c["exclude_globs"])


def epr_meta_line() -> str:
    """Headline token: are the codebase's substantial directories OWNED by a self-responsible
    .epr-meta? `covers: subtree` to claim a region. Carries the measure-census clause when any
    file sits at/over its LoC hard ceiling. Absent/error → graceful gate-liveness note."""
    try:
        d = epr_meta_coverage_data()
    except Exception as e:  # noqa: BLE001
        return f"  epr-meta: ⚠ gate-error ({type(e).__name__})"
    arch = ""
    try:
        n_hard = len(epr_meta_measure_data()["hard"])
        if n_hard:
            arch = f" · ⚠ {n_hard} file(s) ≥ LoC hard ceiling (arch-review queue)"
    except Exception:  # noqa: BLE001 — the measure clause must never kill the coverage line
        arch = " · measure-census ⚠ gate-error"
    total = d["covered_count"] + d["gap_count"]
    if total == 0:
        return f"  epr-meta: no governable regions ✅{arch}"
    if d["gap_count"] == 0:
        return (f"  epr-meta: {d['covered_count']}/{total} regions claimed"
                f"   (all governable context owned ✅){arch}")
    return (f"  epr-meta: {d['covered_count']}/{total} regions claimed"
            f"   (⚠ {d['gap_count']} unclaimed → `placement-audit.py --epr-meta` to remediate){arch}")


def epr_meta_mode() -> int:
    """The remediation queue: every substantial directory not yet owned by a `covers: subtree`
    manifest. Each row is resolved by ONE placement — an `.epr-meta` (with rules, or with none and a
    `why:` recording the considered decision) declaring `covers: subtree` at/above that altitude."""
    d = epr_meta_coverage_data()
    total = d["covered_count"] + d["gap_count"]
    if AS_JSON:
        print(json.dumps(d, indent=1))
        return 0
    print(f"EPR-META GOVERNANCE COVERAGE — {d['covered_count']}/{total} substantial regions owned "
          f"(ratio {d['ratio']:.3f})")
    print("=" * 72)
    print("A region is OWNED when an `.epr-meta` at/above it declares `covers: subtree` (it is then")
    print("fully responsible for its subtree — integrity by construction, never re-audited). Resolve")
    print("each gap with ONE placement: rules where there's a mechanizable drift, or a rules-free")
    print("manifest with a `why:` recording the considered 'no edit-time gate needed' decision.\n")
    if d["covered"]:
        print(f"OWNED regions ({d['covered_count']}):")
        for c in d["covered"]:
            print(f"  ✓ {c}")
        print()
    if d["gaps"]:
        print(f"UNCLAIMED regions ({d['gap_count']}) — the remediation queue, by file-count:")
        for g in sorted(d["gaps"], key=lambda g: -g["files"]):
            print(f"  {g['files']:>4}f {g['subdirs']:>2}d  {g['path']:<54} {g['exts']}")
    else:
        print("All governable regions are owned ✅")
    try:
        m = epr_meta_measure_data()
    except Exception as e:  # noqa: BLE001
        print(f"\nMEASURE CENSUS: ⚠ gate-error ({type(e).__name__})")
        return 0
    if m["hard"] or m["soft"]:
        print(f"\nMEASURE CENSUS (LoC ceilings from the policy registry, "
              f"{len(m['hard'])} hard / {len(m['soft'])} soft):")
        for r in m["hard"]:
            print(f"  ⛔ {r['loc']:>6} ≥{r['hard']:>5}  {r['path']}   [{r['rule']}] → architecture-"
                  f"review queue (modularization plan in genesis/data/timeline/backlog/)")
        for r in m["soft"]:
            print(f"  ⚠ {r['loc']:>6} ≥{r['soft']:>5}  {r['path']}   [{r['rule']}] soft — growth "
                  f"pressure; new logic likely belongs in a module")
        for e in m.get("errors", []):
            print(f"  ! {e}")
    # INTERVENOR CENSUS: how many governance mechanisms have no way to end (Meadows'
    # shifting-the-burden-to-the-intervenor trap). Lands HERE, on a surface that already has a
    # reader, rather than as a new SessionStart line — a census of un-retired instruments that
    # minted its own always-on surface would be the joke telling itself.
    try:
        from _lib import intervenor_census as _ic
        print("\n" + _ic.render_census(_ic.census_data(ROOT)))
    except Exception as e:  # noqa: BLE001 — instrument liveness: say the gate died, never vanish
        print(f"\nINTERVENOR CENSUS: ⚠ gate-error ({type(e).__name__}: {e})")
    # DECISION-POINT CENSUS (plan P3.2): the derived read-model over every seam-registry.yaml
    # plus both concern-canon homes (policies.yaml + concerns.yaml). Thin wiring only — all
    # logic lives in _lib/seam_census.py.
    try:
        from _lib import seam_census as _sc
        print("\n" + _sc.render_census(_sc.census_data(ROOT)))
    except Exception as e:  # noqa: BLE001 — instrument liveness: say the gate died, never vanish
        print(f"\nDECISION-POINT CENSUS: ⚠ gate-error ({type(e).__name__})")
    # CONCERN x SEAM MATRIX (P4.3) + CASCADE (P4.2) + FORECAST (P4.4). Thin wiring only; every
    # gate is independently try-wrapped so one dead instrument never takes the others with it,
    # and each says so out loud rather than vanishing (the 2026-06-09 scope-reconcile lesson).
    # READ-ONLY here by construction: this is a scoreboard. Filing cascade findings, regenerating
    # the matrix artifacts and appending to the calibration ledger all live behind explicit verbs
    # in `.claude/scripts/seam-audit.py`.
    matrix = None
    try:
        from _lib import seam_matrix as _sm
        matrix = _sm.matrix_data(ROOT)
        print("\n" + _sm.render_matrix(matrix))
        state, detail = _sm.generated_freshness(ROOT, matrix)
        if state != "fresh":
            print(f"\n  generated artifact: ⚠ {state} ({detail})"
                  f" → `python3 .claude/scripts/seam-audit.py --matrix --write`")
    except Exception as e:  # noqa: BLE001
        print(f"\nCONCERN x SEAM MATRIX: ⚠ gate-error ({type(e).__name__}: {e})")
    try:
        from _lib import seam_cascade as _casc
        print("\n" + _casc.render_cascade(_casc.cascade_data(ROOT, matrix=matrix)))
    except Exception as e:  # noqa: BLE001
        print(f"\nCASCADE: ⚠ gate-error ({type(e).__name__}: {e})")
    try:
        from _lib import seam_forecast as _fc
        print("\n" + _fc.render_forecast(ROOT, matrix=matrix))
    except Exception as e:  # noqa: BLE001
        print(f"\nFORECAST: ⚠ gate-error ({type(e).__name__}: {e})")
    return 0


def _gate_subprocess(gate: str, script: str, flag: str, timeout: int = 15) -> str:
    """Run a headline gate's backing script with INSTRUMENT LIVENESS: a gate that
    crashes must surface as `⚠ gate-error`, never silently vanish. (Incident
    2026-06-09: scope-reconcile crashed on a malformed gap-item for days and the
    `scope:` line simply disappeared from SessionStart — a dead instrument read
    as 'nothing to report'. A vanished gate line is itself a finding.)

    Contract: rc==0 + stdout → the line; rc==0 + empty stdout → "" (legitimate
    silence, gate chose to say nothing); rc!=0 → ⚠ gate-error with the first
    stderr line; exception/timeout → ⚠ gate-error with the exception class."""
    import subprocess
    try:
        out = subprocess.run(
            [sys.executable, str(Path(__file__).parent / script), flag],
            capture_output=True, text=True, timeout=timeout)
        if out.returncode != 0:
            err = (out.stderr or "").strip().splitlines()
            tail = err[-1][:90] if err else f"exit {out.returncode}"
            return f"  {gate}: ⚠ gate-error ({script}: {tail})"
        s = out.stdout.strip()
        return "  " + s if s else ""
    except Exception as e:  # noqa: BLE001 — the headline must never die, but must SAY the gate did
        return f"  {gate}: ⚠ gate-error ({type(e).__name__})"


def memkit_line():
    """memory-kit PROCESS-ARTIFACT tier currency (comet retention — bounded, trajectory-preserving)."""
    return _gate_subprocess("memkit", "memkit-retention.py", "--status", timeout=10)


def mempalace_line():
    """MemPalace semantic-index staleness — the 7th store's tripwire (index vs the cleaned surface)."""
    return _gate_subprocess("mempalace", "mempalace-currency.py", "--status")


def cleanup_line():
    """drift-elevation gate — has enough churned since the last cleanup to warrant the memory-stasis-loop?"""
    return _gate_subprocess("cleanup", "cleanup-pressure.py", "--status")


def scope_line():
    """substrate scope-drift gate — is the held/ tree aligned with cluster-state, or are docs ready to EXPAND
    onto the plate (a capability returned) / needing to be HELD (a capability was lost)? The symmetric inverse
    of how the CI layer auto-reconciles: this surfaces the held<->live move so stories + architecture follow
    the substrate without anyone remembering to run it."""
    return _gate_subprocess("scope", "scope-reconcile.py", "--report")


def headline_mode():
    docs, _ = scan_docs()
    surface = [d for d in docs if not d["home"].startswith("_state")]
    mem = scan_memory()
    mem_un = sum(1 for m in mem if not m["linked"])
    no_exit = sum(1 for d in surface if d["home"] in ACTIVE_HOMES and d["bucket"] == "NONE")
    # scope CLAIMED-ONLY to ACTIVE_HOMES — CANONICAL/HISTORY `accepted`/`landed` is settled, not CI-gated
    # (mirrors the scoreboard's landed_unverified filter; keeps the SessionStart line consistent)
    claimed = sum(1 for d in surface
                  if d["home"] in ACTIVE_HOMES and d["bucket"] == "LANDED" and not d["verified"])
    avail, _ = load_cluster_state(CLUSTER_STATE)
    a = sorted(k for k, v in avail.items() if v == "true")
    u = sorted(k for k, v in avail.items() if v != "true")
    pe = True
    if STATE_ROOT.is_dir():
        pe = all(len([f for f in sub.glob("*.md") if f.name not in ("CLAUDE.md", "README.md")]) == 0
                 for sub in STATE_ROOT.iterdir() if sub.is_dir())
    ns = len(scan_stray())  # remedial leg: ungoverned .md at the repo/docs root (residue → 0)
    due = decompose_due_count()
    print("MEMORY BUDGET  (`placement-audit.py --ledger` = per-file queue · `--focus` = testable scope)")
    print(f"  debt: {no_exit} no-status · {mem_un} unlinked-memory · {claimed} claimed-unverified"
          f"   |   pressure-dirs: {'empty ✅' if pe else '⚠ NON-EMPTY (pressure)'}"
          f" · stray: {'clean ✅' if ns == 0 else f'⚠ {ns} (relocate — see scoreboard)'}")
    print(f"  testable env: {', '.join(a) or 'none'}   |   blocked-by-env: {', '.join(u) or 'none'}")
    cov = decompose_coverage(surface)
    print(f"  review: {cov['captured']}/{cov['active']} specs+plans decomposed · "
          f"{cov['uncaptured']} UN-CAPTURED (un-reviewed backlog — `--coverage` for the queue)")
    # BACK fire point: ACTIVE-home docs with terminal status still living plan-shaped → past-due to dissolve
    print(f"  decompose: {due} plan{'' if due == 1 else 's'} past-due to decompose"
          f"   ({'clear ✅' if due == 0 else '⚠ decompose-self → zero residue'})")
    # LEGIBILITY/PATH + PRIORITIZATION/ROADMAP currency: is the MAP walk current vs the
    # architecture seeds, and when was the roadmap synthesis last refreshed?
    try:
        print(map_currency_line())
    except Exception as e:  # noqa: BLE001 — instrument liveness: say the gate died
        print(f"  path: ⚠ gate-error ({type(e).__name__})")
    # PROCESS-ARTIFACT tier: is the memory-kit report dir comet-shaped + within budget?
    ml = memkit_line()
    if ml:
        print(ml)
    # SEMANTIC-INDEX tier: is the MemPalace index current with the cleaned surface?
    pl = mempalace_line()
    if pl:
        print(pl)
    # DRIFT-ELEVATION gate: has enough churned since the last cleanup to warrant the loop (the weekly trigger)?
    cl = cleanup_line()
    if cl:
        print(cl)
    # SUBSTRATE-SCOPE gate: is the held/ tree aligned with cluster-state, or are docs ready to expand/hold?
    sc = scope_line()
    if sc:
        print(sc)
    # DIRECTORY-GOVERNANCE gate: is every substantial code region OWNED by a self-responsible .epr-meta?
    try:
        print(epr_meta_line())
    except Exception as e:  # noqa: BLE001 — instrument liveness
        print(f"  epr-meta: ⚠ gate-error ({type(e).__name__})")
    return 0


def main() -> int:
    if FOCUS:
        return focus_mode()
    if LEDGER:
        return ledger_mode()
    if HEADLINE:
        return headline_mode()
    if COVERAGE:
        return coverage_mode()
    if STASIS:
        return stasis_mode()
    if EPR_META:
        return epr_meta_mode()

    docs, status_vals = scan_docs()
    surface_docs = [d for d in docs if not d["home"].startswith("_state")]
    mem = scan_memory()
    mem_unlinked = [m for m in mem if not m["linked"]]

    no_exit = [d for d in surface_docs if d["home"] in ACTIVE_HOMES and d["bucket"] == "NONE"]
    # CLAIMED-ONLY is the over-claim gate: an ACTIVE-home plan/spec saying "landed" without verification
    # evidence is dangerous debt. But CANONICAL (living architecture truth) and HISTORY (settled/distilled
    # decisions) ARE the verified/settled destinations per PLACEMENT.md — an `accepted`/`landed` status there
    # is the correct steady state, not a deliverable awaiting CI. Scope the gate to ACTIVE_HOMES (mirroring
    # no_exit / drift_dead / orphans on the surrounding lines) to drop that deterministic false-positive class.
    landed_unverified = [d for d in surface_docs
                         if d["home"] in ACTIVE_HOMES and d["bucket"] == "LANDED" and not d["verified"]]
    drift_dead = [d for d in surface_docs if d["home"] in ACTIVE_HOMES and d["bucket"] == "DEAD"]
    orphans = [d for d in surface_docs if not d["inbound"] and not d["out"] and d["home"] in ACTIVE_HOMES]
    history_broken = [d for d in surface_docs if d["home"] == "HISTORY" and not d["has_canonical_link"]]
    # Bidirectional history-edge check (PLACEMENT.md): a HISTORY record must ALSO be
    # linked back FROM canonical. Build a reverse index of every .md basename any
    # CANONICAL doc links outward to; a HISTORY doc with no inbound canonical link is
    # an asymmetric (one-directional) edge.
    canonical_targets = set()
    for d in surface_docs:
        if d["home"] == "CANONICAL":
            canonical_targets.update(d["out"])
    history_no_backref = [d for d in surface_docs
                          if d["home"] == "HISTORY" and d["has_canonical_link"]
                          and d["name"] not in canonical_targets]
    dumps = []
    if RETIRED_DIR.is_dir():
        for f in RETIRED_DIR.glob("*.md"):
            fm = parse_front_matter(f.read_text(errors="replace"))
            if not (fm.get("verified_by") or fm.get("landed_commit")):
                dumps.append(str(f.relative_to(ROOT)))
    stray = scan_stray()  # remedial leg: residue parked outside every governed home
    total = len(surface_docs)
    with_state = sum(1 for d in surface_docs if d["bucket"] != "NONE")
    debt_total = (len(no_exit) + len(landed_unverified) + len(drift_dead) + len(orphans)
                  + len(history_broken) + len(history_no_backref) + len(dumps) + len(mem_unlinked)
                  + len(stray))

    if AS_JSON:
        print(json.dumps({"total_docs": total, "memory_unlinked": len(mem_unlinked),
                          "debt_total": debt_total, "no_exit": len(no_exit),
                          "landed_unverified": len(landed_unverified), "orphans": len(orphans),
                          "history_broken": len(history_broken),
                          "history_no_backref": len(history_no_backref), "dumps": len(dumps),
                          "stray": len(stray)},
                         indent=2, sort_keys=True))
        return 0

    def lst(items, n=6):
        for d in items[:n]:
            print(f"    - {d['path']}")
        if len(items) > n:
            print(f"    … +{len(items)-n} more")

    print(f"PLACEMENT AUDIT — deterministic, no LLM   (root: {ROOT})")
    print(f"\n{'='*72}\nTECH-DEBT SCOREBOARD  (each → 0)\n{'='*72}")
    for n, tag, desc in [
        (len(no_exit), "NO-EXIT", "active doc with no status (dumping-ground signal)"),
        (len(landed_unverified), "CLAIMED-ONLY", "'landed' with NO verification evidence"),
        (len(drift_dead), "DRIFT-DEAD", "superseded/abandoned still in ACTIVE"),
        (len(orphans), "ORPHAN", "active doc with no inbound and no outbound link"),
        (len(history_broken), "HIST-BROKEN", "history record with no canonical back-link (outbound)"),
        (len(history_no_backref), "HIST-NO-BACKREF", "history record no canonical links TO (inbound)"),
        (len(dumps), "DUMP", "_retired/ holding unverified content"),
        (len(mem_unlinked), "MEM-UNLINKED", f"memory entry with no cites: ({len(mem)-len(mem_unlinked)}/{len(mem)} linked)"),
        (len(stray), "STRAY-DOC", "ungoverned .md at repo/docs root (outside every home)"),
    ]:
        print(f"  {n:>4}  {tag:<15} {desc}")
    print(f"  {'-'*4}\n  {debt_total:>4}  DEBT TOTAL")
    if stray:
        print("  stray docs (relocate to a governed home, or remove the breadcrumb):")
        for s in stray:
            print(f"    - [{s['kind']}] {s['path']}")
    print(f"  doc lifecycle-state coverage: {with_state}/{total} ({(100*with_state//total) if total else 0}%)"
          f"   |  distinct status strings: {len(status_vals)}")

    stable = equilibrium_section()

    print(f"\n{'='*72}\nEQUILIBRIUM VERDICT\n{'='*72}")
    if debt_total == 0 and stable:
        print("  ✅ AT EQUILIBRIUM — no content debt, pressure dirs empty, no dumps.")
    else:
        print(f"  ❌ NOT AT EQUILIBRIUM — {debt_total} content-debt items"
              + ("; pressure dirs empty (structurally clean)" if stable else "; structural pressure present"))
        print("     Run `--ledger` for the per-file budget (every file's position + state + next-action).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
