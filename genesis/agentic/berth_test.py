"""Capacity changes must account for both legacy leases and slot leases."""
import runpy
import tempfile
import unittest
from pathlib import Path

Berth = runpy.run_path(str(Path(__file__).parent / "bin/berth"))["Berth"]


class CapacityTransitionTest(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.berth = Berth(self.directory.name, now=lambda: 100)
        for session in ("first", "second", "third"):
            self.berth.moor(session)
        self.berth.capacities["cargo"] = 1

    def holders(self):
        result = self.berth.who("cargo")
        rows = [result] if isinstance(result, dict) else result or []
        return {row["holder"] for row in rows}

    def test_capacity_increase_and_reduction_preserve_incumbents(self):
        self.assertTrue(self.berth.claim("cargo", "first")[0])
        original = dict(self.berth.leases()["cargo"])
        self.berth.capacities["cargo"] = 2
        self.assertTrue(self.berth.claim("cargo", "second")[0])
        self.assertFalse(self.berth.claim("cargo", "third")[0])
        self.assertEqual(self.holders(), {"first", "second"})
        self.assertEqual(self.berth.leases()["cargo"], original)
        self.berth.capacities["cargo"] = 1
        self.assertFalse(self.berth.claim("cargo", "third")[0])
        self.assertTrue(self.berth.claim("cargo", "second")[0])
        self.assertEqual(self.holders(), {"first", "second"})
        self.assertTrue(self.berth.release("cargo", "first"))
        self.assertFalse(self.berth.claim("cargo", "third")[0])
        self.assertTrue(self.berth.release("cargo", "second"))
        self.assertTrue(self.berth.claim("cargo", "third")[0])
        self.assertEqual(self.holders(), {"third"})

    def test_unmoor_releases_slot_without_removing_other_holder(self):
        self.berth.claim("cargo", "first")
        self.berth.capacities["cargo"] = 2
        self.berth.claim("cargo", "second")
        self.berth.unmoor("second")
        self.assertEqual(self.holders(), {"first"})
        self.assertTrue(self.berth.claim("cargo", "third")[0])

    def test_expired_legacy_holder_does_not_consume_a_slot(self):
        self.berth.claim("cargo", "first", ttl_s=1)
        self.berth.now = lambda: 102
        self.berth.capacities["cargo"] = 2
        self.assertTrue(self.berth.claim("cargo", "second")[0])
        self.assertTrue(self.berth.claim("cargo", "third")[0])
        self.assertEqual(self.holders(), {"second", "third"})


if __name__ == "__main__":
    unittest.main()
