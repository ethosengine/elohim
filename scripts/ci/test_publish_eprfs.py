"""Offline publisher decisions against fake registry and Cargo responses."""

from __future__ import annotations

import hashlib
import gzip
import io
import json
import os
import sys
import tarfile
import tempfile
import unittest
from pathlib import Path
from unittest import mock
from urllib.error import HTTPError, URLError

sys.path.insert(0, str(Path(__file__).parent))
import publish_eprfs as publisher


NAME = "eprfs-core"
VERSION = "0.1.0"
BASE = "https://nexus.example.invalid/repository/cargo-internal"


def archive(
    files: dict[str, bytes] | None = None,
    *,
    name: str = NAME,
    modes: dict[str, int] | None = None,
    link: str | None = None,
    duplicate: str | None = None,
) -> bytes:
    files = files or {"Cargo.toml": b"[package]\nname='eprfs-core'\nversion='0.1.0'\n", "src/lib.rs": b"pub fn x() {}"}
    modes = modes or {}
    stream = io.BytesIO()
    with tarfile.open(fileobj=stream, mode="w:gz") as tar:
        for path, contents in files.items():
            info = tarfile.TarInfo(f"{name}-{VERSION}/{path}")
            info.mode = modes.get(path, 0o644)
            info.size = len(contents)
            tar.addfile(info, io.BytesIO(contents))
        if duplicate:
            info = tarfile.TarInfo(f"{name}-{VERSION}/{duplicate}")
            info.size = 1
            tar.addfile(info, io.BytesIO(b"x"))
        if link:
            info = tarfile.TarInfo(f"{name}-{VERSION}/{link}")
            info.type = tarfile.SYMTYPE
            info.linkname = "../../outside"
            tar.addfile(info)
    return stream.getvalue()


def record(payload: bytes, name: str = NAME) -> dict:
    return {
        "name": name,
        "vers": VERSION,
        "cksum": hashlib.sha256(payload).hexdigest(),
        "deps": [],
        "yanked": False,
    }


class Response:
    def __init__(self, body: bytes):
        self.body = io.BytesIO(body)

    def __enter__(self):
        return self

    def __exit__(self, *_args):
        self.body.close()

    def read(self, size: int) -> bytes:
        return self.body.read(size)


