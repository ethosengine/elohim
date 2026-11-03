import importlib.util
import io
from pathlib import Path
import tarfile
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("distribution", Path(__file__).with_name("source-distribution.py"))
distribution = importlib.util.module_from_spec(spec)
spec.loader.exec_module(distribution)


class DistributionTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        (self.root / ".cargo").mkdir()
        (self.root / ".cargo/config.toml").write_text('[registries.elohim]\nindex="sparse+https://example.invalid/"\n')
        for index, crate in enumerate(distribution.CRATES):
            path = self.root / crate
            (path / "src").mkdir(parents=True)
            (path / "Cargo.toml").write_text(f'[package]\nname="component-{index}"\nversion="1.0.0"\nlicense="Apache-2.0"\n')
            (path / "src/lib.rs").write_text("pub fn value() -> u8 { 1 }\n")
        (self.root / distribution.CHECKER / "Cargo.lock").write_text('version=4\n[[package]]\nname="component-0"\nversion="1.0.0"\n')
        (self.root / "elohim/eprfs/Cargo.toml").write_text('[workspace]\nmembers=["eprfs-meta","eprfs-core","unrelated"]\n[workspace.package]\nversion="0.3.2"\n[workspace.dependencies]\nserde="1"\n[patch.crates-io]\nunrelated={path="outside"}\n')

    def test_archive_is_deterministic_and_holds_exact_sources(self):
        files = distribution.source_files(self.root)
        archive = distribution.archive_bytes(files)
        self.assertEqual(archive, distribution.archive_bytes(dict(reversed(list(files.items())))))
        self.assertTrue(distribution.cid(archive).startswith("bafkrei"))
        with tarfile.open(fileobj=io.BytesIO(archive), mode="r:gz") as unpacked:
            self.assertTrue(all(member.isfile() and member.mtime == 0 for member in unpacked))
            path = distribution.PREFIX + distribution.CHECKER + "/src/lib.rs"
            self.assertEqual(unpacked.extractfile(path).read(), files[distribution.CHECKER + "/src/lib.rs"])
        original = distribution.cid(archive)
        (self.root / distribution.CHECKER / "src/lib.rs").write_text("changed")
        self.assertNotEqual(original, distribution.cid(distribution.archive_bytes(distribution.source_files(self.root))))

    def test_workspace_projection_preserves_inheritance_only(self):
        files = distribution.source_files(self.root)
        projected = files["elohim/eprfs/Cargo.toml"].decode()
        self.assertIn('serde="1"', projected)
        self.assertNotIn("unrelated", projected)
        self.assertIn('members = ["eprfs-core", "eprfs-meta"]', projected)

    def test_symlink_and_undeclared_dependency_refuse(self):
        source = self.root / distribution.CHECKER / "src/lib.rs"
        source.unlink()
        source.symlink_to(self.root / distribution.CRATES[1] / "src/lib.rs")
        with self.assertRaisesRegex(ValueError, "symlink"):
            distribution.source_files(self.root)
        source.unlink()
        source.write_text("source")
        manifest = self.root / distribution.CHECKER / "Cargo.toml"
        with manifest.open("a") as file:
            file.write('[dependencies]\nmissing={path="../unknown"}\n')
        with self.assertRaisesRegex(ValueError, "undeclared first-party"):
            distribution.source_files(self.root)

    def test_lockfile_rejects_unheld_registry_or_path(self):
        path = self.root / distribution.CHECKER / "Cargo.lock"
        for declaration in ('name="missing"', 'name="component-0"\nsource="git+https://example.invalid/repo"'):
            path.write_text(f'version=4\n[[package]]\n{declaration}\nversion="1.0.0"\n')
            with self.assertRaises(ValueError):
                distribution.source_files(self.root)


if __name__ == "__main__":
    unittest.main()
