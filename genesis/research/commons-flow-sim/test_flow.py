#!/usr/bin/env python3
"""Hand-worked checks for flow.py over fixtures/tiny/Cargo.lock.

The fixture: workspace crates `app` and `app-cli` are one entry work, relying on `a` and `b`;
`a` and `b` each rely on `c 1.0.0`; `c 1.0.0` relies on nothing recorded; `c 2.0.0` is in the
lockfile but unreached. Lens for the tests: entry keeps 1/2, upstream keeps 1/2, equal split,
$1,000 in, tiers keep 2/3 / 2/3 / all.

Worked by hand, no positional share, no floor:
  entry: labour 500, cites a 250, b 250
  a: labour 125, cite c 125          b: labour 125, cite c 125
  c: inbox 250, labour 125; its "rests on" half (125) has no record -> rust commons
  rust commons 125: keeps 250/3, passes 125/3; software keeps 250/9, passes 125/9 = planetary
  makers 875 + commons 125 = 1000

Floor 130: a->c and b->c (125 each) fall below it and pool at the rust commons with descent;
  c receives nothing; makers 750, commons 250.

Positional share, cap 1/5, half-point 2 (a, b fan-in 1 -> 1/15; c fan-in 2 -> 1/10):
  a: inbox 250, positional 50/3, labour 350/3, cite c 350/3 (same for b)
  c: inbox 700/3, positional 70/3, labour 105, no-record 105
  makers 500 + 700/3 + 105 = 2515/3; commons 170/3 + 105 = 485/3
"""

import json
import os
import sys
import tempfile
import unittest
from fractions import Fraction as Fr

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import flow  # noqa: E402

LOCK = os.path.join(HERE, "fixtures", "tiny", "Cargo.lock")
A, B, C = "a 0.1.0", "b 0.2.0", "c 1.0.0"


def lens(floor="0", cap="0", half=1, rule="equal"):
    raw = {
        "lens": "test/v0", "status": "test lens",
        "inflow": {"amount": "1000", "unit": "USD"},
        "entry": {"keep_for_own_labour": "1/2"},
        "upstream": {"keep_for_own_labour": "1/2"},
        "split_rule": {"rule": rule},
        "positional_share": {"cap": cap, "half_point": half},
        "precision_floor": {"amount": floor},
        "tiers": [
            {"id": "rust", "keep": "2/3", "parent": "software"},
            {"id": "software", "keep": "2/3", "parent": "planetary"},
            {"id": "planetary", "keep": "1", "parent": None},
        ],
        "tier_of": {"entry": "rust", "registry": "rust"},
    }
    fd, path = tempfile.mkstemp(suffix=".json")
    with os.fdopen(fd, "w") as fh:
        json.dump(raw, fh)
    try:
        return flow.load_lens(path)
    finally:
        os.unlink(path)


class Graph(unittest.TestCase):
    def test_workspace_collapses_and_unreached_is_pruned(self):
        with open(LOCK) as fh:
            g = flow.build_graph(flow.parse_lock(fh.read()))
        self.assertEqual(g["workspace"], ["app 0.0.1", "app-cli 0.0.1"])
        self.assertEqual(g["out"][flow.ROOT], [A, B])
        self.assertEqual(g["out"][A], [C])
        self.assertEqual(g["out"][B], [C])  # the "name version (source)" form resolved
        self.assertNotIn("c 2.0.0", g["out"])
        self.assertEqual(g["fan_in"][C], 2)

    def test_degrees_and_paths(self):
        r = flow.run(LOCK, lens())
        self.assertEqual(r["deg"], {flow.ROOT: 0, A: 1, B: 1, C: 2})
        self.assertEqual(r["paths"][C], 2)
        self.assertEqual(r["work"]["naive_per_path_pushes"], 4)


