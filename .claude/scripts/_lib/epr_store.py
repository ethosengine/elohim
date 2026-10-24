"""Content-addressed device store for the native ``epr`` evaluator.

Only :func:`land` changes the committed head; gates merely ingest candidates. The installed
launcher imports this file with ``python -I``, so its hot path is stdlib-only.
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
from typing import Any, Iterator, Mapping

DEFAULT_STORE = Path("/projects/.claude-config/tools/bin-store/epr")
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


def store_root() -> Path:
    return Path(os.environ.get("EPR_STORE", DEFAULT_STORE)).expanduser().resolve(strict=False)


def _root(path: Path | None) -> Path:
    return (path or store_root()).expanduser().resolve(strict=False)


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


def ingest(
    binary: Path,
    *,
    root: Path | None = None,
    repo: Path | None = None,
    source_commit: str | None = None,
    gate_run: str | None = None,
    tree_clean: bool | None = None,
) -> dict[str, Any]:
    """Copy privately, then hash and identify exactly the bytes that will be published."""
    root = _root(root)
    binary = binary.resolve()
    if not binary.is_file() or not os.access(binary, os.X_OK):
        raise StoreRefusal("binary-unusable", f"not an executable file: {binary}")
    if source_commit is None or tree_clean is None:
        repo = repo_root(repo)
        observed_commit, observed_clean = git_state(repo)
        source_commit = source_commit or observed_commit
        if tree_clean is None:
            tree_clean = observed_clean

    with store_lock(root):
        staging = root / ".staging"
        staging.mkdir(parents=True, exist_ok=True)
        fd, staged_name = tempfile.mkstemp(prefix="epr.", dir=staging)
        os.close(fd)
        staged = Path(staged_name)
        try:
            shutil.copyfile(binary, staged)
            staged.chmod(0o555)
            cid = hash_file(staged)
            identity = binary_identity(staged)
            if identity.get("cid") != cid:
                raise StoreRefusal(
                    "identity-mismatch", f"binary reports {identity.get('cid')!r}, bytes are {cid}"
                )
            artifact = _artifact(root, cid)
            artifact.mkdir(parents=True, exist_ok=True)
            destination = artifact / "epr"
            if destination.is_file() and hash_file(destination) == cid:
                staged.unlink()
                destination.chmod(0o555)
            else:
                os.rename(staged, destination)
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
            record.setdefault("version", identity.get("version"))
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
            staged.unlink(missing_ok=True)


def _append_refusal_locked(root: Path, cid: str, kind: str, detail: str) -> None:
    record_path = _artifact(root, cid) / "record.json"
    record = _read_json(record_path) or {"cid": cid}
    refusals = record.setdefault("refusals", [])
    if not isinstance(refusals, list):
        refusals = record["refusals"] = []
    refusals.append({"kind": kind, "detail": detail, "at": now()})
    _atomic_json(record_path, record)


def _health_check(binary: Path, cid: str, repo: Path) -> None:
    import subprocess

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


def _verified_artifact(root: Path, cid: str) -> bool:
    try:
        binary = _artifact(root, cid) / "epr"
        return binary.is_file() and os.access(binary, os.X_OK) and hash_file(binary) == cid
    except (OSError, StoreRefusal):
        return False


def _usable_artifact(root: Path, cid: str) -> bool:
    """Exec-path check: landed artifacts are immutable, so do not hash them again here."""
    try:
        binary = _artifact(root, cid) / "epr"
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
    root: Path, cid: str, commit: str, checkout: Path, revert: bool
) -> tuple[dict[str, Any], str | None, dict[str, Any] | None]:
    artifact = _artifact(root, cid)
    record = _read_json(artifact / "record.json")
    if record is None or not _verified_artifact(root, cid):
        raise StoreRefusal("candidate-absent", f"{cid} is not a verified store candidate")
    source = _source_for_commit(record, commit)
    if source is None:
        raise StoreRefusal(
            "commit-mismatch", f"candidate has no ingest provenance for commit {commit!r}"
        )
    if source.get("tree_clean") is not True:
        raise StoreRefusal("dirty-build", f"{cid} was ingested from a dirty tree at {commit}")
    head, clean = git_state(checkout)
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
            if not isinstance(ancestor, str) or not _git_succeeds(
                checkout, "merge-base", "--is-ancestor", ancestor, commit
            ):
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
    revert: bool = False,
) -> dict[str, Any]:
    root = _root(root)
    checkout = repo_root(repo)
    try:
        with store_lock(root):
            record, expected_current, expected_metadata = _validate_land_locked(
                root, cid, commit, checkout, revert
            )
            _touch_used(root, cid)
        _health_check(_artifact(root, cid) / "epr", cid, checkout)
        with store_lock(root):
            record, old_current, current_metadata = _validate_land_locked(
                root, cid, commit, checkout, revert
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
) -> dict[str, Any]:
    root = _root(root)
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
        if selected is None or not _usable_artifact(root, selected):
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


def _launcher_aliases(root: Path) -> set[str]:
    candidates = {Path(__file__).resolve().with_name("epr-launch"), root.parent / "bin/epr-launch"}
    source = Path(__file__).resolve()
    for parent in source.parents:
        candidate = parent / "genesis/agentic/bin/epr-launch"
        if candidate.is_file():
            candidates.add(candidate)
            break
    return {os.path.realpath(candidate) for candidate in candidates}


# Build outputs the launcher supersedes: a PATH link into one of these is drift, never a choice.
_SUPERSEDED_ROOTS = ("/projects/.cargo-target-pool/", "/tmp/eprfs-gate-target")


def shadows(*, root: Path | None = None, path_env: str | None = None) -> list[dict[str, Any]]:
    """Every ``epr`` on PATH that a caller would reach before the launcher.

    The launcher only works if it is the one PATH answers with: a stale link ahead of it (a
    ``~/.local/bin/epr`` into an old pool build) silently brings back every drift it retires.
    ``superseded`` marks a link into a build output, safe to repoint; anything else is a
    binary someone put there, which is reported and never touched.
    """
    root = _root(root)
    aliases = _launcher_aliases(root)
    found: list[dict[str, Any]] = []
    for directory in (path_env if path_env is not None else os.environ.get("PATH", "")).split(":"):
        if not directory:
            continue
        candidate = Path(directory) / "epr"
        if not (candidate.is_symlink() or candidate.exists()):
            continue
        if os.path.realpath(candidate) in aliases:
            return found
        target = os.readlink(candidate) if candidate.is_symlink() else None
        dangling = target is not None and not candidate.exists()
        superseded = target is not None and (
            dangling or os.path.realpath(candidate).startswith(_SUPERSEDED_ROOTS)
            or os.path.realpath(candidate).startswith(str(root) + "/")
        )
        found.append({"path": str(candidate), "target": target, "superseded": superseded})
    # No launcher on PATH at all: nothing is shadowed — whatever PATH reaches simply runs, and
    # `status` already says whether a head exists. Shadowing needs a launcher behind it.
    return []


def _candidate_override(root: Path, value: str) -> tuple[Path, str, str]:
    candidate = Path(value).expanduser()
    if not candidate.is_file() or not os.access(candidate, os.X_OK):
        raise StoreRefusal("binary-unusable", f"EPR_BIN is not executable: {candidate}")
    real = os.path.realpath(candidate)
    if real in _launcher_aliases(root):
        raise StoreRefusal("launcher-alias", f"EPR_BIN resolves to the epr launcher: {candidate}")
    return Path(real), "candidate", hash_file(Path(real))


def _candidate_exec_override(root: Path, value: str) -> tuple[Path, str, None]:
    candidate = Path(value).expanduser().resolve(strict=False)
    if not candidate.is_file() or not os.access(candidate, os.X_OK):
        raise StoreRefusal("binary-unusable", f"EPR_BIN is not executable: {candidate}")
    if os.path.realpath(candidate) in _launcher_aliases(root):
        raise StoreRefusal("launcher-alias", f"EPR_BIN resolves to the epr launcher: {candidate}")
    return candidate, "candidate", None


def resolve_current(*, root: Path | None = None) -> tuple[Path, str, str]:
    root = _root(root)
    cid = _read_link(root, "current")
    if cid is None or not _usable_artifact(root, cid):
        raise StoreRefusal("head-absent", "the store has no executable committed current head")
    return _artifact(root, cid) / "epr", "current", cid


def resolve(
    *, root: Path | None = None, proc_root: Path | None = None, start_pid: int | None = None,
    environ: Mapping[str, str] | None = None,
    pin_lock_timeout: float = MUTATION_LOCK_TIMEOUT_SECONDS,
    warn: Any = None,
) -> tuple[Path, str, str]:
    root = _root(root)
    values = os.environ if environ is None else environ
    override = values.get("EPR_BIN", "").strip()
    if override:
        return _candidate_override(root, override)
    explicit = values.get("EPR_PIN_CID", "").strip()
    if explicit:
        cid = _require_cid(explicit)
        if not _usable_artifact(root, cid):
            raise StoreRefusal("candidate-absent", f"explicit pin {cid} is not executable")
        return _artifact(root, cid) / "epr", "explicit-pin", cid
    identity = session_identity(proc_root=proc_root, start_pid=start_pid, environ=values)
    if identity:
        record = _read_json(_pin_path(root, str(identity["session"])))
        if not _valid_pin(record):
            try:
                record = pin(
                    root=root, proc_root=proc_root, start_pid=start_pid, environ=values,
                    lock_timeout=pin_lock_timeout,
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
                    executable, _, cid = resolve_current(root=root)
                    if warn is not None:
                        warn(
                            "epr store lock timed out after 5s; using current without "
                            "publishing a session pin"
                        )
                    return executable, "current-lock-timeout", cid
                if warn is not None:
                    warn("epr store lock timed out after 5s; using the existing session pin")
        cid = _require_cid(str(record.get("cid", "")))
        if not _usable_artifact(root, cid):
            raise StoreRefusal("candidate-absent", f"session pin {cid} is not executable")
        return _artifact(root, cid) / "epr", "session-pin", cid
    return resolve_current(root=root)


def resolve_exec(
    *, root: Path | None = None, proc_root: Path | None = None, start_pid: int | None = None,
    environ: Mapping[str, str] | None = None, warn: Any = None,
) -> tuple[Path, str, str | None]:
    """Resolve for ``exec`` without hashing any binary on the launch path."""
    root = _root(root)
    values = os.environ if environ is None else environ
    override = values.get("EPR_BIN", "").strip()
    if override:
        return _candidate_exec_override(root, override)
    return resolve(
        root=root, proc_root=proc_root, start_pid=start_pid, environ=values,
        pin_lock_timeout=EXEC_LOCK_TIMEOUT_SECONDS, warn=warn,
    )


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
    *, root: Path | None = None, commit: str | None = None, create_pin: bool = False
) -> dict[str, Any]:
    root = _root(root)
    head = _read_link(root, "current")
    identity = session_identity()
    active_session = str(identity["session"]) if identity else None
    pinned = None
    if active_session:
        pinned_record = _read_json(_pin_path(root, active_session))
        if not _valid_pin(pinned_record) and create_pin and head:
            pinned_record = pin(root=root)
        if _valid_pin(pinned_record):
            pinned = pinned_record.get("cid")
    try:
        executable, source, resolved_cid = resolve(root=root)
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


def bootstrap(*, root: Path | None = None, repo: Path | None = None) -> dict[str, Any]:
    """Seed or resume a headless store; an existing candidate may finish without rebuilding."""
    import subprocess

    root = _root(root)
    try:
        cid = _read_link(root, "current")
        if cid is None or not _verified_artifact(root, cid):
            raise StoreRefusal("head-absent", "the store has no verified committed current head")
        return _read_json(_artifact(root, cid) / "record.json") or {"cid": cid}
    except StoreRefusal:
        pass
    checkout = repo_root(repo)
    commit, clean = git_state(checkout)
    if not clean:
        raise StoreRefusal("bootstrap-dirty", "bootstrap requires a clean source tree")
    for record in _records(root):
        cid = record.get("cid")
        source = _source_for_commit(record, commit)
        if (isinstance(cid, str) and source is not None
                and source.get("tree_clean") is True and _verified_artifact(root, cid)):
            return land(cid, commit, root=root, repo=checkout)
    target = _bootstrap_target(checkout)
    result = subprocess.run(
        ["cargo", "build", "--locked", "--bin", "epr"], cwd=checkout / "elohim/eprfs",
        env=_bootstrap_environment(checkout, target), check=False,
    )
    if result.returncode != 0:
        raise StoreRefusal("bootstrap-build-failed", f"cargo build exited {result.returncode}")
    record = ingest(target / "debug/epr", root=root, repo=checkout, source_commit=commit,
                    gate_run="bootstrap", tree_clean=clean)
    return land(record["cid"], commit, root=root, repo=checkout)


def _parse_bool(value: str) -> bool:
    import argparse

    if value.lower() in {"true", "1", "yes"}:
        return True
    if value.lower() in {"false", "0", "no"}:
        return False
    raise argparse.ArgumentTypeError("expected true or false")


def parser() -> argparse.ArgumentParser:
    import argparse

    cli = argparse.ArgumentParser(description="content-addressed epr device store")
    cli.add_argument("--store", type=Path, help="override EPR_STORE")
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
    exec_parser = verbs.add_parser("exec")
    exec_parser.add_argument("args", nargs=argparse.REMAINDER)
    status_parser = verbs.add_parser("status")
    status_parser.add_argument("--json", action="store_true")
    status_parser.add_argument("--commit")
    status_parser.add_argument("--pin", action="store_true")
    verbs.add_parser("gc")
    verbs.add_parser("shadows")
    source_parser = verbs.add_parser("source-state")
    source_parser.add_argument("--repo", type=Path)
    bootstrap_parser = verbs.add_parser("bootstrap")
    bootstrap_parser.add_argument("--repo", type=Path)
    return cli


def main(argv: list[str] | None = None) -> int:
    arguments = sys.argv[1:] if argv is None else argv
    exec_root: Path | None = None
    exec_index = 0
    if arguments[:1] == ["--store"] and len(arguments) >= 3:
        exec_root = Path(arguments[1])
        exec_index = 2
    if arguments[exec_index:exec_index + 1] == ["exec"]:
        call_args = arguments[exec_index + 1:]
        if call_args[:1] == ["--"]:
            call_args = call_args[1:]
        try:
            executable, _, cid = resolve_exec(
                root=exec_root,
                warn=lambda message: print(f"WARNING: {message}", file=sys.stderr),
            )
            if cid is not None:
                _touch_used(_root(exec_root), cid)
            os.execv(str(executable), [str(executable), *call_args])
        except StoreRefusal as refusal:
            print(f"REFUSED {refusal.kind}: {refusal.detail}", file=sys.stderr)
            return 127

    import json

    args = parser().parse_args(arguments)
    root = _root(args.store)
    try:
        if args.verb == "ingest":
            value = ingest(args.binary, root=root, repo=args.repo,
                           source_commit=args.source_commit, gate_run=args.gate_run,
                           tree_clean=args.tree_clean)
            print(json.dumps(value, sort_keys=True))
        elif args.verb == "land":
            print(json.dumps(land(args.cid, args.commit, root=root, repo=args.repo,
                                  revert=args.revert), sort_keys=True))
        elif args.verb == "pin":
            print(json.dumps(pin(root=root), sort_keys=True))
        elif args.verb == "resolve":
            resolved = resolve_current(root=root) if args.current else resolve(root=root)
            print(resolved[0])
        elif args.verb == "exec":
            raise AssertionError("exec is handled before argparse")
        elif args.verb == "status":
            value = status(root=root, commit=args.commit, create_pin=args.pin)
            print(json.dumps(value, sort_keys=True) if args.json else _status_text(value))
        elif args.verb == "gc":
            print(json.dumps(gc(root=root), sort_keys=True))
        elif args.verb == "shadows":
            print(json.dumps(shadows(root=root), sort_keys=True))
        elif args.verb == "source-state":
            checkout = repo_root(args.repo)
            head, clean = git_state(checkout)
            dirty = _git(checkout, "status", "--porcelain", "--untracked-files=normal", "--",
                         *source_paths(checkout))
            print(json.dumps({"head": head, "clean": clean, "status": dirty}, sort_keys=True))
        elif args.verb == "bootstrap":
            print(json.dumps(bootstrap(root=root, repo=args.repo), sort_keys=True))
    except StoreRefusal as refusal:
        print(f"REFUSED {refusal.kind}: {refusal.detail}", file=sys.stderr)
        return 127 if args.verb == "exec" else 2
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
