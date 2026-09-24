"""resource-limit-raise-design-signal — the compute-layer design gate.

WHAT THIS PINS. A raise of a declared k8s resource LIMIT is not a setting change; it is System 1
amplifying the regulator, and past two prior stamps on one unit it is Meadows'
shifting-the-burden-to-the-intervenor wearing an operations costume. The validator under test turns
that into an `ask` carrying a NUMBER — requested vs a demand model composed from facts the tree
already declares (archetype canonical budget, ratified `resourceOverride`, the Rakia
`limits.cpu_m` overcommit ratification, the `bridges/k8s::share()` split, `edgenodeArcFactor`, the
doorway manifests' hosted cast) — so the author argues with the model rather than with prose.

The primary fixture is REAL: the CPU raise drafted and reverted on 2026-09-13 (matthew
4000m -> 10000m, adam 8000m -> 12000m, plus both halves of adam's split manifest 4000m -> 6000m).
The diff is applied positionally to the live files to synthesize post-content, with the on-disk file
as the pre-state — exactly the shape the PreToolUse hook hands the validator. If that raise ever
stops firing, this suite says so.

The silence cases matter as much as the fires: a LOWER value, a NEW unit with no pre-state, an
unparseable pending edit (abstain — the resolver is fail-open by design), a non-limit edit to the
same file, and any manifest YAML that no human's deployment record claims.

Run: python3 .claude/scripts/_lib/__tests__/resource_limit_raise_test.py  (exit 0 = pass)
Bespoke assert-based harness — matches this __tests__ dir's convention (pytest is not installed).
"""
import io
import json
import re
import shutil
import sys
import tempfile
from contextlib import redirect_stderr
from pathlib import Path

_here = Path(__file__).resolve()
REPO = None
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        REPO = _here
        break
    _here = _here.parent
if REPO is None:  # pragma: no cover
    print("FAIL: could not locate repo root", file=sys.stderr)
    sys.exit(1)

from _lib import epr_meta  # noqa: E402

VALIDATE = epr_meta.REFERENCE_VALIDATORS["epr:validator-resource-limit-raise-design-signal"]
DEPLOYMENTS = REPO / "genesis/orchestrator/data/deployments.json"
BUDGETS = REPO / "genesis/data/devices/archetype-resource-budgets.json"
CAPACITY = REPO / "genesis/data/rakia/compute-capacity.json"
MEASURES = REPO / ".claude/epr-meta/measures.yaml"
# The real 2026-09-13 raise, drafted and reverted. It was first captured in a session scratchpad,
# which did not outlive the session; this tracked copy was rebuilt 2026-09-20 by replaying that
# session's own edits against the live tree. A test must not depend on a path outside the repo.
FIXTURE_DIFF = Path(__file__).resolve().parent / "fixtures" / "cpu-raise-reverted-2026-09-13.diff"

_passed = 0
_failures: list[str] = []


def check(label: str, cond: bool, detail: str = "") -> None:
    global _passed
    if cond:
        _passed += 1
        print(f"  ✅ {label}")
    else:
        _failures.append(label + (f" — {detail}" if detail else ""))
        print(f"  ❌ {label}" + (f" — {detail}" if detail else ""))


def fire(write: dict):
    """Run the validator, capturing its stderr detail. Returns (verdict_or_False, stderr_text)."""
    buf = io.StringIO()
    with redirect_stderr(buf):
        result = VALIDATE(write)
    return result, buf.getvalue()


