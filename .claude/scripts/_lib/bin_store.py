"""Registry-driven content-addressed device store for followed command-line tools.

Only :func:`land` changes the committed head; gates merely ingest candidates. The installed
launcher imports this file with ``python -I``, so its hot path is stdlib-only. This Python
implementation is the local-rung sketch described by the household-follows-latest design; it
retires into the native adoption path when that path owns tooling releases.
"""

from __future__ import annotations

import contextlib
import datetime as dt
import fcntl
import os
import re
import shutil
import sys
import tempfile
import time
from pathlib import Path
from typing import Any, Callable, Iterator, Mapping

DEFAULT_BASE = Path("/projects/.claude-config/tools/bin-store")
DEFAULT_STORE = DEFAULT_BASE / "epr"
DEFAULT_REGISTRY = Path(__file__).resolve().parents[3] / "genesis/agentic/bin-tools.json"
_ACTIVE_REGISTRY: dict[str, Any] | None = None
SESSION_ENV = ("ELOHIM_SESSION_ID", "CLAUDE_CODE_SESSION_ID", "CLAUDE_SESSION_ID")
CID_RE = re.compile(r"sha256:[0-9a-f]{64}\Z")
GRACE = dt.timedelta(days=14)
EXEC_LOCK_TIMEOUT_SECONDS = 5.0
MUTATION_LOCK_TIMEOUT_SECONDS = 60.0


class StoreRefusal(RuntimeError):
    def __init__(self, kind: str, detail: str):
        super().__init__(detail)
        self.kind = kind
        self.detail = detail


def load_registry(path: Path | None = None) -> dict[str, Any]:
    import json

    registry_path = (path or DEFAULT_REGISTRY).expanduser().resolve(strict=False)
    try:
        value = json.loads(registry_path.read_text(encoding="utf-8"))
        tools = value["tools"]
    except (OSError, ValueError, KeyError, TypeError) as error:
        raise StoreRefusal("registry-invalid", f"cannot read {registry_path}: {error}") from error
    if not isinstance(tools, dict):
        raise StoreRefusal("registry-invalid", f"{registry_path}: tools must be an object")
    return value


def tool_config(tool: str, registry: Mapping[str, Any] | None = None) -> dict[str, Any]:
    tools = (registry or _ACTIVE_REGISTRY or load_registry()).get("tools", {})
    config = tools.get(tool) if isinstance(tools, Mapping) else None
    if not isinstance(config, dict):
        raise StoreRefusal("tool-unknown", f"no followed tool named {tool!r}")
    bins = config.get("bins")
    if not isinstance(bins, list) or not bins or not all(isinstance(name, str) for name in bins):
        raise StoreRefusal("registry-invalid", f"tool {tool!r} has no valid bins")
    return config


def invoked_tool(
    invoked: str, registry: Mapping[str, Any] | None = None
) -> tuple[str, str, bool]:
    name = Path(invoked).name
    found: list[tuple[str, str, bool]] = []
    selected = registry or _ACTIVE_REGISTRY or load_registry()
    tools = selected.get("tools", {})
    for tool, raw_config in tools.items():
        if not isinstance(raw_config, Mapping):
            continue
        raw_bins = raw_config.get("bins")
        raw_aliases = raw_config.get("aliases")
        names_binary = isinstance(raw_bins, list) and name in raw_bins
        names_alias = isinstance(raw_aliases, Mapping) and name in raw_aliases
        if not names_binary and not names_alias:
            continue
        config = tool_config(str(tool), selected)
        if names_binary:
            found.append((str(tool), name, False))
        aliases = config.get("aliases", {})
        if names_alias and isinstance(aliases, dict):
            found.append((str(tool), str(aliases[name]), True))
    if len(found) != 1:
        raise StoreRefusal(
            "launcher-name-invalid", f"{name!r} resolves to {len(found)} followed tools"
        )
    return found[0]


def store_root(tool: str = "epr", environ: Mapping[str, str] | None = None) -> Path:
    values = os.environ if environ is None else environ
    declared = values.get(f"{tool.upper()}_STORE", "").strip()
    if declared:
        return Path(declared).expanduser().resolve(strict=False)
    base = Path(values.get("BIN_STORE", DEFAULT_BASE)).expanduser().resolve(strict=False)
    return base / tool


def _root(path: Path | None, tool: str = "epr") -> Path:
    return (path or store_root(tool)).expanduser().resolve(strict=False)


def _utcnow() -> dt.datetime:
    return dt.datetime.now(dt.timezone.utc)


def now() -> str:
    return _utcnow().isoformat().replace("+00:00", "Z")


def _parse_time(value: Any) -> dt.datetime | None:
    if not isinstance(value, str):
        return None
    try:
        parsed = dt.datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        return None
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=dt.timezone.utc)
    return parsed.astimezone(dt.timezone.utc)


def hash_file(path: Path) -> str:
    import hashlib

    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return f"sha256:{digest.hexdigest()}"


def hash_bytes(value: bytes) -> str:
    import hashlib

    return f"sha256:{hashlib.sha256(value).hexdigest()}"


def canonical_sums(checksums: Mapping[str, str]) -> str:
    return "".join(f"{checksums[name].removeprefix('sha256:')}  {name}\n" for name in sorted(checksums))


def _require_cid(cid: str) -> str:
    if not CID_RE.fullmatch(cid):
        raise StoreRefusal("invalid-cid", f"not an epr sha256 CID: {cid!r}")
    return cid


def _artifact(root: Path, cid: str) -> Path:
    return root / _require_cid(cid)


