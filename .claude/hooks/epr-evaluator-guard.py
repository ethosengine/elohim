#!/usr/bin/env python3
"""SessionStart guard: is the NATIVE governance evaluator present and current?

WHY A SESSION HOOK AND NOT THE DEVFILE. `epr` installs into `$CARGO_HOME/bin`,
which lives in the container image — a workspace restart wipes it while the
cargo-pool build artifacts, which live on the /projects PVC, survive. So the
binary needs re-establishing per workspace, which is exactly a session concern.
It is deliberately NOT a devfile `postStart`: a postStart failure aborts
whole-workspace startup, and a cargo build is precisely the kind of step that
fails. Same reasoning applies here, which is why this hook is registered
`async: true` and can never fail a session — a cold slot means minutes of
compile, and no one should wait on it to start thinking.

WHAT IT PROTECTS. `epr` is the decision authority for the `.epr-meta` governance
plane at both live surfaces (the PreToolUse hook and the git gate). When it
cannot be resolved the Python evaluator decides and says so — an honest,
announced degradation, but one where the cross-host check does not run and a
divergence has no way to surface. That is the state the whole
`governance-plane-single-evaluator` habit exists to prevent, so leaving the
binary's presence to chance would quietly reinstate it.

TWO MODES, AND ONLY ONE OF THEM COMPILES.

  * THIS repo is where `epr` is developed, so here the binary is built from
    source and kept in step with the tree. That is dogfooding: the governance
    plane of the repo that authors the evaluator is governed BY that evaluator,
    at the commit it currently is, not at whatever commit someone last built.

  * A CONSUMER workspace should receive `epr` PREBAKED into its container image
    — compiled once at image build, prewarmed, static. There the sources are not
    present and a missing binary is a PACKAGING defect, not something to repair
    by compiling at session start. This guard therefore refuses to build when the
    crate is absent and says what is actually wrong, rather than attempting a
    source build that could not succeed and would be the wrong answer if it did.

The distinction is load-bearing rather than cosmetic: "compile it if missing" as
a universal rule would turn every consumer workspace into a build host and hide
image-packaging regressions behind a slow local success.

STALENESS IS PART OF PRESENCE — IN DEV MODE ONLY. Every decision is stamped with
the evaluator's content address, so a stale binary decides with old law and says
so in the ledger — detectable, but only after the fact. Rebuilding when sources
move closes that ahead of time. On a warm pool slot the reinstall is ~0.5s. With
no sources present there is nothing to be stale against, so a baked binary is
never second-guessed.

Resolution is delegated to `_lib.epr_client.resolve_binary()` — the SAME
function the gates use. A second resolver here could disagree with the client
about whether the evaluator exists, and two implementations of one predicate is
the exact fork this governance work spent itself removing.
"""
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

_here = Path(__file__).resolve()
REPO = None
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        REPO = _here
        break
    _here = _here.parent

# Crates compiled INTO the binary. A source change in any of them makes an
# installed `epr` stale; the runtime registries it merely reads are not here,
# because those are re-read per invocation and never baked in.
_SOURCE_ROOTS = ("elohim/eprfs", "elohim/epr", "elohim/epr-rea")
_SOURCE_SUFFIXES = (".rs", ".toml", ".lock")
_CRATE = "elohim/eprfs/epr-cli"
# A cold slot compiles the world; a warm one relinks in well under a second.
_BUILD_TIMEOUT = 900


def _say(line: str) -> None:
    print(f"EPR EVALUATOR: {line}")


def _newest_source_mtime() -> float:
    newest = 0.0
    for rel in _SOURCE_ROOTS:
        root = REPO / rel
        if not root.is_dir():
            continue
        for path in root.rglob("*"):
            if path.suffix not in _SOURCE_SUFFIXES:
                continue
            # Build outputs are not sources; a target dir under the tree would
            # make everything permanently "fresh" by moving with each build.
            if "target" in path.parts:
                continue
            try:
                newest = max(newest, path.stat().st_mtime)
            except OSError:
                continue
    return newest


