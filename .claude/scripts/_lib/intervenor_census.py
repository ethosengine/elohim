"""The intervenor census — how many of our governance mechanisms have no way to end.

Meadows names *Shifting the Burden to the Intervenor* as a system trap: an intervention added
to correct a problem erodes the system's own capacity to solve it, so more intervention is
required, forever. Her Way Out is a design obligation on the intervenor — *"restore or enhance
the system's own ability to solve its problems, THEN REMOVE YOURSELF."*

The survey (`epr:meadows-systems-dynamics-cross-pollination-2026-08-11` §5) adjudicated us
against that trap and found the exposure total: every hook, sentinel, ledger, compose-gate rule
and stasis loop in `.claude/` was added because the system did not reliably do something on its
own, and not one carried a removal condition. This module is the meter for that, and the ONLY
thing it does is count. It never removes an intervenor and never proposes removing one — a
removal is a governance act, and an instrument that retired rules on its own authority would be
a considerably worse intervenor than the ones it counts.

WHAT IS COUNTED — three declared populations, and the scope is deliberately narrow:

  1. `.epr-meta` rules       — the compose-gate manifests (`retire-when:` on the rule)
  2. `.epr-meta` policies    — the define-once-bind-many registry (`retire-when:` on the policy)
  3. `.claude/hooks/*.py`    — PreToolUse/PostToolUse/SessionStart hooks (`RETIRE_WHEN = "..."`)

WHAT IS NOT — subagents, slash commands, stasis loops, the finding ledgers, and the session-start
injectors are all intervenors too and are NOT in this count. That is a scope choice, not an
oversight, and it is why the measure narrows its CLAIM rather than widening its interval: the
count is exact over the population it names, and says plainly which populations it excludes.
Meadows' own indicator criterion — an aggregate must show its mechanism, not just its number.

The measure is a LEVEL (`elohim/epr/src/measure.rs` `MeasureKind::Level`): a stock of un-retired
intervenors at a point in time, not a rate. Declaring that is not decoration — the whole reason
this vocabulary exists is that a level compared against a rate is the unit error the survey found
shipped in `spatial_capacity.rs`.

RETIRE_WHEN for this module itself: see below. The recursion is the honest test, not a joke —
a gate that counts un-retired intervenors is an intervenor, and if it could not state its own
exit that would be evidence the mechanism is wrong.
"""
from __future__ import annotations

import ast
from pathlib import Path

try:
    import yaml  # type: ignore
except Exception:  # noqa: BLE001 — PyYAML-absent is a legitimate degraded state, not a crash
    yaml = None

from . import epr_meta as _em
from . import frontmatter as fm

# The census's own removal condition (the recursion this module is required to face).
#
# It is a real condition, checkable by this module's own output, and it is deliberately about
# the SYSTEM's habit rather than about the instrument: the census exists to make un-retired
# intervenors visible while adding a removal condition is still an unfamiliar act. Once
# declaring an exit is what authors simply do — evidenced by the count holding at zero across
# a period long enough to cover normal churn — the meter is measuring a habit the system already
# has, which is exactly the state Meadows says the intervenor should withdraw from.
#
# Note what would make this dishonest: if the count reached zero because authors wrote
# `retire-when: never — it just is` everywhere. That is why the bare-`never` shape is refused at
# the gate (`epr_meta._validate_retire_when`) and why `never` answers are reported SEPARATELY
# below rather than folded into the retired total. A zero that hides behind unreasoned nevers is
# rule-beating, and the census would be its instrument.
RETIRE_WHEN = (
    "when `lacking` holds at 0 for 8 consecutive weeks with `never_declared` not growing "
    "faster than `total` — at that point declaring an exit is the system's own habit and this "
    "meter is measuring something that no longer needs a meter"
)

HOOKS_DIR = ".claude/hooks"