class RegistryTests(unittest.TestCase):
    def test_registry_is_pinned_and_redirect_handler_refuses_hop(self):
        with tempfile.TemporaryDirectory() as temporary:
            repo = Path(temporary)
            (repo / ".cargo").mkdir()
            (repo / ".cargo/config.toml").write_text(
                '[registries.elohim]\nindex="sparse+https://other.invalid/repository/cargo-internal/"\n'
            )
            with mock.patch.dict(os.environ, {}, clear=True):
                with self.assertRaisesRegex(publisher.PublishError, "pinned Nexus"):
                    publisher.registry_base(repo)
        self.assertIsNone(publisher.NoRedirect().redirect_request(None, None, 302, "", {}, BASE))

    def test_only_404_means_missing(self):
        for code in (404, 401, 500, 302):
            with self.subTest(code=code):
                error = HTTPError(BASE, code, "secret-token", {}, None)
                with mock.patch.object(publisher.OPENER, "open", side_effect=error):
                    if code == 404:
                        self.assertIsNone(publisher.index_record(BASE, NAME, VERSION))
                    else:
                        with self.assertRaisesRegex(publisher.PublishError, f"HTTP {code}") as caught:
                            publisher.index_record(BASE, NAME, VERSION)
                        self.assertNotIn("secret-token", str(caught.exception))

    def test_malformed_duplicate_and_bounded_index_fail(self):
        for data in (b"not-json", (json.dumps(record(b"one")) + "\n" + json.dumps(record(b"two"))).encode()):
            with self.subTest(data=data[:12]), mock.patch.object(publisher.OPENER, "open", return_value=Response(data)):
                with self.assertRaises(publisher.PublishError):
                    publisher.index_record(BASE, NAME, VERSION)
        with mock.patch.object(publisher, "MAX_INDEX", 2), mock.patch.object(
            publisher.OPENER, "open", return_value=Response(b"large")
        ):
            with self.assertRaisesRegex(publisher.PublishError, "size limit"):
                publisher.index_record(BASE, NAME, VERSION)

    def test_download_config_must_stay_at_same_nexus_endpoint(self):
        with mock.patch.object(publisher.OPENER, "open", return_value=Response(
            json.dumps({"dl": "https://other.invalid/crates"}).encode()
        )):
            with self.assertRaisesRegex(publisher.PublishError, "differs"):
                publisher.download_base(BASE)
        with mock.patch.object(publisher.OPENER, "open", return_value=Response(
            json.dumps({"dl": BASE + "/crates"}).encode()
        )):
            self.assertEqual(publisher.download_base(BASE), BASE + "/crates")

    def test_existing_receipt_verifies_checksum_and_payload(self):
        manifest = b"[package]\nname='eprfs-core'\nversion='0.1.0'\n"
        local = archive({"Cargo.toml": manifest, "src/lib.rs": b"same", ".cargo_vcs_info.json": b"local"})
        remote = archive({"Cargo.toml": manifest, "src/lib.rs": b"same", ".cargo_vcs_info.json": b"remote"})
        with mock.patch.object(publisher, "index_record", return_value=record(remote)), mock.patch.object(
            publisher, "get", return_value=remote
        ):
            self.assertTrue(publisher.verify_existing(BASE, BASE + "/crates", NAME, VERSION, local))
        with mock.patch.object(publisher, "index_record", return_value=record(b"other")), mock.patch.object(
            publisher, "get", return_value=remote
        ):
            with self.assertRaisesRegex(publisher.PublishError, "checksum"):
                publisher.verify_existing(BASE, BASE + "/crates", NAME, VERSION, local)
        changed = archive({"Cargo.toml": manifest, "src/lib.rs": b"changed"})
        with mock.patch.object(publisher, "index_record", return_value=record(changed)), mock.patch.object(
            publisher, "get", return_value=changed
        ):
            with self.assertRaisesRegex(publisher.PublishError, "immutable version collision"):
                publisher.verify_existing(BASE, BASE + "/crates", NAME, VERSION, local)

    def test_matching_archive_rejects_damaged_dependency_index(self):
        name = "eprfs-storage"
        manifest = (
            b"[package]\nname='eprfs-storage'\nversion='0.1.0'\n"
            b"[dependencies.eprfs-core]\nversion='0.1.0'\nregistry='elohim'\n"
            b"features=['wire']\noptional=true\n"
            b"[features]\nwire=['dep:eprfs-core']\n"
        )
        payload = archive({"Cargo.toml": manifest}, name=name)
        good_dep = {
            "name": "eprfs-core", "req": "^0.1.0", "features": ["wire"],
            "optional": True, "default_features": True, "target": None,
            "kind": "normal", "registry": None,
        }
        good = {**record(payload, name), "deps": [good_dep], "features": {"wire": ["dep:eprfs-core"]}}
        for bad in (
            {**good, "deps": [{**good_dep, "name": "lost-alias"}]},
            {**good, "deps": [{**good_dep, "registry": "https://github.com/rust-lang/crates.io-index"}]},
            {**good, "deps": [{**good_dep, "optional": False}]},
            {**good, "deps": [{**good_dep, "features": []}]},
            {**good, "deps": [{**good_dep, "req": "^0.0.9"}]},
            {**good, "features": {}},
        ):
            with self.subTest(bad=bad), mock.patch.object(publisher, "index_record", return_value=bad), mock.patch.object(
                publisher, "get", return_value=payload
            ):
                with self.assertRaisesRegex(publisher.PublishError, "index"):
                    publisher.verify_existing(BASE, BASE + "/crates", name, VERSION, payload)
        with mock.patch.object(publisher, "index_record", return_value=good), mock.patch.object(
            publisher, "get", return_value=payload
        ):
            self.assertTrue(publisher.verify_existing(BASE, BASE + "/crates", name, VERSION, payload))

        malformed = (
            {**good_dep, "optional": 1},
            {**good_dep, "default_features": 1},
            {**good_dep, "features": "wire"},
            {**good_dep, "package": ""},
            {**good_dep, "kind": "mystery"},
            {**good_dep, "target": 1},
        )
        for dep in malformed:
            with self.subTest(malformed=dep), mock.patch.object(publisher, "index_record", return_value={**good, "deps": [dep]}), mock.patch.object(
                publisher, "get", return_value=payload
            ):
                with self.assertRaisesRegex(publisher.PublishError, "malformed"):
                    publisher.verify_existing(BASE, BASE + "/crates", name, VERSION, payload)
        with mock.patch.object(publisher, "index_record", return_value={
            **good, "deps": [good_dep, {"name": "ignored", "kind": "dev", "optional": 1}]
        }), mock.patch.object(publisher, "get", return_value=payload):
            with self.assertRaisesRegex(publisher.PublishError, "malformed"):
                publisher.verify_existing(BASE, BASE + "/crates", name, VERSION, payload)