def _pool_slot() -> str | None:
    """The cargo-pool slot for the eprfs workspace, so the install reuses warm
    artifacts instead of compiling into a throwaway temp dir (the `cargo install`
    default). Absence is fine — it just means a slower build."""
    try:
        proc = subprocess.run(["cargo-pool", "key"], cwd=str(REPO / "elohim/eprfs"),
                              capture_output=True, text=True, timeout=15)
        for line in proc.stdout.splitlines():
            if line.startswith("slot_dev="):
                return line.split("=", 1)[1].strip()
    except Exception:  # noqa: BLE001 — the slot is an optimization, never a requirement
        pass
    return None


def _install() -> tuple[bool, str]:
    env = dict(os.environ)
    # Native build: the ambient getrandom flag is for Holochain WASM and breaks it.
    env["RUSTFLAGS"] = ""
    args = ["cargo", "install", "--debug", "--locked", "--path", _CRATE]
    slot = _pool_slot()
    if slot:
        args += ["--target-dir", slot]
    try:
        proc = subprocess.run(args, cwd=str(REPO), env=env, capture_output=True,
                              text=True, timeout=_BUILD_TIMEOUT)
    except subprocess.TimeoutExpired:
        return False, f"build exceeded {_BUILD_TIMEOUT}s"
    except Exception as error:  # noqa: BLE001
        return False, f"could not run cargo install: {error}"
    if proc.returncode != 0:
        tail = [ln for ln in proc.stderr.splitlines() if ln.strip()][-1:] or ["(no stderr)"]
        return False, tail[0][:200]
    return True, ""


def _is_crate_home() -> bool:
    """True when this workspace CONTAINS the evaluator's source — i.e. it is the
    repo where `epr` is developed. Everywhere else the binary arrives prebaked in
    the image and building it locally would be the wrong repair."""
    return (REPO / _CRATE / "Cargo.toml").is_file()


def main() -> None:
    if REPO is None:
        return
    from _lib import epr_client  # noqa: PLC0415 — needs the sys.path bootstrap above

    dev_mode = _is_crate_home()
    on_path = shutil.which("epr")
    reason = None
    if not on_path:
        reason = "not on PATH"
    elif dev_mode:
        # Only meaningful where sources exist to be newer. A baked binary in a
        # consumer image has nothing to be stale against and is left alone.
        try:
            if Path(on_path).stat().st_mtime < _newest_source_mtime():
                reason = "stale (sources newer than the installed binary)"
        except OSError:
            reason = "unreadable"

    if reason is None:
        mode = "dev (built from this tree)" if dev_mode else "packaged (prebaked image)"
        _say(f"`{on_path}` present and current — native governance decisions active [{mode}].")
        return

    if not dev_mode:
        # Consumer workspace: the image should have carried it. Compiling here
        # would mask an image regression behind a slow local success.
        _say(f"⚠ {reason}, and this workspace has no evaluator source to build from.")
        _say("This is an IMAGE PACKAGING gap, not a local one: `epr` is meant to be baked into "
             "the workspace image. Governance still evaluates — the Python evaluator decides and "
             "the gates say so — but the cross-host check is not running. Report the image, or "
             "set EPR_BIN to a known-good binary.")
        return

    _say(f"{reason} — installing from this tree (native evaluator; the gates fall back to the "
         f"Python evaluator until this lands, announced, never silent).")
    started = time.monotonic()
    ok, detail = _install()
    elapsed = time.monotonic() - started
    if not ok:
        _say(f"⚠ install FAILED after {elapsed:.0f}s: {detail}")
        _say("Governance still evaluates — the Python evaluator decides and the gates say so. "
             "What is missing is the cross-host check. Fix with: "
             "RUSTFLAGS='' cargo install --debug --locked --path elohim/eprfs/epr-cli")
        return

    resolved = epr_client.resolve_binary()
    _say(f"installed in {elapsed:.0f}s → {resolved or shutil.which('epr')}")


if __name__ == "__main__":
    try:
        main()
    except Exception as error:  # noqa: BLE001 — a guard must never break session startup
        print(f"EPR EVALUATOR: guard error (ignored): {error}")
