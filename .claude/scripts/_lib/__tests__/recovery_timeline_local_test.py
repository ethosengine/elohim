#!/usr/bin/env python3
"""Focused tests for the mesh recovery timeline reader."""
import importlib.util
import io
import json
import os
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path


ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "..", ".."))
spec = importlib.util.spec_from_file_location(
    "recovery_timeline",
    os.path.join(ROOT, "genesis", "scripts", "recovery-timeline.py"),
)
timeline = importlib.util.module_from_spec(spec)
spec.loader.exec_module(timeline)


def record(
    scenario,
    shape,
    recovered,
    seconds,
    *,
    receipt=None,
    legs=(),
    zome_path="alive",
    run="1",
):
    labels = {"run": run}
    if scenario is not None:
        labels["scenario"] = scenario
    return {
        "shape": shape,
        "peer": "jessica",
        "recovered": recovered,
        "time_to_recover_s": seconds,
        "failing_legs": list(legs),
        "labels": labels,
        "conductor_receipt_max_s": {"recovering": None, "survivor": receipt},
        "zome_path": zome_path,
    }


class RecoveryTimeline(unittest.TestCase):
    def test_summary_uses_recovered_times_and_excludes_null_receipts(self):
        rows = [
            record("homo-dual", "warm", True, 10, receipt=None, run="1"),
            record("homo-dual", "warm", True, 20, receipt=2.5, run="2"),
            record("homo-dual", "warm", False, 300, legs=("P2",), run="3"),
        ]

        summary = timeline.summarize(rows)

        self.assertEqual(summary["runs"], 3)
        self.assertEqual(summary["recovered"], 2)
        self.assertEqual(summary["median"], 15)
        self.assertEqual(summary["min"], 10)
        self.assertEqual(summary["max"], 20)
        self.assertEqual(summary["spread"], 10)
        self.assertEqual(summary["receipt_max"], 2.5)
        self.assertEqual(summary["failing_legs"], {"P2"})

    def test_table_keeps_missing_scenario_and_null_receipt_honest(self):
        rows = [
            record(None, "cold", False, 300, receipt=None, legs=("P1",)),
            record("homo-iroh", "cold", False, 304, receipt=None, legs=("P3", "P4")),
        ]

        rendered = timeline.render_table(rows)

        self.assertIn("<unlabeled>", rendered)
        self.assertIn("| homo-iroh | cold | 1 | 0 | — | — | — | — |", rendered)
        self.assertNotIn("0.0", rendered)

    def test_series_renders_null_receipt_as_absent(self):
        rendered = timeline.render_series(
            [record("split-libp2p-iroh", "warm", False, 300, legs=("P0", "P1", "P2"))]
        )

        self.assertIn("split-libp2p-iroh", rendered)
        self.assertIn("| 300 | — | alive | P0,P1,P2 |", rendered)

    def test_before_after_exposes_reliability_performance_spread_and_leg_delta_per_shape(self):
        # A warm-scenario improvement must render on its own row from the cold
        # row for the same scenario, so a cold regression can never hide
        # behind a warm gain (spec: one line per scenario x shape).
        before = [
            record("homo-dual", "warm", True, 10, legs=("P1",)),
            record("homo-dual", "cold", True, 30, legs=("P2",), run="2"),
            record("homo-dual", "cold", False, 300, legs=("P1", "P2"), run="3"),
        ]
        after = [
            record("homo-dual", "warm", True, 8, legs=("P2",)),
            record("homo-dual", "cold", True, 12, legs=("P3",), run="2"),
            record("homo-dual", "cold", True, 10, run="3"),
        ]

        rendered = timeline.render_comparison(before, after)

        self.assertIn(
            "| homo-dual | warm | 1/1 | 1/1 | 0 | 10 | 8 | -2 | 0 | 0 | 0 | P1 | P2 |",
            rendered,
        )
        self.assertIn(
            "| homo-dual | cold | 1/2 | 2/2 | +50 | 30 | 11 | -19 | 0 | 2 | +2 | P1,P2 | P3 |",
            rendered,
        )

    def test_before_after_scenario_shape_only_on_after_side_renders_dashes(self):
        before: list[dict] = []
        after = [record("homo-iroh", "cold", True, 5)]

        rendered = timeline.render_comparison(before, after)

        self.assertIn(
            "| homo-iroh | cold | — | 1/1 | — | — | 5 | — | — | 0 | — | — | — |",
            rendered,
        )

    def test_cli_table_reads_jsonl(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "recovery.jsonl"
            rows = [record("homo-dual", "warm", True, 9)]
            path.write_text("\n".join(json.dumps(row) for row in rows) + "\n", encoding="utf-8")
            output = io.StringIO()
            with redirect_stdout(output):
                result = timeline.main(["--table", str(path)])

        self.assertEqual(result, 0)
        self.assertIn("| homo-dual | warm | 1 | 1 | 9 | 9 | 9 |", output.getvalue())

    def test_cli_names_malformed_json_line(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "recovery.jsonl"
            path.write_text('{}\n{"broken"\n', encoding="utf-8")
            error = io.StringIO()
            with redirect_stderr(error):
                result = timeline.main(["--series", str(path)])

        self.assertEqual(result, 2)
        self.assertIn(f"{path}:2: invalid JSON", error.getvalue())


if __name__ == "__main__":
    unittest.main()