class ArchiveTests(unittest.TestCase):
    def test_huge_declared_member_refuses_before_extraction(self):
        raw = bytearray(gzip.decompress(archive()))
        # First tar header: forge a 1 GiB logical member inside a tiny gzip.
        raw[124:136] = f"{1 << 30:011o}\0".encode()
        raw[148:156] = b"        "
        raw[148:156] = f"{sum(raw[:512]):06o}\0 ".encode()
        payload = gzip.compress(raw)
        self.assertLess(len(payload), 1024)
        with self.assertRaisesRegex(publisher.PublishError, "member size limit"):
            publisher.archive_files(payload, NAME, VERSION)

    def test_sparse_member_refuses_before_extraction(self):
        sparse = tarfile.TarInfo(f"{NAME}-{VERSION}/Cargo.toml")
        sparse.type = tarfile.GNUTYPE_SPARSE
        sparse.size = 1
        sparse.sparse = [(0, 1)]
        fake = mock.MagicMock()
        fake.__enter__.return_value = fake
        fake.__iter__.return_value = iter([sparse])
        fake.extractfile.side_effect = AssertionError("sparse bytes must not be read")
        with mock.patch.object(publisher.tarfile, "open", return_value=fake):
            with self.assertRaisesRegex(publisher.PublishError, "sparse|non-regular"):
                publisher.archive_files(archive(), NAME, VERSION)
        fake.extractfile.assert_not_called()

    def test_gzip_expansion_includes_tar_headers_and_padding(self):
        payload = archive()
        self.assertLess(sum(len(value) for value in (b"manifest", b"same")), 500)
        with mock.patch.object(publisher, "MAX_UNPACKED", 500):
            with self.assertRaisesRegex(publisher.PublishError, "unpacked size limit"):
                publisher.archive_files(payload, NAME, VERSION)
        stream = io.BytesIO()
        with tarfile.open(fileobj=stream, mode="w:gz", format=tarfile.PAX_FORMAT, pax_headers={"comment": "x" * 4000}) as tar:
            info = tarfile.TarInfo(f"{NAME}-{VERSION}/Cargo.toml")
            info.size = 1
            tar.addfile(info, io.BytesIO(b"x"))
        with mock.patch.object(publisher, "MAX_UNPACKED", 2000):
            with self.assertRaisesRegex(publisher.PublishError, "unpacked size limit"):
                publisher.archive_files(stream.getvalue(), NAME, VERSION)

    def test_local_archive_is_read_with_a_size_cap(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "sample.crate"
            path.write_bytes(b"x" * 20)
            with mock.patch.object(publisher, "MAX_ARCHIVE", 10):
                with self.assertRaisesRegex(publisher.PublishError, "size limit"):
                    publisher.read_local_archive(path, NAME, VERSION)

    def test_modes_source_and_lock_are_part_of_equivalence(self):
        original = archive({"Cargo.toml": b"manifest", "Cargo.lock": b"lock", "src/lib.rs": b"same"})
        originals = publisher.archive_files(original, NAME, VERSION)
        for changed in (
            archive({"Cargo.toml": b"different", "Cargo.lock": b"lock", "src/lib.rs": b"same"}),
            archive({"Cargo.toml": b"manifest", "Cargo.lock": b"different", "src/lib.rs": b"same"}),
            archive({"Cargo.toml": b"manifest", "Cargo.lock": b"lock", "src/lib.rs": b"same"}, modes={"src/lib.rs": 0o755}),
        ):
            self.assertNotEqual(originals, publisher.archive_files(changed, NAME, VERSION))

    def test_unsafe_entries_are_rejected(self):
        for payload in (archive(link="escape"), archive(duplicate="src/lib.rs"), archive({"Cargo.toml": b"x", "../escape": b"x"})):
            with self.subTest(size=len(payload)), self.assertRaises(publisher.PublishError):
                publisher.archive_files(payload, NAME, VERSION)
        with mock.patch.object(publisher, "MAX_MEMBERS", 1):
            with self.assertRaisesRegex(publisher.PublishError, "too many"):
                publisher.archive_files(archive(), NAME, VERSION)


class PrecheckTests(unittest.TestCase):
    def setUp(self):
        self.root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        self.workspace = self.root / "elohim/eprfs"
        self.crate = self.workspace / NAME
        (self.crate / "src").mkdir(parents=True)
        (self.workspace / "Cargo.toml").write_text('[workspace.package]\nversion="0.1.0"\n')
        self.files = {
            "Cargo.toml": b"[package]\nname='eprfs-core'\nversion='0.1.0'\n",
            "Cargo.toml.orig": b"[package]\nname='eprfs-core'\nversion.workspace=true\n",
            "Cargo.lock": b"generated lock",
            ".cargo_vcs_info.json": b"published revision",
            "src/lib.rs": b"pub fn x() {}",
        }
        (self.crate / "Cargo.toml").write_bytes(self.files["Cargo.toml.orig"])
        (self.crate / "Cargo.lock").write_bytes(b"local lock")
        (self.crate / "src/lib.rs").write_bytes(self.files["src/lib.rs"])
        self.name = NAME
        self.enterContext(mock.patch.dict(os.environ, {}, clear=True))
        self.enterContext(mock.patch.object(publisher, "__file__", str(self.root / "scripts/ci/publish_eprfs.py")))
        self.enterContext(mock.patch.object(sys, "argv", ["publish_eprfs.py", "--precheck"]))
        self.enterContext(mock.patch.object(publisher, "registry_base", return_value=BASE))
        self.enterContext(mock.patch.object(publisher, "cargo", side_effect=AssertionError("no Cargo in precheck")))
        self.git = self.enterContext(mock.patch.object(publisher.subprocess, "run", return_value=mock.Mock(
            returncode=0,
            stdout=b"elohim/eprfs/eprfs-core/Cargo.toml\0elohim/eprfs/eprfs-core/Cargo.lock\0elohim/eprfs/eprfs-core/src/lib.rs\0",
        )))

    def responses(self):
        payload = archive(self.files, name=self.name)
        return [
            Response(json.dumps({"dl": BASE + "/crates"}).encode()),
            Response(json.dumps(record(payload, self.name)).encode()),
            Response(payload),
        ]

    def run_precheck(self, responses=None):
        with mock.patch.object(publisher, "CRATES", (self.name,)), mock.patch.object(
            publisher.OPENER, "open", side_effect=responses if responses is not None else self.responses()
        ), mock.patch.object(sys, "stdout", new_callable=io.StringIO) as stdout, mock.patch.object(
            sys, "stderr", new_callable=io.StringIO
        ) as stderr:
            status = publisher.main()
        return status, stdout.getvalue(), stderr.getvalue()

    def test_matching_published_tree_passes_without_credentials_or_target(self):
        status, stdout, stderr = self.run_precheck()
        self.assertEqual(status, 0, stderr)
        self.assertEqual(stdout, f"verified existing: {NAME} {VERSION}\neprfs precheck complete\n")
        self.assertEqual(stderr, "")
        self.git.assert_called_once_with(
            ["git", "ls-files", "-z", "--", "elohim/eprfs/eprfs-core"],
            cwd=self.root, capture_output=True,
        )

    def test_changed_source_or_original_manifest_refuses(self):
        for path in ("src/lib.rs", "Cargo.toml"):
            with self.subTest(path=path):
                local = self.crate / path
                original = local.read_bytes()
                local.write_bytes(b"changed")
                status, _, stderr = self.run_precheck()
                local.write_bytes(original)
                self.assertEqual(status, 1)
                self.assertIn("packaged source differs", stderr)
                self.assertIn(f"({path + '.orig' if path == 'Cargo.toml' else path}); bump the version", stderr)

    def test_missing_local_source_refuses(self):
        (self.crate / "src/lib.rs").unlink()
        status, _, stderr = self.run_precheck()
        self.assertEqual(status, 1)
        self.assertIn("packaged source differs", stderr)
        self.assertIn("(src/lib.rs)", stderr)

    def test_tracked_file_missing_from_archive_refuses(self):
        (self.crate / "src/new file.rs").write_bytes(b"new")
        self.git.return_value.stdout += b"elohim/eprfs/eprfs-core/src/new file.rs\0"
        status, _, stderr = self.run_precheck()
        self.assertEqual(status, 1)
        self.assertIn("packaged source differs", stderr)
        self.assertIn("(src/new file.rs)", stderr)

    def test_absent_version_passes(self):
        responses = self.responses()
        responses[1] = HTTPError(BASE, 404, "missing", {}, None)
        status, stdout, stderr = self.run_precheck(responses)
        self.assertEqual(status, 0, stderr)
        self.assertEqual(stdout, f"absent: {NAME} {VERSION}\neprfs precheck complete\n")
        self.git.assert_not_called()

    def test_unreachable_registry_is_claimed(self):
        for stage in range(3):
            for error in (OSError("offline"), URLError("offline"), HTTPError(BASE, 503, "unavailable", {}, None)):
                with self.subTest(stage=stage, error=type(error).__name__):
                    responses = self.responses()
                    responses[stage] = error
                    status, stdout, stderr = self.run_precheck(responses)
                    self.assertEqual(status, 0)
                    self.assertIn("precheck: claimed (registry unreachable:", stderr)
                    self.assertEqual(stdout, "eprfs precheck complete\n")

    def test_checksum_malformed_index_and_missing_archive_refuse(self):
        for stage, response, message in (
            (2, Response(archive()), "checksum"),
            (1, Response(b"not-json"), "malformed"),
            (2, HTTPError(BASE, 404, "missing", {}, None), "HTTP 404"),
        ):
            with self.subTest(message=message):
                responses = self.responses()
                responses[stage] = response
                status, _, stderr = self.run_precheck(responses)
                self.assertEqual(status, 1)
                self.assertIn(message, stderr)
                self.assertNotIn("claimed", stderr)

    def test_missing_crate_directory_refuses(self):
        self.name = "eprfs-host"
        status, _, stderr = self.run_precheck()
        self.assertEqual(status, 1)
        self.assertIn("crate directory is absent", stderr)

    def test_seam_contracts_uses_its_own_directory_and_version(self):
        self.name = "elohim-seam-contracts"
        self.crate = self.root / "crates/seam-contracts"
        (self.crate / "src").mkdir(parents=True)
        (self.workspace / "Cargo.toml").write_text('[workspace.package]\nversion="0.2.0"\n')
        self.files["Cargo.toml.orig"] = b"[package]\nname='elohim-seam-contracts'\nversion='0.1.0'\n"
        (self.crate / "Cargo.toml").write_bytes(self.files["Cargo.toml.orig"])
        (self.crate / "src/lib.rs").write_bytes(self.files["src/lib.rs"])
        self.git.return_value.stdout = b"crates/seam-contracts/Cargo.toml\0crates/seam-contracts/src/lib.rs\0"
        status, stdout, stderr = self.run_precheck()
        self.assertEqual(status, 0, stderr)
        self.assertIn(f"verified existing: {self.name} 0.1.0", stdout)
        self.git.assert_called_once_with(
            ["git", "ls-files", "-z", "--", "crates/seam-contracts"],
            cwd=self.root, capture_output=True,
        )


class PublicationTests(unittest.TestCase):
    def run_one(self, *, upload_result: bool, receipts: list[bool], publish: bool = True):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            workspace = root / "elohim/eprfs"
            workspace.mkdir(parents=True)
            (workspace / "Cargo.toml").write_text('[workspace.package]\nversion="0.1.0"\n')
            target = root.parent / (root.name + "-target")
            (target / "package").mkdir(parents=True)
            (target / "package" / f"{NAME}-{VERSION}.crate").write_bytes(archive())
            try:
                env = {"CARGO_TARGET_DIR": str(target), "CARGO_REGISTRIES_ELOHIM_TOKEN": "secret-token"}
                with mock.patch.dict(os.environ, env, clear=True), mock.patch.object(
                    publisher, "CRATES", (NAME,)
                ), mock.patch.object(publisher, "registry_base", return_value=BASE), mock.patch.object(
                    publisher, "download_base", return_value=BASE + "/crates"
                ), mock.patch.object(publisher, "cargo", side_effect=[True, upload_result] if publish else [True]) as calls, mock.patch.object(
                    publisher, "verify_existing", side_effect=receipts
                ), mock.patch.object(publisher.time, "sleep"):
                    publisher.run(root, publish)
                return calls.call_args_list
            finally:
                for child in (target / "package").iterdir():
                    child.unlink()
                (target / "package").rmdir()
                target.rmdir()

    def test_check_never_uploads(self):
        calls = self.run_one(upload_result=True, receipts=[False], publish=False)
        self.assertEqual(len(calls), 1)
        self.assertIn("--locked", calls[0].args[1])
        self.assertEqual(calls[0].args[1][0], "package")
        self.assertEqual(calls[0].args[2]["CARGO_REGISTRIES_ELOHIM_INDEX"], "sparse+" + BASE + "/")

    def test_failed_upload_requires_verified_existing_payload(self):
        calls = self.run_one(upload_result=False, receipts=[False, True])
        self.assertEqual(calls[1].args[1][0], "publish")
        self.assertIn("--locked", calls[1].args[1])
        with self.assertRaisesRegex(publisher.PublishError, "absent"):
            self.run_one(upload_result=False, receipts=[False, False])

    def test_success_waits_for_verified_index_archive(self):
        self.run_one(upload_result=True, receipts=[False, False, True])
        with self.assertRaisesRegex(publisher.PublishError, "absent from index"):
            self.run_one(upload_result=True, receipts=[False] * 13)

    def test_cargo_failure_output_is_not_reported(self):
        with mock.patch.object(publisher.subprocess, "run", return_value=mock.Mock(returncode=101, stderr=b"secret-token")):
            self.assertFalse(publisher.cargo(Path("/tmp"), ["package"], {}))


class PrerequisitePublicationTests(unittest.TestCase):
    def setUp(self):
        self.root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        self.target = Path(self.enterContext(tempfile.TemporaryDirectory()))
        self.seam = self.root / "crates/seam-contracts"
        self.eprfs = self.root / "elohim/eprfs"
        self.seam.mkdir(parents=True)
        self.eprfs.mkdir(parents=True)
        (self.seam / "Cargo.toml").write_text(
            '[workspace]\n[package]\nname="elohim-seam-contracts"\nversion="0.1.0"\n'
        )
        (self.eprfs / "Cargo.toml").write_text('[workspace.package]\nversion="0.2.0"\n')
        (self.target / "package").mkdir()
        self.names = (
            "elohim-seam-contracts", "eprfs-core", "eprfs-host",
            "eprfs-storage", "eprfs-meta", "eprfs-local",
        )
        for name in self.names:
            version = "0.1.0" if name == "elohim-seam-contracts" else "0.2.0"
            stream = io.BytesIO()
            with tarfile.open(fileobj=stream, mode="w:gz") as tar:
                contents = f'[package]\nname="{name}"\nversion="{version}"\n'.encode()
                info = tarfile.TarInfo(f"{name}-{version}/Cargo.toml")
                info.mode = 0o644
                info.size = len(contents)
                tar.addfile(info, io.BytesIO(contents))
            (self.target / "package" / f"{name}-{version}.crate").write_bytes(stream.getvalue())
        self.enterContext(mock.patch.dict(os.environ, {
            "CARGO_TARGET_DIR": str(self.target),
            "CARGO_REGISTRIES_ELOHIM_TOKEN": "secret-token",
        }, clear=True))
        self.enterContext(mock.patch.object(publisher, "registry_base", return_value=BASE))
        self.enterContext(mock.patch.object(publisher, "download_base", return_value=BASE + "/crates"))
        self.enterContext(mock.patch.object(publisher.time, "sleep"))
        self.events = []

        def cargo(workspace, arguments, env):
            self.events.append((arguments[0], arguments[-1]))
            return True

        self.cargo = self.enterContext(mock.patch.object(publisher, "cargo", side_effect=cargo))
        self.receipts = self.enterContext(mock.patch.object(publisher, "verify_existing"))

    def test_prerequisite_is_verified_before_dependents_use_their_own_version(self):
        pending = iter([False, False, True] + [False, True] * 5)

        def receipt(index, download, name, version, payload):
            result = next(pending)
            self.events.append(("receipt", name, version, result))
            return result

        self.receipts.side_effect = receipt
        publisher.run(self.root, publish=True)
        self.assertEqual(self.events[:6], [
            ("package", "elohim-seam-contracts"),
            ("receipt", "elohim-seam-contracts", "0.1.0", False),
            ("publish", "elohim-seam-contracts"),
            ("receipt", "elohim-seam-contracts", "0.1.0", False),
            ("receipt", "elohim-seam-contracts", "0.1.0", True),
            ("package", "eprfs-core"),
        ])
        self.assertEqual(len(self.cargo.call_args_list), 12)
        for position, name in enumerate(self.names):
            workspace = self.seam if position == 0 else self.eprfs
            package_calls = self.cargo.call_args_list[position * 2:position * 2 + 2]
            for command, call in zip(("package", "publish"), package_calls):
                self.assertEqual(call.args[0], workspace)
                self.assertEqual(call.args[1], [
                    command, "--locked", "--registry", "elohim", "--no-verify", "-p", name,
                ])
        for call in self.receipts.call_args_list:
            name, version = call.args[2:4]
            self.assertEqual(version, "0.1.0" if name == self.names[0] else "0.2.0")

    def test_verified_existing_prerequisite_is_not_uploaded_again(self):
        self.receipts.side_effect = [True] + [False, True] * 5
        publisher.run(self.root, publish=True)
        self.assertEqual(self.events[:2], [
            ("package", "elohim-seam-contracts"), ("package", "eprfs-core"),
        ])
        self.assertNotIn(("publish", "elohim-seam-contracts"), self.events)
        self.assertEqual(len(self.cargo.call_args_list), 11)

    def test_failed_prerequisite_upload_stops_before_dependent_packaging(self):
        self.cargo.side_effect = [True, False]
        self.receipts.side_effect = [False, False]
        with self.assertRaisesRegex(
            publisher.PublishError, "elohim-seam-contracts 0.1.0: upload failed"
        ):
            publisher.run(self.root, publish=True)
        self.assertEqual(len(self.cargo.call_args_list), 2)
        for call in self.cargo.call_args_list:
            self.assertEqual(call.args[0], self.seam)
            self.assertEqual(call.args[1][-1], "elohim-seam-contracts")

    def test_prerequisite_collision_stops_before_any_upload_or_dependent(self):
        self.receipts.side_effect = publisher.PublishError(
            "immutable version collision; bump the version"
        )
        with self.assertRaisesRegex(publisher.PublishError, "immutable version collision"):
            publisher.run(self.root, publish=True)
        self.assertEqual(self.events, [("package", "elohim-seam-contracts")])

    def test_check_packages_both_workspaces_without_uploading(self):
        self.receipts.return_value = False
        publisher.run(self.root, publish=False)
        self.assertEqual(self.events, [("package", name) for name in self.names])
        self.assertEqual(self.cargo.call_args_list[0].args[0], self.seam)
        self.assertEqual(self.cargo.call_args_list[1].args[0], self.eprfs)


if __name__ == "__main__":
    unittest.main()
