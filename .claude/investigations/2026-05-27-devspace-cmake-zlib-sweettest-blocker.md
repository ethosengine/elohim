# Investigation: Che devspace cmake/zlib breaks sweettest-check in pre-push gate

**Filed:** 2026-05-27 by Claude during the JivaVolumePolicy-shem placement migration session.
**Resolved:** 2026-05-27 — diagnosed + Dockerfile fix written (see Resolution at bottom).
**Severity:** Local-only — blocks the husky pre-push gate but does NOT affect CI builds. CI uses Nix-managed cmake/openssl and passes cleanly.
**Last known good:** Image build 2026-05-17 — the patch step was *present* in the Dockerfile but only patched the wrong registry source path; the bug was latent until any sweettest-touching commit triggered pre-push in the devspace.
**Workaround in place (pre-fix):** `HUSKY=0 git push` for CI-only commits that don't touch sweettest paths (per memory `feedback_husky_bypass_for_ci_only_changes`).

---

## Symptom

When the husky pre-push gate triggers `sweettest-check` (any commit touching `elohim/holochain/dna/<dna>/zomes/**/*.rs` or `elohim/holochain/tests/sweettest/**`), the `cargo check --tests` invocation fails during `datachannel-sys`'s cmake build step:

```
CMake Error at /usr/share/cmake/Modules/FindOpenSSL.cmake:186 (set_property):
  The link interface of target "OpenSSL::Crypto" contains:
    ZLIB::ZLIB
  but the target was not found.

CMake Generate step failed.  Build files cannot be regenerated correctly.

thread 'main' panicked at /opt/rust/cargo/registry/.../cmake-0.1.58/src/lib.rs:1132:5:
command did not execute successfully, got: exit status: 1

build script failed, must exit now
```

The same error fires twice — once for `OpenSSL::Crypto`, once for `OpenSSL::SSL`.

---

## What's been ruled out

These were verified on the Che devspace pod (`elohim-devspace`, 2026-05-27):

| Hypothesis | Evidence against |
|---|---|
| Missing zlib library | `/lib64/libz.so` and `/lib64/libz.so.1` both present (`ldconfig -p \| grep libz`) |
| Missing zlib headers | `/usr/include/zlib.h` exists |
| Missing libssl-dev | System OpenSSL 3.5.1 present with headers; `pkg-config --modversion openssl` returns `3.5.1` |
| `OPENSSL_NO_VENDOR=1` would help | Tried — `datachannel-sys` ignores it and still builds vendored OpenSSL 3.6.2 from source. The cmake invocation for `libdatachannel` then points at that vendored install, and that's where FindOpenSSL.cmake adds the broken interface. |
| sccache poisoning | Bypassed with `RUSTC_WRAPPER=""` and the cmake error still fires — this is below the sccache layer. |
| `BINDGEN_EXTRA_CLANG_ARGS` related | Already set correctly to `/usr/lib/clang/20/include` in the gate; the failure is in cmake, not bindgen. |

---

## The chain

1. `cargo check --manifest-path elohim/holochain/tests/sweettest/Cargo.toml --tests` triggers compile of test deps
2. Test deps pull in `datachannel-sys 0.23.0+0.23.2` (transitively, via the holochain `network` features)
3. `datachannel-sys`'s build script always builds a vendored OpenSSL 3.6.2 from source first (this completes successfully — perl + gcc run cleanly to ~1500 lines of output)
4. Then `datachannel-sys`'s build script invokes `cmake` on `libdatachannel`'s CMakeLists, passing `-DOPENSSL_ROOT_DIR=<vendored-install>` etc.
5. `libdatachannel`'s CMakeLists calls `find_package(OpenSSL REQUIRED)`
6. `/usr/share/cmake/Modules/FindOpenSSL.cmake` (cmake 3.30.5) runs. At line 186:
   ```cmake
   set_property( TARGET ${target} APPEND PROPERTY INTERFACE_LINK_LIBRARIES ZLIB::ZLIB )
   ```
   It sets `ZLIB::ZLIB` as a link interface on `OpenSSL::Crypto` AND `OpenSSL::SSL` — but the `ZLIB::ZLIB` target doesn't exist because `find_package(ZLIB)` wasn't called by libdatachannel's CMakeLists.
7. cmake's "Generate" phase fails because of the unresolved target dependency.

Specifically, FindOpenSSL.cmake line 139 has a conditional `find_package(ZLIB)` call but its condition isn't satisfied in this configuration (likely because OpenSSL is being consumed as a static lib via `OPENSSL_USE_STATIC_LIBS=TRUE` and the find-module is taking a path that doesn't auto-find zlib first).

---

## Directions worth exploring

These are ordered by likely effort, lowest first. Pick the one that fits the time budget and the operator's appetite for devspace-image work.