def _hook_retire_when(path: Path) -> str | None:
    """Read a module-level `RETIRE_WHEN = "..."` out of a hook WITHOUT importing it.

    Hooks are executable scripts with side effects at import (they read stdin, write ledgers,
    print advisories); importing them to introspect a constant would make the census itself a
    mutation. `ast` reads the source, so a census run can never fire a hook."""
    try:
        tree = ast.parse(path.read_text(encoding="utf-8", errors="replace"))
    except Exception:  # noqa: BLE001 — an unparseable hook is a finding for a different gate
        return None
    for node in tree.body:  # module level only: a RETIRE_WHEN inside a function is not a decl
        if not isinstance(node, ast.Assign):
            continue
        for target in node.targets:
            if isinstance(target, ast.Name) and target.id == "RETIRE_WHEN":
                if isinstance(node.value, ast.Constant) and isinstance(node.value.value, str):
                    return node.value.value.strip() or None
    return None


def _is_never(condition: str) -> bool:
    return bool(_em._RETIRE_NEVER_RE.match(condition.strip()))


def census_data(root: Path) -> dict:
    """Walk the three declared populations. Pure read; returns raw rows for rendering."""
    root = Path(root)
    rows: list[dict] = []

    # 1 + 2. Compose-gate manifests and the policy registry.
    for meta in sorted(root.rglob(".epr-meta")):
        rel = meta.relative_to(root).as_posix()
        # Fixtures govern nothing — a manifest under a test-fixture tree exists to be PARSED by
        # the parity suite, never to gate a real write. Counting them would inflate the stock
        # with rows no exit could ever apply to, which is the measure lying about its own
        # population rather than about its arithmetic.
        if ("/node_modules/" in f"/{rel}" or rel.startswith(".claude/worktrees/")
                or "/fixtures/" in f"/{rel}" or "/__tests__/" in f"/{rel}"):
            continue
        try:
            cfg = yaml.safe_load(fm.parse_file(meta).raw_block) or {} if yaml else {}
        except Exception:  # noqa: BLE001 — unparseable manifests are check_meta's finding
            continue
        for rule in cfg.get("rules", []) or []:
            if not isinstance(rule, dict):
                continue
            # A policy BINDING owns placement, never semantics — its removal condition is the
            # policy's, so resolve through the ref instead of scoring the binding as un-retired.
            ref = rule.get("policy")
            rows.append({
                "population": "epr-meta-rule",
                "where": rel,
                "id": rule.get("id", "?"),
                "class": rule.get("class"),
                "binds": ref,
                "retire_when": rule.get("retire-when"),
            })

    policies: dict[str, str | None] = {}
    if yaml is not None:
        preg = root / _em.POLICY_REGISTRY_REL
        if preg.is_file():
            try:
                data = yaml.safe_load(preg.read_text()) or {}
            except Exception:  # noqa: BLE001
                data = {}
            for pol in data.get("policies", []) or []:
                if not isinstance(pol, dict) or not pol.get("id"):
                    continue
                key = f"{pol['id']}@{pol.get('version')}"
                policies[key] = pol.get("retire-when")
                if pol.get("status") == "superseded":
                    continue  # already retired by lineage — counting it would double-count
                rows.append({
                    "population": "epr-meta-policy",
                    "where": _em.POLICY_REGISTRY_REL,
                    "id": key,
                    "class": pol.get("class"),
                    "binds": None,
                    "retire_when": pol.get("retire-when"),
                })

    # Resolve binding rows against the registry they bind.
    for row in rows:
        if row["binds"] and not row["retire_when"]:
            row["retire_when"] = policies.get(row["binds"])
            row["inherited"] = bool(row["retire_when"])

    # 3. Hooks — and this module, which is one of them in every sense that matters.
    #
    # A gate that counts un-retired intervenors is itself an intervenor, and a census that
    # excluded itself from its own population would be measuring everyone else's accretion while
    # exempting its own. So the meter appears in its own reading. It is not in HOOKS_DIR, so it
    # is added explicitly rather than found — which is the honest shape: being counted here is a
    # deliberate submission to the rule, not an accident of where the file happens to live.
    hooks = root / HOOKS_DIR
    if hooks.is_dir():
        for hook in sorted(hooks.glob("*.py")):
            rows.append({
                "population": "hook",
                "where": f"{HOOKS_DIR}/{hook.name}",
                "id": hook.stem,
                "class": None,
                "binds": None,
                "retire_when": _hook_retire_when(hook),
            })
    rows.append({
        "population": "hook",
        "where": ".claude/scripts/_lib/intervenor_census.py",
        "id": "intervenor-census (this meter)",
        "class": None,
        "binds": None,
        "retire_when": RETIRE_WHEN,
    })

    total = len(rows)
    declared = [r for r in rows if r["retire_when"]]
    never = [r for r in declared if _is_never(str(r["retire_when"]))]
    lacking = [r for r in rows if not r["retire_when"]]
    return {
        "rows": rows,
        "total": total,
        "declared": len(declared),
        "never_declared": len(never),
        "lacking": len(lacking),
        "by_population": {
            p: {
                "total": sum(1 for r in rows if r["population"] == p),
                "lacking": sum(1 for r in rows if r["population"] == p and not r["retire_when"]),
            }
            for p in ("epr-meta-rule", "epr-meta-policy", "hook")
        },
    }


