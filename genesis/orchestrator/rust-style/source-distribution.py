#!/usr/bin/env python3
"""Assemble the standalone checker's source, without vendoring registry crates.

This is a checker-specific source distribution, not a Cargo registry package.
The byte-addressed archive is a local holding; its CID confers no adoption authority.
"""
import argparse
import base64
import gzip
import hashlib
import io
import json
from pathlib import Path
import tarfile
import tomllib


CHECKER = "genesis/orchestrator/rust-style"
SCAN = f"{CHECKER}/scan"
CRATES = (CHECKER, SCAN)
PREFIX = "rust-style-source/"
CRATES_IO_SOURCE = "registry+https://github.com/rust-lang/crates.io-index"
PUBLISHED_EPRFS = {"eprfs-core": "0.3.3", "eprfs-meta": "0.3.4"}


def cid(data):
    return "b" + base64.b32encode(b"\x01\x55\x12\x20" + hashlib.sha256(data).digest()).decode().lower().rstrip("=")


def read_file(root, relative):
    path = root / relative
    for parent in (path, *path.parents):
        if parent == root:
            break
        if parent.is_symlink():
            raise ValueError(f"symlink source refused: {relative}")
    if not path.is_file() or path.stat().st_size > 10 * 1024 * 1024:
        raise ValueError(f"source is absent, nonregular or oversized: {relative}")
    return path.read_bytes()


def source_files(root):
    root = root.resolve()
    files = {}
    known = {(root / crate).resolve() for crate in CRATES}
    local_packages = set()
    licenses = []
    for crate in CRATES:
        directory = root / crate
        manifest_bytes = read_file(root, f"{crate}/Cargo.toml")
        manifest = tomllib.loads(manifest_bytes.decode())
        if "patch" in manifest:
            raise ValueError(f"undeclared patch closure: {crate}")
        for table in ("dependencies", "build-dependencies", "dev-dependencies"):
            for dependency in manifest.get(table, {}).values():
                if isinstance(dependency, dict) and "path" in dependency:
                    if (directory / dependency["path"]).resolve() not in known:
                        raise ValueError(f"undeclared first-party dependency in {crate}")
        package = manifest["package"]
        local_packages.add(package["name"])
        license_id = package.get("license", "Apache-2.0")
        licenses.append(f"{package['name']}: {license_id} (see its Cargo.toml)\n")
        paths = [directory / "Cargo.toml"]
        for name in ("Cargo.lock", "README.md", "DISTRIBUTION.md", "build.rs", "LICENSE", "LICENSE.txt", "COPYING"):
            if (directory / name).exists():
                paths.append(directory / name)
        for name in ("src", "tests"):
            subtree = directory / name
            if subtree.is_symlink():
                raise ValueError(f"symlink source refused: {subtree}")
            if subtree.exists():
                paths.extend(path for path in subtree.rglob("*") if path.is_file() or path.is_symlink())
        for path in paths:
            relative = path.relative_to(root).as_posix()
            files[relative] = read_file(root, relative)
    checker_manifest = tomllib.loads(files[f"{CHECKER}/Cargo.toml"].decode())
    for name, version in PUBLISHED_EPRFS.items():
        dependency = checker_manifest["dependencies"].get(name)
        if not isinstance(dependency, dict) or dependency.get("version") != f"={version}":
            raise ValueError(f"{name} must use exact published version {version}")
        if dependency.get("registry") != "elohim" or "path" in dependency:
            raise ValueError(f"{name} must resolve only from the elohim registry")
    # This checked-in configuration declares the registry endpoint and profiles, never credentials.
    files[".cargo/config.toml"] = read_file(root, ".cargo/config.toml")
    for name in ("LICENSE", "LICENSE.txt", "LICENSE.md", "COPYING"):
        if (root / name).exists():
            files[name] = read_file(root, name)
    files["LICENSE-NOTICES.txt"] = ("Declared source licenses; this list does not relicense the component sources.\n" + "".join(licenses)).encode()
    lock = tomllib.loads(files[f"{CHECKER}/Cargo.lock"].decode())
    registry = tomllib.loads(files[".cargo/config.toml"].decode())["registries"]["elohim"]["index"]
    elohim_source = registry
    packages = {(package["name"], package["version"]): package for package in lock["package"]}
    for package in lock["package"]:
        if "source" not in package and package["name"] not in local_packages:
            raise ValueError(f"undeclared lockfile path package: {package['name']}")
        if "source" in package:
            if package["source"] not in (CRATES_IO_SOURCE, elohim_source):
                raise ValueError(f"unheld non-registry dependency: {package['name']}")
            if not package.get("checksum"):
                raise ValueError(f"registry dependency lacks checksum: {package['name']}")
    for name, version in PUBLISHED_EPRFS.items():
        package = packages.get((name, version))
        if package is None or package.get("source") != elohim_source:
            raise ValueError(f"lockfile does not pin {name} {version} from elohim")
    seam = [package for package in lock["package"] if package["name"] == "elohim-seam-contracts"]
    if len(seam) != 1 or seam[0].get("source") != elohim_source:
        raise ValueError("lockfile does not pin elohim-seam-contracts from elohim")
    derivation = {
        "format": "rust-style-source-distribution@2",
        "purpose": "standalone checker source build; no registry publication or adoption asserted",
        "registryDependencies": {
            "eprfs-core": "=0.3.3 from elohim, pinned by Cargo.lock checksum",
            "eprfs-meta": "=0.3.4 from elohim, pinned by Cargo.lock checksum",
            "elohim-seam-contracts": "transitive from elohim, pinned by Cargo.lock checksum",
        },
        "files": {path: cid(data) for path, data in sorted(files.items())},
    }
    files["source-distribution.json"] = (json.dumps(derivation, sort_keys=True, separators=(",", ":")) + "\n").encode()
    return files


def archive_bytes(files):
    raw = io.BytesIO()
    with tarfile.open(fileobj=raw, mode="w", format=tarfile.USTAR_FORMAT) as archive:
        for path, data in sorted(files.items()):
            info = tarfile.TarInfo(PREFIX + path)
            info.size, info.mode, info.mtime = len(data), 0o644, 0
            info.uid = info.gid = 0
            info.uname = info.gname = ""
            archive.addfile(info, io.BytesIO(data))
    compressed = io.BytesIO()
    with gzip.GzipFile(filename="", fileobj=compressed, mode="wb", mtime=0, compresslevel=9) as stream:
        stream.write(raw.getvalue())
    return compressed.getvalue()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    data = archive_bytes(source_files(args.root))
    address = cid(data)
    args.out.mkdir(parents=True, exist_ok=True)
    target = args.out / f"{address}.tar.gz"
    try:
        with target.open("xb") as stream:
            stream.write(data)
    except FileExistsError:
        if target.is_symlink() or target.read_bytes() != data:
            raise ValueError("existing addressed archive differs; refusing overwrite")
    print(json.dumps({"cid": address, "sha256": hashlib.sha256(data).hexdigest(), "bytes": len(data), "archive": str(target)}))


if __name__ == "__main__":
    main()
