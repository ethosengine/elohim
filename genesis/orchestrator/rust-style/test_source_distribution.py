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
        for crate in distribution.CRATES:
            path = self.root / crate
            (path / "src").mkdir(parents=True)
            if crate == distribution.CHECKER:
                manifest = (
                    '[package]\nname="elohim-rust-style"\nversion="0.1.0"\nlicense="Apache-2.0"\n'
                    '[workspace]\nmembers=["scan"]\n'
                    '[dependencies]\n'
                    'elohim-rust-style-scan={path="scan"}\n'
                    'eprfs-core={version="=0.3.3",registry="elohim"}\n'
                    'eprfs-meta={version="=0.3.4",registry="elohim"}\n'
                )
            else:
                manifest = (
                    '[package]\nname="elohim-rust-style-scan"\nversion="0.1.0"\n'
                    'license="Apache-2.0"\n[dependencies]\n'
                )
            (path / "Cargo.toml").write_text(manifest)
            (path / "src/lib.rs").write_text("pub fn value() -> u8 { 1 }\n")
        checksum = "0" * 64
        (self.root / distribution.CHECKER / "Cargo.lock").write_text(
            'version=4\n'
            '[[package]]\nname="elohim-rust-style"\nversion="0.1.0"\n'
            'dependencies=["elohim-rust-style-scan"]\n'
            '[[package]]\nname="elohim-rust-style-scan"\nversion="0.1.0"\n'
            '[[package]]\nname="eprfs-core"\nversion="0.3.3"\n'
            'source="sparse+https://example.invalid/"\n'
            f'checksum="{checksum}"\n'
            '[[package]]\nname="eprfs-meta"\nversion="0.3.4"\n'
            'source="sparse+https://example.invalid/"\n'
            f'checksum="{checksum}"\n'
            '[[package]]\nname="elohim-seam-contracts"\nversion="0.1.0"\n'
            'source="sparse+https://example.invalid/"\n'
            f'checksum="{checksum}"\n'
        )

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

    def test_registry_dependency_sources_are_not_archived(self):
        local = self.root / "elohim/eprfs/eprfs-core/src"
        local.mkdir(parents=True)
        (local / "lib.rs").write_text("must not be archived")
        files = distribution.source_files(self.root)
        self.assertFalse(any(path.startswith("elohim/eprfs/") for path in files))
        self.assertNotIn("build.rs", {Path(path).name for path in files})

    def test_symlink_and_undeclared_dependency_refuse(self):
        source = self.root / distribution.CHECKER / "src/lib.rs"
        source.unlink()
        target = self.root / "outside.rs"
        target.write_text("pub fn outside() {}\n")
        source.symlink_to(target)
        with self.assertRaisesRegex(ValueError, "symlink"):
            distribution.source_files(self.root)
        source.unlink()
        source.write_text("source")
        manifest = self.root / distribution.CHECKER / "Cargo.toml"
        with manifest.open("a") as file:
            file.write('missing={path="../unknown"}\n')
        with self.assertRaisesRegex(ValueError, "undeclared first-party"):
            distribution.source_files(self.root)

    def test_lockfile_rejects_unheld_registry_or_path(self):
        path = self.root / distribution.CHECKER / "Cargo.lock"
        valid = path.read_text()
        for declaration in (
            '[[package]]\nname="missing"\nversion="1.0.0"\n',
            '[[package]]\nname="foreign"\nversion="1.0.0"\nsource="git+https://example.invalid/repo"\nchecksum="0"\n',
        ):
            path.write_text(valid + declaration)
            with self.assertRaises(ValueError):
                distribution.source_files(self.root)


if __name__ == "__main__":
    unittest.main()