def lacking_measure(data: dict) -> dict:
    """The census as a first-class measure, on the wire vocabulary of
    `elohim/epr/src/measure.rs` — a LEVEL, witnessed, exact.

    Why `witnessed` and not `estimated`: every row was read off disk this run, and the count is
    exact OVER THE POPULATION IT NAMES. The uncertainty here is not in the counting, it is in the
    scope — and the honest move for scope uncertainty is to narrow the claim in the `basis`, not
    to widen an interval around a number that is not actually uncertain. Widening here would
    manufacture doubt about arithmetic in order to gesture at a boundary question, which is the
    mirror image of the false precision this ontology exists to prevent."""
    lacking = data["lacking"]
    pops = data["by_population"]
    return {
        "value": float(lacking),
        "kind": "level",
        "confidence": {
            "claim": "witnessed",
            "interval": {"lo": float(lacking), "hi": float(lacking)},
            "basis": (
                f"census of {data['total']} intervenors read off disk in three declared "
                f"populations — {pops['epr-meta-rule']['total']} .epr-meta rules, "
                f"{pops['epr-meta-policy']['total']} registry policies, "
                f"{pops['hook']['total']} hooks; {data['never_declared']} of the "
                f"{data['declared']} declared are reasoned `never`. EXCLUDED from the "
                f"population (not from the trap): subagents, slash commands, stasis loops, "
                f"finding ledgers, and the SessionStart injectors"
            ),
        },
    }


def render_census(data: dict) -> str:
    """One block for `placement-audit.py --epr-meta`. Deliberately NOT a new session-start
    headline line: the repo covenant's own warning is that this layer grows instruments with no
    reader, and a census of un-retired intervenors that mints a new always-on surface would be
    the joke telling itself."""
    m = lacking_measure(data)
    pops = data["by_population"]
    flag = "✅" if data["lacking"] == 0 else "⚠"
    out = [
        f"INTERVENOR CENSUS (kind: level — un-retired intervenors, Meadows' "
        f"shifting-the-burden trap):",
        f"  {flag} {data['lacking']} of {data['total']} have NO removal condition   "
        f"[rules {pops['epr-meta-rule']['lacking']}/{pops['epr-meta-rule']['total']} · "
        f"policies {pops['epr-meta-policy']['lacking']}/{pops['epr-meta-policy']['total']} · "
        f"hooks {pops['hook']['lacking']}/{pops['hook']['total']}]",
        f"  declared: {data['declared']} ({data['never_declared']} reasoned `never` — a floor "
        f"is an honest answer, and counting it is the point)",
        f"  basis: {m['confidence']['basis']}",
    ]
    if data["lacking"]:
        out.append("  → add `retire-when: <condition>` to the rule/policy, or "
                   "`RETIRE_WHEN = \"...\"` at a hook's module level. A condition, never a date.")
    return "\n".join(out)