@contextlib.contextmanager
def store_lock(root: Path, *, timeout: float = MUTATION_LOCK_TIMEOUT_SECONDS) -> Iterator[None]:
    """Serialize every store mutation, including pin publication and garbage collection."""
    root.mkdir(parents=True, exist_ok=True)
    with (root / ".lock").open("a+b") as lock:
        deadline = time.monotonic() + timeout
        while True:
            try:
                fcntl.flock(lock.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
                break
            except BlockingIOError:
                if time.monotonic() >= deadline:
                    raise StoreRefusal(
                        "lock-timeout", f"epr store lock remained busy for {timeout:g}s"
                    ) from None
                time.sleep(0.05)
        try:
            yield
        finally:
            fcntl.flock(lock.fileno(), fcntl.LOCK_UN)


def _atomic_json(path: Path, value: Any) -> None:
    import json

    path.parent.mkdir(parents=True, exist_ok=True)
    fd, name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as target:
            json.dump(value, target, sort_keys=True, separators=(",", ":"))
            target.write("\n")
            target.flush()
            os.fsync(target.fileno())
        os.replace(name, path)
    finally:
        try:
            os.unlink(name)
        except FileNotFoundError:
            pass


def _atomic_copy(source: Path, destination: Path, mode: int) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    fd, name = tempfile.mkstemp(prefix=f".{destination.name}.", dir=destination.parent)
    temporary = Path(name)
    try:
        with source.open("rb") as reader, os.fdopen(fd, "wb") as target:
            shutil.copyfileobj(reader, target)
            target.flush()
            os.fsync(target.fileno())
        temporary.chmod(mode)
        os.replace(temporary, destination)
    finally:
        temporary.unlink(missing_ok=True)


def _atomic_link(root: Path, name: str, cid: str) -> None:
    _require_cid(cid)
    root.mkdir(parents=True, exist_ok=True)
    temporary = root / f".{name}.{os.getpid()}.{time.time_ns()}"
    os.symlink(cid, temporary)
    try:
        os.replace(temporary, root / name)
    finally:
        temporary.unlink(missing_ok=True)


def _read_link(root: Path, name: str) -> str | None:
    try:
        target = os.readlink(root / name)
    except OSError:
        return None
    cid = Path(target).name
    return cid if CID_RE.fullmatch(cid) else None


def _read_json(path: Path) -> dict[str, Any] | None:
    import json

    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return None
    return value if isinstance(value, dict) else None


# The workspace checkout is owned by another uid than the one hooks run as, so git refuses it as
# "dubious ownership". The store only reads this device's own repo; trust it for these calls.
_GIT_TRUST = ("-c", "safe.directory=*")


def _git(repo: Path, *args: str) -> str:
    import subprocess

    result = subprocess.run(
        ["git", *_GIT_TRUST, *args], cwd=repo, capture_output=True, text=True, timeout=15,
        check=False
    )
    if result.returncode != 0:
        detail = (result.stderr or result.stdout).strip().splitlines()
        raise StoreRefusal("git-unavailable", detail[-1] if detail else "git did not answer")
    return result.stdout.strip()


def _git_succeeds(repo: Path, *args: str) -> bool:
    import subprocess

    try:
        result = subprocess.run(
            ["git", *_GIT_TRUST, *args], cwd=repo, capture_output=True, text=True, timeout=15,
        check=False
        )
    except (OSError, subprocess.SubprocessError) as error:
        raise StoreRefusal("git-unavailable", str(error)) from error
    return result.returncode == 0


def repo_root(path: Path | None = None) -> Path:
    configured = os.environ.get("EPR_REPO_ROOT")
    start = Path(configured) if configured else (path or Path.cwd())
    return Path(_git(start, "rev-parse", "--show-toplevel"))


def pinned_sha(repo: Path, tool: str) -> str:
    source = tool_config(tool).get("source", {})
    path = source.get("path") if isinstance(source, dict) else None
    if not isinstance(path, str):
        return _git(repo, "rev-parse", "HEAD")
    line = _git(repo, "ls-tree", "HEAD", "--", path).splitlines()
    match = re.match(r"160000 commit ([0-9a-f]{40})\t", line[0] if line else "")
    if not match:
        raise StoreRefusal("gitlink-invalid", f"{path} is not a gitlink at HEAD")
    return match.group(1)


def brit_git_dir(repo: Path) -> Path:
    configured = os.environ.get("BRIT_GIT_DIR", "").strip()
    if configured:
        return Path(configured).expanduser().resolve(strict=False)
    common = Path(_git(repo, "rev-parse", "--git-common-dir"))
    if not common.is_absolute():
        common = (repo / common).resolve()
    return common / "modules/elohim/brit"


def _brit_git(repo: Path, *args: str, timeout: int = 30) -> "subprocess.CompletedProcess[str]":
    import subprocess

    return subprocess.run(
        ["git", *_GIT_TRUST, "-C", str(brit_git_dir(repo)), *args],
        capture_output=True, text=True, timeout=timeout, check=False,
        env={key: value for key, value in os.environ.items()
             if key not in {"GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE", "GIT_PREFIX"}},
    )


def brit_descends(repo: Path, ancestor: str, descendant: str) -> bool:
    """Check ancestry in brit's object store, fetching only when one of the pins is absent."""
    missing = [
        sha for sha in (ancestor, descendant)
        if _brit_git(repo, "cat-file", "-e", f"{sha}^{{commit}}").returncode != 0
    ]
    if missing:
        fetched = _brit_git(repo, "fetch", "origin", *missing, timeout=120)
        if fetched.returncode != 0:
            detail = (fetched.stderr or fetched.stdout).strip().splitlines()
            raise StoreRefusal(
                "brit-fetch-failed", detail[-1] if detail else "could not fetch missing brit pins"
            )
    return _brit_git(repo, "merge-base", "--is-ancestor", ancestor, descendant).returncode == 0


EPR_WORKSPACE_MANIFEST = "elohim/eprfs/Cargo.toml"


def source_paths(repo: Path) -> list[str]:
    """The repo-relative dirs whose bytes build epr: every path package in the eprfs graph.

    The main checkout is never wholly clean (hooks append to `.claude/data` ledgers), so
    "clean" must mean epr's own inputs. When cargo cannot answer, the whole tree is the
    scope: a refusal is safer than certifying bytes nobody looked at.
    """
    import json
    import subprocess

    try:
        result = subprocess.run(
            ["cargo", "metadata", "--format-version", "1", "--offline",
             "--manifest-path", str(repo / EPR_WORKSPACE_MANIFEST)],
            capture_output=True, text=True, timeout=60, check=False,
            env={**os.environ, "RUSTFLAGS": ""},
        )
        packages = json.loads(result.stdout)["packages"] if result.returncode == 0 else None
    except (OSError, subprocess.SubprocessError, ValueError, KeyError):
        packages = None
    if not packages:
        return ["."]
    resolved = repo.resolve()
    paths = {"elohim/eprfs"}
    for package in packages:
        if package.get("source") is not None:
            continue
        directory = Path(package["manifest_path"]).resolve().parent
        try:
            paths.add(directory.relative_to(resolved).as_posix())
        except ValueError:
            return ["."]
    return sorted(paths)


def git_state(repo: Path) -> tuple[str, bool]:
    return (
        _git(repo, "rev-parse", "HEAD"),
        not _git(repo, "status", "--porcelain", "--untracked-files=normal", "--",
                 *source_paths(repo)),
    )


def binary_identity(binary: Path) -> dict[str, Any]:
    import json
    import subprocess

    try:
        result = subprocess.run(
            [str(binary), "--version"], capture_output=True, text=True, timeout=15, check=False
        )
    except (OSError, subprocess.SubprocessError) as error:
        raise StoreRefusal("identity-failed", f"{binary}: {error}") from error
    if result.returncode != 0:
        detail = (result.stderr or result.stdout).strip().splitlines()
        raise StoreRefusal(
            "identity-failed",
            detail[-1] if detail else f"{binary} --version exited {result.returncode}",
        )
    try:
        identity = json.loads(result.stdout)
    except ValueError as error:
        raise StoreRefusal("identity-invalid", "epr --version did not return JSON") from error
    if not isinstance(identity, dict):
        raise StoreRefusal("identity-invalid", "epr --version returned non-object JSON")
    return identity


def _release_sources(tool: str, source: Path) -> dict[str, Path]:
    names = list(tool_config(tool)["bins"])
    source = source.expanduser().resolve()
    if len(names) == 1 and source.is_file():
        paths = {names[0]: source}
    else:
        paths = {name: source / name for name in names}
    for name, path in paths.items():
        if not path.is_file() or not os.access(path, os.X_OK):
            raise StoreRefusal("binary-unusable", f"{tool} release has no executable {name}: {path}")
    return paths


def _release_checksums(tool: str, source: Path, paths: Mapping[str, Path]) -> dict[str, str]:
    checksums = {name: hash_file(path) for name, path in paths.items()}
    if len(paths) == 1:
        return checksums
    sums_path = source.expanduser().resolve() / "SHA256SUMS"
    try:
        declared = sums_path.read_text(encoding="utf-8")
    except OSError as error:
        raise StoreRefusal("checksums-absent", f"{tool} release has no SHA256SUMS: {error}") from error
    expected = canonical_sums(checksums)
    if declared != expected:
        raise StoreRefusal("checksums-mismatch", "SHA256SUMS is not the canonical checksum set")
    return checksums


def _release_identity(
    tool: str, paths: Mapping[str, Path], checksums: Mapping[str, str], source_commit: str
) -> tuple[str, str | None]:
    if len(paths) == 1:
        cid = next(iter(checksums.values()))
    else:
        cid = hash_bytes(canonical_sums(checksums).encode())
    if tool == "epr":
        identity = binary_identity(paths["epr"])
        if identity.get("cid") != cid:
            raise StoreRefusal(
                "identity-mismatch", f"binary reports {identity.get('cid')!r}, bytes are {cid}"
            )
        return cid, identity.get("version")
    if tool == "brit":
        import subprocess

        result = subprocess.run(
            [str(paths["brit"]), "--version"], capture_output=True, text=True,
            timeout=15, check=False,
        )
        output = result.stdout.strip()
        if result.returncode != 0:
            raise StoreRefusal("identity-failed", f"brit --version exited {result.returncode}")
        if re.fullmatch(r"[0-9a-f]{40}", source_commit) and f"source HEAD: {source_commit}" not in output:
            raise StoreRefusal(
                "source-mismatch", f"brit --version does not report source HEAD: {source_commit}"
            )
        return cid, output.splitlines()[0] if output else None
    raise StoreRefusal("tool-unknown", tool)


def ingest(
    binary: Path,
    *,
    tool: str = "epr",
    root: Path | None = None,
    repo: Path | None = None,
    source_commit: str | None = None,
    gate_run: str | None = None,
    tree_clean: bool | None = None,
) -> dict[str, Any]:
    """Copy privately, then hash and identify exactly the bytes that will be published."""
    root = _root(root, tool)
    if source_commit is None or tree_clean is None:
        checkout = repo_root(repo)
        if tool == "epr":
            observed_commit, observed_clean = git_state(checkout)
        else:
            observed_commit, observed_clean = pinned_sha(checkout, tool), True
        source_commit = source_commit or observed_commit
        if tree_clean is None:
            tree_clean = observed_clean
    assert source_commit is not None
    paths = _release_sources(tool, binary)

    with store_lock(root):
        staging_parent = root / ".staging"
        staging_parent.mkdir(parents=True, exist_ok=True)
        staged = Path(tempfile.mkdtemp(prefix=f"{tool}.", dir=staging_parent))
        try:
            for name, path in paths.items():
                shutil.copyfile(path, staged / name)
                (staged / name).chmod(0o555)
            if len(paths) > 1:
                try:
                    shutil.copyfile(
                        binary.expanduser().resolve() / "SHA256SUMS", staged / "SHA256SUMS"
                    )
                except OSError as error:
                    raise StoreRefusal(
                        "checksums-absent", f"{tool} release has no SHA256SUMS: {error}"
                    ) from error
            staged_paths = _release_sources(tool, staged if len(paths) > 1 else staged / next(iter(paths)))
            checksums = _release_checksums(tool, staged, staged_paths)
            cid, version = _release_identity(tool, staged_paths, checksums, source_commit)
            (staged / "SHA256SUMS").unlink(missing_ok=True)
            artifact = _artifact(root, cid)
            if artifact.exists():
                for name, checksum in checksums.items():
                    destination = artifact / name
                    try:
                        valid = (
                            destination.is_file()
                            and os.access(destination, os.X_OK)
                            and hash_file(destination) == checksum
                        )
                    except OSError:
                        valid = False
                    if not valid:
                        _atomic_copy(staged_paths[name], destination, 0o555)
                shutil.rmtree(staged)
            else:
                os.rename(staged, artifact)
            existing = _read_json(artifact / "record.json") or {}
            ingested_at = now()
            source = {
                "source_commit": source_commit,
                "tree_clean": bool(tree_clean),
                "gate_run": gate_run,
                "ingested_at": ingested_at,
            }
            record = dict(existing)
            record.setdefault("cid", cid)
            record.setdefault("rcid", cid)
            record.setdefault("tool", tool)
            record.setdefault("bins", checksums)
            record.setdefault("version", version)
            record.setdefault("source_commit", source_commit)
            record.setdefault("tree_clean", bool(tree_clean))
            record.setdefault("gate_run", gate_run)
            record["ingested_at"] = ingested_at
            record["ring"] = "landed" if existing.get("ring") == "landed" else "dev-context"
            sources = record.get("sources")
            if not isinstance(sources, list):
                sources = []
                if existing.get("source_commit") is not None:
                    sources.append({
                        "source_commit": existing.get("source_commit"),
                        "tree_clean": existing.get("tree_clean") is True,
                        "gate_run": existing.get("gate_run"),
                        "ingested_at": existing.get("ingested_at"),
                    })
            sources.append(source)
            record["sources"] = sources
            _atomic_json(artifact / "record.json", record)
            return record
        finally:
            if staged.exists():
                shutil.rmtree(staged)


def _append_refusal_locked(root: Path, cid: str, kind: str, detail: str) -> None:
    record_path = _artifact(root, cid) / "record.json"
    record = _read_json(record_path) or {"cid": cid}
    refusals = record.setdefault("refusals", [])
    if not isinstance(refusals, list):
        refusals = record["refusals"] = []
    refusals.append({"kind": kind, "detail": detail, "at": now()})
    _atomic_json(record_path, record)


def _health_check(tool: str, root: Path, cid: str, repo: Path) -> None:
    import subprocess

    binary = _artifact(root, cid) / tool_config(tool)["bins"][0]
    if tool == "brit":
        record = _read_json(_artifact(root, cid) / "record.json") or {}
        _release_identity(
            tool,
            {name: _artifact(root, cid) / name for name in tool_config(tool)["bins"]},
            record.get("bins", {}),
            str(record.get("source_commit", "")),
        )
        return
    identity = binary_identity(binary)
    if identity.get("cid") != cid:
        raise StoreRefusal(
            "health-identity-mismatch", f"candidate reports {identity.get('cid')!r}, expected {cid}"
        )
    try:
        result = subprocess.run(
            [str(binary), "flow", "report", "scope"], cwd=repo,
            capture_output=True, text=True, timeout=60, check=False,
        )
    except (OSError, subprocess.SubprocessError) as error:
        raise StoreRefusal("health-flow-failed", str(error)) from error
    if result.returncode != 0:
        detail = (result.stderr or result.stdout).strip().splitlines()
        raise StoreRefusal(
            "health-flow-failed",
            detail[-1] if detail else f"flow report scope exited {result.returncode}",
        )


def _verified_artifact(root: Path, cid: str, tool: str = "epr") -> bool:
    try:
        artifact = _artifact(root, cid)
        record = _read_json(artifact / "record.json") or {}
        checksums = record.get("bins")
        if not isinstance(checksums, dict):
            checksums = {"epr": cid} if tool == "epr" else {}
        names = tool_config(tool)["bins"]
        if set(checksums) != set(names):
            return False
        for name in names:
            binary = artifact / name
            if (not binary.is_file() or not os.access(binary, os.X_OK)
                    or hash_file(binary) != checksums[name]):
                return False
        calculated = next(iter(checksums.values())) if len(names) == 1 else hash_bytes(
            canonical_sums(checksums).encode()
        )
        return calculated == cid
    except (OSError, StoreRefusal):
        return False


def _usable_artifact(root: Path, cid: str, tool: str = "epr", bin_name: str | None = None) -> bool:
    """Exec-path check: landed artifacts are immutable, so do not hash them again here."""
    try:
        binary = _artifact(root, cid) / (bin_name or tool_config(tool)["bins"][0])
        return binary.is_file() and os.access(binary, os.X_OK)
    except (OSError, StoreRefusal):
        return False


def _source_for_commit(record: dict[str, Any], commit: str) -> dict[str, Any] | None:
    sources = record.get("sources")
    if isinstance(sources, list):
        for source in reversed(sources):
            if isinstance(source, dict) and source.get("source_commit") == commit:
                return source
    if record.get("source_commit") == commit:
        return record
    return None


def _current_state(root: Path) -> tuple[str | None, dict[str, Any] | None]:
    return _read_link(root, "current"), _read_json(root / "current.json")


def _validate_land_locked(
    root: Path, cid: str, commit: str, checkout: Path, revert: bool, tool: str = "epr"
) -> tuple[dict[str, Any], str | None, dict[str, Any] | None]:
    artifact = _artifact(root, cid)
    record = _read_json(artifact / "record.json")
    if record is None or not _verified_artifact(root, cid, tool):
        raise StoreRefusal("candidate-absent", f"{cid} is not a verified store candidate")
    source = _source_for_commit(record, commit)
    if source is None:
        raise StoreRefusal(
            "commit-mismatch", f"candidate has no ingest provenance for commit {commit!r}"
        )
    if source.get("tree_clean") is not True:
        raise StoreRefusal("dirty-build", f"{cid} was ingested from a dirty tree at {commit}")
    head, clean = git_state(checkout) if tool == "epr" else (pinned_sha(checkout, tool), True)
    if head != commit:
        raise StoreRefusal("head-mismatch", f"checkout HEAD {head} does not match {commit}")
    if not clean:
        raise StoreRefusal("checkout-dirty", f"checkout is dirty: {checkout}")
    old_current, current = _current_state(root)
    if old_current:
        if current is None or current.get("cid") != old_current:
            raise StoreRefusal(
                "current-metadata-invalid", "current.json does not describe the current artifact"
            )
        if old_current != cid and not revert:
            ancestor = current.get("commit")
            descends = (
                isinstance(ancestor, str)
                and (
                    _git_succeeds(checkout, "merge-base", "--is-ancestor", ancestor, commit)
                    if tool == "epr"
                    else brit_descends(checkout, ancestor, commit)
                )
            )
            if not descends:
                raise StoreRefusal(
                    "non-descendant",
                    f"{commit} does not descend from landed head commit {ancestor!r}",
                )
    return record, old_current, current


def _record_refusal(root: Path, cid: str, refusal: StoreRefusal) -> None:
    if refusal.kind == "lock-timeout" or not CID_RE.fullmatch(cid):
        return
    try:
        with store_lock(root):
            _append_refusal_locked(root, cid, refusal.kind, refusal.detail)
    except StoreRefusal:
        pass


def land(
    cid: str, commit: str, *, root: Path | None = None, repo: Path | None = None,
    revert: bool = False, tool: str = "epr",
    attestation_judge: Callable[[Path, str], Mapping[str, Any]] | None = None,
) -> dict[str, Any]:
    root = _root(root, tool)
    checkout = repo_root(repo)
    try:
        with store_lock(root):
            record, expected_current, expected_metadata = _validate_land_locked(
                root, cid, commit, checkout, revert, tool
            )
            _touch_used(root, cid)
        if tool == "brit":
            _require_brit_attestation(checkout, commit, attestation_judge)
        _health_check(tool, root, cid, checkout)
        with store_lock(root):
            record, old_current, current_metadata = _validate_land_locked(
                root, cid, commit, checkout, revert, tool
            )
            if (old_current, current_metadata) != (expected_current, expected_metadata):
                raise StoreRefusal(
                    "current-moved", "the committed head moved while candidate health was checked"
                )
            landed_at = now()
            if old_current and old_current != cid:
                _atomic_link(root, "previous", old_current)
            _atomic_link(root, "current", cid)
            _atomic_json(root / "current.json", {
                "cid": cid,
                "commit": commit,
                "landed_at": landed_at,
                "previous": old_current,
            })
            record_path = _artifact(root, cid) / "record.json"
            record["ring"] = "reverted" if revert else "landed"
            record["landed_at"] = landed_at
            _atomic_json(record_path, record)
            return record
    except StoreRefusal as refusal:
        _record_refusal(root, cid, refusal)
        raise


def _default_proc_root(proc_root: Path | None) -> Path:
    """The process table to walk; ``EPR_PROC_ROOT`` lets tests run under a real Claude ancestor."""
    return proc_root if proc_root is not None else Path(os.environ.get("EPR_PROC_ROOT", "/proc"))


def _proc_fields(proc_root: Path, pid: int) -> tuple[int, str] | None:
    try:
        stat = (proc_root / str(pid) / "stat").read_text(encoding="utf-8")
        close = stat.rfind(")")
        fields = stat[close + 2:].split()
        return int(fields[1]), fields[19]
    except (OSError, ValueError, IndexError):
        return None


def session_identity(
    *, proc_root: Path | None = None, start_pid: int | None = None,
    environ: Mapping[str, str] | None = None,
) -> dict[str, Any] | None:
    """Find the nearest ``claude`` ancestor and bind its pid to proc starttime."""
    proc_root = _default_proc_root(proc_root)
    pid = os.getppid() if start_pid is None else start_pid
    seen: set[int] = set()
    while pid > 0 and pid not in seen:
        seen.add(pid)
        fields = _proc_fields(proc_root, pid)
        if fields is None:
            break
        ppid, starttime = fields
        try:
            comm = (proc_root / str(pid) / "comm").read_text(encoding="utf-8").strip()
        except OSError:
            comm = ""
        if comm == "claude":
            return {
                "session": f"claude-{pid}-{starttime}",
                "owner_pid": pid,
                "owner_starttime": starttime,
            }
        pid = ppid
    values = os.environ if environ is None else environ
    for name in SESSION_ENV:
        value = values.get(name, "").strip()
        if value:
            return {"session": value}
    return None


def session_id(
    *, proc_root: Path | None = None, start_pid: int | None = None,
    environ: Mapping[str, str] | None = None,
) -> str | None:
    identity = session_identity(proc_root=proc_root, start_pid=start_pid, environ=environ)
    return str(identity["session"]) if identity else None


def _session_name(value: str) -> str:
    safe = re.sub(r"[^A-Za-z0-9_.-]", "_", value)[:160]
    if not safe:
        raise StoreRefusal("session-invalid", "session id has no usable characters")
    return safe


def _pin_path(root: Path, session: str) -> Path:
    return root / "sessions" / _session_name(session)


def _valid_pin(record: dict[str, Any] | None) -> bool:
    return bool(record and isinstance(record.get("cid"), str) and CID_RE.fullmatch(record["cid"]))


def _publish_pin(path: Path, record: dict[str, Any]) -> None:
    import json

    path.parent.mkdir(parents=True, exist_ok=True)
    fd, name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    temporary = Path(name)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as target:
            json.dump(record, target, sort_keys=True, separators=(",", ":"))
            target.write("\n")
            target.flush()
            os.fsync(target.fileno())
        os.link(temporary, path)
    finally:
        temporary.unlink(missing_ok=True)


def pin(
    *, root: Path | None = None, cid: str | None = None,
    proc_root: Path | None = None, start_pid: int | None = None,
    environ: Mapping[str, str] | None = None,
    lock_timeout: float = MUTATION_LOCK_TIMEOUT_SECONDS,
    tool: str = "epr",
) -> dict[str, Any]:
    root = _root(root, tool)
    identity = session_identity(proc_root=proc_root, start_pid=start_pid, environ=environ)
    if identity is None:
        raise StoreRefusal("session-absent", "no claude ancestor or session environment is present")
    path = _pin_path(root, str(identity["session"]))
    existing = _read_json(path)
    if _valid_pin(existing):
        return existing
    with store_lock(root, timeout=lock_timeout):
        existing = _read_json(path)
        if _valid_pin(existing):
            return existing
        path.unlink(missing_ok=True)
        selected = _require_cid(cid) if cid else _read_link(root, "current")
        if selected is None or not _usable_artifact(root, selected, tool):
            raise StoreRefusal("head-absent", "the store has no executable committed current head")
        record = {"cid": selected, "pinned_at": now(), **identity}
        try:
            _publish_pin(path, record)
        except FileExistsError:
            raced = _read_json(path)
            if _valid_pin(raced):
                return raced
            raise StoreRefusal("pin-unreadable", f"racing pin is incomplete: {path}")
        return record


def _launcher_aliases(root: Path, tool: str = "epr") -> set[str]:
    candidates = {root.parent / "bin" / tool}
    return {os.path.realpath(candidate) for candidate in candidates}


# Build outputs the launcher supersedes: a PATH link into one of these is drift, never a choice.
_SUPERSEDED_ROOTS = ("/projects/.cargo-target-pool/", "/tmp/eprfs-gate-target")


def shadows(
    *, tool: str = "epr", root: Path | None = None, path_env: str | None = None
) -> list[dict[str, Any]]:
    """Every registered tool name on PATH that a caller reaches before its launcher.

    The launcher only works if it is the one PATH answers with: a stale link ahead of it (a
    ``~/.local/bin/epr`` into an old pool build) silently brings back every drift it retires.
    ``superseded`` marks a link into a build output, safe to repoint; anything else is a
    binary someone put there, which is reported and never touched.
    """
    root = _root(root, tool)
    aliases = _launcher_aliases(root, tool)
    found: list[dict[str, Any]] = []
    config = tool_config(tool)
    names = [*config["bins"], *config.get("aliases", {}).keys()]
    for name in names:
        before: list[dict[str, Any]] = []
        for directory in (path_env if path_env is not None else os.environ.get("PATH", "")).split(":"):
            if not directory:
                continue
            candidate = Path(directory) / name
            if not (candidate.is_symlink() or candidate.exists()):
                continue
            if os.path.realpath(candidate) in aliases:
                found.extend(before)
                break
            target = os.readlink(candidate) if candidate.is_symlink() else None
            dangling = target is not None and not candidate.exists()
            superseded = target is not None and (
                dangling or os.path.realpath(candidate).startswith(_SUPERSEDED_ROOTS)
                or os.path.realpath(candidate).startswith(str(root) + "/")
            )
            before.append({
                "name": name, "path": str(candidate), "target": target,
                "superseded": superseded,
            })
    # No launcher on PATH at all: nothing is shadowed — whatever PATH reaches simply runs, and
    # `status` already says whether a head exists. Shadowing needs a launcher behind it.
    return found


def _candidate_override(
    root: Path, value: str, tool: str = "epr", bin_name: str = "epr"
) -> tuple[Path, str, str]:
    candidate = Path(value).expanduser()
    if not candidate.is_file() or not os.access(candidate, os.X_OK):
        raise StoreRefusal("binary-unusable", f"candidate for {tool}/{bin_name} is not executable: {candidate}")
    real = os.path.realpath(candidate)
    if real in _launcher_aliases(root, tool):
        raise StoreRefusal("launcher-alias", f"candidate resolves to the {tool} launcher: {candidate}")
    return Path(real), "candidate", hash_file(Path(real))


def _candidate_exec_override(
    root: Path, value: str, tool: str = "epr", bin_name: str = "epr"
) -> tuple[Path, str, None]:
    candidate = Path(value).expanduser().resolve(strict=False)
    if not candidate.is_file() or not os.access(candidate, os.X_OK):
        raise StoreRefusal("binary-unusable", f"candidate for {tool}/{bin_name} is not executable: {candidate}")
    if os.path.realpath(candidate) in _launcher_aliases(root, tool):
        raise StoreRefusal("launcher-alias", f"candidate resolves to the {tool} launcher: {candidate}")
    return candidate, "candidate", None


def resolve_current(
    *, tool: str = "epr", bin_name: str | None = None, root: Path | None = None
) -> tuple[Path, str, str]:
    root = _root(root, tool)
    selected_bin = bin_name or tool_config(tool)["bins"][0]
    cid = _read_link(root, "current")
    if cid is None or not _usable_artifact(root, cid, tool, selected_bin):
        raise StoreRefusal("head-absent", "the store has no executable committed current head")
    return _artifact(root, cid) / selected_bin, "current", cid


def resolve(
    *, root: Path | None = None, proc_root: Path | None = None, start_pid: int | None = None,
    environ: Mapping[str, str] | None = None,
    pin_lock_timeout: float = MUTATION_LOCK_TIMEOUT_SECONDS,
    warn: Any = None,
    tool: str = "epr", bin_name: str | None = None, session: bool = True,
) -> tuple[Path, str, str]:
    root = _root(root, tool)
    selected_bin = bin_name or tool_config(tool)["bins"][0]
    values = os.environ if environ is None else environ
    candidate_env = tool_config(tool).get("candidateEnv", {}).get(selected_bin, [])
    override = next((values.get(name, "").strip() for name in candidate_env
                     if values.get(name, "").strip()), "")
    if override:
        return _candidate_override(root, override, tool, selected_bin)
    explicit = values.get(f"{tool.upper()}_PIN_CID", "").strip()
    if explicit:
        cid = _require_cid(explicit)
        if not _usable_artifact(root, cid, tool, selected_bin):
            raise StoreRefusal("candidate-absent", f"explicit pin {cid} is not executable")
        return _artifact(root, cid) / selected_bin, "explicit-pin", cid
    identity = (session_identity(proc_root=proc_root, start_pid=start_pid, environ=values)
                if session else None)
    if identity:
        record = _read_json(_pin_path(root, str(identity["session"])))
        if not _valid_pin(record):
            try:
                record = pin(
                    root=root, proc_root=proc_root, start_pid=start_pid, environ=values,
                    lock_timeout=pin_lock_timeout, tool=tool,
                )
            except StoreRefusal as refusal:
                if refusal.kind != "lock-timeout":
                    raise
                if pin_lock_timeout != EXEC_LOCK_TIMEOUT_SECONDS:
                    raise
                raced = _read_json(_pin_path(root, str(identity["session"])))
                if _valid_pin(raced):
                    record = raced
                else:
                    executable, _, cid = resolve_current(root=root, tool=tool, bin_name=selected_bin)
                    if warn is not None:
                        warn(
                            f"{tool} store lock timed out after 5s; using current without "
                            "publishing a session pin"
                        )
                    return executable, "current-lock-timeout", cid
                if warn is not None:
                    warn(f"{tool} store lock timed out after 5s; using the existing session pin")
        cid = _require_cid(str(record.get("cid", "")))
        if not _usable_artifact(root, cid, tool, selected_bin):
            raise StoreRefusal("candidate-absent", f"session pin {cid} is not executable")
        return _artifact(root, cid) / selected_bin, "session-pin", cid
    return resolve_current(root=root, tool=tool, bin_name=selected_bin)


def resolve_exec(
    *, root: Path | None = None, proc_root: Path | None = None, start_pid: int | None = None,
    environ: Mapping[str, str] | None = None, warn: Any = None,
    tool: str = "epr", bin_name: str | None = None, session: bool = True,
) -> tuple[Path, str, str | None]:
    """Resolve for ``exec`` without hashing any binary on the launch path."""
    root = _root(root, tool)
    selected_bin = bin_name or tool_config(tool)["bins"][0]
    values = os.environ if environ is None else environ
    candidate_env = tool_config(tool).get("candidateEnv", {}).get(selected_bin, [])
    override = next((values.get(name, "").strip() for name in candidate_env
                     if values.get(name, "").strip()), "")
    if override:
        return _candidate_exec_override(root, override, tool, selected_bin)
    return resolve(
        root=root, proc_root=proc_root, start_pid=start_pid, environ=values,
        pin_lock_timeout=EXEC_LOCK_TIMEOUT_SECONDS, warn=warn, tool=tool,
        bin_name=selected_bin, session=session,
    )


def long_running(tool: str, args: list[str], registry: Mapping[str, Any] | None = None) -> bool:
    """Whether these arguments start a process that outlives the session that launched it.

    Such a process follows the device head and switches at its own restart; it must never carry
    the launching session's pin, which goes stale when that session ends.
    """
    prefixes = tool_config(tool, registry).get("longRunning", [])
    return any(isinstance(p, list) and p and args[:len(p)] == p for p in prefixes)


def daemon_lease(
    root: Path, cid: str, *, pid: int | None = None, proc_root: Path | None = None,
) -> dict[str, Any] | None:
    """Pin ``cid`` to the daemon process itself, so gc keeps it exactly as long as it runs.

    ``exec`` keeps the pid, so the launcher's own pid and start time are the daemon's. Best
    effort: the 14-day `.used` grace still covers a daemon whose lease could not be written.
    """
    pid = os.getpid() if pid is None else pid
    fields = _proc_fields(_default_proc_root(proc_root), pid)
    if fields is None:
        return None
    key = f"daemon-{pid}-{fields[1]}"
    record = {"cid": cid, "pinned_at": now(), "session": key,
              "owner_pid": pid, "owner_starttime": fields[1]}
    try:
        with store_lock(root, timeout=EXEC_LOCK_TIMEOUT_SECONDS):
            path = _pin_path(root, key)
            path.unlink(missing_ok=True)
            _publish_pin(path, record)
    except (StoreRefusal, OSError):
        return None
    return record


def launch(
    invoked: str, args: list[str], *, registry: Mapping[str, Any] | None = None,
    environ: Mapping[str, str] | None = None, execv: Callable[[str, list[str]], Any] = os.execv,
) -> None:
    tool, bin_name, _ = invoked_tool(invoked, registry)
    values = os.environ if environ is None else environ
    root = store_root(tool, values)
    daemon = long_running(tool, args, registry)
    executable, _, cid = resolve_exec(
        root=root, tool=tool, bin_name=bin_name, environ=values,
        warn=lambda message: print(f"WARNING: {message}", file=sys.stderr),
        session=not daemon,
    )
    if cid is not None:
        _touch_used(root, cid)
        if daemon:
            daemon_lease(root, cid)
    execv(str(executable), [Path(invoked).name, *args])


def _touch_used(root: Path, cid: str) -> None:
    marker = _artifact(root, cid) / ".used"
    try:
        os.utime(marker, None)
        return
    except FileNotFoundError:
        pass
    try:
        descriptor = os.open(marker, os.O_WRONLY | os.O_CREAT, 0o444)
        os.close(descriptor)
        os.utime(marker, None)
    except OSError as error:
        raise StoreRefusal("usage-lease-failed", f"could not refresh {marker}: {error}") from error


def _owner_live(record: dict[str, Any], proc_root: Path) -> bool | None:
    pid = record.get("owner_pid")
    starttime = record.get("owner_starttime")
    if not isinstance(pid, int) or not isinstance(starttime, str):
        return None
    fields = _proc_fields(proc_root, pid)
    return bool(fields and fields[1] == starttime)


def _records(root: Path) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    try:
        children = list(root.iterdir())
    except OSError:
        return records
    for child in children:
        if not child.is_dir() or not CID_RE.fullmatch(child.name):
            continue
        record = _read_json(child / "record.json")
        if record is not None:
            records.append(record)
    return sorted(records, key=lambda row: str(row.get("ingested_at", "")), reverse=True)


def status(
    *, root: Path | None = None, commit: str | None = None, create_pin: bool = False,
    tool: str = "epr", bin_name: str | None = None,
) -> dict[str, Any]:
    root = _root(root, tool)
    head = _read_link(root, "current")
    identity = session_identity()
    active_session = str(identity["session"]) if identity else None
    pinned = None
    if active_session:
        pinned_record = _read_json(_pin_path(root, active_session))
        if not _valid_pin(pinned_record) and create_pin and head:
            pinned_record = pin(root=root, tool=tool)
        if _valid_pin(pinned_record):
            pinned = pinned_record.get("cid")
    try:
        executable, source, resolved_cid = resolve(root=root, tool=tool, bin_name=bin_name)
        resolved = str(executable)
    except StoreRefusal:
        source, resolved_cid, resolved = "unresolved", None, None
    if pinned is None and source == "session-pin":
        pinned = resolved_cid
    candidates = _records(root)
    if commit is not None:
        candidates = [row for row in candidates if _source_for_commit(row, commit) is not None]
    return {
        "session": active_session, "session_cid": pinned, "head": head,
        "behind": bool(pinned and head and pinned != head), "source": source,
        "resolved_cid": resolved_cid, "resolved": resolved, "candidates": candidates,
    }


def gc(
    *, root: Path | None = None, proc_root: Path | None = None, at: dt.datetime | None = None
) -> dict[str, Any]:
    root = _root(root)
    proc_root = _default_proc_root(proc_root)
    current_time = at or _utcnow()
    with store_lock(root):
        keep = {cid for cid in (_read_link(root, "current"), _read_link(root, "previous")) if cid}
        for record in _records(root):
            cid = record.get("cid")
            ingested = _parse_time(record.get("ingested_at"))
            if isinstance(cid, str) and ingested and current_time - ingested < GRACE:
                keep.add(cid)
            if isinstance(cid, str):
                try:
                    used = dt.datetime.fromtimestamp(
                        (_artifact(root, cid) / ".used").stat().st_mtime, tz=dt.timezone.utc
                    )
                except OSError:
                    used = None
                if used is not None and current_time - used < GRACE:
                    keep.add(cid)
        removed_sessions: list[str] = []
        sessions = root / "sessions"
        if sessions.is_dir():
            for path in sessions.iterdir():
                record = _read_json(path)
                if not _valid_pin(record):
                    path.unlink(missing_ok=True)
                    removed_sessions.append(path.name)
                    continue
                live = _owner_live(record, proc_root)
                pinned = _parse_time(record.get("pinned_at"))
                ownerless_grace = live is None and pinned and current_time - pinned < GRACE
                if live is True or ownerless_grace:
                    keep.add(str(record["cid"]))
                    continue
                path.unlink(missing_ok=True)
                removed_sessions.append(path.name)
        removed: list[str] = []
        for record in _records(root):
            cid = record.get("cid")
            if not isinstance(cid, str) or cid in keep:
                continue
            shutil.rmtree(_artifact(root, cid))
            removed.append(cid)
        return {"kept": sorted(keep), "removed": sorted(removed),
                "removed_sessions": removed_sessions}


def _bootstrap_target(repo: Path) -> Path:
    import subprocess

    configured = os.environ.get("CARGO_TARGET_DIR")
    if configured:
        return Path(configured).expanduser().resolve(strict=False)
    tracked_pool = repo / "genesis/agentic/bin/cargo-pool"
    pool = str(tracked_pool) if os.access(tracked_pool, os.X_OK) else shutil.which("cargo-pool")
    if pool is None:
        raise StoreRefusal("bootstrap-pool-unavailable", "cargo-pool is not executable")
    try:
        result = subprocess.run(
            [pool, "key"], cwd=repo / "elohim/eprfs", capture_output=True, text=True,
            timeout=20, check=False,
        )
    except (OSError, subprocess.SubprocessError) as error:
        raise StoreRefusal("bootstrap-pool-unavailable", str(error)) from error
    for line in result.stdout.splitlines():
        if line.startswith("slot_dev=") and line.split("=", 1)[1].strip():
            return Path(line.split("=", 1)[1].strip())
    detail = (result.stderr or result.stdout).strip().splitlines()
    raise StoreRefusal("bootstrap-pool-unavailable",
                       detail[-1] if detail else f"cargo-pool key exited {result.returncode}")


def _bootstrap_environment(repo: Path, target: Path) -> dict[str, str]:
    environment = dict(os.environ)
    policy = _read_json(repo / "genesis/agentic/pool-policy.json") or {}
    overrides = policy.get("cargo_env_overrides", {})
    if isinstance(overrides, dict):
        for project in ("*", "eprfs"):
            values = overrides.get(project, {})
            if isinstance(values, dict):
                environment.update({str(name): str(value) for name, value in values.items()})
    environment.update({"RUSTFLAGS": "", "CARGO_TARGET_DIR": str(target)})
    return environment


def _judge_brit_attestation(checkout: Path, pin: str) -> dict[str, Any]:
    import json
    import subprocess

    script = checkout / "genesis/orchestrator/gate-attest.mjs"
    path = str(tool_config("brit")["source"]["path"])
    result = subprocess.run(
        ["node", str(script), "--judge", path, "--sha", pin], cwd=checkout,
        capture_output=True, text=True, timeout=75, check=False,
    )
    try:
        verdict = json.loads(result.stdout)
    except ValueError as error:
        detail = (result.stderr or result.stdout).strip().splitlines()
        raise StoreRefusal(
            "attestation-unreachable", detail[-1] if detail else "attestation judge did not answer"
        ) from error
    if not isinstance(verdict, dict):
        raise StoreRefusal("attestation-unreachable", "attestation judge returned no verdict")
    return verdict


def _require_brit_attestation(
    checkout: Path, pin: str,
    attestation_judge: Callable[[Path, str], Mapping[str, Any]] | None,
) -> None:
    verdict = dict((attestation_judge or _judge_brit_attestation)(checkout, pin))
    if (verdict.get("outcome"), verdict.get("tier"), verdict.get("conclusion")) == (
        "pass", "witnessed", "success"
    ):
        return
    line = str(verdict.get("line") or verdict.get("reason") or "attestation did not succeed")
    kind = "attestation-unreachable" if verdict.get("conclusion") == "unreachable" else "attestation-refused"
    raise StoreRefusal(kind, line)


def _download_brit_release(pin: str, destination: Path) -> Path | None:
    import subprocess

    release = f"brit-build-{pin[:12]}"
    result = subprocess.run(
        ["gh", "release", "download", release, "--repo", "ethosengine/brit", "--dir", str(destination)],
        capture_output=True, text=True, timeout=120, check=False,
    )
    if result.returncode != 0:
        return None
    for name in tool_config("brit")["bins"]:
        try:
            (destination / name).chmod(0o555)
        except OSError as error:
            raise StoreRefusal("release-invalid", f"downloaded release lacks {name}: {error}") from error
    return destination


def _write_sums(directory: Path, names: list[str]) -> None:
    checksums = {name: hash_file(directory / name) for name in names}
    (directory / "SHA256SUMS").write_text(canonical_sums(checksums), encoding="utf-8")


def _build_brit_release(checkout: Path, pin: str, destination: Path) -> Path:
    import subprocess

    target = _bootstrap_target(checkout) / "brit-follow" / pin[:12]
    worktree = Path(tempfile.mkdtemp(prefix="brit-follow-worktree-"))
    shutil.rmtree(worktree)
    added = _brit_git(checkout, "worktree", "add", "--detach", str(worktree), pin, timeout=120)
    if added.returncode != 0:
        detail = (added.stderr or added.stdout).strip().splitlines()
        raise StoreRefusal("brit-worktree-failed", detail[-1] if detail else "git worktree add failed")
    try:
        environment = dict(os.environ)
        environment.update({
            "RUSTFLAGS": "",
            "CARGO_TARGET_DIR": str(target),
            "CARGO_BUILD_JOBS": "1",
        })
        environment.update(_registry_credential(environment))
        result = subprocess.run(
            [
                "cargo", "build", "--locked", "--profile", "release-github",
                "-p", "brit-cli", "--bin", "brit",
                "-p", "brit-build-ref", "--bin", "brit-build-ref",
                "-p", "brit-verify", "--bin", "brit-verify",
                "--no-default-features", "--features", "brit-cli/max-pure",
            ],
            cwd=worktree, env=environment, check=False,
        )
        if result.returncode != 0:
            raise StoreRefusal("brit-build-failed", f"cargo build exited {result.returncode}")
        destination.mkdir(parents=True, exist_ok=True)
        for name in tool_config("brit")["bins"]:
            shutil.copyfile(target / "release-github" / name, destination / name)
            (destination / name).chmod(0o555)
        _write_sums(destination, tool_config("brit")["bins"])
        return destination
    finally:
        _brit_git(checkout, "worktree", "remove", "--force", str(worktree), timeout=120)


def _registry_credential(environ: Mapping[str, str]) -> dict[str, str]:
    """The cargo credential for brit's `elohim` Nexus registry, or nothing.

    The registry is auth-required for cargo even for reads. A device's NPM_TOKEN (the bot's
    Nexus token) serves as that read credential when no explicit cargo token is set; the
    value is passed through the environment only and never printed.
    """
    token = environ.get("CARGO_REGISTRIES_ELOHIM_TOKEN", "").strip()
    if not token and environ.get("NPM_TOKEN", "").strip():
        token = f"Bearer {environ['NPM_TOKEN'].strip()}"
    if not token:
        return {}
    return {
        "CARGO_REGISTRIES_ELOHIM_TOKEN": token,
        "CARGO_REGISTRY_GLOBAL_CREDENTIAL_PROVIDERS": "cargo:token",
        "CARGO_REGISTRIES_ELOHIM_INDEX": environ.get(
            "CARGO_REGISTRIES_ELOHIM_INDEX",
            "sparse+https://nexus.ethosengine.com/repository/cargo-internal/",
        ),
    }


def _brit_bytes(
    checkout: Path, pin: str, destination: Path,
    downloader: Callable[[str, Path], Path | bool | None] | None,
) -> Path:
    downloaded = (downloader or _download_brit_release)(pin, destination)
    if downloaded:
        return Path(downloaded) if not isinstance(downloaded, bool) else destination
    if _registry_credential(os.environ):
        return _build_brit_release(checkout, pin, destination)
    raise StoreRefusal(
        "brit-bytes-unavailable",
        f"no brit-build-{pin[:12]} release and no CARGO_REGISTRIES_ELOHIM_TOKEN or NPM_TOKEN",
    )


def _adopt_brit_pin(
    root: Path, checkout: Path, pin: str, *, gate_run: str,
    downloader: Callable[[str, Path], Path | bool | None] | None,
    attestation_judge: Callable[[Path, str], Mapping[str, Any]] | None,
) -> dict[str, Any]:
    for record in _records(root):
        cid = record.get("cid")
        source = _source_for_commit(record, pin)
        if (isinstance(cid, str) and source is not None
                and source.get("tree_clean") is True and _verified_artifact(root, cid, "brit")):
            return land(
                cid, pin, root=root, repo=checkout, tool="brit",
                attestation_judge=attestation_judge,
            )
    with tempfile.TemporaryDirectory(prefix="brit-release-") as temporary:
        release = _brit_bytes(checkout, pin, Path(temporary), downloader)
        record = ingest(
            release, tool="brit", root=root, repo=checkout, source_commit=pin,
            gate_run=gate_run, tree_clean=True,
        )
    return land(
        record["cid"], pin, root=root, repo=checkout, tool="brit",
        attestation_judge=attestation_judge,
    )


def bootstrap(
    *, root: Path | None = None, repo: Path | None = None, tool: str = "epr",
    downloader: Callable[[str, Path], Path | bool | None] | None = None,
    attestation_judge: Callable[[Path, str], Mapping[str, Any]] | None = None,
) -> dict[str, Any]:
    """Seed or resume a headless store; an existing candidate may finish without rebuilding."""
    import subprocess

    root = _root(root, tool)
    checkout = repo_root(repo)
    if tool == "brit":
        current = _read_link(root, "current")
        if current is not None and _verified_artifact(root, current, tool):
            return _read_json(_artifact(root, current) / "record.json") or {"cid": current}
        pin_sha = pinned_sha(checkout, tool)
        return _adopt_brit_pin(
            root, checkout, pin_sha, gate_run="bootstrap", downloader=downloader,
            attestation_judge=attestation_judge,
        )
    try:
        cid = _read_link(root, "current")
        if cid is None or not _verified_artifact(root, cid, tool):
            raise StoreRefusal("head-absent", "the store has no verified committed current head")
        return _read_json(_artifact(root, cid) / "record.json") or {"cid": cid}
    except StoreRefusal:
        pass
    commit, clean = git_state(checkout)
    if not clean:
        raise StoreRefusal("bootstrap-dirty", "bootstrap requires a clean source tree")
    for record in _records(root):
        cid = record.get("cid")
        source = _source_for_commit(record, commit)
        if (isinstance(cid, str) and source is not None
                and source.get("tree_clean") is True and _verified_artifact(root, cid)):
            return land(cid, commit, root=root, repo=checkout)
    return _build_and_land(root, checkout, commit, gate_run="bootstrap")


def _build_and_land(root: Path, checkout: Path, commit: str, *, gate_run: str) -> dict[str, Any]:
    import subprocess

    target = _bootstrap_target(checkout)
    result = subprocess.run(
        ["cargo", "build", "--locked", "--bin", "epr"], cwd=checkout / "elohim/eprfs",
        env=_bootstrap_environment(checkout, target), check=False,
    )
    if result.returncode != 0:
        raise StoreRefusal(f"{gate_run}-build-failed", f"cargo build exited {result.returncode}")
    record = ingest(target / "debug/epr", root=root, repo=checkout, source_commit=commit,
                    gate_run=gate_run, tree_clean=True)
    return land(record["cid"], commit, root=root, repo=checkout)


PUBLISHED_REF = "refs/remotes/origin/dev"


def follow(
    *, root: Path | None = None, repo: Path | None = None, tool: str = "epr",
    downloader: Callable[[str, Path], Path | bool | None] | None = None,
    attestation_judge: Callable[[Path, str], Mapping[str, Any]] | None = None,
) -> dict[str, Any]:
    """Adopt a newer published dev commit this checkout already holds.

    Landing on dev publishes to the household; a device that only pulled must still converge, so
    at session start it builds and lands the checkout's HEAD when all of these hold: HEAD is
    contained in the published dev ref (someone landed it), it descends from the landed head's
    commit, epr's own sources changed between them, and those sources are clean. Anything else
    answers why it did nothing, and the head stays where it is.
    """
    root = _root(root, tool)
    landed = _read_json(root / "current.json") or {}
    head_commit = landed.get("commit")
    if not isinstance(head_commit, str) or _read_link(root, "current") is None:
        return {"followed": False, "why": "no landed head; bootstrap seeds the first one"}
    checkout = repo_root(repo)
    if tool == "brit":
        superproject_head = _git(checkout, "rev-parse", "HEAD")
        published_ref = str(tool_config(tool)["source"]["publishedRef"])
        if not _git_succeeds(
            checkout, "merge-base", "--is-ancestor", superproject_head, published_ref
        ):
            raise StoreRefusal(
                "unpublished", f"HEAD {superproject_head[:12]} is not published on dev"
            )
        target = pinned_sha(checkout, tool)
        if target == head_commit:
            return {"followed": False, "why": "the landed brit pin is this checkout's pin"}
        if not brit_descends(checkout, head_commit, target):
            raise StoreRefusal(
                "non-descendant", f"brit pin {target[:12]} does not descend from {head_commit[:12]}"
            )
        return {
            "followed": True,
            **_adopt_brit_pin(
                root, checkout, target, gate_run="follow", downloader=downloader,
                attestation_judge=attestation_judge,
            ),
        }
    commit, clean = git_state(checkout)
    if commit == head_commit:
        return {"followed": False, "why": "the landed head is this checkout's HEAD"}
    if not _git_succeeds(checkout, "merge-base", "--is-ancestor", commit, PUBLISHED_REF):
        return {"followed": False, "why": f"HEAD {commit[:12]} is not published on dev"}
    if not _git_succeeds(checkout, "merge-base", "--is-ancestor", head_commit, commit):
        return {"followed": False, "why": f"HEAD does not descend from the landed {head_commit[:12]}"}
    if _git_succeeds(checkout, "diff", "--quiet", head_commit, commit, "--", *source_paths(checkout)):
        return {"followed": False, "why": "no epr source change since the landed head"}
    if not clean:
        return {"followed": False, "why": "epr sources are dirty in this checkout"}
    for record in _records(root):
        cid = record.get("cid")
        source = _source_for_commit(record, commit)
        if (isinstance(cid, str) and source is not None
                and source.get("tree_clean") is True and _verified_artifact(root, cid)):
            return {"followed": True, **land(cid, commit, root=root, repo=checkout)}
    return {"followed": True, **_build_and_land(root, checkout, commit, gate_run="follow")}


def _parse_bool(value: str) -> bool:
    import argparse

    if value.lower() in {"true", "1", "yes"}:
        return True
    if value.lower() in {"false", "0", "no"}:
        return False
    raise argparse.ArgumentTypeError("expected true or false")


def parser() -> argparse.ArgumentParser:
    import argparse

    cli = argparse.ArgumentParser(description="content-addressed followed-tool device store")
    cli.add_argument("--registry", type=Path, help="followed-tool registry")
    cli.add_argument("--tool", help="registered tool name")
    cli.add_argument("--store", type=Path, help="override the selected tool's store root")
    verbs = cli.add_subparsers(dest="verb", required=True)
    ingest_parser = verbs.add_parser("ingest")
    ingest_parser.add_argument("binary", type=Path)
    ingest_parser.add_argument("--source-commit")
    ingest_parser.add_argument("--gate-run")
    ingest_parser.add_argument("--tree-clean", type=_parse_bool)
    ingest_parser.add_argument("--repo", type=Path)
    land_parser = verbs.add_parser("land")
    land_parser.add_argument("cid")
    land_parser.add_argument("--commit", required=True)
    land_parser.add_argument("--repo", type=Path)
    land_parser.add_argument("--revert", action="store_true")
    verbs.add_parser("pin")
    resolve_parser = verbs.add_parser("resolve")
    resolve_parser.add_argument("--current", action="store_true")
    launch_parser = verbs.add_parser("launch")
    launch_parser.add_argument("--invoked", required=True)
    launch_parser.add_argument("args", nargs=argparse.REMAINDER)
    status_parser = verbs.add_parser("status")
    status_parser.add_argument("--json", action="store_true")
    status_parser.add_argument("--commit")
    status_parser.add_argument("--pin", action="store_true")
    verbs.add_parser("gc")
    verbs.add_parser("shadows")
    source_parser = verbs.add_parser("source-state")
    source_parser.add_argument("--repo", type=Path)
    follow_parser = verbs.add_parser("follow")
    follow_parser.add_argument("--repo", type=Path)
    bootstrap_parser = verbs.add_parser("bootstrap")
    bootstrap_parser.add_argument("--repo", type=Path)
    return cli


def main(argv: list[str] | None = None) -> int:
    import json

    global _ACTIVE_REGISTRY
    args = parser().parse_args(sys.argv[1:] if argv is None else argv)
    _ACTIVE_REGISTRY = load_registry(args.registry)
    if args.verb == "launch":
        try:
            call_args = args.args[1:] if args.args[:1] == ["--"] else args.args
            launch(args.invoked, call_args, registry=_ACTIVE_REGISTRY)
        except StoreRefusal as refusal:
            print(f"REFUSED {refusal.kind}: {refusal.detail}", file=sys.stderr)
            return 127
    if not args.tool:
        print("REFUSED tool-required: --tool is required", file=sys.stderr)
        return 2
    tool_config(args.tool, _ACTIVE_REGISTRY)
    root = _root(args.store, args.tool)
    try:
        if args.verb == "ingest":
            value = ingest(args.binary, tool=args.tool, root=root, repo=args.repo,
                           source_commit=args.source_commit, gate_run=args.gate_run,
                           tree_clean=args.tree_clean)
            print(json.dumps(value, sort_keys=True))
        elif args.verb == "land":
            print(json.dumps(land(args.cid, args.commit, root=root, repo=args.repo,
                                  revert=args.revert, tool=args.tool), sort_keys=True))
        elif args.verb == "pin":
            print(json.dumps(pin(root=root, tool=args.tool), sort_keys=True))
        elif args.verb == "resolve":
            resolved = (
                resolve_current(root=root, tool=args.tool)
                if args.current else resolve(root=root, tool=args.tool)
            )
            print(resolved[0])
        elif args.verb == "status":
            value = status(root=root, commit=args.commit, create_pin=args.pin, tool=args.tool)
            print(json.dumps(value, sort_keys=True) if args.json else _status_text(value))
        elif args.verb == "gc":
            print(json.dumps(gc(root=root), sort_keys=True))
        elif args.verb == "shadows":
            print(json.dumps(shadows(root=root, tool=args.tool), sort_keys=True))
        elif args.verb == "source-state":
            checkout = repo_root(args.repo)
            if args.tool == "epr":
                head, clean = git_state(checkout)
                dirty = _git(checkout, "status", "--porcelain", "--untracked-files=normal", "--",
                             *source_paths(checkout))
            else:
                head, clean, dirty = pinned_sha(checkout, args.tool), True, ""
            print(json.dumps({"head": head, "clean": clean, "status": dirty}, sort_keys=True))
        elif args.verb == "follow":
            print(json.dumps(follow(root=root, repo=args.repo, tool=args.tool), sort_keys=True))
        elif args.verb == "bootstrap":
            print(json.dumps(bootstrap(root=root, repo=args.repo, tool=args.tool), sort_keys=True))
    except StoreRefusal as refusal:
        print(f"REFUSED {refusal.kind}: {refusal.detail}", file=sys.stderr)
        return 2
    return 0


def _status_text(value: dict[str, Any]) -> str:
    session = value.get("session") or "none"
    session_cid = value.get("session_cid") or "none"
    head = value.get("head") or "none"
    state = "behind" if value.get("behind") else value.get("source", "unresolved")
    return "\n".join((f"session: {session} cid={session_cid}", f"head: {head}",
                       f"state: {state}", f"resolved: {value.get('resolved') or 'none'}"))


if __name__ == "__main__":
    raise SystemExit(main())
