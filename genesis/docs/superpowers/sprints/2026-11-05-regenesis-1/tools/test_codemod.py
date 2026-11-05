#!/usr/bin/env python3
"""Tests for address-codemod.py and address-profile-lint.sh (temp fixture tree)."""
import importlib.util, json, os, subprocess, sys, tempfile, unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("codemod", HERE / "address-codemod.py")
cm = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cm)

H64 = "ab" * 32
BAF = "bafkrei" + "a" * 52


def w(root, rel, body):
    p = root / rel
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(body)


def build_tree(root):
    # address: ident + regex gate + fixture literals
    w(root, "elohim/elohim-storage/src/blob.rs",
      'let blob_hash = x;\nlet v = Regex::new("^bafkrei[a-z2-7]{52}$");\n')
    w(root, "elohim/elohim-storage/tests/fixture_a.rs", f'const A: &str = "sha256-{H64}";\nconst B: &str = "{BAF}";\n')
    # digest: dedup key, bare hex in blob context? -> digest via dedup keyword
    w(root, "doorway/doorway-service/src/dedup.rs", f'// dedup checksum for /blob/ {H64}\n')
    # cite: fingerprint in frontmatter
    w(root, "genesis/docs/spec.md", "---\ncites:\n  - fp: sha256:0123456789abcdef\n---\nbody\n")
    # oci
    w(root, "scripts/ci/build-storage-image.sh", f'IMG=harbor/x@sha256:{H64}\n')
    # record
    w(root, "elohim/eprfs/src/rec.rs", 'const R: &str = "bafyreiabcdefghijklmnopqrstuvwxyz";\n')
    # prose
    w(root, "genesis/notes.md", f"The old form was sha256-{H64} in prose.\n")
    # a mechanical-skipped dna file
    w(root, "elohim/holochain/dna/x/lib.rs", "pub blob_hash: String,\n")
    # collision file
    w(root, "elohim/elohim-storage/src/coll.rs", "struct S { blob_hash: String, blob_cid: String }\n")


class Codemod(unittest.TestCase):
    def setUp(self):
        self.td = tempfile.TemporaryDirectory()
        self.root = Path(self.td.name) / "repo"
        self.root.mkdir()
        build_tree(self.root)
        self.allow = cm.Allow(HERE / "allowlist.yaml")

    def tearDown(self):
        self.td.cleanup()

    def census(self):
        hits = cm.census_scan(self.root, self.allow, walk=True)
        by = {}
        for h in hits:
            by.setdefault(h.cls, []).append(h)
        return by

    def test_census_exact_counts(self):
        by = self.census()
        cnt = {c: len(by.get(c, [])) for c in cm.CLASSES}
        # address: blob_hash (blob.rs) + bafkrei regex + sha256- + bafkrei literal (fixture)
        #          + blob_hash (dna) + blob_hash,blob_cid? (coll.rs: only blob_hash)  => 6
        self.assertEqual(cnt["address"], 6, cnt)
        self.assertEqual(cnt["digest"], 1, cnt)
        self.assertEqual(cnt["cite"], 1, cnt)
        self.assertEqual(cnt["oci"], 1, cnt)
        self.assertEqual(cnt["record"], 1, cnt)
        self.assertEqual(cnt["prose"], 1, cnt)
        self.assertEqual(by["oci"][0].track, "seeder+CI")
        self.assertEqual(by["record"][0].track, "eprfs")

    def test_dry_run_only_mechanical(self):
        plan, notes = cm.plan_all(self.root, self.allow, True)
        self.assertEqual(set(plan), {"elohim/elohim-storage/src/blob.rs",
                                     "elohim/elohim-storage/tests/fixture_a.rs"})
        rules = sorted(e["rule"] for es in plan.values() for e in es)
        self.assertEqual(rules, ["fixture-bafkrei", "fixture-sha256", "ident-rename", "regex-bafkrei"])
        # DNA is skipped, collision reported, nothing in cite/oci/record/prose files
        self.assertTrue(any(k == "collision" for ns in notes.values() for k, _, _ in ns))
        self.assertIn("elohim/holochain/dna/x/lib.rs", notes)
        before = {p: (self.root / p).read_text() for p in ["scripts/ci/build-storage-image.sh", "genesis/notes.md"]}
        cm.plan_all(self.root, self.allow, True)
        for p, t in before.items():
            self.assertEqual((self.root / p).read_text(), t)

    def run_cm(self, *args):
        return subprocess.run([sys.executable, str(HERE / "address-codemod.py"), "--root", str(self.root),
                               "--out", str(Path(self.td.name) / "out"), "--walk", *args],
                              capture_output=True, text=True)

    def test_apply_idempotent_and_receipt(self):
        r = self.run_cm("apply", "--track", "elohim-storage", "--allow-coupled")
        self.assertEqual(r.returncode, 0, r.stderr)
        blob = (self.root / "elohim/elohim-storage/src/blob.rs").read_text()
        self.assertIn("blob_cid", blob)
        self.assertIn("bafkr4i[a-z2-7]", blob)
        fx = (self.root / "elohim/elohim-storage/tests/fixture_a.rs").read_text()
        self.assertNotIn("sha256-", fx)
        self.assertIn("bafkr4i" + "a" * 52, fx)
        snap = {p: p.read_text() for p in self.root.rglob("*") if p.is_file()}
        rec = Path(self.td.name) / "out/codemod-receipt.jsonl"
        n1 = len(rec.read_text().splitlines())
        self.assertEqual(n1, 4)
        r2 = self.run_cm("apply", "--track", "elohim-storage", "--allow-coupled")
        self.assertIn("applied 0", r2.stdout)
        self.assertEqual(snap, {p: p.read_text() for p in self.root.rglob("*") if p.is_file()})
        self.assertEqual(len(rec.read_text().splitlines()), n1)
        row = json.loads(rec.read_text().splitlines()[0])
        self.assertEqual(set(row), {"path", "class", "rule", "line", "before", "after", "track"})
        # never touched
        self.assertIn("sha256:" + H64, (self.root / "scripts/ci/build-storage-image.sh").read_text())
        self.assertIn("blob_hash", (self.root / "elohim/holochain/dna/x/lib.rs").read_text())

    def test_coupled_needs_flag(self):
        self.run_cm("apply", "--track", "elohim-storage")
        self.assertIn("blob_hash", (self.root / "elohim/elohim-storage/src/blob.rs").read_text())

    def test_blake3_cid_helper(self):
        c = cm.hex_to_blake3_cid(H64)
        self.assertTrue(c.startswith("bafkr4i"), c)
        self.assertEqual(len(c), 59)