# ── unified-diff applier (positional; the fixture is a plain git diff) ────────────────────────
def apply_diff(diff_text: str) -> dict:
    """{repo_relative_path: post_content} for every file section of a unified diff."""
    out: dict[str, str] = {}
    path = None
    lines = diff_text.splitlines(keepends=True)
    i = 0
    src: list[str] = []
    offset = 0

    def flush():
        if path is not None:
            out[path] = "".join(src)

    while i < len(lines):
        line = lines[i]
        if line.startswith("diff --git "):
            flush()
            path = line.split(" b/", 1)[1].strip()
            src = (REPO / path).read_text().splitlines(keepends=True)
            offset = 0
            i += 1
            continue
        m = re.match(r"^@@ -(\d+)(?:,(\d+))? \+\d+(?:,\d+)? @@", line)
        if m:
            cursor = int(m.group(1)) - 1 + offset
            i += 1
            while i < len(lines) and not lines[i].startswith(("@@", "diff --git ")):
                h = lines[i]
                if h.startswith(" "):
                    cursor += 1
                elif h.startswith("-"):
                    assert src[cursor].rstrip("\n") == h[1:].rstrip("\n"), \
                        f"diff context mismatch at {path}:{cursor + 1}"
                    del src[cursor]
                    offset -= 1
                elif h.startswith("+"):
                    src.insert(cursor, h[1:])
                    cursor += 1
                    offset += 1
                i += 1
            continue
        i += 1
    flush()
    return out


# ── synthetic repo: a controlled tree carrying the ledgers at their repo-relative paths ───────
def synth_repo(tmp: Path, humans: list, doorway_cast: int | None = None) -> Path:
    (tmp / ".git").mkdir(parents=True, exist_ok=True)
    for rel in ("genesis/data/devices", "genesis/data/rakia", "genesis/orchestrator/data",
                "genesis/orchestrator/manifests/humans", "genesis/orchestrator/manifests/doorway",
                ".claude/epr-meta"):
        (tmp / rel).mkdir(parents=True, exist_ok=True)
    shutil.copy(BUDGETS, tmp / "genesis/data/devices/archetype-resource-budgets.json")
    shutil.copy(CAPACITY, tmp / "genesis/data/rakia/compute-capacity.json")
    shutil.copy(MEASURES, tmp / ".claude/epr-meta/measures.yaml")
    dep = tmp / "genesis/orchestrator/data/deployments.json"
    dep.write_text(json.dumps({"schemaVersion": 1, "humans": humans}, indent=2))
    if doorway_cast is not None:
        (tmp / "genesis/orchestrator/manifests/doorway/alpha-b.yaml").write_text(
            "            - name: DOORWAY_MAX_AGENTS_PER_CONDUCTOR\n"
            f"              value: \"{doorway_cast}\"\n")
    return dep


def human(name, archetype="device-family-node-base", **kw):
    base = {"name": name, "pattern": "consolidated", "deviceArchetype": archetype,
            "edgenodeCpuLimit": "4000m", "edgenodeMemoryLimit": "8Gi",
            "edgenodeCpuRequest": "1500m", "edgenodeMemoryRequest": "2Gi"}
    base.update(kw)
    return base


def post_of(dep: Path, mutate) -> str:
    doc = json.loads(dep.read_text())
    mutate(doc)
    return json.dumps(doc, indent=2)


print("\n── 1. the REAL fixture: the 2026-09-13 CPU raise, drafted and reverted ──")
check("fixture diff is present", FIXTURE_DIFF.exists(), str(FIXTURE_DIFF))
patched = apply_diff(FIXTURE_DIFF.read_text())
check("diff applies to the live tree (3 files)", len(patched) == 3, ", ".join(sorted(patched)))

v, err = fire({"path": str(DEPLOYMENTS), "content": patched["genesis/orchestrator/data/deployments.json"],
               "is_new": False})
check("FIRES on the reverted deployments.json raise", bool(v))
check("  class is `ask` — a refer, never a refuse", getattr(v, "cls", None) == "ask")
check("  refer reason names the unattenuated raise",
      getattr(v, "refer_reason", None) == "resource-limit-raise-unattenuated")
check("  both raised units are named", "adam" in (v.reason or "") and "matthew" in (v.reason or ""))
check("  reads BREACH, not APPROACH (past the ratified tolerance)", "BREACH" in err, err[:200])
check("  carries the algedonic wire kind", "algedonic-breach" in err)
check("  matthew: predicted 4000m from the archetype canonical, requested 10000m",
      "requested 10000m vs MODEL-PREDICTED 4000m" in err)
check("  matthew: ratio 2.50x against the ratified 1.25x band",
      "ratio 2.50x vs tolerance 1.25x" in err)
