"""Publish the fixed eprfs library set with verified immutable Nexus receipts.

Publication is dependency ordered but not atomic. A rerun succeeds for an
existing version only when its downloaded payload matches the local package.
"""

from __future__ import annotations

import argparse
from collections import Counter
import gzip
import hashlib
import io
import json
import os
import subprocess
import sys
import tarfile
import time
import tomllib
from pathlib import Path
from urllib.error import HTTPError, URLError
from urllib.parse import urlsplit
from urllib.request import HTTPRedirectHandler, build_opener


CRATES = ("eprfs-core", "eprfs-host", "eprfs-storage", "eprfs-meta", "eprfs-local")
PINNED_INDEX = "https://nexus.ethosengine.com/repository/cargo-internal"
MAX_ARCHIVE = 128 * 1024 * 1024
MAX_UNPACKED = 512 * 1024 * 1024
MAX_INDEX = 8 * 1024 * 1024
MAX_MEMBERS = 100_000


class PublishError(Exception):
    pass


class NoRedirect(HTTPRedirectHandler):
    def redirect_request(self, request, fp, code, message, headers, newurl):
        return None


OPENER = build_opener(NoRedirect)


def sparse_path(name: str) -> str:
    return f"{name[:2]}/{name[2:4]}/{name}"


def registry_base(repo: Path) -> str:
    config = tomllib.loads((repo / ".cargo/config.toml").read_text())
    index = os.environ.get("CARGO_REGISTRIES_ELOHIM_INDEX", config["registries"]["elohim"]["index"])
    if not index.startswith("sparse+https://"):
        raise PublishError("elohim registry must use a sparse HTTPS index")
    parsed = urlsplit(index.removeprefix("sparse+"))
    if not parsed.hostname or parsed.username or parsed.password or parsed.query or parsed.fragment:
        raise PublishError("invalid elohim registry index URL")
    base = index.removeprefix("sparse+").rstrip("/")
    if base != PINNED_INDEX:
        raise PublishError("elohim registry index differs from pinned Nexus endpoint")
    return base


def get(url: str, limit: int, label: str, *, missing_ok: bool = False) -> bytes | None:
    # No redirects: the verified Nexus host alone supplies index and archives.
    try:
        with OPENER.open(url, timeout=20) as response:
            data = response.read(limit + 1)
    except HTTPError as error:
        if missing_ok and error.code == 404:
            return None
        raise PublishError(f"{label}: registry HTTP {error.code}") from None
    except (OSError, URLError) as error:
        raise PublishError(f"{label}: registry read failed ({type(error).__name__})") from None
    if len(data) > limit:
        raise PublishError(f"{label}: registry response exceeds size limit")
    return data


def download_base(index_base: str) -> str:
    data = get(f"{index_base}/config.json", 64 * 1024, "registry config")
    try:
        advertised = json.loads(data)["dl"]
    except (ValueError, KeyError, TypeError):
        raise PublishError("registry download config is invalid") from None
    expected = f"{index_base}/crates"
    if advertised != expected:
        raise PublishError("registry download endpoint differs from expected Nexus endpoint")
    return expected


def index_record(index_base: str, name: str, version: str) -> dict | None:
    data = get(f"{index_base}/{sparse_path(name)}", MAX_INDEX, f"{name} index", missing_ok=True)
    if data is None:
        return None
    try:
        records = [json.loads(line) for line in data.splitlines() if line]
    except (ValueError, UnicodeDecodeError):
        raise PublishError(f"{name} index is malformed") from None
    if any(not isinstance(record, dict) for record in records):
        raise PublishError(f"{name} index is malformed")
    matches = [record for record in records if record.get("vers") == version]
    if len(matches) > 1:
        raise PublishError(f"{name} {version}: duplicate index records")
    if not matches:
        return None
    record = matches[0]
    checksum = record.get("cksum")
    if (
        record.get("name") != name
        or not isinstance(checksum, str)
        or len(checksum) != 64
        or any(char not in "0123456789abcdef" for char in checksum)
        or record.get("yanked", False)
        or not isinstance(record.get("deps"), list)
    ):
        raise PublishError(f"{name} {version}: invalid or yanked index record")
    return record


