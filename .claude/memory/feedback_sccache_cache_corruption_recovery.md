---
name: sccache-cache-corruption-recovery
description: "When sccache produces null-byte output (clippy 'unclosed delimiter' errors at random source positions), the underlying S3 cache likely has corrupted entries. SCCACHE_RECACHE=1 only repaves COMPILATION entries — NOT probe-response entries. A poisoned probe blob requires direct S3 DELETE (the specific key) or full bucket wipe via Garage admin. Procedure verified on 2026-05-17."
metadata:
  node_type: memory
  type: feedback
---

When sccache surfaces cache corruption — symptoms include `error: unknown start of token: \u{0}` from rustc, clippy errors of the form `error: this file contains an unclosed delimiter` pointing at cached source positions, or build failures that disappear when `RUSTC_WRAPPER=""` is set — the corrupted cache entries need to be cleared. This memory captures only the verified recovery steps. The deeper question of *why* the cache became corrupted in the first place, or why some downstream invocation paths surface it differently, is **not isolated** as of this writing.

## Verified findings

### `SCCACHE_RECACHE=1` only repaves compilation entries

sccache stores two kinds of cache entries:

1. **Compilation entries** — the result of running `rustc` against a specific source + flags + env hash. These are what most people think of as "the cache."
2. **Probe-response entries** — short answers to cargo's metadata-probe invocations like `rustc - --crate-name ___ --print=file-names ...`, used by cargo to detect rustc capabilities and crate features.

`SCCACHE_RECACHE=1` forces ignore-read-cache + write-fresh-entries on compilation invocations. **It does NOT touch probe-response entries.** A poisoned probe blob will survive any number of RECACHE builds. This was the surface gap on the 2026-05-17 first attempt: the operator ran RECACHE expecting full-bucket repair; one corrupted probe blob persisted because it was probe-keyed, not compilation-keyed.

### Recovery procedures (in order of severity)

**Option A — Targeted S3 DELETE of the specific poisoned key**

When the failure message references a specific cache key (e.g. `5/1/3/5139ed8f8204b9503a9eac24ce50ab2580396f662bc16c54149e39cc476d5dea`), the cleanest recovery is direct deletion of that S3 object:

```bash
# via garage CLI (from a node with cluster access)
garage bucket delete-object sccache-elohim "5/1/3/5139ed8f8204b9503a9eac24ce50ab2580396f662bc16c54149e39cc476d5dea"

# OR via S3 API with appropriate Garage admin credentials
aws --endpoint-url http://garage.ethosengine.svc.cluster.local:3900 \
  s3api delete-object --bucket sccache-elohim \
  --key "5/1/3/5139ed8f8204b9503a9eac24ce50ab2580396f662bc16c54149e39cc476d5dea"
```

After the DELETE, sccache treats the next probe with that hash as a cache miss and writes a fresh entry.

**Option B — Full bucket wipe (when the corruption surface is unknown)**

When you don't know which specific blobs are corrupted, the cleanest reset is to wipe the entire bucket via Garage admin. Verified on 2026-05-17: 47,386 objects deleted from `s3://sccache-elohim/` via Garage admin; bucket `ls` returned empty; HEAD on the previously-poisoned probe key returned 404. The next build was a full cold compile, but every entry written afterward was fresh.

Wipe is expensive (cold cache for everyone reading from the bucket afterward) but unambiguous.

**Option C — `RUSTC_WRAPPER=""` escape hatch (in-band, one-off)**

For a single build invocation that needs to ship NOW without dealing with the cache:

```bash
RUSTFLAGS="" RUSTC_WRAPPER="" cargo build ...
```

This bypasses sccache entirely. The build pays full compile cost. Does NOT repair anything; just sidesteps the broken cache for this one invocation.

## Unisolated: why downstream builds via cargo wrapper produce null bytes

After the bucket wipe on 2026-05-17, a focused re-verify (`cargo clean -p elohim-epr && cargo build -p elohim-epr` with `RUSTC_WRAPPER=sccache`) failed with `error: unknown start of token: \u{0}` even though the cold-cache full-workspace release build immediately prior had written 275 fresh entries with 0 read errors and 0 write errors.

Root cause **NOT** isolated as of this writing. **Plausible candidates** that should be bisected in a follow-up session before any concrete claim is made:

- Stale `target/` `.rmeta` from before the wipe still being consumed by the per-crate build (`cargo clean -p X` only cleans X's artifacts, not its deps')
- Cargo passing a response file (`@/tmp/rustc-XXXX`) whose contents are empty/null due to a transient disk/tmpfs issue
- The system-wide `RUSTFLAGS='--cfg getrandom_backend="custom"'` interacting with rustc in a way that produces output rustc later rejects, being misread as null bytes
- A pipe/stdin/stdout handling difference between sccache invoked via cargo's `RUSTC_WRAPPER` path versus sccache invoked directly (the symptom in the prior session was "direct sccache call works, cargo-wrapped doesn't" — that asymmetry points toward this category, not toward a content-return bug)

A claim like "sccache 0.15.0 returns zero bytes on cache miss" was raised during the 2026-05-17 investigation and is **internally inconsistent** with the same session's observation that 275 cache-miss writes completed successfully (every successful miss → write requires sccache to have called real rustc, gotten real output, and stored it — a "zero bytes on miss" mechanism would have prevented those writes). Do not memorialize that hypothesis without a standalone reproducer (empty bucket + minimal crate + the cfg flag → observe what sccache actually returns).

## How to apply

1. **When you see `\u{0}` token errors or "unclosed delimiter" at suspicious positions in compile output**, suspect cache corruption first.
2. **Try the targeted DELETE (Option A)** if the failure cites a specific cache key.
3. **Escalate to full bucket wipe (Option B)** when the corruption surface is unknown or DELETE doesn't reach.
4. **Use `RUSTC_WRAPPER=""` (Option C)** as an in-band escape hatch when you need to ship a single build and the cache is too compromised to recover from in this moment.
5. **Don't memorialize unverified root-cause claims.** If a hypothesis can't be reproduced standalone with a minimal example, the memory should say "unisolated" — not "sccache has bug X" — to keep future agents from acting on bad ground.

Related: 2026-05-17 graph-native sprint where the poisoning first surfaced; 2026-05-18 cargo-registry rollout T2 where the wipe procedure was verified and the unisolated downstream symptom was documented honestly rather than papered over.