check("  adam: predicted from the RATIFIED override (8000m), not the class floor",
      "requested 12000m vs MODEL-PREDICTED 8000m" in err and "ratified resourceOverride" in err)
check("  the ratification is quoted, not assumed",
      "ratified 125% on limits.cpu_m by operator" in err)
check("  recurrence: matthew's two prior stamps are counted and named",
      "matthew has been raised 2 time(s)" in err and "$cpuBumpComment" in err)
check("  recurrence at the watermark says a design pass is due FIRST",
      "DESIGN PASS IS DUE BEFORE ANOTHER RAISE" in err)
check("  adam's first raise is reported as first", "adam carries NO prior raise stamp" in err)
check("  the model's own home is disclosed as a projection",
      "Envelope" in err and "`bound`, never `demand`" in err)
check("  the undeclared corpus is stated, not papered over",
      "NO manifest in this tree declares a corpus" in err)
check("  the three questions are asked in order",
      all(k in err for k in ("(a) WHAT DISTURBANCE", "(b) WHICH SEAM WE OWN",
                             "(c) WHAT IS THE RESTORATION CONDITION")))
check("  the stamp's cited diagnosis RESOLVES on disk, so the ask reads differently",
      "design references PRESENT" in err
      and "fleet-full-arc-conductor-saturation-and-coordinated-warmup-2026-09-11" in err)
check("  and still says a citation is not an attenuation",
      "a citation is not an attenuation" in err)
check("  hosted cast is read from the doorway manifest, not invented",
      "DOORWAY_MAX_AGENTS_PER_CONDUCTOR=32" in err)

print("\n── 2. the YAML half of the same raise (adam renders from manifests) ──")
for rel in ("genesis/orchestrator/manifests/humans/adam-firstman.yaml",
            "genesis/orchestrator/manifests/humans/adam-firstman-conductor.yaml"):
    v2, err2 = fire({"path": str(REPO / rel), "content": patched[rel], "is_new": False})
    is_conductor = "conductor" in rel
    check(f"FIRES on {Path(rel).name} (4000m -> 6000m)", bool(v2))
    check("  predicted as 1/2 of adam's declared 8000m via bridges/k8s share()",
          "requested 6000m vs MODEL-PREDICTED 4000m" in err2 and "bridges/k8s share()" in err2)
    check("  past the bound at 1.50x", "ratio 1.50x vs tolerance 1.25x" in err2)
    check("  attributed to the owning unit via deployments.json manifest lookup",
          "adam resources.limits.cpu" in err2)
    if is_conductor:
        check("  the conductor half is recognised as the conductor half",
              "limits-block#" in err2)

print("\n── 3. silence: what must NOT fire ──")
with tempfile.TemporaryDirectory() as td:
    tmp = Path(td)
    dep = synth_repo(tmp, [human("pete", edgenodeCpuLimit="4000m")], doorway_cast=32)

    lower = post_of(dep, lambda d: d["humans"][0].update(edgenodeCpuLimit="2000m"))
    v3, _ = fire({"path": str(dep), "content": lower, "is_new": False})
    check("silent on a LOWER value", v3 is False)

    same = dep.read_text()
    v4, _ = fire({"path": str(dep), "content": same, "is_new": False})
    check("silent on an EQUAL value (no-op write)", v4 is False)

    nonlimit = post_of(dep, lambda d: d["humans"][0].update(edgenodeDbPoolSize="20"))
    v5, _ = fire({"path": str(dep), "content": nonlimit, "is_new": False})
    check("silent on a NON-LIMIT edit to the same file", v5 is False)

    newhuman = post_of(dep, lambda d: d["humans"].append(
        human("brand-new", edgenodeCpuLimit="64000m")))
    v6, _ = fire({"path": str(dep), "content": newhuman, "is_new": False})
    check("silent on a NEW unit with no pre-state (archetype alignment owns its class)",
          v6 is False)

    v7, _ = fire({"path": str(dep), "content": "{ not json", "is_new": False})
    check("ABSTAINS on an unparseable pending edit (fail-open by design)", v7 is False)

    v8, _ = fire({"path": str(dep), "content": None, "is_new": False})
    check("abstains when content is unresolved", v8 is False)

    v9, _ = fire({"path": str(dep), "content": dep.read_text(), "is_new": True})
    check("silent on a brand-new deployments.json (nothing to compare against)", v9 is False)

    stray = tmp / "genesis/orchestrator/manifests/humans/nobody-claims-this.yaml"
    stray.write_text("spec:\n  limits:\n    cpu: \"1000m\"\n")
    v10, _ = fire({"path": str(stray),
                   "content": "spec:\n  limits:\n    cpu: \"9000m\"\n", "is_new": False})
    check("silent on a manifest YAML no deployment record claims (self-scoping)", v10 is False)

    bad_unit = post_of(dep, lambda d: d["humans"][0].update(edgenodeCpuLimit="lots-of-cpu"))
    v11, _ = fire({"path": str(dep), "content": bad_unit, "is_new": False})
    check("abstains on an unparseable QUANTITY rather than guessing", v11 is False)

