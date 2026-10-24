#!/usr/bin/env python3
"""SessionStart guard for a verified epr head and persistent device launcher.

The guard is fail-open. It never changes the PATH entry until ``current`` resolves to bytes whose
hash matches its CID. A missing or headless store is resumable through ``bootstrap``.
"""

from __future__ import annotations

import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path


def _repo() -> Path | None:
    here = Path(__file__).resolve()
    for parent in here.parents:
        if (parent / "genesis/agentic/bin/epr-store").is_file():
            return parent
    return None


def _say(line: str) -> None:
    print(f"EPR EVALUATOR: {line}")


def _run(cli: Path, repo: Path, *args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [str(cli), *args], cwd=repo, capture_output=True, text=True,
        timeout=900 if args[:1] in (("bootstrap",), ("follow",)) else 20, check=False,
    )


def _detail(result: subprocess.CompletedProcess[str]) -> str:
    lines = (result.stderr or result.stdout).strip().splitlines()
    return lines[-1] if lines else f"exit {result.returncode}"


def _write_if_changed(content: bytes, destination: Path, mode: int) -> None:
    try:
        if destination.read_bytes() == content and destination.stat().st_mode & 0o777 == mode:
            return
    except OSError:
        pass
    destination.parent.mkdir(parents=True, exist_ok=True)
    fd, name = tempfile.mkstemp(prefix=f".{destination.name}.", dir=destination.parent)
    temporary = Path(name)
    try:
        with os.fdopen(fd, "wb") as target:
            target.write(content)
            target.flush()
            os.fsync(target.fileno())
        temporary.chmod(mode)
        os.replace(temporary, destination)
    finally:
        temporary.unlink(missing_ok=True)


def _cargo_bin_epr() -> Path:
    cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo")).expanduser().resolve()
    return cargo_home / "bin/epr"


def _adopt_clobber(cli: Path, repo: Path, destination: Path) -> str | None:
    """A plain binary where the launcher belongs was written by a hand `cargo install`.

    Keep it as a dev-context build (runnable by pinning its CID) before the launcher takes the
    path back, so a hand-built epr is never lost — and say so. Returns the store path it now
    lives at, which is what a failed install restores to.
    """
    if destination.is_symlink() or not destination.is_file():
        return None
    adopted = _run(cli, repo, "ingest", str(destination), "--source-commit", "unknown",
                   "--tree-clean", "false", "--gate-run", "adopted-from-path")
    if adopted.returncode != 0:
        raise RuntimeError(f"{destination} is a plain binary the store refused: {_detail(adopted)}")
    import json

    record = json.loads(adopted.stdout)
    store = Path(os.environ["EPR_STORE"])
    _say(f"kept the binary at {destination} as dev-context {record['cid']} "
         f"(run it with EPR_PIN_CID={record['cid']})")
    return str(store / record["cid"] / "epr")


def _repoint_shadows(cli: Path, repo: Path, launcher: Path) -> list[str]:
    """Repoint stale links ahead of the launcher on PATH; report anything else by name."""
    import json

    listed = _run(cli, repo, "shadows")
    if listed.returncode != 0:
        return [f"shadow check failed: {_detail(listed)}"]
    for shadow in json.loads(listed.stdout or "[]"):
        path = Path(shadow["path"])
        if not shadow["superseded"]:
            continue
        temporary = path.with_name(f".epr-launch.{os.getpid()}.{time.time_ns()}")
        temporary.symlink_to(launcher)
        try:
            os.replace(temporary, path)
        finally:
            temporary.unlink(missing_ok=True)
        _say(f"repointed stale {path} (was → {shadow['target']}) at the launcher")
    # Re-list: a repointed link may now be the first `epr` on PATH, hiding what came after it.
    relisted = _run(cli, repo, "shadows")
    return [
        f"{shadow['path']} (not a build-output link; remove it or move it after "
        f"{_cargo_bin_epr().parent} on PATH)"
        for shadow in json.loads(relisted.stdout or "[]")
    ] if relisted.returncode == 0 else [f"shadow re-check failed: {_detail(relisted)}"]