def archive_contents(data: bytes, name: str, version: str) -> tuple[dict[str, tuple[str, str, int]], bytes]:
    if len(data) > MAX_ARCHIVE:
        raise PublishError(f"{name} {version}: archive exceeds size limit")
    prefix = f"{name}-{version}/"
    files: dict[str, tuple[str, str, int]] = {}
    seen: set[str] = set()
    manifest = b""
    logical_body_bytes = 0
    try:
        # Bound headers, PAX extensions, padding, and file bodies together.
        with gzip.GzipFile(fileobj=io.BytesIO(data)) as compressed:
            tar_bytes = compressed.read(MAX_UNPACKED + 1)
        if len(tar_bytes) > MAX_UNPACKED:
            raise PublishError(f"{name} {version}: archive unpacked size limit")
        with tarfile.open(fileobj=io.BytesIO(tar_bytes), mode="r:") as archive:
            for count, member in enumerate(archive, start=1):
                if count > MAX_MEMBERS:
                    raise PublishError(f"{name} {version}: too many archive entries")
                if member.issparse() or member.sparse is not None:
                    raise PublishError(f"{name} {version}: sparse archive entry")
                if member.type in (tarfile.REGTYPE, tarfile.AREGTYPE):
                    if member.size < 0 or member.size > MAX_UNPACKED - logical_body_bytes:
                        raise PublishError(f"{name} {version}: member size limit")
                    logical_body_bytes += member.size
                elif member.type != tarfile.DIRTYPE:
                    raise PublishError(f"{name} {version}: non-regular archive entry")
                if member.isdir() and member.name.rstrip("/") == f"{name}-{version}":
                    relative = "."
                else:
                    if not member.name.startswith(prefix):
                        raise PublishError(f"{name} {version}: archive entry outside crate root")
                    relative = member.name[len(prefix):]
                    if member.isdir():
                        relative = relative.rstrip("/")
                    if (
                        not relative
                        or relative.startswith("/")
                        or "\\" in relative
                        or any(part in ("", ".", "..") for part in relative.split("/"))
                    ):
                        raise PublishError(f"{name} {version}: unsafe archive path")
                if relative in seen:
                    raise PublishError(f"{name} {version}: duplicate archive path")
                seen.add(relative)
                if member.isdir():
                    files[relative] = ("directory", "", member.mode & 0o7777)
                elif member.isfile():
                    stream = archive.extractfile(member)
                    if stream is None:
                        raise PublishError(f"{name} {version}: unreadable archive entry")
                    contents = stream.read(member.size + 1)
                    if len(contents) != member.size:
                        raise PublishError(f"{name} {version}: truncated archive entry")
                    if relative == "Cargo.toml":
                        manifest = contents
                    if relative != ".cargo_vcs_info.json":
                        files[relative] = ("file", hashlib.sha256(contents).hexdigest(), member.mode & 0o7777)
                else:
                    raise PublishError(f"{name} {version}: non-regular archive entry")
    except (tarfile.TarError, EOFError, OSError):
        raise PublishError(f"{name} {version}: unreadable archive") from None
    if "Cargo.toml" not in files:
        raise PublishError(f"{name} {version}: archive lacks Cargo.toml")
    return files, manifest


def archive_files(data: bytes, name: str, version: str) -> dict[str, tuple[str, str, int]]:
    return archive_contents(data, name, version)[0]


def read_local_archive(path: Path, name: str, version: str) -> bytes:
    with path.open("rb") as stream:
        data = stream.read(MAX_ARCHIVE + 1)
    if len(data) > MAX_ARCHIVE:
        raise PublishError(f"{name} {version}: local archive size limit")
    return data


def normalized_route(spec: dict) -> str:
    registry = spec.get("registry")
    registry_index = spec.get("registry-index")
    if registry == "elohim" or registry_index in (PINNED_INDEX, PINNED_INDEX + "/", "sparse+" + PINNED_INDEX, "sparse+" + PINNED_INDEX + "/"):
        return "elohim"
    if registry is None and registry_index in (None, "https://github.com/rust-lang/crates.io-index", "sparse+https://index.crates.io/"):
        return "crates.io"
    raise PublishError("packaged manifest has unsupported dependency registry route")