class Flow(unittest.TestCase):
    def test_no_floor_by_hand(self):
        r = flow.run(LOCK, lens())
        self.assertEqual(r["kept"], {flow.ROOT: 500, A: 125, B: 125, C: 125})
        self.assertEqual(r["tier_kept"], {"rust": Fr(250, 3), "software": Fr(250, 9),
                                          "planetary": Fr(125, 9)})
        self.assertEqual(r["tier_kept_by_kind"]["planetary"]["no-record"], Fr(125, 9))
        self.assertTrue(r["conserved"])
        self.assertEqual(r["work"]["edges_touched"], 4)
        self.assertEqual(r["work"]["hot_spot"], C)
        self.assertEqual(r["work"]["hot_spot_messages_this_window"], 2)

    def test_floor_pools_with_descent_and_never_deletes(self):
        r = flow.run(LOCK, lens(floor="130"))
        self.assertEqual(r["kept"], {flow.ROOT: 500, A: 125, B: 125, C: 0})
        self.assertEqual(r["tier_received"]["rust"], 250)
        self.assertEqual(sorted(x["reason"] for x in r["residue_log"]),
                         [f"descent:{A}->{C}", f"descent:{B}->{C}"])
        self.assertEqual(r["work"]["edges_pooled_below_floor"], 2)
        self.assertEqual(r["reached_but_below_floor"], 1)
        self.assertTrue(r["conserved"])

    def test_positional_share_by_hand(self):
        r = flow.run(LOCK, lens(cap="1/5", half=2))
        self.assertEqual(r["kept"][A], Fr(350, 3))
        self.assertEqual(r["inbox"][C], Fr(700, 3))
        self.assertEqual(r["kept"][C], 105)
        self.assertEqual(sum(r["kept"].values()), Fr(2515, 3))
        self.assertEqual(r["tier_received"]["rust"], Fr(485, 3))
        self.assertTrue(r["conserved"])

    def test_fan_in_rule_conserves(self):
        self.assertTrue(flow.run(LOCK, lens(rule="fan_in", cap="1/5", half=2))["conserved"])


class Invariants(unittest.TestCase):
    def test_ratios_sum_to_one_and_every_ratio_has_a_reason(self):
        lines = flow.split_locally(A, Fr(250), [C], 1, {C: 2}, False, lens(cap="1/5", half=2))
        self.assertEqual(sum(l["ratio"] for l in lines), 1)
        flow.verify_split(A, lines, [C])

    def test_a_share_with_no_cite_record_is_refused(self):
        lines = flow.split_locally(A, Fr(250), [C], 1, {}, False, lens())
        lines[-1]["to"] = "z 9.9.9"  # a share to something the node does not cite
        with self.assertRaises(AssertionError):
            flow.verify_split(A, lines, [C])

    def test_a_split_that_does_not_sum_to_one_is_refused(self):
        lines = flow.split_locally(A, Fr(250), [C], 1, {}, False, lens())
        lines[0]["ratio"] += Fr(1, 10**12)
        with self.assertRaises(AssertionError):
            flow.verify_split(A, lines, [C])

    def test_a_second_inflow_reuses_every_ratio_table(self):
        tables, lz = {}, lens(floor="130")
        first = flow.run(LOCK, lz, tables)
        second = flow.run(LOCK, lz, tables)
        self.assertEqual(first["work"]["ratio_tables_judged"], 3)  # entry, a, b (c never reached)
        self.assertEqual(second["work"]["ratio_tables_judged"], 0)
        self.assertEqual(second["work"]["ratio_tables_reused"], 3)
        self.assertEqual(second["kept"], first["kept"])

    def test_omitting_cites_does_not_enrich_the_omitter(self):
        # A leaf keeps exactly its labour share; its "rests on" share goes to the commons.
        lines = flow.split_locally(C, Fr(100), [], 0, {}, False, lens())
        kinds = {l["kind"]: l["ratio"] for l in lines}
        self.assertEqual(kinds, {"labour": Fr(1, 2), "no-record": Fr(1, 2)})


class Inward(unittest.TestCase):
    def test_retraction_at_c_by_hand(self):
        sig = flow.inward(flow.run(LOCK, lens()), "c")
        self.assertEqual(sig["target"], C)
        self.assertEqual(sig["dependents_by_reverse_degree"], {1: 2, 2: 1})  # a, b; then entry
        self.assertEqual(sig["pushed_messages"], 4)        # entry->a, entry->b, a->c, b->c
        self.assertEqual(sig["pulled_checks_per_window"], 4)
        self.assertEqual(sig["pulled_windows_to_farthest"], 2)
        self.assertEqual(sig["value_through_target"], 250)


class Mixing(unittest.TestCase):
    def test_c_is_fully_mixed_in_the_fixture(self):
        # c is reached equally through both branches and is the only degree-2 node, so its
        # route composition equals the background: total variation 0.
        m = flow.mixing(flow.run(LOCK, lens()))
        self.assertEqual(m["branches"], 2)
        self.assertAlmostEqual(m["by_degree"][2]["tv_weighted"], 0.0)


class Lockfile(unittest.TestCase):
    def test_ambiguous_bare_dependency_is_refused(self):
        text = ('[[package]]\nname = "app"\nversion = "0.1.0"\ndependencies = [\n "c",\n]\n\n'
                '[[package]]\nname = "c"\nversion = "1.0.0"\nsource = "registry+x"\n\n'
                '[[package]]\nname = "c"\nversion = "2.0.0"\nsource = "registry+x"\n')
        with self.assertRaises(ValueError):
            flow.build_graph(flow.parse_lock(text))


if __name__ == "__main__":
    unittest.main(verbosity=2)