def _install_link(launcher: Path, adopted: str | None = None) -> tuple[Path, str | None, bool]:
    destination = _cargo_bin_epr()
    destination.parent.mkdir(parents=True, exist_ok=True)
    try:
        if destination.is_symlink() and destination.resolve() == launcher.resolve():
            return destination, None, False
    except OSError:
        pass
    try:
        previous_target = os.readlink(destination)
    except OSError:
        previous_target = adopted
    temporary = destination.with_name(f".epr-launch.{os.getpid()}.{time.time_ns()}")
    temporary.symlink_to(launcher)
    try:
        os.replace(temporary, destination)
    finally:
        temporary.unlink(missing_ok=True)
    return destination, previous_target, True


def _restore_link(destination: Path, previous_target: str | None, changed: bool) -> None:
    if not changed:
        return
    if previous_target is None:
        destination.unlink(missing_ok=True)
        return
    temporary = destination.with_name(f".epr-restore.{os.getpid()}.{time.time_ns()}")
    temporary.symlink_to(previous_target)
    try:
        os.replace(temporary, destination)
    finally:
        temporary.unlink(missing_ok=True)


def _installed_healthy(destination: Path, store: Path) -> tuple[bool, str]:
    result = subprocess.run(
        [str(destination), "--version"],
        env={"PATH": "/usr/bin:/bin", "EPR_STORE": str(store)},
        capture_output=True,
        text=True,
        timeout=20,
        check=False,
    )
    return result.returncode == 0, _detail(result)


def main() -> None:
    repo = _repo()
    if repo is None:
        return
    store = Path(
        os.environ.get("EPR_STORE", "/projects/.claude-config/tools/bin-store/epr")
    ).expanduser().resolve()
    os.environ["EPR_STORE"] = str(store)
    cli = repo / "genesis/agentic/bin/epr-store"
    head = _run(cli, repo, "resolve", "--current")
    if head.returncode == 0:
        # A device that only pulled a newer published dev still converges: build and land it.
        followed = _run(cli, repo, "follow", "--repo", str(repo))
        if followed.returncode != 0:
            _say(f"⚠ could not follow published dev; the landed head stays: {_detail(followed)}")
        elif '"followed": true' in followed.stdout:
            _say("followed published dev: built and landed this checkout's HEAD")
    if head.returncode != 0:
        bootstrap = _run(cli, repo, "bootstrap", "--repo", str(repo))
        if bootstrap.returncode != 0:
            _say(f"⚠ no verified head; launcher left untouched: {_detail(bootstrap)}")
            return
        head = _run(cli, repo, "resolve", "--current")
        if head.returncode != 0:
            _say(f"⚠ bootstrap returned without a verified head; launcher left untouched: {_detail(head)}")
            return

    persistent_bin = store.parent / "bin"
    launcher = persistent_bin / "epr-launch"
    store_module = (persistent_bin / "epr_store.py").resolve()
    _write_if_changed((repo / ".claude/scripts/_lib/epr_store.py").read_bytes(), store_module, 0o644)
    template = (repo / "genesis/agentic/bin/epr-launch").read_text(encoding="utf-8")
    rendered = (
        template.replace("@EPR_PYTHON@", str(Path(sys.executable).expanduser().resolve()))
        .replace("@EPR_STORE_PY@", str(store_module))
        .encode()
    )
    _write_if_changed(rendered, launcher.resolve(), 0o755)
    adopted = _adopt_clobber(cli, repo, _cargo_bin_epr())
    destination, previous_target, changed = _install_link(launcher.resolve(), adopted)
    healthy, detail = _installed_healthy(destination, store)
    if not healthy:
        _restore_link(destination, previous_target, changed)
        _say(f"⚠ installed launcher verification failed; previous link restored: {detail}")
        return
    for remaining in _repoint_shadows(cli, repo, launcher.resolve()):
        _say(f"⚠ an epr ahead of the launcher on PATH still shadows it: {remaining}")

    pinned = _run(cli, repo, "pin")
    if pinned.returncode == 0:
        _say(f"verified head · persistent launcher {destination} · process-ancestry pin active")
    elif "session-absent" in (pinned.stderr or ""):
        _say(f"verified head · persistent launcher {destination} · no session ancestry to pin")
    else:
        _say(f"verified head · persistent launcher {destination} · pin refused: {_detail(pinned)}")


if __name__ == "__main__":
    try:
        main()
    except Exception as error:  # noqa: BLE001 — SessionStart repair must never block the session
        _say(f"guard error (ignored): {error}")
