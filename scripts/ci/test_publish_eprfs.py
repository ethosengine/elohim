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
from urllib.error import HTTPError

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


if __name__ == "__main__":
    unittest.main()
