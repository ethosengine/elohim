"""Cross-language parity: the Slice-2 `eprfs cid --body --short` CLI must emit, for every corpus
file, exactly what the Python `cite_graph.fingerprint()` fast-path emits — proving the cite
fingerprint is one sha2-256 digest rendered two ways (Rust CID short-form ↔ Python hashlib).

Skip convention (mirrors brit's cite_parity.rs oracle test): if the `eprfs` binary is not built,
print a skip line and exit 0 — parity is unprovable without the single-source encoder, not failed.

Run: python3 .claude/scripts/_lib/__tests__/cite_cid_parity_test.py (exit 0 = pass or skip)."""
import os
import shutil
import subprocess
import sys
from pathlib import Path

here = Path(__file__).resolve()
repo = here
for _ in range(8):
    if (repo / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(repo / ".claude" / "scripts"))
        break
    repo = repo.parent

from _lib import cite_graph as cg  # noqa: E402


def _find_eprfs() -> str | None:
    """Locate the `eprfs` CLI binary: PATH, env CARGO_TARGET_DIR, or the known pool/workspace slots."""
    onpath = shutil.which("eprfs")
    if onpath:
        return onpath
    candidates = []
    if os.environ.get("CARGO_TARGET_DIR"):
        candidates.append(Path(os.environ["CARGO_TARGET_DIR"]) / "debug" / "eprfs")
    candidates += [
        Path("/projects/.cargo-target-pool/family/integ/crates/dev/debug/eprfs"),
        repo / "elohim" / "eprfs" / "target" / "debug" / "eprfs",
    ]
    for c in candidates:
        if c.is_file() and os.access(c, os.X_OK):
            return str(c)
    return None


def _corpus_files() -> list[Path]:
    """The shared cite corpus (brit's fixtures — the same md the oracle test walks)."""
    root = repo / "elohim" / "brit" / "brit-epr" / "tests" / "fixtures" / "cite_corpus"
    return sorted(root.rglob("*.md")) if root.is_dir() else []


def main() -> int:
    binary = _find_eprfs()
    if binary is None:
        print("  ⏭  eprfs CLI not built; skipping cite↔CID parity (build eprfs-cli to enable)")
        return 0
    files = _corpus_files()
    if not files:
        print("  ⏭  cite corpus absent; skipping")
        return 0
    checked = 0
    for f in files:
        py = cg.fingerprint(f)
        cli = subprocess.run(
            [binary, "cid", str(f), "--body", "--short"],
            capture_output=True, text=True, check=True,
        ).stdout.strip()
        assert cli == py, f"PARITY FAIL {f.name}: cli={cli} python={py}"
        checked += 1
        print(f"  ✅ {f.name}: {py} (cli==python)")
    print(f"\n  {checked} corpus files: eprfs cid --body --short == cite_graph.fingerprint() ✅")
    return 0


if __name__ == "__main__":
    sys.exit(main())