class Lint(unittest.TestCase):
    def setUp(self):
        self.td = tempfile.TemporaryDirectory()
        self.root = Path(self.td.name) / "repo"
        self.root.mkdir()
        w(self.root, "scripts/ci/build-storage-image.sh", f"IMG=harbor/x@sha256:{H64}\n")
        w(self.root, "scripts/ci/other.sh", f"IMG=harbor/x@sha256:{H64}\n")
        w(self.root, "elohim/eprfs/epr-cli/src/flow/cites.rs", f'// sha256-{H64}\n')
        w(self.root, "genesis/docs/a.md", f"sha256-{H64}\n")

    def tearDown(self):
        self.td.cleanup()

    def lint(self, **env):
        e = dict(os.environ, ADDRESS_LINT_ROOT=str(self.root), **env)
        return subprocess.run(["bash", str(HERE / "address-profile-lint.sh")], capture_output=True,
                              text=True, env=e)

    def test_passes_allowlisted_oci_and_keep_paths(self):
        r = self.lint()
        self.assertEqual(r.returncode, 0, r.stdout)
        self.assertIn("ok (0 violations)", r.stdout)

    def test_refuses_planted_sha256(self):
        w(self.root, "elohim/elohim-storage/src/x.rs", f'let a = "sha256-{H64}";\n')
        r = self.lint()
        self.assertEqual(r.returncode, 1, r.stdout)
        self.assertIn("elohim/elohim-storage/src/x.rs", r.stdout)
        self.assertIn("1 violation(s) in 1 file(s)", r.stdout)

    def test_refuses_identifier_and_gate_and_honours_escape(self):
        w(self.root, "doorway/a.rs", "let blob_hash = 1;\nlet r = \"^bafkrei[a-z2-7]{52}\";\n"
                                       "let ok = blob_hash; // address-lint: allow\n")
        r = self.lint()
        self.assertEqual(r.returncode, 1)
        self.assertIn("2 violation(s)", r.stdout)

    def test_extra_allowlist(self):
        w(self.root, "elohim/x/y.rs", f'"sha256-{H64}"\n')
        extra = Path(self.td.name) / "al.txt"
        extra.write_text("elohim/x/*\n")
        self.assertEqual(self.lint().returncode, 1)
        self.assertEqual(self.lint(ADDRESS_LINT_ALLOWLIST=str(extra)).returncode, 0)


if __name__ == "__main__":
    unittest.main(verbosity=2)