def index_route(value: object) -> str:
    if value is None or value in (PINNED_INDEX, PINNED_INDEX + "/", "sparse+" + PINNED_INDEX, "sparse+" + PINNED_INDEX + "/"):
        return "elohim"
    if value in ("https://github.com/rust-lang/crates.io-index", "sparse+https://index.crates.io/"):
        return "crates.io"
    raise PublishError("index has unsupported dependency registry route")


def dependency_rows(table: dict, kind: str, target: str | None) -> list[tuple]:
    rows = []
    for alias, value in table.items():
        spec = {"version": value} if isinstance(value, str) else value
        if not isinstance(spec, dict) or not isinstance(spec.get("version"), str):
            raise PublishError(f"packaged manifest has invalid {kind} dependency")
        rows.append((
            alias,
            spec.get("package", alias),
            spec["version"],
            kind,
            target,
            bool(spec.get("optional", False)),
            bool(spec.get("default-features", True)),
            tuple(sorted(spec.get("features", []))),
            normalized_route(spec),
        ))
    return rows


def validate_index_contract(record: dict, manifest_bytes: bytes, name: str, version: str) -> None:
    try:
        manifest = tomllib.loads(manifest_bytes.decode("utf-8"))
        package = manifest.get("package", {})
        if package.get("name") != name or package.get("version") != version:
            raise PublishError(f"{name} {version}: packaged manifest identity differs from index")
        expected = []
        for kind in ("dependencies", "build-dependencies"):
            expected.extend(dependency_rows(manifest.get(kind, {}), "normal" if kind == "dependencies" else "build", None))
        for target, table in manifest.get("target", {}).items():
            for kind in ("dependencies", "build-dependencies"):
                expected.extend(dependency_rows(table.get(kind, {}), "normal" if kind == "dependencies" else "build", target))
        actual = []
        for dep in record["deps"]:
            if (
                not isinstance(dep, dict)
                or not isinstance(dep.get("name"), str)
                or not dep["name"]
                or not isinstance(dep.get("req"), str)
                or not dep["req"]
                or (dep.get("package") is not None and (not isinstance(dep["package"], str) or not dep["package"]))
                or dep.get("kind") not in (None, "normal", "build", "dev")
                or (dep.get("target") is not None and not isinstance(dep["target"], str))
                or type(dep.get("optional", False)) is not bool
                or type(dep.get("default_features", True)) is not bool
                or not isinstance(dep.get("features"), list)
                or any(not isinstance(feature, str) for feature in dep["features"])
                or (dep.get("registry") is not None and not isinstance(dep["registry"], str))
            ):
                raise PublishError("index dependency is malformed")
            kind = dep.get("kind") or "normal"
            if kind == "dev":
                continue
            req = dep.get("req")
            if not isinstance(req, str):
                raise PublishError("index dependency requirement is malformed")
            actual.append((
                dep.get("name"),
                dep.get("package") or dep.get("name"),
                req,
                kind,
                dep.get("target"),
                dep.get("optional", False),
                dep.get("default_features", True),
                tuple(sorted(dep.get("features", []))),
                index_route(dep.get("registry")),
            ))
        # Cargo canonicalizes a bare version requirement to a caret floor.
        canonical = lambda row: (*row[:2], row[2] if row[2].startswith(("^", "=", "~", ">", "<")) else "^" + row[2], *row[3:])
        if Counter(map(canonical, expected)) != Counter(map(canonical, actual)):
            raise PublishError("index dependency identity, features, or routing differs from packaged manifest")
        explicit = manifest.get("features", {})
        index_features: dict[str, set[str]] = {}
        for field in ("features", "features2"):
            features = record.get(field, {})
            if not isinstance(features, dict):
                raise PublishError("index features are malformed")
            for feature, enabled in features.items():
                if not isinstance(feature, str) or not isinstance(enabled, list) or any(not isinstance(item, str) for item in enabled):
                    raise PublishError("index features are malformed")
                index_features.setdefault(feature, set()).update(enabled)
        optional = {row[0] for row in expected if row[5]}
        for feature, enabled in explicit.items():
            if set(enabled) != index_features.pop(feature, None):
                raise PublishError(f"index feature {feature} differs from packaged manifest")
        for feature, enabled in index_features.items():
            if feature not in optional or enabled not in ({f"dep:{feature}"}, {feature}):
                raise PublishError(f"index feature {feature} is not a packaged implicit optional feature")
    except (UnicodeDecodeError, tomllib.TOMLDecodeError, TypeError, AttributeError):
        raise PublishError(f"{name} {version}: invalid packaged manifest or index contract") from None


