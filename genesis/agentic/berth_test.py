"""berth: capacity transitions across lease shapes, and the mesh lease as the dev berth's router
(lease class + ttl, measure refused with a declared override, ledger locking, overrun pain).

Run: python3 -m unittest genesis.agentic.berth_test
"""
import contextlib
import io
import json
import os
import runpy
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path
from unittest import mock

REPO = Path(__file__).resolve().parents[2]

# The test runner's marker: a berth that sees it (this process, and every berth subprocess a test
# spawns, which inherits it) never folds a record into the checkout it lives in through the real
# emitter. A suite that wants a fold injects an emitter or points the store at a scratch checkout.
os.environ["BERTH_UNDER_TEST"] = "1"

_NS = runpy.run_path(str(Path(__file__).parent / "bin/berth"))
Berth = _NS["Berth"]
BerthUsage = _NS["BerthUsage"]
render_status = _NS["render_status"]
MISSING_POLICY = "/nonexistent/pool-policy.json"
CLASSES = {"mesh": {"verify": {"default_ttl_s": 1800, "max_ttl_s": 3600},
                    "measure": {"allowed": False, "alternative": "just measure <scope>"},
                    "mesh": {"daemon": True}}}


class CapacityTransitionTest(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.berth = Berth(self.directory.name, obs=FakeObservation(), now=lambda: 100, policy_path=MISSING_POLICY)
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

    def test_each_slot_shows_renewal_expiry_and_closure(self):
        self.berth.capacities["cargo"] = 2
        for session in ("first", "second"):
            self.berth.claim("cargo", session, ttl_s=600, closure=f"key-{session}")
        self.berth.now = lambda: 400
        self.berth.claim("cargo", "first", ttl_s=600)
        status = render_status(self.berth)
        self.assertIn("cargo       2/2", status)
        self.assertIn("last renewed 0s ago, expires in 10m at 1970-01-01T00:16:40Z) closure=key-first", status)
        self.assertIn("last renewed 5m ago, expires in 5m at 1970-01-01T00:11:40Z) closure=key-second", status)
        holders = self.berth.who("cargo")
        self.assertEqual([(l["last_renewed_at"], l["expires_at"]) for l in holders], [(400, 1000), (100, 700)])
        claims = [r for r in self.berth.ledger(100) if r["kind"] == "claim"]
        self.assertEqual([r["closure"] for r in claims], ["key-first", "key-second"])



class FakeObservation:
    """Stands in for .claude/hooks/_observation: records emits, never spawns `epr`."""

    def __init__(self, available=True):
        self._available = available
        self.emits = []  # pain emits only; the per-record dev-berth-events@1 folds land in `events`
        self.events = []

    def available(self):
        return self._available

    def emit(self, measure, subject, value, *, reason, env=None, root=None):
        row = {"measure": measure, "subject": subject, "value": value, "reason": reason, "env": env,
               "root": root}
        (self.events if measure == "dev-berth-events@1" else self.emits).append(row)
        return True


class TestRunnerNeverFoldsIntoThisCheckoutTest(unittest.TestCase):
    """Under the test runner a berth with the real emitter (none injected) refuses to fold into the
    checkout it lives in, so a suite run never grows this checkout's flow store; a scratch store
    still folds."""

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.real = FakeObservation()
        saved = sys.modules.get("_observation")
        sys.modules["_observation"] = self.real  # what `import _observation` finds: the "real" one
        self.addCleanup(lambda: sys.modules.__setitem__("_observation", saved) if saved is not None
                        else sys.modules.pop("_observation", None))
        self.clock = [100.0]
        self.berth = Berth(self.directory.name, now=lambda: self.clock[0], policy_path=MISSING_POLICY)

    def test_records_and_pain_never_fold_into_this_checkout(self):
        self.assertEqual(os.environ.get("BERTH_UNDER_TEST"), "1")
        self.berth.moor("dev")
        self.berth.claim("cargo", "dev")
        self.berth.say("dev", "on the record")
        self.berth.release("cargo", "dev")
        self.berth._emit_pain(5, "a pain")
        self.assertEqual(self.real.events, [])
        self.assertEqual(self.real.emits, [])

    def test_a_scratch_checkout_still_folds(self):
        scratch = os.path.join(self.directory.name, "scratch-checkout")
        self.berth._fold_root = lambda ev: (scratch, None)
        self.berth.say("dev", "folds to the scratch checkout")
        self.assertEqual([e["root"] for e in self.real.events], [scratch])

    def test_a_repointed_repo_root_folds_into_its_scratch_checkout(self):
        scratch = os.path.join(self.directory.name, "scratch-root")
        g = Berth._fold_root.__globals__
        saved = g["REPO_ROOT"]
        g["REPO_ROOT"] = scratch
        self.addCleanup(g.__setitem__, "REPO_ROOT", saved)
        self.berth.say("dev", "into the scratch checkout")
        self.assertEqual([e["root"] for e in self.real.events], [scratch])

    def test_outside_the_runner_this_checkout_folds_as_before(self):
        os.environ.pop("BERTH_UNDER_TEST")
        self.addCleanup(os.environ.__setitem__, "BERTH_UNDER_TEST", "1")
        self.berth.say("dev", "a real record")
        self.assertEqual([e["root"] for e in self.real.events], [_NS["REPO_ROOT"]])


class FoldEveryKindTest(unittest.TestCase):
    """Every ledger record is also one dev-berth-events@1 fold (habit ledgers-fold-once). The subject is
    always `.`: `epr flow note` refuses a bare resource name (`cargo`) as an unknown atom, so the resource
    rides in env."""

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.clock = [1000.0]
        self.obs = FakeObservation()
        self.berth = Berth(self.directory.name, now=lambda: self.clock[0], obs=self.obs, policy_path=MISSING_POLICY)

    def folds(self):
        return self.obs.events

    def test_fold_every_kind_emits_once(self):
        b = self.berth
        steps = [
            ("moor", ".", lambda: b.moor("s1", model="m")),
            ("claim", ".", lambda: b.claim("cargo", "s1", note="why")),
            ("renew", ".", lambda: b.claim("cargo", "s1", note="again")),
            ("release", ".", lambda: b.release("cargo", "s1")),
            ("override", ".", lambda: b.record("override", resource="mesh", session="s1", ttl_s=60)),
            ("note", ".", lambda: b.say("s1", "hello")),
            ("shed", ".", lambda: b.record("shed", resource="cargo", guard="ram-guard", level=2)),
            ("unmoor", ".", lambda: b.unmoor("s1")),
            ("refuse", ".", lambda: b.record("refuse", resource="mesh", session="s1", reason="held")),
            ("overrun", ".", lambda: b.record("overrun", resource="mesh", holder="s1", ttl_s=60, held_s=90)),
        ]
        for kind, subject, act in steps:
            before = len(self.folds())
            act()
            new = self.folds()[before:]
            kinds = [e["reason"] for e in new]
            self.assertIn(kind, kinds, kind)
            self.assertEqual(kinds.count(kind), 1, kind)
            fold = [e for e in new if e["reason"] == kind][0]
            self.assertEqual((fold["subject"], fold["value"]), (subject, 1), kind)
            row = [r for r in b.ledger(1000) if r["kind"] == kind][-1]
            for key, val in row.items():
                if val is None:
                    continue
                self.assertEqual(fold["env"][key], val if isinstance(val, str) else json.dumps(val, sort_keys=True, separators=(",", ":")) if isinstance(val, (list, dict)) else str(val), f"{kind}.{key}")
        claim = [e for e in self.folds() if e["reason"] == "claim"][0]
        self.assertEqual((claim["env"]["resource"], claim["env"]["session"], claim["env"]["note"]), ("cargo", "s1", "why"))

    def test_fold_emitter_runs_with_the_berth_lock_released(self):
        """claim/release call record() inside the re-entrant lock; the epr subprocess must not."""
        seen = []
        b = self.berth

        def emit(measure, subject, value, *, reason, env=None, root=None):
            fd = os.open(b.lock_path, os.O_RDWR)
            try:
                import fcntl
                fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)  # raises if the lock is held
                fcntl.flock(fd, fcntl.LOCK_UN)
                seen.append((reason, b._lock_depth.get("berth")))
            finally:
                os.close(fd)
            return True

        b._obs = type("O", (), {"available": lambda self: True, "emit": staticmethod(emit)})()
        b.moor("s1")
        b.claim("cargo", "s1")
        b.release("cargo", "s1")
        self.assertEqual([r for r, _ in seen], ["moor", "claim", "release"])
        self.assertTrue(all(depth == 0 for _, depth in seen))

    def test_fold_emitter_missing_costs_fold_not_lease(self):
        self.obs._available = False
        for session in ("s1", "s2"):
            self.berth.moor(session)
        self.assertTrue(self.berth.claim("cargo", "s1")[0])
        self.assertEqual(self.berth.who("cargo")["holder"], "s1")
        failing = FakeObservation()
        failing.emit = lambda *a, **k: False
        b = Berth(self.directory.name, now=lambda: self.clock[0], obs=failing, policy_path=MISSING_POLICY)
        self.assertTrue(b.release("cargo", "s1"))
        self.assertTrue(b.claim("cargo", "s2")[0])
        self.assertEqual([r["kind"] for r in b.ledger(1000)][-2:], ["release", "claim"])
        boom = FakeObservation()
        boom.emit = lambda *a, **k: (_ for _ in ()).throw(RuntimeError("x"))
        b2 = Berth(self.directory.name, now=lambda: self.clock[0], obs=boom, policy_path=MISSING_POLICY)
        self.assertTrue(b2.release("cargo", "s2"))


