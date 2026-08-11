"""`retire-when:` — the intervenor's removal condition, on both governance gates plus the census.

Meadows' *Shifting the Burden to the Intervenor* trap, made countable. Two claims are under test
and they are different in kind:

  1. THE GATE refuses the two shapes that carry no information — an empty condition, and a bare
     `never` with no reason. It deliberately does NOT check whether the condition is true; that
     is not a thing a validator can know, and pretending otherwise would be the false precision
     the sibling measure ontology exists to prevent.
  2. THE CENSUS counts honestly — resolves a binding's condition through the policy it binds
     (semantics are policy-owned), skips superseded policies and test fixtures, and INCLUDES
     ITSELF, because a meter of un-retired intervenors that exempted itself would be measuring
     everyone else's accretion while excusing its own.

`retire-when` is excluded from `policy_content_hash` on purpose, and the hash-stability test
below pins that: if declaring an exit forced a policy-version bump, exits would be the most
expensive thing in the registry to add — precisely the wrong gradient for the trap it springs.

Run: python3 -m pytest .claude/scripts/_lib/__tests__/intervenor_retire_when_test.py -v
"""
from __future__ import annotations

import sys
from pathlib import Path

here = Path(__file__).resolve()
REPO = None
for _ in range(8):
    if (here / ".claude" / "scripts" / "_lib").is_dir():
        REPO = here
        sys.path.insert(0, str(here / ".claude" / "scripts"))
        break
    here = here.parent
assert REPO is not None, "could not locate repo root (.claude/scripts/_lib) from test file"

from _lib import epr_meta as _em  # noqa: E402
from _lib import intervenor_census as _ic  # noqa: E402


def _rule(**kw):
    base = {"id": "r", "class": "measure", "measure": {"kind": "level"}}
    base.update(kw)
    return {"epr-meta-version": 1, "rules": [base]}


# ---------------------------------------------------------------- the gate

def test_retire_when_is_optional():
    """The key is additive. Every rule authored before this existed must stay valid, or the
    gate would turn a governance improvement into a repo-wide breakage."""
    assert _em.validate_meta(_rule()) == []


def test_a_real_condition_passes():
    assert _em.validate_meta(_rule(**{
        "retire-when": "when the doc-corpus stock holds emission/absorption <= 1.0 for a quarter"
    })) == []


def test_empty_condition_is_refused():
    errs = _em.validate_meta(_rule(**{"retire-when": "   "}))
    assert errs and "non-empty condition" in errs[0]


def test_non_string_condition_is_refused():
    """A date is the shape this key exists to refuse — but the type check is what catches a
    YAML-parsed date object, since `retire-when: 2026-12-01` parses to a `datetime.date`."""
    import datetime
    errs = _em.validate_meta(_rule(**{"retire-when": datetime.date(2026, 12, 1)}))
    assert errs and "non-empty condition" in errs[0]


def test_bare_never_is_refused_but_reasoned_never_passes():
    """The whole value of admitting `never` is that a floor becomes COUNTABLE rather than
    silent. A bare `never` is indistinguishable from not having asked, so it is refused; a
    reasoned one is a legitimate terminal answer."""
    bare = _em.validate_meta(_rule(**{"retire-when": "never"}))
    assert bare and "indistinguishable from not having asked" in bare[0]
    reasoned = _em.validate_meta(_rule(**{
        "retire-when": "never: this is a format precondition, not a habit that can be learned"
    }))
    assert reasoned == []


def test_retire_when_is_a_known_key():
    """Regression guard: `_KNOWN_RULE_KEYS` drives the typo check, so a key validated but not
    registered there would be reported as a typo by the very gate that accepts it."""
    assert "retire-when" in _em._KNOWN_RULE_KEYS


def test_a_binding_may_not_declare_its_own_retire_when():
    """Semantics are policy-owned. A binding declaring its own exit would be re-deciding the
    policy's lifecycle from the placement side."""
    assert "retire-when" not in _em._BINDING_KEYS
    errs = _em.validate_meta({"epr-meta-version": 1, "rules": [
        {"id": "b", "policy": "some-policy@1", "retire-when": "when x holds"}]})
    assert errs and "must not redeclare" in errs[0]


# ------------------------------------------------- the registry path + hash

