#!/usr/bin/env python3
"""Assemble the checker's declared first-party source closure, without publishing.

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
CRATES = (
    CHECKER,
    "elohim/eprfs/eprfs-meta",
    "elohim/eprfs/eprfs-core",
    "crates/seam-contracts",
)
PREFIX = "rust-style-source/"


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


def projected_workspace(original):
    text = original.decode()
    parsed = tomllib.loads(text)
    required = {"eprfs-meta", "eprfs-core"}
    if not required.issubset(parsed["workspace"]["members"]):
        raise ValueError("expected eprfs members absent")
    # Preserve inheritance declarations verbatim. Omitted members and the iroh-only
    # patch cannot contribute to this closure; no crate source/manifest is patched.
    sections = []
    active = False
    for line in text.splitlines(keepends=True):
        if line.startswith("["):
            active = line.strip() in ("[workspace.package]", "[workspace.dependencies]")
        if active:
            sections.append(line)
    projected = '[workspace]\nresolver = "2"\nmembers = ["eprfs-core", "eprfs-meta"]\n\n' + "".join(sections)
    result = tomllib.loads(projected)
    for table in ("package", "dependencies"):
        if result["workspace"][table] != parsed["workspace"][table]:
            raise ValueError("workspace inheritance changed in projection")
    return projected.encode()


def source_files(root):
    root = root.resolve()
    files = {}
    known = {(root / crate).resolve() for crate in CRATES}
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
        license_id = package.get("license", "Apache-2.0")
        if isinstance(license_id, dict):
            license_id = "Apache-2.0 (inherited from the included eprfs workspace)"
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
    original = read_file(root, "elohim/eprfs/Cargo.toml")
    files["elohim/eprfs/Cargo.toml"] = projected_workspace(original)
    # Cargo resolves registry aliases even on versioned local path dependencies.
    # This checked-in configuration declares endpoints/profiles, never credentials.
    files[".cargo/config.toml"] = read_file(root, ".cargo/config.toml")
    for name in ("LICENSE", "LICENSE.txt", "LICENSE.md", "COPYING"):
        if (root / name).exists():
            files[name] = read_file(root, name)
    files["LICENSE-NOTICES.txt"] = ("Declared source licenses; this list does not relicense the component sources.\n" + "".join(licenses)).encode()
    lock = tomllib.loads(files[f"{CHECKER}/Cargo.lock"].decode())
    names = {tomllib.loads(files[f"{crate}/Cargo.toml"].decode())["package"]["name"] for crate in CRATES}
    for package in lock["package"]:
        if "source" not in package and package["name"] not in names:
            raise ValueError(f"undeclared lockfile path package: {package['name']}")
        if "source" in package and package["source"] != "registry+https://github.com/rust-lang/crates.io-index":
            raise ValueError(f"unheld non-crates.io dependency: {package['name']}")
    derivation = {
        "format": "rust-style-source-distribution@1",
        "purpose": "standalone source build; no registry publication or adoption asserted",
        "workspaceProjection": {
            "sourceCid": cid(original),
            "members": ["eprfs-core", "eprfs-meta"],
            "inheritance": "workspace.package and workspace.dependencies preserved verbatim",
            "omitted": "unrelated members, iroh patch and workspace-only profiles",
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