class LeaseClassTest(unittest.TestCase):
    """The mesh lease routes: verify is bounded, measure is refused with its alternative."""

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.clock = [1000.0]
        self.obs = FakeObservation()
        self.berth = self.make()
        self.berth.classes = json.loads(json.dumps(CLASSES))
        for session in ("dev", "other"):
            self.berth.moor(session)

    def make(self):
        b = Berth(self.directory.name, now=lambda: self.clock[0], obs=self.obs, policy_path=MISSING_POLICY)
        b.classes = json.loads(json.dumps(CLASSES))
        return b

    def rows(self, kind=None):
        return [r for r in self.berth.ledger(1000) if kind is None or r["kind"] == kind]

    def test_policy_file_declares_the_mesh_classes_and_mooring_ttl(self):
        b = Berth(self.directory.name, obs=FakeObservation(), now=lambda: 0)
        self.assertEqual(b.classes["mesh"]["verify"], {"default_ttl_s": 1800, "max_ttl_s": 3600})
        self.assertIs(b.classes["mesh"]["measure"]["allowed"], False)
        self.assertEqual(b.mooring_ttl_s, 10800)
        self.assertNotIn("cargo", b.classes)

    def test_defaults_on_mesh_not_cargo(self):
        ok, lease, _ = self.berth.claim("mesh", "dev")
        self.assertTrue(ok)
        self.assertEqual((lease["class"], lease["ttl_s"], lease["claimed_at"]), ("verify", 1800, 1000.0))
        ok, lease, _ = self.berth.claim("cargo", "dev")
        self.assertTrue(ok)
        self.assertEqual(lease["class"], "verify")
        self.assertIsNone(lease["ttl_s"])

    def test_measure_refused_exit_3_naming_the_alternative(self):
        ok, lease, reason = self.berth.claim("mesh", "dev", klass="measure", ttl_s=7200)
        self.assertFalse(ok)
        self.assertIsNone(lease)
        self.assertIn("measure-class work does not run on the dev berth", reason)
        self.assertIn("`just measure <scope>`", reason)
        self.assertIn("rung A: --on adam", reason)
        self.assertEqual(self.rows("refuse"), [])
        self.assertIsNone(self.berth.raw_lease("mesh"))
        self.assertEqual(self.obs.emits, [])

    def test_measure_refused_through_the_cli_with_exit_3(self):
        main = _NS["main"]
        env = {"BERTH_DIR": self.directory.name, "ELOHIM_SESSION_ID": "dev"}
        old = {k: os.environ.get(k) for k in (*env, "MEASURE_ON_DEV_BERTH")}
        os.environ.update(env)
        os.environ.pop("MEASURE_ON_DEV_BERTH", None)
        try:
            self.assertEqual(main(["berth", "claim", "mesh", "--class", "measure", "--ttl", "600"]), 3)
            self.assertEqual(main(["berth", "claim", "mesh", "--class", "bogus"]), 2)
        finally:
            for k, v in old.items():
                if v is None:
                    os.environ.pop(k, None)
                else:
                    os.environ[k] = v

    def test_verify_ttl_past_the_cap_is_refused(self):
        ok, _, reason = self.berth.claim("mesh", "dev", klass="verify", ttl_s=7200)
        self.assertFalse(ok)
        self.assertIn("declare it measure", reason)
        self.assertEqual(self.rows("refuse"), [])
        self.assertTrue(self.berth.claim("mesh", "dev", klass="verify", ttl_s=3600)[0])

    def test_unknown_class_is_a_usage_error(self):
        with self.assertRaises(BerthUsage):
            self.berth.claim("mesh", "dev", klass="profile")

    def test_live_verify_refuses_another_session(self):
        self.assertTrue(self.berth.claim("mesh", "dev")[0])
        ok, lease, reason = self.berth.claim("mesh", "other")
        self.assertFalse(ok)
        self.assertEqual(lease["holder"], "dev")
        self.assertIn("held by dev", reason)

    def test_expired_verify_renders_expired_and_is_taken_over(self):
        self.berth.claim("mesh", "dev", ttl_s=60, note="test mesh x.feature")
        self.clock[0] += 61
        self.berth.touch("dev")
        self.berth.touch("other")
        status = render_status(self.berth)
        self.assertIn("mesh        EXPIRED dev — held 61s of a 60s verify ttl (takeover on next claim)", status)
        ok, lease, reason = self.berth.claim("mesh", "other")
        self.assertTrue(ok)
        self.assertEqual(reason, "taken over from a stale holder")
        row = self.rows("claim")[-1]
        self.assertEqual((row["took_over_from"], row["expired_after_s"]), ("dev", 60))
        self.assertEqual(lease["claimed_at"], self.clock[0])

    def test_renew_resets_since_not_claimed_at_and_carries_the_seat(self):
        self.berth.claim("mesh", "dev", ttl_s=600)
        self.clock[0] += 300
        ok, lease, reason = self.berth.claim("mesh", "dev", seat="che-a")
        self.assertTrue(ok)
        self.assertEqual(reason, "renewed")
        stored = self.berth.raw_lease("mesh")
        self.assertEqual((stored["claimed_at"], stored["since"]), (1000.0, 1300.0))
        renew = self.rows("renew")[-1]
        self.assertEqual((renew["seat"], renew["claimed_at"], renew["class"]), ("che-a", 1000.0, "verify"))

    def test_renewed_lease_renders_last_renew_and_expiry_after_original_ttl(self):
        self.berth.claim("mesh", "dev", ttl_s=1800, note="pre-push to dev", closure="key-a3")
        for _ in range(9):
            self.clock[0] += 600
            self.assertTrue(self.berth.claim("mesh", "dev", ttl_s=1800)[0])
        self.clock[0] += 60
        status = render_status(self.berth)
        self.assertIn("mesh        held by dev for 1h31 (verify, ttl 30m, last renewed 60s ago, "
                      "expires in 29m at 1970-01-01T02:16:40Z) closure=key-a3 — pre-push to dev", status)
        self.assertNotIn("EXPIRED dev", status)
        self.assertFalse(self.berth.claim("mesh", "other")[0])

    def test_renewed_lease_past_expiry_keeps_expired_wording(self):
        self.berth.claim("mesh", "dev", ttl_s=60, closure="key-a3")
        self.clock[0] += 30
        self.berth.claim("mesh", "dev", ttl_s=60)
        self.clock[0] += 61
        status = render_status(self.berth)
        self.assertIn("mesh        EXPIRED dev — held 1m of a 60s verify ttl (takeover on next claim)", status)
        self.assertNotIn("held by dev", status)
        self.assertIsNone(self.berth.who("mesh"))
        ok, lease, _ = self.berth.claim("mesh", "other")
        self.assertTrue(ok)
        self.assertNotIn("closure", lease)

    def test_closure_and_times_cross_on_claim_and_renew(self):
        self.berth.claim("mesh", "dev", ttl_s=600, closure="key-first")
        claim = self.rows("claim")[-1]
        self.assertEqual((claim["closure"], claim["last_renewed_at"], claim["expires_at"]),
                         ("key-first", 1000, 1600))
        self.clock[0] += 300
        self.berth.claim("mesh", "dev", ttl_s=600, closure="key-second")
        self.clock[0] += 300
        self.berth.claim("mesh", "dev", ttl_s=600)
        self.assertEqual(self.berth.raw_lease("mesh")["closure"], "key-second")
        renew = self.rows("renew")[-1]
        self.assertEqual((renew["closure"], renew["last_renewed_at"], renew["expires_at"]),
                         ("key-second", 1600, 2200))
        events = [e["env"] for e in self.obs.events if e["reason"] in ("claim", "renew")]
        self.assertEqual([e["closure"] for e in events], ["key-first", "key-second", "key-second"])
        self.assertEqual(float(events[-1]["expires_at"]), 2200)

    def test_cli_claim_accepts_closure_and_who_adds_times(self):
        main = _NS["main"]
        with mock.patch.dict(main.__globals__, {"Berth": lambda: self.berth}):
            with contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(main(["berth", "claim", "mesh", "--session", "dev",
                                       "--ttl", "600", "--closure", "key-cli"]), 0)
                self.clock[0] += 300
                self.assertEqual(main(["berth", "claim", "mesh", "--session", "dev",
                                       "--ttl", "600", "--closure", "key-renew"]), 0)
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                self.assertEqual(main(["berth", "who", "mesh"]), 0)
        holder = json.loads(output.getvalue())
        raw = self.berth.raw_lease("mesh")
        self.assertEqual({k: holder[k] for k in raw}, raw)
        self.assertEqual((holder["closure"], holder["last_renewed_at"], holder["expires_at"]),
                         ("key-renew", 1300, 1900))

    def test_cli_closure_requires_a_key(self):
        main = _NS["main"]
        with mock.patch.dict(main.__globals__, {"Berth": lambda: self.berth}):
            for args in (["--closure"], ["--closure", ""], ["--closure", "--offline-ok"]):
                with self.subTest(args=args), contextlib.redirect_stderr(io.StringIO()):
                    self.assertEqual(main(["berth", "claim", "cargo", "--session", "dev", *args]), 2)
        self.assertIsNone(self.berth.who("cargo"))

    def test_legacy_lease_without_closure_gets_derived_json_times(self):
        legacy = {"resource": "cargo", "holder": "dev", "since": 900, "ttl_s": 600}
        self.berth._write_leases({"cargo": legacy})
        self.assertEqual(self.berth.who("cargo"), {**legacy, "last_renewed_at": 900, "expires_at": 1500})
        self.assertIn("last renewed 1m ago, expires in 8m at 1970-01-01T00:25:00Z", render_status(self.berth))
        self.assertEqual(self.berth.raw_lease("cargo"), legacy)

    def test_renew_without_ttl_stops_at_the_class_cap(self):
        self.berth.claim("mesh", "dev", ttl_s=1800)
        self.clock[0] += 1500
        self.berth.touch("dev")
        self.assertTrue(self.berth.claim("mesh", "dev", ttl_s=1800)[0])
        self.clock[0] += 1500
        self.berth.touch("dev")
        self.assertTrue(self.berth.claim("mesh", "dev")[0])
        self.assertEqual(self.berth.raw_lease("mesh")["ttl_s"], 600)  # 1000 + 3600 - 4000
        self.clock[0] += 599
        self.berth.touch("dev")
        self.clock[0] += 2
        self.berth.touch("dev")
        # past the cap: the lease lapsed; a bare renew cannot resurrect the old hold
        ok, _, reason = self.berth.claim("mesh", "dev")
        self.assertTrue(ok)
        self.assertEqual(reason, "re-claimed after its own lease lapsed")

    def test_renew_refused_at_the_cap_without_explicit_ttl(self):
        self.berth.claim("mesh", "dev", ttl_s=3600)
        self.clock[0] += 3500
        self.berth.touch("dev")
        self.berth.claim("mesh", "dev", ttl_s=3600)  # explicit: capped at 3600 from now
        self.clock[0] += 200  # 3700 s since claimed_at, lease still live
        self.berth.touch("dev")
        ok, _, reason = self.berth.claim("mesh", "dev")
        self.assertFalse(ok)
        self.assertIn("reached its 3600s cap", reason)
        self.assertEqual(self.rows("refuse")[-1]["reason"], "max_ttl")
        self.assertFalse(self.berth.claim("mesh", "dev", ttl_s=99999)[0])  # an explicit ttl is itself capped
        ok, _, _ = self.berth.claim("mesh", "dev", ttl_s=3600)
        self.assertTrue(ok)
        self.assertEqual(self.berth.raw_lease("mesh")["ttl_s"], 3600)

    def test_override_writes_a_row_and_emits_exactly_once(self):
        ok, lease, reason = self.berth.claim("mesh", "dev", klass="measure", ttl_s=5400,
                                             note="K0 A/B window", override=True)
        self.assertTrue(ok)
        self.assertEqual(lease["class"], "measure")
        override = self.rows("override")
        self.assertEqual(len(override), 1)
        self.assertEqual((override[0]["holder"], override[0]["ttl_s"], override[0]["reason"]),
                         ("dev", 5400, "K0 A/B window"))
        self.assertEqual(len(self.obs.emits), 1)
        self.assertEqual((self.obs.emits[0]["measure"], self.obs.emits[0]["value"]),
                         ("dev-berth-held-by-measure@1", 5400))
        self.clock[0] += 6000
        self.assertIsNone(self.berth.overrun_check("mesh"))  # one pain per lease
        self.assertEqual(len(self.obs.emits), 1)

    def test_override_needs_a_declared_ttl(self):
        with self.assertRaises(BerthUsage):
            self.berth.claim("mesh", "dev", klass="measure", override=True)

    def test_overrun_check_emits_once_per_lease(self):
        self.berth.claim("mesh", "dev", ttl_s=1800)
        self.clock[0] += 1800
        self.assertIsNone(self.berth.overrun_check())
        self.clock[0] += 100
        self.assertEqual(self.berth.overrun_check(), 1900)
        self.assertEqual(self.obs.emits[-1]["value"], 1900)
        self.clock[0] += 100
        self.assertIsNone(self.berth.overrun_check())
        self.assertEqual(len(self.obs.emits), 1)
        self.assertEqual(len(self.rows("overrun")), 1)
        # a new lease is a new pain
        self.berth.release("mesh", "dev")
        self.berth.touch("dev")
        self.berth.claim("mesh", "dev", ttl_s=60)
        self.clock[0] += 61
        self.assertEqual(self.berth.overrun_check(), 61)
        self.assertEqual(len(self.obs.emits), 2)

    def test_overrun_check_never_emits_without_an_emitter(self):
        self.obs._available = False
        self.berth.claim("mesh", "dev", ttl_s=60)
        self.clock[0] += 120
        self.assertIsNone(self.berth.overrun_check())
        self.assertEqual(self.obs.emits, [])
        self.assertFalse(os.path.isdir(self.berth.emitted_dir) and os.listdir(self.berth.emitted_dir))
        self.obs._available = True  # the emitter returns: the unrecorded overrun lands then
        self.assertEqual(self.berth.overrun_check(), 120)


