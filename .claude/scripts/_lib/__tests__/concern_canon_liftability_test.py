"""Concern-canon liftability FRESHNESS half (seam-concern-contract plan, task P0.4).

Run: python3 .claude/scripts/_lib/__tests__/concern_canon_liftability_test.py   (exit 0 = pass)
     python3 .claude/scripts/_lib/__tests__/concern_canon_liftability_test.py --write-fixture

The Rust half (`elohim/epr/tests/concern_canon_liftability.rs`) proves that the pinned bytes
re-address as CIDv1(dag-cbor, sha2-256) through the real `elohim_epr::cid::compute_cid` path —
the P5 brit lift is a codec wrap of bytes that already exist, never a rewrite. It reads a FROZEN
fixture, because Rust has no YAML in this crate's dep tree and adding one to a publishable leaf
crate to read a `.claude/` tooling file would invert the dependency.

This half closes the gap that freezing opens. Without it the pair is a MIRROR: the Rust test would
happily stay green against a fixture that no longer resembles the live canon row. Here the body is
regenerated from `.claude/epr-meta/concerns.yaml` through `epr_meta.policy_content_hash`'s exact
canonicalization and asserted byte-identical to the fixture — so a canon edit that leaves the
fixture behind fails LOUD, on this side, at the moment the canon moves.

It also asserts the property the whole two-home split rests on: BOTH registries hash their rows
with the SAME canonicalization, so a class that graduates from concerns.yaml to policies.yaml
(when P4.1 registers its validator) keeps its content address and its lineage is a supersession
edge, never a re-mint.
"""
import hashlib
import json
import sys
from pathlib import Path

here = Path(__file__).resolve()
REPO = None
for _ in range(8):
    if (here / ".claude" / "scripts" / "_lib").is_dir():
        REPO = here
        sys.path.insert(0, str(here / ".claude" / "scripts"))
        break
    here = here.parent

import yaml  # noqa: E402

from _lib import epr_meta  # noqa: E402

CONCERNS = REPO / ".claude" / "epr-meta" / "concerns.yaml"
POLICIES = REPO / ".claude" / "epr-meta" / "policies.yaml"
FIXTURE = REPO / "elohim" / "epr" / "tests" / "vectors" / "concern_canon_row_c0.json"
FIXTURE_ROW_ID = "c0-plane-location"

_passed = 0


def check(label, cond):
    global _passed
    assert cond, f"FAIL: {label}"
    _passed += 1
    print(f"  ✅ {label}")


def canonical_body(row: dict) -> bytes:
    """The EXACT bytes `epr_meta.policy_content_hash` hashes — one canonicalization, never two."""
    body = {k: v for k, v in row.items() if k not in epr_meta._HASH_EXCLUDE_KEYS}
    return json.dumps(body, sort_keys=True, separators=(",", ":"),
                      ensure_ascii=True).encode("ascii")


def rows(path: Path, key: str) -> list:
    return (yaml.safe_load(path.read_text()) or {}).get(key, []) or []


def main() -> int:
    concerns = rows(CONCERNS, "concerns")
    policies = rows(POLICIES, "policies")

    print("concern-canon liftability (freshness / anti-mirror half):")

    # ── 1. the fixture IS the live row, byte for byte ────────────────────────────────────────
    row = next((r for r in concerns if r.get("id") == FIXTURE_ROW_ID), None)
    check(f"canon row `{FIXTURE_ROW_ID}` present in concerns.yaml", row is not None)
    body = canonical_body(row)

    if "--write-fixture" in sys.argv:
        FIXTURE.parent.mkdir(parents=True, exist_ok=True)
        FIXTURE.write_bytes(body)
        print(f"  ✍ wrote {FIXTURE.relative_to(REPO)} ({len(body)} bytes)")

    check("Rust liftability fixture exists", FIXTURE.is_file())
    check(
        "fixture bytes == the live canon row's canonical body (re-run with --write-fixture if "
        "the row moved deliberately)",
        FIXTURE.read_bytes() == body,
    )

    # ── 2. the pin the Rust test transcribes is the pin the registry carries ─────────────────
    digest = "sha256:" + hashlib.sha256(body).hexdigest()
    check("row contentHash == sha256(canonical body)", row.get("contentHash") == digest)
    rust = (REPO / "elohim" / "epr" / "tests" / "concern_canon_liftability.rs").read_text()
    check("Rust test's PINNED_CONTENT_HASH matches the registry pin", digest in rust)

    # ── 3. every canon row is pinned + declares its brit standing shape ──────────────────────
    STANDING = {"record", "attestation", "fold", "pin"}
    canon = concerns + [p for p in policies if str(p.get("id", "")).startswith("c2-")]
    check("canon spans 16 classes across the two homes", len(canon) == 16)
    for r in canon:
        rid = r.get("id")
        check(f"{rid}: pinned", isinstance(r.get("contentHash"), str))
        check(f"{rid}: standing shape in {sorted(STANDING)}", r.get("standing") in STANDING)
        check(f"{rid}: resolution_mode declared", r.get("resolution_mode") == "pinned")
        check(f"{rid}: pin reproduces (epr-meta-pin.py canonicalization)",
              epr_meta.policy_content_hash(r) == r["contentHash"])

    # ── 4. ONE canonicalization across BOTH homes — so graduation keeps the address ──────────
    # A class moving concerns.yaml -> policies.yaml (P4.1 registers its validator) must be a
    # supersession edge over a stable content address, never a re-mint under a second scheme.
    for r in policies:
        check(f"policies.yaml `{r.get('id')}`: same canonicalization",
              epr_meta.policy_content_hash(r) == r.get("contentHash"))

    # ── 5. no canon row advertises enforcement it does not serve (the canon's own C7) ────────
    for r in concerns:
        check(f"{r['id']}: no phantom validator/class key",
              "validator" not in r and "class" not in r)

    print(f"\n{_passed} checks passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