def verify_existing(index_base: str, download: str, name: str, version: str, local: bytes) -> bool:
    record = index_record(index_base, name, version)
    if record is None:
        return False
    remote = get(f"{download}/{name}/{version}/download", MAX_ARCHIVE, f"{name} {version} archive")
    if hashlib.sha256(remote).hexdigest() != record["cksum"]:
        raise PublishError(f"{name} {version}: registry archive checksum differs from index")
    local_files, manifest = archive_contents(local, name, version)
    remote_files, _ = archive_contents(remote, name, version)
    if local_files != remote_files:
        raise PublishError(f"{name} {version}: immutable version collision; bump the version")
    validate_index_contract(record, manifest, name, version)
    return True


def cargo(workspace: Path, arguments: list[str], env: dict[str, str]) -> bool:
    result = subprocess.run(["cargo", *arguments], cwd=workspace, env=env, capture_output=True)
    # Do not echo Cargo output: a credential provider may print sensitive data.
    return result.returncode == 0


def run(repo: Path, publish: bool) -> None:
    workspace = repo / "elohim/eprfs"
    manifest = tomllib.loads((workspace / "Cargo.toml").read_text())
    version = manifest["workspace"]["package"]["version"]
    target = os.environ.get("CARGO_TARGET_DIR")
    if not target or not Path(target).is_absolute() or Path(target).resolve().is_relative_to(repo.resolve()):
        raise PublishError("CARGO_TARGET_DIR must be an absolute slot outside the source checkout")
    index_base = registry_base(repo)
    download = download_base(index_base)
    env = os.environ.copy()
    env.update(
        RUSTFLAGS="",
        RUSTC_WRAPPER="",
        CARGO_REGISTRY_GLOBAL_CREDENTIAL_PROVIDERS="cargo:token",
        CARGO_REGISTRIES_ELOHIM_INDEX="sparse+" + index_base + "/",
    )
    print(f"eprfs library target version: {version}", flush=True)
    for name in CRATES:
        package_args = ["package", "--locked", "--registry", "elohim", "--no-verify", "-p", name]
        if not cargo(workspace, package_args, env):
            raise PublishError(f"{name} {version}: locked packaging failed")
        archive = Path(target) / "package" / f"{name}-{version}.crate"
        if not archive.is_file():
            raise PublishError(f"{name} {version}: Cargo package archive is absent")
        local = read_local_archive(archive, name, version)
        archive_files(local, name, version)
        if verify_existing(index_base, download, name, version, local):
            print(f"verified existing: {name} {version}", flush=True)
            continue
        if not publish:
            print(f"absent (check only): {name} {version}", flush=True)
            continue
        if not env.get("CARGO_REGISTRIES_ELOHIM_TOKEN"):
            raise PublishError("cargo registry write credential is missing")
        publish_args = ["publish", "--locked", "--registry", "elohim", "--no-verify", "-p", name]
        uploaded = cargo(workspace, publish_args, env)
        if read_local_archive(archive, name, version) != local:
            raise PublishError(f"{name} {version}: Cargo changed the packaged archive")
        if not uploaded:
            if not verify_existing(index_base, download, name, version, local):
                raise PublishError(f"{name} {version}: upload failed and version is absent")
            print(f"verified existing after failed upload: {name} {version}", flush=True)
            continue
        for attempt in range(12):
            if verify_existing(index_base, download, name, version, local):
                break
            if attempt < 11:
                time.sleep(5)
        else:
            raise PublishError(f"{name} {version}: uploaded version absent from index")
        print(f"verified published: {name} {version}", flush=True)
    print("eprfs publish complete" if publish else "eprfs check complete")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check", action="store_true", help="package and compare without upload")
    mode.add_argument("--publish", action="store_true", help="publish absent versions after checks")
    args = parser.parse_args()
    repo = Path(__file__).resolve().parents[2]
    try:
        run(repo, args.publish)
    except (PublishError, OSError, KeyError, ValueError) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