class LedgerSafetyTest(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.berth = Berth(self.directory.name, obs=FakeObservation(), now=lambda: 5, policy_path=MISSING_POLICY)

    def test_ledger_tolerates_a_nul_prefixed_and_a_torn_line(self):
        self.berth.record("note", session="a", text="before")
        with open(self.berth.ledger_path, "ab") as f:
            f.write(b"\x00\x00\x00" + json.dumps({"at": 5, "kind": "note", "text": "padded"}).encode() + b"\n")
            f.write(b'{"at": 5, "kind": "no\n')
        self.berth.record("note", session="a", text="after")
        texts = [ev.get("text") for ev in self.berth.ledger(10)]
        self.assertEqual(texts, ["before", "padded", "after"])

    def test_forked_writers_never_interleave(self):
        writers, per = 6, 200
        pids = []
        for w in range(writers):
            pid = os.fork()
            if pid == 0:
                try:
                    b = Berth(self.directory.name, obs=FakeObservation(), now=lambda: 5, policy_path=MISSING_POLICY)
                    for i in range(per):
                        b.record("note", session=f"w{w}", text=f"{w}:{i}:" + "x" * 512)
                        b.claim("cargo", f"w{w}") if i % 50 == 0 else None
                finally:
                    os._exit(0)
            pids.append(pid)
        for pid in pids:
            os.waitpid(pid, 0)
        with open(self.berth.ledger_path, "rb") as f:
            raw = f.read()
        self.assertNotIn(b"\x00", raw)
        lines = raw.decode().splitlines()
        parsed = [json.loads(line) for line in lines]  # every line whole
        notes = [ev for ev in parsed if ev["kind"] == "note"]
        self.assertEqual(len(notes), writers * per)
        with open(self.berth.leases_path) as f:
            json.load(f)


class EnvelopeResolutionTest(unittest.TestCase):
    """Berth consumes the one P1 identity; mooring age cannot choose another session."""
    def test_shared_shim_selects_owner_and_records_source(self):
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as directory:
            berth = Berth(directory, obs=FakeObservation(), policy_path=MISSING_POLICY)
            berth.moor("newer-unrelated")
            identity = {"session": "codex-p1", "ownerPid": None}
            with patch.dict(_NS["main"].__globals__, {"Berth": lambda: berth, "resolve_session_identity": lambda: identity}):
                self.assertEqual(_NS["main"](["berth", "claim", "cargo"]), 0)
            row = [r for r in berth.ledger(100) if r["kind"] == "claim"][-1]
            self.assertEqual((row["session"], row["session_source"]), ("codex-p1", "envelope"))

    def test_absent_p1_never_uses_newest_mooring(self):
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as directory:
            berth = Berth(directory, obs=FakeObservation(), policy_path=MISSING_POLICY)
            berth.moor("unrelated")
            with patch.dict(_NS["main"].__globals__, {"Berth": lambda: berth, "resolve_session_identity": lambda: {"session": None}}):
                self.assertEqual(_NS["main"](["berth", "claim", "cargo"]), 4)
            self.assertIsNone(berth.raw_lease("cargo"))


class DaemonClassTest(unittest.TestCase):
    """`mesh` is the daemon lease: no ttl, never taken over by expiry; lanes restricted; transitions explicit."""

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.clock = [1000.0]
        self.berth = Berth(self.directory.name, now=lambda: self.clock[0], obs=FakeObservation(),
                           policy_path=MISSING_POLICY)
        self.berth.classes = json.loads(json.dumps(CLASSES))
        for session in ("dev", "other"):
            self.berth.moor(session)

    def test_daemon_lease_takes_no_ttl(self):
        with self.assertRaises(BerthUsage):
            self.berth.claim("mesh", "dev", klass="mesh", ttl_s=1800)
        ok, lease, _ = self.berth.claim("mesh", "dev", klass="mesh")
        self.assertTrue(ok)
        self.assertIsNone(lease["ttl_s"])

    def test_daemon_renders_no_expiry_and_json_uses_null(self):
        self.berth.claim("mesh", "dev", klass="mesh")
        self.clock[0] += 600
        self.assertIn("mesh        held by dev for 10m (mesh, last renewed 10m ago, no expiry)",
                      render_status(self.berth))
        holder = self.berth.who("mesh")
        self.assertEqual(holder["last_renewed_at"], 1000)
        self.assertIsNone(holder["expires_at"])

    def test_daemon_lease_never_expires_while_its_holder_lives(self):
        self.berth.claim("mesh", "dev", klass="mesh")
        self.clock[0] += 100000
        self.berth.touch("dev")
        self.berth.touch("other")
        self.assertFalse(self.berth.claim("mesh", "other")[0])
        self.assertIsNone(self.berth.overrun_check())
        self.clock[0] += 20000  # dev's mooring goes stale (3 h ttl); other keeps heartbeating
        self.berth.touch("other")
        ok, _, reason = self.berth.claim("mesh", "other")
        self.assertTrue(ok)
        self.assertEqual(reason, "taken over from a stale holder")

    def test_a_lane_under_the_holders_own_daemon_lease_is_covered(self):
        self.berth.claim("mesh", "dev", klass="mesh", closure="key-daemon")
        ok, _, reason = self.berth.claim("mesh", "dev", klass="verify", ttl_s=1800, closure="key-lane")
        self.assertTrue(ok)
        self.assertIn("renewed — covered by your mesh daemon lease", reason)
        self.assertEqual(self.berth.raw_lease("mesh")["class"], "mesh")
        self.assertIsNone(self.berth.raw_lease("mesh")["ttl_s"])
        self.assertEqual(self.berth.raw_lease("mesh")["closure"], "key-daemon")
        renew = [r for r in self.berth.ledger(100) if r["kind"] == "renew"][-1]
        self.assertEqual(renew["closure"], "key-lane")

    def test_class_transition_on_a_held_lease_is_refused(self):
        self.berth.claim("mesh", "dev", klass="verify")
        ok, _, reason = self.berth.claim("mesh", "dev", klass="mesh")
        self.assertFalse(ok)
        self.assertIn("class transition", reason)
        ok, _, _ = self.berth.claim("mesh", "dev", klass="measure", ttl_s=600, override=True)
        self.assertFalse(ok)
        self.assertEqual(self.berth.raw_lease("mesh")["class"], "verify")
        self.assertTrue(self.berth.release("mesh", "dev"))
        self.assertTrue(self.berth.claim("mesh", "dev", klass="mesh")[0])

    def test_owner_check_refuses_a_stop_under_another_holder_and_force_is_on_the_record(self):
        self.berth.claim("mesh", "other", klass="mesh")
        ok, _, reason = self.berth.owner_check("mesh", "dev", "stop")
        self.assertFalse(ok)
        self.assertIn("held by other", reason)
        self.assertTrue(self.berth.owner_check("mesh", "other", "stop")[0])
        ok, _, _ = self.berth.owner_check("mesh", "dev", "stop", force=True, note="operator")
        self.assertTrue(ok)
        row = [r for r in self.berth.ledger(100) if r["kind"] == "override"][-1]
        self.assertEqual((row["action"], row["holder"]), ("stop", "other"))
        self.assertTrue(self.berth.release("mesh", "dev", force=True))
        self.assertIsNone(self.berth.raw_lease("mesh"))
        self.assertEqual([r for r in self.berth.ledger(100) if r["kind"] == "release"][-1]["forced_from"], "other")


class IncarnationTest(unittest.TestCase):
    """A pid alone is not an identity: the mooring carries the process's start time."""

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.starts = {4242: 111}
        self.berth = Berth(self.directory.name, obs=FakeObservation(), now=lambda: 1000.0, policy_path=MISSING_POLICY)
        self.berth.pid_alive = lambda pid: int(pid) in self.starts
        self.berth.pid_start = lambda pid: self.starts.get(int(pid))

    def test_mooring_records_pid_start_and_a_reused_pid_is_dead(self):
        m = self.berth.moor("sess", pid=4242)
        self.assertEqual((m["pid"], m["pid_start"]), (4242, 111))
        self.assertTrue(self.berth.is_live(self.berth.mooring("sess")))
        self.starts[4242] = 222  # the Claude process died; the pid now names another process
        self.assertFalse(self.berth.is_live(self.berth.mooring("sess")))
        self.berth.touch("sess", pid=4242)  # the hook re-asserts: new incarnation recorded
        self.assertTrue(self.berth.is_live(self.berth.mooring("sess")))


class _FileObservation:
    """A cross-process fake emitter: every emit is one line in a file, slowly (widens the race)."""

    def __init__(self, path):
        self.path = path

    def available(self):
        return True

    def emit(self, measure, subject, value, *, reason, env=None, root=None):
        if measure == "dev-berth-events@1":  # the per-record fold is not the once-per-lease pain emit
            return True
        time.sleep(0.2)
        with open(self.path, "a") as f:
            f.write(f"{measure} {value}\n")
        return True


class ConcurrencyTest(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.dir = self.directory.name

    def _fork(self, fn):
        pid = os.fork()
        if pid == 0:
            try:
                fn()
            finally:
                os._exit(0)
        return pid

    def test_concurrent_heartbeats_never_read_as_a_dead_holder(self):
        holder = Berth(self.dir, obs=FakeObservation(), policy_path=MISSING_POLICY)
        holder.moor("holder", pid=os.getpid())
        self.assertTrue(holder.claim("cargo", "holder", ttl_s=600)[0])
        from importlib.machinery import SourceFileLoader
        from importlib.util import module_from_spec, spec_from_file_location
        path = str(REPO / ".claude/hooks/ram-guard.py")
        spec = spec_from_file_location("rg_test", path, loader=SourceFileLoader("rg_test", path))
        ram_guard = module_from_spec(spec)
        spec.loader.exec_module(ram_guard)  # the hook's own heartbeat path, not a copy of it
        os.environ["BERTH_DIR"] = self.dir
        self.addCleanup(os.environ.pop, "BERTH_DIR", None)

        def beat():
            b = Berth(self.dir, obs=FakeObservation(), policy_path=MISSING_POLICY)
            for i in range(400):
                b.touch("holder", pid=os.getppid())
                if i % 4 == 0:
                    ram_guard.berth_touch("holder")

        kids = [self._fork(beat) for _ in range(2)]
        rival = Berth(self.dir, obs=FakeObservation(), policy_path=MISSING_POLICY)
        rival.moor("rival")
        outcomes = [rival.claim("cargo", "rival")[0] for _ in range(300)]
        for kid in kids:
            os.waitpid(kid, 0)
        self.assertFalse(any(outcomes), "a heartbeat mid-write read as a dead holder and was taken over")
        self.assertEqual(rival.leases()["cargo"]["holder"], "holder")

    def test_two_overrun_checks_emit_once(self):
        emits = os.path.join(self.dir, "emits.txt")
        seed = Berth(self.dir, obs=FakeObservation(), policy_path=MISSING_POLICY)
        now = time.time()
        seed._write_leases({"mesh": {"resource": "mesh", "holder": "h", "since": now - 100,
                                     "claimed_at": now - 100, "ttl_s": 10, "class": "verify"}})
        kids = [self._fork(lambda: Berth(self.dir, obs=_FileObservation(emits),
                                         policy_path=MISSING_POLICY).overrun_check()) for _ in range(4)]
        for kid in kids:
            os.waitpid(kid, 0)
        with open(emits) as f:
            self.assertEqual(len(f.read().splitlines()), 1)


STEWARD_POLICY = {"berth": {
    "resources": {"mesh": 1, "cargo": 2, "disk-heavy": 1,
                  "dev-integration": {"capacity": 1, "scope": "steward"},
                  "jenkins-dispatch": {"capacity": 1, "scope": "steward"},
                  "alpha-fleet": {"capacity": 1, "scope": "steward"},
                  "lane:": {"capacity": 1, "scope": "steward"}},
    "channel": {"freshness_s": 90, "confirm_wait_s": 10,
                "crosses": {"dev-berth-events@1": ["claim", "release", "override"]}}}}
DEVICE_A = "did:key:z6MkDeviceA"
DEVICE_B = "did:key:z6MkDeviceB"
# 2026-10-23T15:28:09Z, unix milliseconds.
SINCE_MS = 1792769289000
NOW_MS = SINCE_MS + 600_000

STUB_EPR = r"""#!/usr/bin/env python3
# A stand-in `epr`: answers `peer leases --json` with the next view in views.jsonl (the last one
# repeats) and logs every leases call; anything else (the fold emitter) succeeds silently.
import json, os, sys
here = os.path.dirname(os.path.abspath(__file__))
if sys.argv[1:3] == ["session", "prepare-berth"]:
    # Match preparation's filesystem contract; ownership is covered by native fixtures.
    os.makedirs(os.path.join(os.path.dirname(os.environ["ELOHIM_DEVICE_KEY_FILE"]), "sessions"), exist_ok=True)
if sys.argv[1:3] != ["peer", "leases"]:
    sys.exit(0)
with open(os.path.join(here, "calls.log"), "a") as f:
    f.write(json.dumps(sys.argv[1:]) + "\n")
with open(os.path.join(here, "calls.log")) as f:
    n = sum(1 for _ in f)
with open(os.path.join(here, "views.jsonl")) as f:
    views = [line for line in f.read().splitlines() if line.strip()]
line = views[min(n, len(views)) - 1]
if line.startswith("EXIT "):
    code, _, message = line[5:].partition(" ")
    sys.stderr.write(message + "\n")
    sys.exit(int(code))
print(line)
"""


CHANNEL_STORE = "/projects/elohim"  # where this device's `epr peer serve` projects from


def view(**resources):
    """A device's view, as `epr peer leases --json` prints it for a device of its steward."""
    out = {"now": NOW_MS, "confirm_wait_s": 10, "standing": "device", "device": DEVICE_B,
           "store": CHANNEL_STORE, "seen": {DEVICE_A: 7}}
    out.update({k.replace("_", "-"): v for k, v in resources.items()})
    return out


def not_a_device(why="this workspace has no device key (/nix/xdg/config/elohim/device/ed25519.seed)"):
    return {"now": NOW_MS, "confirm_wait_s": 10, "standing": "not_a_device", "device": None, "store": None,
            "why": why}


def held_by_a(override=False):
    return {"verdict": "held_by", "device": DEVICE_A, "session": "claude-1", "since": SINCE_MS,
            "override": override}


def unconfirmed_a(session="codex-1", last_seen=NOW_MS - 180_000):
    return {"verdict": "unconfirmed", "session": session, "device": DEVICE_A, "last_seen": last_seen}


def pending(session="codex-1"):
    return {"verdict": "pending", "session": session, "waiting_on": [DEVICE_A]}


def effective(session="codex-1"):
    return {"verdict": "effective", "session": session}


class WorktreeIntentTest(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        root = Path(self.directory.name)
        self.main = root / "main checkout"
        self.worktree = root / "linked worktree"
        self.worktree.mkdir()
        git_dir = self.main / ".git" / "worktrees" / "linked"
        git_dir.mkdir(parents=True)
        (git_dir / "commondir").write_text("../..\n")
        (self.worktree / ".git").write_text(f"gitdir: {git_dir}\n")
        self.policy = root / "pool-policy.json"
        self.policy.write_text(json.dumps(STEWARD_POLICY))
        self.epr = root / "epr"
        self.epr.touch()
        self.epr.chmod(0o755)
        self.obs = FakeObservation()
        self.berth = self.make()
        # An existing device counter must not accidentally resolve the store for the note.
        (Path(self.berth.dir) / "dseq").write_text("7\n")
        patcher = mock.patch.dict(Berth._fold_root.__globals__, {"REPO_ROOT": str(self.worktree)})
        patcher.start()
        self.addCleanup(patcher.stop)
        runner = mock.patch("subprocess.run")
        self.peer_run = runner.start()
        self.addCleanup(runner.stop)

    def make(self):
        b = Berth(str(Path(self.directory.name) / "berth"), obs=self.obs,
                  policy_path=str(self.policy))
        b.epr = str(self.epr)
        return b

    def answer(self, out):
        self.peer_run.return_value = subprocess.CompletedProcess([], 0, json.dumps(out), "")

    def test_worktree_intent_reaches_peer_before_first_claim(self):
        for relative in (False, True):
            with self.subTest(relative_gitdir=relative):
                if relative:
                    (self.worktree / ".git").write_text("gitdir: ../main checkout/.git/worktrees/linked\n")
                b = self.make()
                Path(b._channel_cache_path()).unlink(missing_ok=True)
                self.peer_run.reset_mock()
                out = view()
                out["store"] = str(self.main)
                self.answer(out)
                intent = "/intent land slice A2 after the current holder releases"
                row = b.say("hook-session", intent, to="peer-session")
                self.peer_run.assert_called_once_with(
                    [str(self.epr), "peer", "leases", "--json", "--root", str(self.main),
                     "--policy", str(self.policy)],
                    capture_output=True, text=True, timeout=10)
                event = self.obs.events[-1]
                self.assertEqual(event["root"], str(self.main), "the store the peer serves")
                self.assertEqual((event["reason"], event["env"]["text"], event["env"]["to"]),
                                 ("note", intent, "peer-session"))
                self.assertEqual(event["env"]["dseq"], str(row["dseq"]))
                self.assertEqual(b.leases(), {}, "the intent crosses before any claim")

    def test_worktree_mooring_crosses_before_the_channel_is_cached(self):
        # 2026-10-25 T8 run t8-20261025-02: the holder ran in a worktree on a device admitted minutes
        # earlier. Its lease crossed but its mooring folded into the worktree, so the contender never
        # saw it.
        b = self.make()
        Path(b._channel_cache_path()).unlink(missing_ok=True)
        self.peer_run.reset_mock()
        out = view()
        out["store"] = str(self.main)
        self.answer(out)
        b._fold("moor", {"kind": "moor", "session": "t8-holder", "dseq": 8})
        self.assertEqual(self.peer_run.call_count, 1, "asked once, from a worktree with nothing cached")
        self.assertEqual(self.obs.events[-1]["root"], str(self.main), "the store the peer serves")
        b._fold("moor", {"kind": "moor", "session": "t8-contender", "dseq": 9})
        self.assertEqual(self.peer_run.call_count, 1, "a known store is never asked again")

    def test_worktree_intents_refresh_a_stale_channel_cache(self):
        Path(self.berth._channel_cache_path()).write_text(str(self.worktree) + "\n")
        out = view()
        served = Path(self.directory.name) / "served checkout"
        out["store"] = str(served)
        self.answer(out)
        self.berth.say("session", "/intent first")
        self.make().say("session", "/intent next")
        self.assertEqual(self.peer_run.call_count, 2, "each process resolves the currently served store")
        self.assertEqual(self.berth.cached_channel_root(), str(served))
        self.assertEqual([e["root"] for e in self.obs.events], [str(served), str(served)])

    def test_unknown_device_store_never_folds_an_intent_into_the_worktree(self):
        self.peer_run.return_value = subprocess.CompletedProcess([], 1, "", "channel store unavailable")
        self.berth.say("session", "/intent blocked")
        self.assertEqual(self.obs.events, [])
        self.assertIn("channel store unavailable", self.berth.fold_failures[-1][2])

    def test_non_device_intents_stay_in_the_checkout(self):
        self.answer(not_a_device())
        self.berth.say("session", "/intent local")
        self.assertEqual(self.obs.events[-1]["root"], str(self.worktree))


class StewardClaimTest(unittest.TestCase):
    """Steward-scoped resources (dev-integration, jenkins-dispatch, alpha-fleet, lane:<id>) are shared
    by every device of one person. A claim first reads `epr peer leases --json`: another device's
    lease there refuses at once and nothing is written. Otherwise it writes its lease (to the device's
    channel store, carrying what it had folded) and reads the view until it leaves pending. `held by`
    and `unconfirmed:` refuse (exit 3) and release the lease; --offline-ok overrides only
    `unconfirmed`, on the record. A workspace that is not one of its steward's devices claims locally.
    Container-scoped resources never ask."""

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        root = Path(self.directory.name)
        self.policy = root / "pool-policy.json"
        self.policy.write_text(json.dumps(STEWARD_POLICY))
        self.bin = root / "bin"
        self.bin.mkdir()
        self.epr = self.bin / "epr"
        self.epr.write_text(STUB_EPR)
        self.epr.chmod(0o755)
        self.clock = [SINCE_MS / 1000 + 590]
        self.slept = []
        self.obs = FakeObservation()
        # An enrolled device by default: its key is where `just status device` looks for it.
        self.key = root / "device" / "ed25519.seed"
        self.key.parent.mkdir()
        self.key.write_bytes(b"\0" * 32)
        patcher = mock.patch.dict(os.environ, {"ELOHIM_DEVICE_KEY_FILE": str(self.key)})
        patcher.start()
        self.addCleanup(patcher.stop)
        # Peer/actor authority is stubbed here, without a real envelope owner.
        # The native session_envelope suite covers the real lifecycle guard.
        guard = mock.patch.dict(Berth.moor.__globals__, {
            "envelope_guard": lambda session, store: contextlib.nullcontext(),
        })
        guard.start()
        self.addCleanup(guard.stop)
        self.berth = self.make()
        for session in ("codex-1", "other"):
            self.berth.moor(session)

    def make(self, policy=None):
        b = Berth(os.path.join(self.directory.name, "berth"), now=lambda: self.clock[0], obs=self.obs,
                  policy_path=str(policy or self.policy))
        b.epr = str(self.epr)
        b.sleep = self.sleep
        return b

    def sleep(self, seconds):
        self.slept.append(seconds)
        self.clock[0] += seconds

    def views(self, *views):
        (self.bin / "views.jsonl").write_text(
            "".join((v if isinstance(v, str) else json.dumps(v)) + "\n" for v in views))
        try:
            (self.bin / "calls.log").unlink()
        except FileNotFoundError:
            pass

    def calls(self):
        try:
            return [json.loads(line) for line in (self.bin / "calls.log").read_text().splitlines()]
        except FileNotFoundError:
            return []

    def rows(self, kind=None):
        return [r for r in self.berth.ledger(1000) if kind is None or r["kind"] == kind]

    def claim(self, resource="dev-integration", session="codex-1", **kw):
        err = io.StringIO()
        with contextlib.redirect_stderr(err):
            ok, lease, reason = self.berth.claim(resource, session, **kw)
        return ok, lease, reason, err.getvalue()

    def test_every_record_carries_a_device_counter_that_survives_restarts_and_a_lost_counter(self):
        b = Berth(os.path.join(self.directory.name, "counted"), now=lambda: self.clock[0], obs=self.obs,
                  policy_path=MISSING_POLICY)
        b.dseq_stores = lambda: []  # this checkout's real store is not the fixture's
        b.moor("s1")
        b.claim("cargo", "s1")
        # The clock steps back: the counter does not.
        self.clock[0] -= 3600
        b.release("cargo", "s1")
        self.assertEqual([r["dseq"] for r in b.ledger(1000)], [1, 2, 3])
        self.assertEqual([e["env"]["dseq"] for e in self.obs.events[-3:]], ["1", "2", "3"])
        # A new berth process continues it.
        b2 = Berth(b.dir, now=lambda: self.clock[0], obs=self.obs, policy_path=MISSING_POLICY)
        b2.dseq_stores = lambda: []
        self.assertEqual(b2.say("s1", "hello")["dseq"], 4)
        # A lost counter resumes above every number this device has written: its ledger, and its own
        # records in its stores (never another device's, folded via it).
        os.remove(os.path.join(b.dir, "dseq"))
        store = Path(self.directory.name) / "channel" / ".eprfs" / "status"
        store.mkdir(parents=True)
        (store / "flows.jsonl").write_text(
            json.dumps({"record": {"classifiedAs": ["run:observation", ".", "env:dseq=41"]}}) + "\n"
            + json.dumps({"record": {"classifiedAs": ["run:observation", ".", "env:dseq=99",
                                                      f"via:{DEVICE_A}"]}}) + "\n")
        b3 = Berth(b.dir, now=lambda: self.clock[0], obs=self.obs, policy_path=MISSING_POLICY)
        b3.dseq_stores = lambda: [str(store.parent.parent)]
        self.assertEqual(b3.say("s1", "again")["dseq"], 42)
        self.assertEqual(b3.say("s1", "and again")["dseq"], 43)

    def test_a_claim_whose_record_cannot_reach_the_channel_store_refuses(self):
        self.views(view(), view(dev_integration=effective()))
        landed = self.obs.emit
        self.obs.emit = lambda *a, **k: False if k.get("reason") == "claim" else landed(*a, **k)
        ok, lease, reason, _ = self.claim()
        self.assertFalse(ok)
        self.assertIsNone(lease)
        self.assertIn(f"did not reach this device's channel store ({CHANNEL_STORE})", reason)
        self.assertIsNone(self.berth.who("dev-integration"))
        self.assertEqual(len(self.calls()), 1, "refused before the settle")

    def test_a_renew_that_cannot_reach_the_channel_store_keeps_the_lease_and_says_so(self):
        self.views(view(), view(dev_integration=effective()))
        self.assertTrue(self.claim()[0])
        # Another berth process renews; it cannot read where the channel store is.
        self.berth = self.make()
        self.views("EXIT 1 epr peer: the steward lease view is unavailable")
        ok, lease, reason, _ = self.claim()
        self.assertIsNone(ok, "kept, but not confirmed")
        self.assertIn("did not reach this device's channel store", reason)
        self.assertIn("the steward lease view is unavailable", reason)
        self.assertEqual(self.berth.who("dev-integration")["holder"], "codex-1")
        self.assertEqual(self.rows("release"), [])
        # Through the CLI: the lease is kept and the exit is non-zero, naming the error.
        r = self.run_cli("claim", "dev-integration", "--session", "codex-1")
        self.assertEqual(r.returncode, 1, r.stderr)
        self.assertIn("did not reach this device's channel store", r.stderr)
        self.assertNotIn("REFUSED", r.stderr)

    def test_a_lost_counter_on_a_device_resumes_from_its_channel_store(self):
        # The berth dir is lost (no counter, no ledger); this device's own records sit in the channel
        # store `epr peer leases` names, not in the checkout this berth runs from.
        channel = Path(self.directory.name) / "channel"
        (channel / ".eprfs" / "status").mkdir(parents=True)
        (channel / ".eprfs" / "status" / "flows.jsonl").write_text(
            json.dumps({"record": {"classifiedAs": ["run:observation", ".", "env:dseq=57"]}}) + "\n"
            + json.dumps({"record": {"classifiedAs": ["run:observation", ".", "env:dseq=99",
                                                      f"via:{DEVICE_A}"]}}) + "\n")
        checkout = Path(self.directory.name) / "checkout"
        (checkout / ".eprfs" / "status").mkdir(parents=True)
        (checkout / ".eprfs" / "status" / "flows.jsonl").write_text(
            json.dumps({"record": {"classifiedAs": ["run:observation", ".", "env:dseq=3"]}}) + "\n")
        g = Berth.dseq_stores.__globals__
        saved = g["REPO_ROOT"]
        g["REPO_ROOT"] = str(checkout)
        self.addCleanup(g.__setitem__, "REPO_ROOT", saved)
        out = view()
        out["store"] = str(channel)
        self.views(out)
        b = Berth(os.path.join(self.directory.name, "lost"), now=lambda: self.clock[0], obs=self.obs,
                  policy_path=str(self.policy))
        b.epr = str(self.epr)
        self.assertEqual(b.say("codex-1", "after the berth dir was lost")["dseq"], 58)
        self.assertEqual(len(self.calls()), 1, "asked once where the channel store is")

    def test_unmoor_names_a_steward_release_that_did_not_cross(self):
        self.views(view(), view(dev_integration=effective()))
        self.assertTrue(self.claim()[0])
        landed = self.obs.emit
        self.obs.emit = lambda *a, **k: False if k.get("reason") == "release" else landed(*a, **k)
        main = _NS["main"]
        g = main.__globals__
        saved = g["Berth"]
        berth = self.berth
        g["Berth"] = lambda *a, **k: berth
        err, out = io.StringIO(), io.StringIO()
        try:
            with contextlib.redirect_stderr(err), contextlib.redirect_stdout(out):
                rc = main(["berth", "unmoor", "--session", "codex-1"])
        finally:
            g["Berth"] = saved
        self.assertEqual(rc, 1, err.getvalue())
        self.assertIn("dev-integration: released here, but the release record did not reach this device's "
                      "channel store", err.getvalue())
        self.assertIn("the other devices still see it held", err.getvalue())
        self.assertIsNone(self.berth.mooring("codex-1"))

    def test_the_exit_codes_document_exit_1(self):
        doc = _NS["__doc__"]
        self.assertRegex(doc, r"1 (a|the) (renew|release)")

    def test_a_pre_check_held_by_releases_this_sessions_unsettled_lease(self):
        with self.berth._locked():
            leases = self.berth.leases()
            leases["dev-integration"] = self.berth._new_lease("dev-integration", "codex-1", None, None, "verify", None)
            self.berth._write_leases(leases)
        self.views(view(dev_integration=held_by_a()))
        ok, lease, reason, _ = self.claim()
        self.assertFalse(ok)
        self.assertEqual(reason, f"held by {DEVICE_A}/claude-1 since 2026-10-23T15:28:09Z")
        self.assertIsNone(self.berth.who("dev-integration"))
        self.assertEqual(self.rows("release")[-1]["resource"], "dev-integration")

    def test_status_lists_other_devices_read_from_this_devices_fold(self):
        device_c = "did:key:z6MkDeviceC"
        out = view(dev_integration=held_by_a(override=True),
                   lane_t7={"verdict": "held_by", "device": DEVICE_A, "session": "claude-2",
                            "since": SINCE_MS + 60_000, "override": False},
                   alpha_fleet=effective())
        out["lane:t7"] = out.pop("lane-t7")
        out["others"] = {
            DEVICE_A: {"seq": 9, "last_seen": NOW_MS - 12_000,
                       "moorings": {"claude-1": SINCE_MS - 60_000, "claude-2": SINCE_MS}},
            device_c: {"seq": None, "last_seen": None, "moorings": {}},
        }
        self.views(out)
        status = render_status(self.berth)
        section = status.split("other devices", 1)[1]
        self.assertIn(f"device {DEVICE_A}  head seq 9 folded 12s ago", section)
        self.assertIn("moored claude-1", section)
        self.assertIn("moored claude-2", section)
        self.assertIn(f"dev-integration  held by {DEVICE_A}/claude-1 since 2026-10-23T15:28:09Z"
                      f" — an override of this device's silence (offline-ok)", section)
        self.assertIn(f"lane:t7  held by {DEVICE_A}/claude-2 since 2026-10-23T15:29:09Z", section)
        self.assertIn(f"device {device_c}  SILENT: no head folded yet", section)
        # This device's own steward lease is not another device's.
        self.assertNotIn("alpha-fleet", section)
        self.assertEqual(len(self.calls()), 1)

    def test_a_folded_mooring_past_the_mooring_ttl_is_stale_not_live(self):
        out = view()
        out["others"] = {DEVICE_A: {"seq": 9, "last_seen": NOW_MS - 1_000, "moorings": {
            "claude-1": NOW_MS - 60_000,
            "claude-old": NOW_MS - (self.berth.mooring_ttl_s + 60) * 1000}}}
        self.views(out)
        section = render_status(self.berth).split("other devices", 1)[1]
        self.assertIn("      moored claude-1 since ", section)
        self.assertNotIn("      moored claude-old", section)
        self.assertIn(f"      STALE claude-old — no moor record within the {self.berth.mooring_ttl_s}s mooring "
                      f"ttl (last ", section)
        self.assertNotIn("not live", section)

    def test_status_says_why_there_is_no_other_devices_section(self):
        self.views(not_a_device())
        self.assertIn("other devices: none — this workspace is not one of its steward's devices",
                      render_status(self.berth))
        self.views("EXIT 1 epr peer: the roster does not read")
        self.assertIn("other devices: unknown — the steward lease view could not be computed: "
                      "epr peer: the roster does not read", render_status(self.make()))
        b = Berth(os.path.join(self.directory.name, "container-only"), now=lambda: self.clock[0], obs=self.obs,
                  policy_path=MISSING_POLICY)
        b.epr = str(self.epr)
        self.views(view())
        self.assertNotIn("other devices", render_status(b))
        self.assertEqual(self.calls(), [], "a berth with no steward resources never asks")

    def test_a_mooring_folds_to_the_channel_store_once_this_device_knows_it(self):
        channel = os.path.join(self.directory.name, "served-checkout")
        out = view()
        out["store"] = channel
        self.views(out)
        self.berth.peer_leases()
        # A later berth process (another worktree's session) moors: its mooring crosses.
        later = self.make()
        later.moor("codex-2")
        later.unmoor("codex-2")
        roots = [e["root"] for e in self.obs.events if e["reason"] in ("moor", "unmoor")
                 and e["env"].get("session") == "codex-2"]
        self.assertEqual(roots, [channel, channel])
        # A workspace that is not a device: moorings stay in this checkout.
        self.views(not_a_device())
        later.peer_leases()
        self.make().moor("codex-3")
        self.assertEqual([e["root"] for e in self.obs.events if e["env"].get("session") == "codex-3"],
                         [_NS["REPO_ROOT"]])

    def test_the_repository_policy_declares_the_steward_resources(self):
        b = Berth(os.path.join(self.directory.name, "real"), obs=FakeObservation(), now=lambda: 0,
                  policy_path=str(REPO / "genesis/agentic/pool-policy.json"))
        self.assertIsNone(b.policy_error)
        for name in ("dev-integration", "jenkins-dispatch", "alpha-fleet", "lane:t6"):
            self.assertTrue(b.steward_scoped(name), name)
        for name in ("mesh", "cargo", "disk-heavy", "lane:"):
            self.assertFalse(b.steward_scoped(name), name)
        self.assertEqual(b.confirm_wait_s, 10)
        self.assertEqual(b.freshness_s, 90)
        self.assertEqual(b.capacities["cargo"], 2)
        self.assertEqual(b.capacities["dev-integration"], 1)

    def test_steward_claim_unconfirmed_names_device(self):
        # The pre-check, then the settle: pending, then unconfirmed past the wait.
        self.views(view(), view(dev_integration=pending()), view(dev_integration=unconfirmed_a()))
        ok, lease, reason, _ = self.claim()
        self.assertFalse(ok)
        self.assertIsNone(lease)
        self.assertEqual(reason, f"unconfirmed: {DEVICE_A} last seen 3m ago")
        self.assertNotIn("free", reason)
        # It waited, polling once a second (each poll is an `epr` that parses the whole flow
        # store), and asked about its own claim only.
        self.assertEqual(len(self.calls()), 3)
        self.assertEqual(_NS["POLL_S"], 1.0)
        self.assertTrue(self.slept and all(s == 1.0 for s in self.slept), self.slept)
        self.assertIn("--json", self.calls()[0])
        # The pending lease does not linger as held.
        self.assertIsNone(self.berth.who("dev-integration"))
        self.assertEqual(self.rows("release")[-1]["resource"], "dev-integration")
        # A device never folded reads "never seen".
        self.views(view(), view(dev_integration=unconfirmed_a(last_seen=None)))
        self.assertEqual(self.claim()[2], f"unconfirmed: {DEVICE_A} never seen")

    def test_unconfirmed_age_reads_seconds_minutes_then_hours(self):
        fmt = _NS["steward_reason"]
        ages = [(119_999, "119s"), (120_000, "2m"), (7_199_999, "119m"), (7_200_000, "2h"), (9_000_000, "2h")]
        for age_ms, text in ages:
            self.assertEqual(fmt(unconfirmed_a(last_seen=NOW_MS - age_ms), NOW_MS),
                             f"unconfirmed: {DEVICE_A} last seen {text} ago")

    def test_steward_claim_held_names_holder(self):
        self.views(view(dev_integration=held_by_a()))
        for offline_ok in (False, True):
            ok, lease, reason, _ = self.claim(offline_ok=offline_ok)
            self.assertFalse(ok)
            self.assertIsNone(lease)
            # An override never overrides a held lease.
            self.assertEqual(reason, f"held by {DEVICE_A}/claude-1 since 2026-10-23T15:28:09Z")
            self.assertIsNone(self.berth.who("dev-integration"))
        # Refused at once on the pre-check: nothing waited on, and no lease ever written.
        self.assertEqual(self.slept, [])
        self.assertEqual(self.rows("claim"), [])
        self.assertEqual(self.rows("override"), [])
        self.assertEqual(len(self.calls()), 2)

    def test_a_lease_already_folded_refuses_a_claim_whose_clock_runs_behind_it_writing_nothing(self):
        # A's claim (stamped by A's clock at SINCE_MS) is folded here; this device's clock runs 20
        # minutes behind it, so a claim written now would carry the earlier claimed_at. It is never
        # written: the view already shows A's lease.
        self.clock[0] = SINCE_MS / 1000 - 1200
        self.views(view(dev_integration=held_by_a()))
        ok, lease, reason, _ = self.claim()
        self.assertFalse(ok)
        self.assertEqual(reason, f"held by {DEVICE_A}/claude-1 since 2026-10-23T15:28:09Z")
        self.assertEqual([r for r in self.rows() if r["kind"] in ("claim", "release")], [])
        self.assertEqual([e for e in self.obs.events if e["reason"] == "claim"], [])

    def test_offline_ok_writes_override(self):
        self.views(view(), view(dev_integration=unconfirmed_a()))
        ok, lease, reason, _ = self.claim(offline_ok=True)
        self.assertTrue(ok, reason)
        self.assertEqual(self.berth.who("dev-integration")["holder"], "codex-1")
        self.assertTrue(self.berth.who("dev-integration").get("effective"))
        row = self.rows("override")[-1]
        self.assertEqual((row["resource"], row["session"], row["holder"]), ("dev-integration", "codex-1", "codex-1"))
        self.assertEqual(row["overrode"], DEVICE_A)
        self.assertEqual(row["claimed_at"], lease["claimed_at"])
        self.assertIn(f"unconfirmed: {DEVICE_A} last seen 3m ago", row["reason"])
        # The override row crosses like any berth record, from the channel store.
        self.assertEqual(self.obs.events[-1]["reason"], "override")
        self.assertEqual(self.obs.events[-1]["env"]["overrode"], DEVICE_A)
        self.assertEqual(self.obs.events[-1]["root"], CHANNEL_STORE)

    def test_a_steward_claim_lands_in_the_channel_store_carrying_what_it_had_folded(self):
        # This berth's own checkout is not where the device's peer projects from: the claim, and its
        # release, are written to the channel store the view names, so they cross.
        self.views(view(), view(dev_integration=effective()))
        ok, lease, reason, _ = self.claim()
        self.assertTrue(ok, reason)
        claim = [e for e in self.obs.events if e["reason"] == "claim"][-1]
        self.assertEqual(claim["root"], CHANNEL_STORE)
        self.assertEqual(json.loads(claim["env"]["seen"]), {DEVICE_A: 7})
        self.assertEqual(self.rows("claim")[-1]["seen"], {DEVICE_A: 7})
        self.assertTrue(self.berth.release("dev-integration", "codex-1"))
        self.assertEqual([e for e in self.obs.events if e["reason"] == "release"][-1]["root"], CHANNEL_STORE)
        # Container records stay in this checkout's store.
        self.assertTrue(self.berth.claim("cargo", "codex-1")[0])
        self.assertEqual([e for e in self.obs.events if e["reason"] == "claim"][-1]["root"], _NS["REPO_ROOT"])

    def test_single_device_steward_claim_effective(self):
        self.views(view(), json.dumps(dict(view(), **{"lane:t6": effective()})))
        ok, lease, reason, _ = self.claim("lane:t6", note="the lease view")
        self.assertTrue(ok, reason)
        self.assertEqual(len(self.calls()), 2, "the pre-check and one read: effective at once")
        self.assertEqual(self.slept, [], "no other admitted device: effective at once, no wait")
        self.assertEqual(self.berth.who("lane:t6")["holder"], "codex-1")

    def test_a_renew_of_an_effective_lease_does_not_settle_again_and_is_never_released(self):
        self.views(view(), view(dev_integration=effective()))
        self.assertTrue(self.claim()[0])
        # The other device then goes silent; the view can no longer even be read. Renewing keeps
        # the lease: effective once, effective for its life.
        self.views("EXIT 1 epr peer: the steward lease view is unavailable")
        ok, lease, reason, _ = self.claim()
        self.assertTrue(ok, reason)
        self.assertEqual(reason, "renewed")
        self.assertEqual(self.berth.who("dev-integration")["holder"], "codex-1")
        self.assertEqual(self.rows("release"), [])

    def test_a_renew_that_reads_unconfirmed_is_refused_but_keeps_its_lease(self):
        # A lease written but never settled (a claim interrupted mid-wait) is renewed.
        with self.berth._locked():
            leases = self.berth.leases()
            leases["dev-integration"] = self.berth._new_lease("dev-integration", "codex-1", None, None, "verify", None)
            self.berth._write_leases(leases)
        self.views(view(), view(dev_integration=pending()), view(dev_integration=unconfirmed_a()))
        ok, lease, reason, _ = self.claim()
        self.assertFalse(ok)
        self.assertEqual(reason, f"unconfirmed: {DEVICE_A} last seen 3m ago")
        self.assertEqual(self.berth.who("dev-integration")["holder"], "codex-1", "never released on renew")
        self.assertEqual([r for r in self.rows("release") if r["resource"] == "dev-integration"], [])

    def test_not_a_device_claims_locally_with_one_line(self):
        self.views(not_a_device())
        ok, lease, reason, err = self.claim()
        self.assertTrue(ok, reason)
        self.assertEqual(self.berth.who("dev-integration")["holder"], "codex-1")
        self.assertEqual(err.splitlines(), [
            "berth: dev-integration is steward-scoped but this workspace is not one of its steward's devices "
            "(run: just status device); claiming locally only"])
        self.assertEqual(len(self.calls()), 1, "no settle: container-scoped")
        self.assertEqual([e for e in self.obs.events if e["reason"] == "claim"][-1]["root"], _NS["REPO_ROOT"])

    def test_a_revoked_device_is_refused_naming_the_contest(self):
        self.views(dict(not_a_device(), standing="revoked", device=DEVICE_B, why="this device's binding is contested"))
        ok, lease, reason, _ = self.claim()
        self.assertFalse(ok)
        self.assertEqual(reason, "revoked: this device's binding is contested")
        self.assertEqual(self.rows("claim"), [])

    def test_container_scope_never_calls_peer(self):
        self.views("EXIT 1 a container claim must never ask")
        self.assertTrue(self.berth.claim("cargo", "codex-1")[0])
        self.assertTrue(self.berth.owner_check("cargo", "other", "stop")[0] is False)
        self.berth.epr = "/nonexistent/epr"
        self.assertTrue(self.berth.claim("mesh", "codex-1")[0])
        self.assertEqual(self.calls(), [])

    def test_missing_epr_refuses_a_steward_claim_naming_the_cure(self):
        # An enrolled device (its key is present) needs `epr` to read the view.
        self.berth.epr = "/nonexistent/epr"
        ok, lease, reason, _ = self.claim()
        self.assertFalse(ok)
        self.assertIsNone(lease)
        self.assertIn("EPR_BIN", reason)
        self.assertIn("`epr device arrive`", reason)
        self.assertIsNone(self.berth.who("dev-integration"))
        self.assertEqual(self.rows("claim"), [], "no lease is written that could not be confirmed")
        self.assertFalse(self.berth.owner_check("dev-integration", "codex-1", "deploy")[0])

    def test_without_epr_a_workspace_with_no_device_key_is_not_a_device(self):
        # No key where `just status device` looks: not a device, so no `epr` is needed to know it.
        self.key.unlink()
        self.berth.epr = "/nonexistent/epr"
        self.views("EXIT 1 a workspace with no device key is never asked")
        ok, lease, reason, err = self.claim()
        self.assertTrue(ok, reason)
        self.assertEqual(self.berth.who("dev-integration")["holder"], "codex-1")
        self.assertEqual(err.splitlines(), [
            "berth: dev-integration is steward-scoped but this workspace is not one of its steward's devices "
            "(run: just status device); claiming locally only"])
        self.assertEqual(self.calls(), [])
        err = io.StringIO()
        with contextlib.redirect_stderr(err):
            self.assertTrue(self.berth.owner_check("alpha-fleet", "other", "deploy")[0])
        self.assertEqual(len(err.getvalue().splitlines()), 1)
        section = "\n".join(_NS["_other_devices_lines"](self.berth))
        self.assertIn("other devices: none — this workspace is not one of its steward's devices", section)
        self.assertIn(str(self.key), section)

    def test_berth_resolves_epr_through_the_native_shim(self):
        sys.path.insert(0, str(REPO / ".claude/scripts"))
        from _lib import epr_client
        b = self.make()
        b.epr = None
        with mock.patch.object(epr_client, "resolve_binary", return_value=str(self.epr)) as resolve:
            self.assertEqual(b.epr_bin(), str(self.epr))
            resolve.assert_called_once()
        with mock.patch.dict(os.environ, {"ELOHIM_DEVICE_KEY_FILE": str(self.key)}):
            self.assertEqual(Path(b.device_key_path()), self.key)

    def test_unconfirmed_with_this_devices_daemon_down_names_epr_peer_serve(self):
        self.views(view(), view(dev_integration=unconfirmed_a(), serving=False))
        ok, _, reason, _ = self.claim()
        self.assertFalse(ok)
        self.assertEqual(reason.splitlines(), [
            f"unconfirmed: {DEVICE_A} last seen 3m ago",
            "(this device's peer daemon is not running: start it with epr peer serve)"])
        # With the daemon running, the other device's silence is the only reason.
        self.views(view(), view(dev_integration=unconfirmed_a(), serving=True))
        self.assertEqual(self.claim()[2], f"unconfirmed: {DEVICE_A} last seen 3m ago")
        # A view that cannot say (an older epr) adds nothing.
        self.views(view(), view(dev_integration=unconfirmed_a()))
        self.assertEqual(self.claim()[2], f"unconfirmed: {DEVICE_A} last seen 3m ago")
        # --offline-ok still overrides: the line is for a refusal only.
        self.views(view(), view(dev_integration=unconfirmed_a(), serving=False))
        ok, _, reason, _ = self.claim(offline_ok=True)
        self.assertTrue(ok, reason)

    def test_a_failed_epr_call_reports_its_first_line_and_names_a_binary_older_than_the_peer_verbs(self):
        stale = Path(self.directory.name) / "stale-epr"
        stale.write_text("#!/bin/sh\n"
                         "echo 'epr: invalid arguments: unknown command `peer`' >&2\n"
                         "echo 'usage: epr [--repo PATH] [--json] <setup|doctor|...>' >&2\n"
                         "exit 2\n")
        stale.chmod(0o755)
        self.berth.epr = str(stale)
        ok, _, reason, _ = self.claim()
        self.assertFalse(ok)
        self.assertEqual(reason, f"the steward lease view could not be computed: epr: invalid arguments: unknown "
                                 f"command `peer` \u2014 {stale} predates the peer verbs: replace it (epr device arrive "
                                 f"installs the native launcher onto PATH) or export EPR_BIN=<a current epr>")
        failing = Path(self.directory.name) / "failing-epr"
        failing.write_text("#!/bin/sh\necho 'epr peer: the roster does not read' >&2\n"
                           "echo '  caused by: line 3' >&2\nexit 2\n")
        failing.chmod(0o755)
        self.berth.epr = str(failing)
        self.assertEqual(self.claim()[2],
                         "the steward lease view could not be computed: epr peer: the roster does not read")

    STALE_STUB = ("#!/bin/sh\necho 'epr: invalid arguments: unknown command `peer`' >&2\n"
                  "echo 'usage: epr <setup|doctor|...>' >&2\nexit 2\n")

    def _stale_path_epr(self):
        stale_dir = Path(self.directory.name) / "path-bin"
        stale_dir.mkdir()
        stale = stale_dir / "epr"
        stale.write_text(self.STALE_STUB)
        stale.chmod(0o755)
        return stale

    def _pool_epr(self, script):
        pool = Path(self.directory.name) / "pool"
        slot = pool / "family" / "slot" / "elohim__eprfs" / "t" / "release"
        slot.mkdir(parents=True)
        (slot / "epr").write_text(script)
        (slot / "epr").chmod(0o755)
        return pool, slot

    def _env(self, stale, pool, **extra):
        env = {"PATH": f"{stale.parent}{os.pathsep}{os.environ.get('PATH', '')}", "CARGO_TARGET_POOL_ROOT": str(pool), **extra}
        patcher = mock.patch.dict(os.environ, env)
        patcher.start()
        self.addCleanup(patcher.stop)
        if "EPR_BIN" not in extra:
            os.environ.pop("EPR_BIN", None)

    def test_a_stale_candidate_never_falls_through_to_a_pool_build(self):
        stale = self._stale_path_epr()
        pool, slot = self._pool_epr(STUB_EPR)
        self._env(stale, pool)
        self.berth.epr = str(stale)
        ok, _, reason, _ = self.claim()
        self.assertFalse(ok, reason)
        self.assertFalse((slot / "calls.log").exists())

    def test_a_stale_epr_on_path_and_no_working_pool_build_names_the_path_and_the_cure(self):
        stale = self._stale_path_epr()
        pool, slot = self._pool_epr(self.STALE_STUB)
        self._env(stale, pool)
        self.berth.epr = str(stale)
        ok, lease, reason, _ = self.claim()
        self.assertFalse(ok)
        self.assertEqual(reason, f"the steward lease view could not be computed: epr: invalid arguments: unknown "
                                 f"command `peer` \u2014 {stale} predates the peer verbs: replace it (epr device arrive "
                                 f"installs the native launcher onto PATH) or export EPR_BIN=<a current epr>")
        self.assertIsNone(self.berth.who("dev-integration"))

    def test_an_explicit_epr_bin_that_is_stale_is_not_fallen_through(self):
        stale = self._stale_path_epr()
        pool, slot = self._pool_epr(STUB_EPR)
        (slot / "views.jsonl").write_text(json.dumps(view()) + "\n")
        self._env(stale, pool, EPR_BIN=str(stale))
        self.berth.epr = str(stale)
        ok, lease, reason, _ = self.claim()
        self.assertFalse(ok)
        self.assertIn(str(stale), reason)
        self.assertIn("epr device arrive", reason)
        self.assertFalse((slot / "calls.log").exists(), "the pool build was never asked")

    def test_an_enrolled_device_whose_view_fails_is_refused(self):
        self.views("EXIT 2 epr peer: the channel store `/x/.eprfs/status/flows.jsonl` that epr peer serve recorded is gone")
        ok, lease, reason, _ = self.claim()
        self.assertFalse(ok)
        self.assertIn("channel store", reason)
        self.assertIsNone(self.berth.who("dev-integration"))
        self.assertEqual(self.rows("claim"), [])
        # A claim whose own lease is missing from the view (its record never reached the channel
        # store) is not effective either.
        self.views(view(), view())
        ok, _, reason, _ = self.claim()
        self.assertFalse(ok)
        self.assertIn("channel store", reason)

    def test_owner_check_reads_the_same_view(self):
        self.views(view(alpha_fleet=held_by_a()))
        ok, _, reason = self.berth.owner_check("alpha-fleet", "codex-1", "deploy")
        self.assertFalse(ok)
        self.assertEqual(reason, f"held by {DEVICE_A}/claude-1 since 2026-10-23T15:28:09Z")
        # --force overrides a local holder, never another device's lease.
        self.assertFalse(self.berth.owner_check("alpha-fleet", "codex-1", "deploy", force=True)[0])
        self.views(view())
        self.assertTrue(self.berth.owner_check("alpha-fleet", "codex-1", "deploy")[0])
        self.views(not_a_device())
        err = io.StringIO()
        with contextlib.redirect_stderr(err):
            self.assertTrue(self.berth.owner_check("alpha-fleet", "codex-1", "deploy")[0])
        self.assertEqual(len(err.getvalue().splitlines()), 1)

    def test_the_settle_never_outlasts_its_budget(self):
        timeouts = []

        def leases(timeout=10):
            self.berth._channel_read = True
            self.berth.channel_root, self.berth.channel_standing = CHANNEL_STORE, "device"
            timeouts.append(timeout)
            self.clock[0] += min(3, timeout)  # each read is slow, and gives up at its timeout
            return (view(dev_integration=pending()) if len(timeouts) > 1 else view()), None

        self.berth.peer_leases = leases
        start = self.clock[0]
        ok, _, reason, _ = self.claim()
        self.assertFalse(ok)
        # The pre-check's read, then a settle bounded by its budget.
        self.assertLessEqual(self.clock[0] - start, 3 + self.berth.confirm_wait_s + 1)
        # Every read during the settle was given at most the time left.
        self.assertTrue(all(t <= self.berth.confirm_wait_s + 1 for t in timeouts[1:]), timeouts)
        self.assertLess(timeouts[-1], timeouts[1])

    def test_a_policy_that_does_not_parse_or_names_an_unknown_scope_refuses(self):
        for text in ('{"berth": {"resources": {"dev-integration": ', 
                     json.dumps({"berth": {"resources": {"dev-integration": {"capacity": 1, "scope": "stewart"}}}})):
            bad = Path(self.directory.name) / "bad-policy.json"
            bad.write_text(text)
            b = self.make(bad)
            self.assertIsNotNone(b.policy_error)
            ok, lease, reason = b.claim("dev-integration", "codex-1")
            self.assertFalse(ok)
            self.assertIn(str(bad), reason)
            self.assertFalse(b.owner_check("dev-integration", "codex-1", "deploy")[0])

    def run_cli(self, *args):
        project = Path(self.directory.name) / "project"
        (project / "genesis" / "agentic").mkdir(parents=True, exist_ok=True)
        (project / "genesis" / "agentic" / "pool-policy.json").write_text(json.dumps(STEWARD_POLICY))
        env = {k: v for k, v in os.environ.items() if k not in ("ELOHIM_SESSION_ID", "CLAUDE_SESSION_ID")}
        env.update({"BERTH_DIR": self.berth.dir, "EPR_BIN": str(self.epr), "CLAUDE_PROJECT_DIR": str(project)})
        return subprocess.run([sys.executable, str(REPO / "genesis/agentic/bin/berth"), *args], env=env,
                              capture_output=True, text=True, timeout=60)

    def test_cli_names_this_devices_stopped_daemon_on_a_second_line(self):
        # berth's own main, over this test's berth (its stub epr, emitter and clock). `_NS` is a
        # copy: main reads the globals it was defined in. The store is a scratch one regardless.
        self.views(view(), view(dev_integration=unconfirmed_a(last_seen=None), serving=False))
        err = io.StringIO()
        main = _NS["main"]
        with mock.patch.dict(main.__globals__, {"Berth": lambda: self.berth}), \
                mock.patch.dict(os.environ, {"BERTH_DIR": self.berth.dir}), \
                contextlib.redirect_stderr(err), contextlib.redirect_stdout(io.StringIO()):
            code = main(["berth", "claim", "dev-integration", "--session", "codex-1"])
        self.assertEqual(code, 3, err.getvalue())
        self.assertEqual(err.getvalue().splitlines(), [
            f"dev-integration: REFUSED — unconfirmed: {DEVICE_A} never seen",
            "(this device's peer daemon is not running: start it with epr peer serve)"])

    def test_cli_refusal_is_exit_3_with_one_reason_line(self):
        project = Path(self.directory.name) / "project"
        (project / "genesis" / "agentic").mkdir(parents=True)
        (project / "genesis" / "agentic" / "pool-policy.json").write_text(json.dumps(STEWARD_POLICY))
        self.views(view(dev_integration=held_by_a()))
        env = {k: v for k, v in os.environ.items() if k not in ("ELOHIM_SESSION_ID", "CLAUDE_SESSION_ID")}
        env.update({"BERTH_DIR": os.path.join(self.directory.name, "cli-berth"), "EPR_BIN": str(self.epr),
                    "CLAUDE_PROJECT_DIR": str(project)})
        for extra in ([], ["--offline-ok"]):
            r = subprocess.run([sys.executable, str(REPO / "genesis/agentic/bin/berth"), "claim", "dev-integration",
                                "--session", "codex-1", *extra], env=env, capture_output=True, text=True, timeout=60)
            self.assertEqual(r.returncode, 3, r.stderr)
            self.assertEqual(r.stderr.splitlines(),
                             [f"dev-integration: REFUSED — held by {DEVICE_A}/claude-1 since 2026-10-23T15:28:09Z"])


class StoreUnavailableTest(unittest.TestCase):
    """The berth store is the host session's organ, never a guest's: an invocation that cannot open
    it exits 4 (the wrapper's proceed-unleased code) with one named line, never a traceback."""

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.store = os.path.join(self.directory.name, "berth")
        os.makedirs(os.path.join(self.store, "moorings"))
        lock = os.path.join(self.store, "berth.lock")
        open(lock, "w").close()
        os.chmod(lock, 0o644)
        os.chmod(self.directory.name, 0o755)
        os.chmod(self.store, 0o755)

    def run_berth(self, *args, preexec=None, **env):
        e = {k: v for k, v in os.environ.items() if k not in ("ELOHIM_SESSION_ID", "CLAUDE_SESSION_ID")}
        e.update(env)
        return subprocess.run([sys.executable, str(REPO / "genesis/agentic/bin/berth"), *args], env=e,
                              capture_output=True, text=True, timeout=30, preexec_fn=preexec, cwd="/")

    @unittest.skipUnless(os.geteuid() == 0, "needs root to drop to the stage guest uid")
    def test_guest_uid_meeting_the_hosts_root_owned_lock_exits_4_and_names_the_path(self):
        def as_guest():
            os.setgroups([])
            os.setgid(65534)
            os.setuid(65534)
        r = self.run_berth("claim", "mesh", "--ttl", "60", "--note", "stage lane", preexec=as_guest,
                           BERTH_DIR=self.store, ELOHIM_SESSION_ID="stage:probe")
        self.assertEqual(r.returncode, 4, r.stderr)
        self.assertNotIn("Traceback", r.stderr)
        self.assertIn("berth: store unavailable (EACCES", r.stderr)
        self.assertIn(os.path.join(self.store, "berth.lock"), r.stderr)
        self.assertIn("lane proceeds unleased", r.stderr)

    def test_help_prints_usage_and_never_takes_a_lease(self):
        for args in (("claim", "--help"), ("release", "-h"), ("--help",), ("who", "--help")):
            r = self.run_berth(*args, BERTH_DIR=self.store, ELOHIM_SESSION_ID="dev")
            self.assertEqual(r.returncode, 0, (args, r.stderr))
            self.assertIn("berth claim", r.stdout, args)
        r = self.run_berth("who", "--", BERTH_DIR=self.store, ELOHIM_SESSION_ID="dev")
        self.assertEqual(r.stdout.strip(), "null")
        leases = os.path.join(self.store, "leases")
        self.assertFalse(os.path.exists(leases) and any("help" in n for n in os.listdir(leases)))

    def test_a_store_that_cannot_be_created_exits_4(self):
        missing = "/proc/berth-no-such-store"  # mkdir under /proc fails for every uid, root included
        r = self.run_berth("claim", "mesh", "--ttl", "60", BERTH_DIR=missing, ELOHIM_SESSION_ID="stage:probe")
        self.assertEqual(r.returncode, 4, r.stderr)
        self.assertNotIn("Traceback", r.stderr)
        self.assertIn(f"berth: store unavailable (", r.stderr)
        self.assertIn(missing, r.stderr)

    def test_an_unrelated_os_error_is_not_masked(self):
        main = _NS["main"]
        g = main.__globals__
        saved = g["_main"]

        def boom(argv):
            raise PermissionError(13, "Permission denied", "/etc/somewhere-else")
        g["_main"] = boom
        try:
            with self.assertRaises(PermissionError):
                main(["berth", "claim", "mesh"])
        finally:
            g["_main"] = saved


@unittest.skipUnless(shutil.which("just") and shutil.which("timeout"), "needs just + timeout")
class JustWrapperTest(unittest.TestCase):
    """The real justfile arms against a stub app_dir: refusals never reach the mesh, exit 4 proceeds,
    and a verify lane is killed at its ttl so a competing claimant can take the berth safely."""

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        root = Path(self.directory.name)
        self.berth_dir = root / "berth"
        self.app = root / "app"
        (self.app / "scripts").mkdir(parents=True)
        self.reached = root / "reached"
        self.lane_pid = root / "lane.pid"
        stub = f"""#!/usr/bin/env bash
# stub hc-mesh.sh: sourced by the test lane, executed by the mesh arms — records that it was reached
echo "$*" >> {self.reached}
if [[ "${{BASH_SOURCE[0]}}" != "$0" ]]; then
  if [[ "${{STUB_MODE:-}}" == "overlong" ]]; then
    mesh_seed_env() {{ echo "$BASHPID" > {self.lane_pid}; sleep 60; }}
  else
    exit 42
  fi
else
  exit 0
fi
"""
        for name in ("hc-mesh.sh", "hc-mesh-transport-matrix.sh"):
            (self.app / "scripts" / name).write_text(stub)
            (self.app / "scripts" / name).chmod(0o755)
        self.env = {k: v for k, v in os.environ.items() if k not in (
            "ELOHIM_SESSION_ID", "CLAUDE_SESSION_ID", "BERTH_CLASS", "BERTH_TTL", "MEASURE_ON_DEV_BERTH",
            "MESH_STOP_FORCE", "BERTH_FENCED")}
        self.env.update({"BERTH_DIR": str(self.berth_dir), "EPR_BIN": "/nonexistent-epr",
                         "ELOHIM_DEVICE_KEY_FILE": str(root / "no-device" / "ed25519.seed"),
                         "PATH": f"{REPO / 'genesis/agentic/bin'}:/usr/local/bin:/usr/bin:/bin",
                         "EPR_UNDER_TEST": "1", "EPR_PROC_ROOT": str(root / "no-proc"), "MESH_DIR": str(root / "no-mesh")})

    def just(self, *args, **env):
        e = dict(self.env, **env)
        return subprocess.run(["just", "--justfile", str(REPO / "justfile"), f"app_dir={self.app}", *args],
                              env=e, capture_output=True, text=True, timeout=120)

    def berth(self, *args, **env):
        return subprocess.run([str(REPO / "genesis/agentic/bin/berth"), *args], env=dict(self.env, **env),
                              capture_output=True, text=True, timeout=30)

    def test_refusals_never_reach_the_mesh(self):
        cases = [({"BERTH_CLASS": "measure"}, 3), ({"BERTH_CLASS": "bogus"}, 2), ({"BERTH_CLASS": "mesh"}, 2),
                 ({"BERTH_TTL": "0"}, 2), ({"BERTH_TTL": "7200"}, 3), ({"BERTH_TTL": "soon"}, 2)]
        for env, want in cases:
            r = self.just("test", "mesh", "x.feature", ELOHIM_SESSION_ID="me", **env)
            self.assertEqual(r.returncode, want, (env, r.stderr[-400:]))
        self.assertFalse(self.reached.exists())

    def test_no_session_is_exit_4_and_the_lane_proceeds_unleased(self):
        self.assertEqual(self.berth("claim", "mesh", "--ttl", "60").returncode, 4)
        r = self.just("test", "mesh", "x.feature")
        self.assertEqual(r.returncode, 42, r.stderr[-400:])  # the stub was sourced: the lane ran
        self.assertTrue(self.reached.exists())

    def test_measure_recipes_are_refused_on_the_dev_berth(self):
        for action in ("matrix", "recovery", "recovery-matrix"):
            r = self.just("mesh", action, ELOHIM_SESSION_ID="me")
            self.assertEqual(r.returncode, 3, (action, r.stderr[-300:]))
            self.assertIn("just measure <scope>", r.stderr)
        self.assertFalse(self.reached.exists())

    def test_stop_is_refused_under_another_live_holder_unless_forced(self):
        self.assertEqual(self.berth("claim", "mesh", "--class", "mesh", "--session", "rival").returncode, 0)
        r = self.just("mesh", "stop", ELOHIM_SESSION_ID="me")
        self.assertEqual(r.returncode, 3, r.stderr[-300:])
        self.assertIn("held by rival", r.stderr)
        self.assertFalse(self.reached.exists())
        r = self.just("mesh", "stop", ELOHIM_SESSION_ID="me", MESH_STOP_FORCE="1")
        self.assertEqual(r.returncode, 0, r.stderr[-300:])
        self.assertTrue(self.reached.exists())
        self.assertIsNone(json.loads(self.berth("who", "mesh").stdout))

    def test_an_overlong_verify_lane_is_killed_at_its_ttl_then_the_berth_is_free(self):
        proc = subprocess.Popen(["just", "--justfile", str(REPO / "justfile"), f"app_dir={self.app}",
                                 "test", "mesh", "x.feature"],
                                env=dict(self.env, ELOHIM_SESSION_ID="me", BERTH_TTL="3", STUB_MODE="overlong"),
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        deadline = time.time() + 10
        while not self.lane_pid.exists() and time.time() < deadline:
            time.sleep(0.1)
        self.assertTrue(self.lane_pid.exists(), "the lane never started")
        rival = self.berth("claim", "mesh", "--session", "rival", "--ttl", "60")
        self.assertEqual(rival.returncode, 3, "a live verify lane must refuse a competing claimant")
        out, err = proc.communicate(timeout=60)
        self.assertEqual(proc.returncode, 3, err[-400:])
        self.assertIn("BUDGET-EXCEEDED: verify lane ran past its 3s lease", err)
        lane = int(self.lane_pid.read_text())
        self.assertFalse(os.path.exists(f"/proc/{lane}"), "the fenced lane outlived its lease")
        rival = self.berth("claim", "mesh", "--session", "rival", "--ttl", "60")
        self.assertEqual(rival.returncode, 0, rival.stderr)


class EnvelopeGuardTest(unittest.TestCase):
    """The real lifecycle guard, which the other berth suites stub out."""

    def test_a_device_with_no_envelope_store_and_an_epr_older_than_envelopes_is_not_fenced(self):
        # Pre-push on 2026-10-25: the device key existed, no envelope had ever been opened, and the
        # `epr` on hand predated envelopes — it printed usage for `session prepare-berth` and exited
        # 0. The guard then opened <device>/sessions/.lock in a directory nobody had created.
        with tempfile.TemporaryDirectory() as base:
            key = Path(base) / "device" / "ed25519.seed"
            key.parent.mkdir()
            key.write_bytes(b"\0" * 32)
            old_epr = Path(base) / "epr"
            old_epr.write_text("#!/bin/sh\necho 'epr session id [--json]'\nexit 0\n")
            old_epr.chmod(0o755)
            guard = Berth.moor.__globals__["envelope_guard"]
            with mock.patch.dict(os.environ, {"ELOHIM_DEVICE_KEY_FILE": str(key), "EPR_BIN": str(old_epr)}):
                with guard("claude-code-1-2", str(Path(base) / "berth")):
                    entered = True
            self.assertTrue(entered)
            self.assertFalse((key.parent / "sessions").exists(), "the guard must not invent an envelope store")

    def test_a_session_pinned_to_an_epr_with_no_session_verb_is_not_fenced(self):
        # Every live session on a device keeps its pin across the landing that brings envelopes.
        with tempfile.TemporaryDirectory() as base:
            key = Path(base) / "device" / "ed25519.seed"
            key.parent.mkdir()
            key.write_bytes(b"\0" * 32)
            (key.parent / "sessions").mkdir()  # another, newer session opened an envelope here
            old_epr = Path(base) / "epr"
            old_epr.write_text("#!/bin/sh\necho 'epr: invalid arguments: unknown command `session`' >&2\nexit 2\n")
            old_epr.chmod(0o755)
            guard = Berth.moor.__globals__["envelope_guard"]
            with mock.patch.dict(os.environ, {"ELOHIM_DEVICE_KEY_FILE": str(key), "EPR_BIN": str(old_epr)}):
                with guard("claude-code-1-2", str(Path(base) / "berth")):
                    entered = True
            self.assertTrue(entered)

    def test_a_device_with_no_epr_at_all_is_not_fenced(self):
        with tempfile.TemporaryDirectory() as base:
            key = Path(base) / "device" / "ed25519.seed"
            key.parent.mkdir()
            key.write_bytes(b"\0" * 32)
            guard = Berth.moor.__globals__["envelope_guard"]
            with mock.patch.dict(os.environ, {"ELOHIM_DEVICE_KEY_FILE": str(key),
                                              "EPR_BIN": str(Path(base) / "absent-epr")}):
                with guard("claude-code-1-2", str(Path(base) / "berth")):
                    entered = True
            self.assertTrue(entered)

    def test_a_current_epr_that_refuses_preparation_still_refuses(self):
        with tempfile.TemporaryDirectory() as base:
            key = Path(base) / "device" / "ed25519.seed"
            key.parent.mkdir()
            key.write_bytes(b"\0" * 32)
            epr = Path(base) / "epr"
            epr.write_text("#!/bin/sh\necho 'envelope-owner-mismatch: claude-code-1-2' >&2\nexit 1\n")
            epr.chmod(0o755)
            guard = Berth.moor.__globals__["envelope_guard"]
            with mock.patch.dict(os.environ, {"ELOHIM_DEVICE_KEY_FILE": str(key), "EPR_BIN": str(epr)}):
                with self.assertRaises(Exception) as refused:
                    with guard("claude-code-1-2", str(Path(base) / "berth")):
                        pass
            self.assertIn("envelope-owner-mismatch", str(refused.exception))


if __name__ == "__main__":
    unittest.main()