print("\n── 4. APPROACH vs BREACH: the band edge is objective ──")
with tempfile.TemporaryDirectory() as td:
    tmp = Path(td)
    # A first raise, inside the ratified 125% band: 4000m -> 4500m on a unit with no stamps.
    dep = synth_repo(tmp, [human("quiet", edgenodeCpuLimit="4000m")])
    v12, err12 = fire({"path": str(dep),
                       "content": post_of(dep, lambda d: d["humans"][0].update(
                           edgenodeCpuLimit="4500m")), "is_new": False})
    check("a FIRST raise inside the ratified band is an APPROACH", bool(v12) and "APPROACH" in err12)
    check("  Approach carries threshold_pct (algedonic.rs: Approach does, Breach does not)",
          (getattr(v12, "evidence", None) or {}).get("threshold_pct") == 100)
    check("  and still asks the three questions", "(a) WHAT DISTURBANCE" in err12)
    check("  missing design references are named as missing",
          "design references MISSING" in err12
          and "no cited diagnosis path that RESOLVES on disk" in err12)

    # Same first raise, but PAST the band: 4000m -> 6000m is 1.50x.
    v13, err13 = fire({"path": str(dep),
                       "content": post_of(dep, lambda d: d["humans"][0].update(
                           edgenodeCpuLimit="6000m")), "is_new": False})
    check("the same first raise PAST the ratified band is a BREACH",
          bool(v13) and "BREACH" in err13)
    check("  Breach carries no threshold_pct",
          "threshold_pct" not in (getattr(v13, "evidence", None) or {}))

    # In-band, but the unit already carries two stamps -> recurrence alone breaches.
    dep2 = synth_repo(tmp, [human("weary", edgenodeCpuLimit="4000m",
                                  **{"$cpuBumpComment": "2026-01-01: band-aid.",
                                     "$comment": "TEMP BUMP (2026-02-02): band-aid."})])
    v14, err14 = fire({"path": str(dep2),
                       "content": post_of(dep2, lambda d: d["humans"][0].update(
                           edgenodeCpuLimit="4500m")), "is_new": False})
    check("recurrence >= 2 escalates an IN-BAND raise to BREACH", bool(v14) and "BREACH" in err14)
    check("  the prior stamps are named as the evidence",
          "weary has been raised 2 time(s)" in err14
          and "$comment" in err14 and "$cpuBumpComment" in err14)
    check("  the watermark is quoted from the measures.yaml lens, not a magic number",
          "recurrence watermark (2, measures.yaml lens resource-limit-raise-recurrence)" in err14)

    # One stamp only -> still Approach in band.
    dep3 = synth_repo(tmp, [human("once", edgenodeCpuLimit="4000m",
                                  **{"$cpuLimitBump": "2026-01-01: band-aid."})])
    v15, err15 = fire({"path": str(dep3),
                       "content": post_of(dep3, lambda d: d["humans"][0].update(
                           edgenodeCpuLimit="4500m")), "is_new": False})
    check("ONE prior stamp, in band, is still an APPROACH", bool(v15) and "APPROACH" in err15)