def test_registry_gate_shares_one_vocabulary_with_the_manifest_gate(tmp_path):
    """Same two refusals on the path that actually feeds the evaluator. A registry that skipped
    the check would be the one place an uninformative exit could enter."""
    import textwrap
    reg = tmp_path / ".claude" / "epr-meta"
    reg.mkdir(parents=True)
    (reg / "policies.yaml").write_text(textwrap.dedent("""
        epr-meta-policies-version: 1
        policies:
          - id: p-bare
            version: 1
            class: measure
            measure: { kind: level, loc-soft: 10 }
            retire-when: never
          - id: p-good
            version: 1
            class: measure
            measure: { kind: level, loc-soft: 10 }
            retire-when: "when the census holds at zero for 8 weeks"
    """).strip() + "\n")
    pols, errs = _em.load_policies(tmp_path)
    assert any("indistinguishable from not having asked" in e for e in errs)
    # Advisory, NOT a load-blocker: an unusable retire condition must never silently
    # un-enforce a live rule. Both policies still load.
    assert "p-bare@1" in pols and "p-good@1" in pols


def test_declaring_an_exit_does_not_move_the_policy_content_hash():
    """The incentive claim, pinned. contentHash covers what a policy DOES to a diff; a removal
    condition says when it stops existing. If adding one forced a version bump plus a re-pin,
    the cheapest thing in the registry would be accretion and the most expensive would be the
    exit — exactly inverted."""
    pol = {"id": "p", "version": 1, "class": "measure", "measure": {"kind": "level"}}
    before = _em.policy_content_hash(pol)
    after = _em.policy_content_hash({**pol, "retire-when": "when the habit holds for a quarter"})
    assert before == after
    assert "retire-when" in _em._HASH_EXCLUDE_KEYS


# ------------------------------------------------------------- the census

def test_census_counts_the_live_repo_and_declares_itself_a_level():
    data = _ic.census_data(REPO)
    m = _ic.lacking_measure(data)
    assert m["kind"] == "level", "a count of un-retired intervenors is a stock, not a rate"
    assert m["value"] == float(data["lacking"])
    # Witnessed and exact: every row was read off disk. The uncertainty here is in SCOPE, and
    # the honest home for scope uncertainty is the basis, not a widened interval around a
    # number that is not actually uncertain.
    assert m["confidence"]["claim"] == "witnessed"
    assert m["confidence"]["interval"]["lo"] == m["confidence"]["interval"]["hi"]
    assert "EXCLUDED from the population" in m["confidence"]["basis"]


def test_the_census_appears_in_its_own_reading():
    """The recursion, mechanical rather than rhetorical. A gate counting un-retired intervenors
    is an intervenor; if it could not be found in its own population the measure would be
    self-exempting."""
    rows = _ic.census_data(REPO)["rows"]
    me = [r for r in rows if "intervenor_census.py" in r["where"]]
    assert len(me) == 1, "the meter must appear exactly once in its own census"
    assert me[0]["retire_when"] == _ic.RETIRE_WHEN
    assert not _ic._is_never(_ic.RETIRE_WHEN), (
        "if this meter's own exit could only be written as `never`, that is evidence the "
        "mechanism is wrong and should be said out loud rather than shipped")


def test_test_fixtures_are_not_counted_as_live_intervenors():
    """Fixture manifests exist to be parsed by the parity suite; they gate no real write, so a
    removal condition could never apply to them. Counting them would inflate the stock."""
    rows = _ic.census_data(REPO)["rows"]
    assert not [r for r in rows if "/fixtures/" in r["where"] or "__tests__" in r["where"]]


def test_a_binding_inherits_its_policys_condition():
    """Placement inherits semantics. A rule that binds a retired-condition policy must not be
    scored as lacking one — that would count the same intervenor's silence twice."""
    rows = _ic.census_data(REPO)["rows"]
    inherited = [r for r in rows if r.get("inherited")]
    assert inherited, (
        "expected at least one binding to inherit its policy's retire condition; if this "
        "fails, either the registry backfill regressed or binding resolution is broken")
    assert all(r["retire_when"] for r in inherited)


def test_superseded_policies_are_not_counted():
    """A superseded policy is already retired by lineage. Counting it would report a debt that
    the registry's own supersession mechanism has settled."""
    import yaml
    data = yaml.safe_load((REPO / _em.POLICY_REGISTRY_REL).read_text())
    superseded = {f"{p['id']}@{p['version']}" for p in data["policies"]
                  if p.get("status") == "superseded"}
    assert superseded, "fixture assumption: the live registry carries supersession lineage"
    counted = {r["id"] for r in _ic.census_data(REPO)["rows"]
               if r["population"] == "epr-meta-policy"}
    assert not (superseded & counted)
