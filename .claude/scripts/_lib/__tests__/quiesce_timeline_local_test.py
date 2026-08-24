#!/usr/bin/env python3
"""quiesce-timeline.py --local: the local mesh gate's lines parse into the SAME
record shape as a fleet build, tagged source=local, with optional labels."""
import importlib.util
import io
import json
import os
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "..", ".."))
spec = importlib.util.spec_from_file_location(
    "qt", os.path.join(ROOT, "genesis", "scripts", "quiesce-timeline.py")
)
qt = importlib.util.module_from_spec(spec)
spec.loader.exec_module(qt)

LOG = """fleet-quiesce[2026-08-24T18:00:00Z]: starting — deadline=1200s poll=10s sustain=33s content=elohim-host-landing
fleet-quiesce[2026-08-24T18:00:00Z]: FAIL A-caughtUp=False B-caughtUp=True A-quiesced=False(actionable=None) — (A-not-caughtUp A-not-quiesced)
fleet-quiesce[2026-08-24T18:00:10Z]: PASS A-caughtUp=True B-caughtUp=True A-quiesced=True — sustained 0s, need 33s
fleet-quiesce[2026-08-24T18:00:40Z]: PASS A-caughtUp=True B-caughtUp=True A-quiesced=True — sustained 30s, need 33s
fleet-quiesce[2026-08-24T18:00:50Z]: PASS A-caughtUp=True B-caughtUp=True A-quiesced=True — sustained 40s, need 33s
FLEET QUIESCENT (A-QUIESCED; B excluded from predicate)
"""


class LocalParse(unittest.TestCase):
    def test_local_log_parses_as_measured_with_source_and_labels(self):
        rec = qt.parse_quiesce(
            None,
            LOG,
            None,
            source="local",
            labels={"scenario": "homo-dual", "shape": "warm"},
        )
        self.assertEqual(rec["outcome"], "measured")
        self.assertEqual(rec["source"], "local")
        self.assertEqual(rec["labels"], {"scenario": "homo-dual", "shape": "warm"})
        self.assertEqual(rec["best_window_s"], 40)
        self.assertEqual(rec["time_to_verdict_s"], 50)
        self.assertEqual(rec["blocking_legs"], {"A-not-caughtUp": 1, "A-not-quiesced": 1})
        self.assertIsNone(rec["build"])

    def test_fleet_default_source(self):
        rec = qt.parse_quiesce(1379, LOG, "SUCCESS")
        self.assertEqual(rec["source"], "fleet")
        self.assertEqual(rec["labels"], {})

    def test_cli_local_prints_a_record(self):
        with tempfile.NamedTemporaryFile("w", suffix=".log", delete=False) as f:
            f.write(LOG)
            path = f.name
        self.addCleanup(os.unlink, path)
        out = io.StringIO()
        with redirect_stdout(out):
            rc = qt.main(["--local", path, "--label", "scenario=homo-dual"])
        self.assertEqual(rc, 0)
        self.assertIn("MEASURED", out.getvalue())
        self.assertIn("homo-dual", out.getvalue())

    def test_recorded_local_row_remains_readable_with_fleet_series(self):
        with tempfile.TemporaryDirectory() as td:
            series = Path(td) / "quiesce-timeline.jsonl"
            gate_log = Path(td) / "gate.log"
            gate_log.write_text(LOG, encoding="utf-8")
            fleet = qt.parse_quiesce(1379, LOG, "SUCCESS")
            series.write_text(json.dumps(fleet) + "\n", encoding="utf-8")

            original_series = qt.SERIES
            qt.SERIES = series
            try:
                out = io.StringIO()
                with redirect_stdout(out):
                    record_rc = qt.main(["--local", str(gate_log), "--record"])
                    series_rc = qt.main(["--series"])
            finally:
                qt.SERIES = original_series

        self.assertEqual(record_rc, 0)
        self.assertEqual(series_rc, 0)
        self.assertIn("1379", out.getvalue())
        self.assertIn("local", out.getvalue())


if __name__ == "__main__":
    unittest.main()
