---
id: task-as-e2-report
status: landed
cites: []
landed_commit: 1b76c2327  # without the review fix d4cf5763b; it rides the E4 storage closure (branch artifact-store/e4)
verified_by: "astra review evidence/e2-astra-review.md; L3 combined gate evidence/gate-L3-storage.log; L3 landing evidence/push-L3.log"
concerns: see Deviations
actor: agent:e2@claude-sonnet-5-5
---

# Track E2: A11 tolerant reader

## What changed
- `eprfs-storage/src/tier_blob.rs`: `decode_manifest` already read the epoch before the epoch-2 schema and answered any epoch other than 2 as `UnsupportedEpoch` (printed `manifest epoch N is unsupported`); kept. `TierBlobManifest` now carries `deny_unknown_fields` (D7: unknown field outside `ext` at a known epoch refuses) and an optional `ext: Option<ManifestExt>`; `ManifestExt` holds the opaque `ext` content as DAG-CBOR bytes, so the manifest stays `Eq`, and re-encode writes it back. `put_tier_blob` sets `ext: None`.
- `eprfs-storage/src/lib.rs` re-exports `ManifestExt`; `Cargo.toml` adds `ipld-core = "0.4"` (already in the lockfile, used by `elohim-epr`).
- `tests/local_tier_blob.rs`: three new tests.

## Tests (`cargo test -p eprfs-storage --test local_tier_blob`: 13 passed, 0 failed)
- `epoch_3_manifest_is_held_with_version_named`: ok
- `unknown_ext_bytes_survive_roundtrip`: ok (decode keeps ext; re-encode reproduces the stored bytes and manifest CID; an unknown field outside `ext` is `ManifestInvalid`)
- `held_manifest_is_never_a_hit_and_records_unavailable_use`: ok (epochs 1, 3, 4, u32::MAX; read and stream both `Unavailable`)
- existing `epoch_1_manifest_is_held_not_read_and_not_a_parse_error`: ok

## Gate
`just gate eprfs`; log `/projects/elohim/genesis/docs/superpowers/sprints/2026-11-07-artifact-store/evidence/gate-E2-eprfs.log` (`EXIT=0`). The first run (`gate-E2-eprfs.run1-red.log`, `EXIT=1`) failed only `epr-cli` `live_corpus_migrate_reports_zero_pending` because this report lacked an `id:` slug; fixed and rerun green.

## Deviations
- Beyond the stated write-set: `put_tier_blob` one line (`ext: None`), `lib.rs` re-export, `Cargo.toml`/`Cargo.lock` dependency. All required by the new field; E3's `read_tier_blob`-area hunk should not conflict but Cargo.toml/lib.rs may need a trivial merge.
- Not renamed to `UnsupportedManifestVersion`; the existing `UnsupportedEpoch` variant and message stay (the epoch-1 test pins them).
- "Records an unavailable Use": Use recording lives in `eprfs-peer::stream_local_blob` (outside write-set and not a dev-dependency of eprfs-storage). The storage test asserts the `Unavailable` outcome that function records as `draw:unavailable` for every non-hit; no peer-level test was added.
- Retention is canonical DAG-CBOR re-encode of the `ext` value (byte-exact for canonical writers; an explicit CBOR `null` ext decodes as absent). The nested `DerivationRecord` still discards its own `ext` on decode (derive.rs, Track A); derive.rs untouched.

## Landing gap found 2026-11-09

L3 (1b76c2327) carried E2 as it stood before the astra review fix. The branch's d4cf5763b ("refuse
explicit null ext instead of normalizing it away") and its test
`explicit_null_ext_is_refused_not_normalized` never reached dev. On dev, an explicit null `ext`
decodes as absent and re-encodes without the key, changing the manifest's bytes and CID. Only a
non-canonical (forged) manifest carries one.

The fix is re-applied as d77171ffd on branch `artifact-store/e2-null-ext` (14/14 in
`local_tier_blob`). It cannot land alone: eprfs's publish precheck requires an `eprfs-storage`
version bump (0.3.6 → 0.3.7) for a changed `tier_blob.rs`. The bump rewrites `elohim/Cargo.lock`,
which dispatches elohim-edge (an alpha roll) and the elohim-storage gate. It rides the E4 storage
closure (branch `artifact-store/e4`, 2026-11-09). Found while checking as-e2 for unlanded patches before removing it.

