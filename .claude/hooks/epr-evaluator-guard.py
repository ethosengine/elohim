#!/usr/bin/env python3
"""Fail-open SessionStart guard for every registry-declared followed tool."""

from __future__ import annotations

import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

SCRIPT_REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(SCRIPT_REPO / ".claude/scripts"))
from _lib import bin_store  # noqa: E402


def _repo() -> Path | None:
    here = Path(__file__).resolve()
    for parent in here.parents:
        if (parent / "genesis/agentic/bin/bin-store").is_file():
            return parent
    return None


def _say(tool: str, line: str) -> None:
    print(f"{tool.upper()} EVALUATOR: {line}")


def _run(cli: Path, repo: Path, tool: str, *args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [str(cli), "--tool", tool, *args], cwd=repo, capture_output=True, text=True,
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


def _cargo_bin(name: str) -> Path:
    cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo")).expanduser().resolve()
    return cargo_home / "bin" / name


def _canonical_sums(paths: dict[str, Path]) -> str:
    rows = []
    for name in sorted(paths):
        digest = hashlib.sha256(paths[name].read_bytes()).hexdigest()
        rows.append(f"{digest}  {name}\n")
    return "".join(rows)


def _adopt_clobbers(
    cli: Path, repo: Path, tool: str, config: dict, store: Path
) -> dict[str, str]:
    aliases = config.get("aliases", {})
    names = [*config["bins"], *aliases]
    plain = {name: _cargo_bin(name) for name in names}
    plain = {name: path for name, path in plain.items() if path.is_file() and not path.is_symlink()}
    if not plain:
        return {}
    with tempfile.TemporaryDirectory(prefix=f"adopt-{tool}-") as temporary:
        root = Path(temporary)
        current_release: Path | None = None
        baseline: dict[str, Path] = {}
        for bin_name in config["bins"]:
            source = plain.get(bin_name)
            if source is None:
                alias = next((name for name, target in aliases.items()
                              if target == bin_name and name in plain), None)
                source = plain.get(alias) if alias else None
            if source is None:
                if current_release is None:
                    resolved = _run(cli, repo, tool, "resolve", "--current")
                    if resolved.returncode != 0:
                        raise RuntimeError(f"cannot preserve {tool} clobber: {_detail(resolved)}")
                    current_release = Path(resolved.stdout.strip()).parent
                source = current_release / bin_name
            baseline[bin_name] = source

        signatures: dict[str, tuple[dict[str, Path], list[str]]] = {}
        for name, source in plain.items():
            target = aliases.get(name) or name
            sources = {**baseline, target: source}
            signature = _canonical_sums(sources)
            if signature not in signatures:
                signatures[signature] = (sources, [])
            signatures[signature][1].append(name)

        adopted_paths: dict[str, str] = {}
        for index, (sources, names) in enumerate(signatures.values()):
            release = root / str(index)
            release.mkdir()
            staged: dict[str, Path] = {}
            for bin_name, source in sources.items():
                destination = release / bin_name
                shutil.copyfile(source, destination)
                destination.chmod(0o755)
                staged[bin_name] = destination
            if len(staged) > 1:
                (release / "SHA256SUMS").write_text(_canonical_sums(staged), encoding="utf-8")
            ingest_source = release if len(staged) > 1 else release / config["bins"][0]
            adopted = _run(
                cli, repo, tool, "ingest", str(ingest_source), "--source-commit", "unknown",
                "--tree-clean", "false", "--gate-run", "adopted-from-path",
            )
            if adopted.returncode != 0:
                raise RuntimeError(f"plain {tool} binary was refused: {_detail(adopted)}")
            record = json.loads(adopted.stdout)
            _say(
                tool,
                f"kept clobbering plain binaries {', '.join(names)} as dev-context "
                f"{record['cid']} (run with {tool.upper()}_PIN_CID={record['cid']})",
            )
            for name in names:
                adopted_paths[name] = str(store / record["cid"] / (aliases.get(name) or name))
    return adopted_paths


def _install_link(launcher: Path, name: str, adopted: str | None) -> tuple[Path, str | None, bool]:
    destination = _cargo_bin(name)
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
    temporary = destination.with_name(f".{name}-launcher.{os.getpid()}.{time.time_ns()}")
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
    temporary = destination.with_name(f".{destination.name}-restore.{os.getpid()}.{time.time_ns()}")
    temporary.symlink_to(previous_target)
    try:
        os.replace(temporary, destination)
    finally:
        temporary.unlink(missing_ok=True)


def _repoint_shadows(cli: Path, repo: Path, tool: str, launcher: Path) -> list[str]:
    listed = _run(cli, repo, tool, "shadows")
    if listed.returncode != 0:
        return [f"shadow check failed: {_detail(listed)}"]
    for shadow in json.loads(listed.stdout or "[]"):
        path = Path(shadow["path"])
        if not shadow["superseded"]:
            continue
        temporary = path.with_name(f".{shadow['name']}-launcher.{os.getpid()}.{time.time_ns()}")
        temporary.symlink_to(launcher)
        try:
            os.replace(temporary, path)
        finally:
            temporary.unlink(missing_ok=True)
        _say(tool, f"repointed stale {path} (was → {shadow['target']}) at the launcher")
    relisted = _run(cli, repo, tool, "shadows")
    if relisted.returncode != 0:
        return [f"shadow re-check failed: {_detail(relisted)}"]
    return [f"{entry['path']} (not a superseded build-output link)"
            for entry in json.loads(relisted.stdout or "[]")]


def _installed_healthy(destination: Path, tool: str, store: Path) -> tuple[bool, str]:
    result = subprocess.run(
        [str(destination), "--version"],
        env={"PATH": "/usr/bin:/bin", f"{tool.upper()}_STORE": str(store)},
        capture_output=True, text=True, timeout=20, check=False,
    )
    return result.returncode == 0, _detail(result)


def _guard_tool(repo: Path, cli: Path, registry_path: Path, tool: str, config: dict) -> None:
    store = bin_store.store_root(tool, os.environ)
    os.environ[f"{tool.upper()}_STORE"] = str(store)
    verified = _run(cli, repo, tool, "bootstrap", "--repo", str(repo))
    if verified.returncode != 0:
        _say(tool, f"⚠ no verified head; launcher left untouched: {_detail(verified)}")
        return
    followed = _run(cli, repo, tool, "follow", "--repo", str(repo))
    if followed.returncode != 0:
        _say(tool, f"⚠ could not follow the published head; landed head stays: {_detail(followed)}")
    elif '"followed": true' in followed.stdout:
        _say(tool, "followed the published head")

    head = _run(cli, repo, tool, "resolve", "--current")
    if head.returncode != 0:
        _say(tool, f"⚠ verified operation left no head; launcher left untouched: {_detail(head)}")
        return
    persistent_bin = store.parent / "bin"
    store_module = (persistent_bin / "bin_store.py").resolve()
    installed_registry = (persistent_bin / "bin-tools.json").resolve()
    _write_if_changed((repo / ".claude/scripts/_lib/bin_store.py").read_bytes(), store_module, 0o644)
    _write_if_changed(registry_path.read_bytes(), installed_registry, 0o444)
    template = (repo / "genesis/agentic/bin/bin-launch").read_text(encoding="utf-8")
    rendered = (
        template.replace("@BIN_STORE_PYTHON@", str(Path(sys.executable).resolve()))
        .replace("@BIN_STORE_PY@", str(store_module))
        .replace("@BIN_TOOLS_JSON@", str(installed_registry))
        .encode()
    )
    launcher = (persistent_bin / tool).resolve()
    _write_if_changed(rendered, launcher, 0o755)
    adopted = _adopt_clobbers(cli, repo, tool, config, store)
    installed = []
    for name in [*config["bins"], *config.get("aliases", {})]:
        installed.append(_install_link(launcher, name, adopted.get(name)))
    primary = next(destination for destination, _, _ in installed
                   if destination.name == config["bins"][0])
    healthy, detail = _installed_healthy(primary, tool, store)
    if not healthy:
        for destination, previous, changed in installed:
            _restore_link(destination, previous, changed)
        _say(tool, f"⚠ installed launcher verification failed; previous links restored: {detail}")
        return
    for remaining in _repoint_shadows(cli, repo, tool, launcher):
        _say(tool, f"⚠ a {tool} command ahead of the launcher still shadows it: {remaining}")
    pinned = _run(cli, repo, tool, "pin")
    if pinned.returncode == 0:
        _say(tool, f"verified head · persistent launcher {primary} · process-ancestry pin active")
    elif "session-absent" in (pinned.stderr or ""):
        _say(tool, f"verified head · persistent launcher {primary} · no session ancestry to pin")
    else:
        _say(tool, f"verified head · persistent launcher {primary} · pin refused: {_detail(pinned)}")


def _main() -> None:
    repo = _repo()
    if repo is None:
        return
    cli = repo / "genesis/agentic/bin/bin-store"
    registry_path = repo / "genesis/agentic/bin-tools.json"
    registry = json.loads(registry_path.read_text(encoding="utf-8"))
    skipped = set(os.environ.get("BIN_STORE_SKIP_TOOLS", "").split(","))
    for tool, config in registry["tools"].items():
        if tool in skipped:
            continue
        try:
            _guard_tool(repo, cli, registry_path, tool, config)
        except Exception as error:  # noqa: BLE001 — one followed tool never blocks the session or another tool
            _say(tool, f"guard error (ignored): {error}")


def main() -> None:
    try:
        _main()
    except Exception as error:  # noqa: BLE001 — initialization failures must not block SessionStart
        _say("bin-store", f"guard initialization error (ignored): {error}")


if __name__ == "__main__":
    main()