### A. Force libdatachannel's cmake to skip the zlib link interface

Pass `-DCMAKE_DISABLE_FIND_PACKAGE_ZLIB=TRUE` to cmake when libdatachannel is configured. Path: `datachannel-sys`'s `build.rs` constructs the cmake invocation; we'd need either (1) a fork of `datachannel-sys` that adds this knob, (2) a `[patch]` override in Cargo.toml pointing at such a fork, or (3) a way to inject extra cmake args via env (some cmake-rs versions honor `CMAKE_<NAME>_ARGS` env or `CMAKE_PREFIX_PATH` tricks). **Caveat**: silencing the zlib link interface may break runtime if libdatachannel actually needs zlib for compression in WebRTC. Verify before shipping.

### B. Make `find_package(ZLIB)` succeed at FindOpenSSL.cmake's point in the run

Pre-create the `ZLIB::ZLIB` imported target before libdatachannel's cmake runs. cmake's FindOpenSSL.cmake line 139 has `find_package(ZLIB)` — figure out why it's skipped or failing silently, then make it succeed. Likely needs a `Find{ZLIB,zlib}-config.cmake` file shipped with zlib1g-dev (Debian/Ubuntu doesn't ship one by default — they rely on cmake's built-in FindZLIB.cmake). Maybe install `cmake-data` or check whether `apt-get install zlib1g-dev` is delivering the cmake config.

### C. Downgrade cmake to a version where FindOpenSSL doesn't add the ZLIB::ZLIB link interface

cmake 3.30 introduced (or made stricter) the `set_property(... INTERFACE_LINK_LIBRARIES ZLIB::ZLIB)` line. Earlier cmake versions (≤ 3.20-ish) might not have this. Downgrade in the Che devspace image to a working cmake version. **Caveat**: other builds in the same devspace might depend on cmake ≥ 3.30 features.

### D. Use system OpenSSL instead of the vendored 3.6.2 path

`datachannel-sys` Cargo.toml has a `vendored` feature flag (line 37). If the sweettest dep graph could disable that feature for the local build, datachannel-sys would link against system OpenSSL 3.5.1 and skip the entire vendored + cmake path. **Investigation needed**: which crate enables `datachannel-sys/vendored`? Likely upstream Holochain or one of its network crates. Patching to `default-features = false` may unblock if the consumer doesn't truly require static linking.

### E. Replace cmake at runtime via a wrapper script

The `datachannel-sys` build script calls `cmake` via the `cmake` crate which `exec`s the binary. We could `PATH`-shadow `/usr/bin/cmake` with a wrapper that injects `-DCMAKE_DISABLE_FIND_PACKAGE_ZLIB=TRUE` into any invocation that targets `libdatachannel`. Hacky but contained — could live in the Che devspace image as a startup script that puts the wrapper first on `PATH`.

### F. Skip sweettest-check entirely in the local pre-push for the Che devspace

Add a Che-specific opt-out so the local pre-push doesn't run sweettest-check, while the CI path keeps running it (where Nix toolchain works). Look in `.husky/pre-push` around the sweettest-check trigger at line ~252. The "Che devcontainer" comment already exists at line 377 — extend the existing pattern: detect Che, skip the cmake-eligible gate, leave a notice on stdout. **Trade-off**: loses pre-push catching of zome/sweettest drift on Che devspaces — CI catches it on the next build instead, with a longer feedback loop.

---

## Where to look in the repo

| File | Why |
|---|---|
| `.husky/pre-push` lines 252-253, 375-393 | The sweettest-check trigger pattern and its execution block. Already has Che-aware comments. |
| `elohim/holochain/tests/sweettest/Cargo.toml` | Where the sweettest test crate's dep tree starts. `datachannel-sys` arrives transitively from holochain's network deps. |
| `Cargo.lock` (repo root + sweettest crate) | Locked `datachannel-sys 0.23.0+0.23.2` and its transitive `openssl-sys` version. Useful for "did this regress when X was bumped" analysis. |
| `/opt/rust/cargo/registry/src/nexus.../datachannel-sys-0.23.0+0.23.2/build.rs` (in devspace) | The build script that builds vendored OpenSSL then invokes cmake. Read to understand the env-var surface and cmake invocation shape. |
| `/usr/share/cmake/Modules/FindOpenSSL.cmake` lines 130-200 (in devspace) | Where `ZLIB::ZLIB` is set on the OpenSSL targets. Confirm the conditional that's supposed to call `find_package(ZLIB)` first. |
| memory `feedback_husky_bypass_for_ci_only_changes.md` | The documented bypass pattern for this class of failure. |

---

## Acceptance

The investigation is complete when one of the following is true:

- A small reproducible repo-level change makes `sweettest-check` pass on the Che devspace without breaking CI. Land it; close.
- The Che devspace image is patched (via the `che-devworkspaces` repo) so the cmake/zlib gap is resolved. Document the image SHA that fixes it.
- A clean opt-out lands in `.husky/pre-push` that skips sweettest-check on Che (detected via env or hostname pattern) AND adds a sweettest-check job to the CI pipeline graph (already there) so the catch isn't lost. Document the trade-off.
- Decision: "this is acceptable, HUSKY=0 is the documented workaround" — but then add an inline `pre-push` notice that explicitly says "if sweettest-check fails with cmake/ZLIB::ZLIB AND your commit doesn't touch sweettest paths, see investigation 2026-05-27-devspace-cmake-zlib-sweettest-blocker.md" so future operators don't re-investigate.

---

## Context links

- The session where this surfaced: pre-push attempts on commits `629df8a74` (placement migration) and `e1744d616` (gate cleanup), sprint `sprint/cross-pillar-cleanup`. Final disposition was `HUSKY=0 git push` after 3 attempts.
- Related: memory `feedback_sccache_cache_corruption_recovery` — separate sccache 0.15.0 bug that was bypassed in the same gate-cleanup commit.

---

## Resolution (2026-05-27)

**Root cause:** The Dockerfile patch at `containers/udi-plus-mem-rust-nix/Dockerfile:94-116` patches the wrong registry source directory.

The patch step's `cargo fetch` ran inside a temp project with no `.cargo/config.toml`, so it used the default `crates.io` index and extracted source into `/opt/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/datachannel-sys-0.23.0+0.23.2/`. The subsequent `find … -print -quit` patched *that one path* and exited.

At devspace runtime, the elohim project's `/projects/elohim/.cargo/config.toml` does:
```toml
[source.crates-io]
replace-with = "elohim-mirror"
[source.elohim-mirror]
registry = "sparse+https://nexus.ethosengine.com/repository/cargo/"
```
Cargo extracts the **same tarball** into a **different** path: `/opt/rust/cargo/registry/src/nexus.ethosengine.com-eec02b636f750b69/datachannel-sys-0.23.0+0.23.2/`. That path was never patched.

Side-by-side confirmation on the live devspace (2026-05-27):
```
$ grep -n 'find_package(OpenSSL\|find_package(ZLIB' /opt/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/datachannel-sys-0.23.0+0.23.2/libdatachannel/CMakeLists.txt
452:	find_package(ZLIB REQUIRED)
453:	find_package(OpenSSL REQUIRED)

$ grep -n 'find_package(OpenSSL\|find_package(ZLIB' /opt/rust/cargo/registry/src/nexus.ethosengine.com-eec02b636f750b69/datachannel-sys-0.23.0+0.23.2/libdatachannel/CMakeLists.txt
452:	find_package(OpenSSL REQUIRED)
```

That single missing line is the bug.

**Fix applied (in che-devworkspaces repo, branch `sccache-014-downgrade-rca-1225`):**

`containers/udi-plus-mem-rust-nix/Dockerfile`:
1. Fetch step now writes `/tmp/fetch/.cargo/config.toml` pointing `crates-io` at the Nexus mirror, matching the runtime project config. Cargo extracts source into the same registry path runtime cargo uses.
2. Patch step changed from `find … -print -quit` (first match) to a `while read` loop over all matches, so any future mirror addition still gets patched.

**Handoff to operator:**
- Jenkins pipeline: `devspaces-udi-plus-mem-rust-nix` at https://jenkins.ethosengine.com/job/devspaces-udi-plus-mem-rust-nix/
- After the change lands on a buildable branch, trigger that job; once green, restart the elohim devspace to pull the new image.
- The udi-plus-mem-rust-nix Dockerfile change does not invalidate the upstream `udi-plus-mem` or `udi-plus` images — only the leaf node needs rebuilding.

**Why CI is unaffected:** The Jenkins `ci-builder-nix` image (Debian + Nix-installer) provides cmake/openssl/zlib via `nix develop`, so the FindOpenSSL.cmake codepath that adds `ZLIB::ZLIB` to the link interface doesn't fire there. Only the devspace image (Fedora + system cmake + vendored OpenSSL via openssl-src crate) is affected.

**Interim unblock if needed before image rebuild:** apply the patch in-pod:
```sh
sed -i 's|^\tfind_package(OpenSSL REQUIRED)$|\tfind_package(ZLIB REQUIRED)\n\tfind_package(OpenSSL REQUIRED)|' \
  /opt/rust/cargo/registry/src/nexus.ethosengine.com-eec02b636f750b69/datachannel-sys-0.23.0+0.23.2/libdatachannel/CMakeLists.txt
```
This survives until cargo re-extracts the tarball (rare unless the cache is cleaned), but is overwritten on devspace recreation. Use only as a bridge.