print("\n── 5. memory is incompressible: no ratified band, and the arc ruling is surfaced ──")
with tempfile.TemporaryDirectory() as td:
    tmp = Path(td)
    dep = synth_repo(tmp, [human("hungry", edgenodeMemoryLimit="8Gi")])
    v16, err16 = fire({"path": str(dep),
                       "content": post_of(dep, lambda d: d["humans"][0].update(
                           edgenodeMemoryLimit="9Gi")), "is_new": False})
    check("a memory raise of any size is past the bound (tolerance 1.00x)",
          bool(v16) and "memory 1.00x (incompressible" in err16 and "PAST THE BOUND" in err16)
    check("  full arc (edgenodeArcFactor absent) surfaces the arc-scaling ruling",
          "full arc" in err16 and "the durable lever is the arc, not the limit" in err16)

    dep2 = synth_repo(tmp, [human("leecher", edgenodeMemoryLimit="8Gi", edgenodeArcFactor="0")])
    _, err17 = fire({"path": str(dep2),
                     "content": post_of(dep2, lambda d: d["humans"][0].update(
                         edgenodeMemoryLimit="9Gi")), "is_new": False})
    check("  a declared leecher (arcFactor 0) does NOT get the full-arc ruling",
          "the durable lever is the arc" not in err17)

print("\n── 6. declaration coherence ──")
# The rule is declared INLINE in two manifests rather than bound to a pinned policy row: when it
# was written the registry sat at 99.1% of the 65,536B MANIFEST cap both governance hosts then
# borrowed for it, so one more row would have dropped every policy-bound rule to "unknown policy —
# rule NOT enforced". de4f75b20 gave the registry its own 1 MiB cap in both hosts
# (`_MAX_REGISTRY_BYTES` / eprfs-meta `MAX_REGISTRY_BYTES`); the inline declaration stays, and this
# check now reads the cap the registry is actually held to.
REGISTRY = REPO / ".claude/epr-meta/policies.yaml"
size = REGISTRY.stat().st_size
check("the policy registry still loads (under the registry cap BOTH hosts enforce)",
      size <= epr_meta._MAX_REGISTRY_BYTES, f"{size}B of {epr_meta._MAX_REGISTRY_BYTES}B")
policies, errs = epr_meta.load_policies(REPO)
check("  every policy-bound rule is still resolvable (no governance outage)",
      bool(policies) and not errs, "; ".join(errs[:2]))

for meta_rel, pattern in (("genesis/orchestrator/data/.epr-meta", "deployments.json"),
                          ("genesis/orchestrator/manifests/.epr-meta", "*.yaml")):
    meta = REPO / meta_rel
    check(f"{meta_rel} parses clean", epr_meta.check_meta(meta) == [])
    merged = epr_meta.merge_rules(epr_meta.collect_cascade(meta.parent / "probe"))
    rule = merged["rules"].get("resource-limit-raise-design-signal")
    check("  the rule reaches this surface", rule is not None)
    check("  declared class is ask (refer, never refuse)", (rule or {}).get("class") == "ask")
    check(f"  scoped to {pattern}", ((rule or {}).get("when") or {}).get("write") == pattern)
    check("  names the validator", (rule or {}).get("validator")
          == "epr:validator-resource-limit-raise-design-signal")
    check("  declares a retire-when (no rule without an exit)", bool((rule or {}).get("retire-when")))

scope = json.loads((REPO / "elohim/sdk/schemas/v1/registries/governance-validators.json").read_text())
refs = {v["ref"]: v["runtime"] for v in scope["validators"]}
check("the validator is mapped in the ONE scope registry",
      refs.get("epr:validator-resource-limit-raise-design-signal") == "python")

measures = (REPO / ".claude/epr-meta/measures.yaml").read_text()
check("the recurrence watermark is DECLARED in the measure registry, not hardcoded",
      "id: resource-limit-raise-recurrence" in measures and "id: resource-limit-raises" in measures)

print(f"\n{_passed} checks passed, {len(_failures)} failed")
if _failures:
    for f in _failures:
        print(f"  FAIL: {f}", file=sys.stderr)
    sys.exit(1)
