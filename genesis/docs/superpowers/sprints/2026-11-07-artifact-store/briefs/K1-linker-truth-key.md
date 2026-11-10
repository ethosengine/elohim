---
id: artifact-store-brief-k1-linker-truth-key
status: superseded
superseded_by: host-compatibility-foundation-plan K1
---

# Brief K1: the derivation key names the linker that actually links

> **Superseded (2026-11-10)** by item K1 of `genesis/docs/superpowers/plans/2026-11-10-host-compatibility-foundation-plan.md` (steward rulings H1–H7): the derivation input keeps source and cargo configuration only, and host tool identity moves into the method, derived from the build. The evidence below (rust-lld links; `ld` is the probable key-split cause; glibc's `crt1.o`/`Scrt1.o` are the soundness gap) still stands as input to that item; the rulings below do not.

**Why.** F2r's cross-device fetch missed because shem's and ethosengine's keys differ. The F2 method
and the git-tree view are byte-identical; only `tool:ld` differs (shem `GNU ld version
2.41-63.el10.alma.1`, ethosengine `2.41-63.el10`). But the system `ld` never touches the output.
shem's built `elohim-storage` carries `.comment` = `Linker: LLD 22.1.2 (… rust-lld bundled with
rustc 1.96.1 31fca3adb)` + `GCC: (GNU) 14.3.1 20251022 (Red Hat 14.3.1-4)`: rustc 1.96 on
`x86_64-unknown-linux-gnu` links through `cc` with its self-contained `rust-lld`. The key is
over-specified on `ld` (a false miss) and under-specified on what the image really contributes:
gcc's startup objects and glibc's link stubs.

**Integrity class (ethosengine, 2026-11-09).** The key is unsound as well as over-specified. It
keys `cc` and `ld` but not glibc's startup objects. Two devices with the same `ld` but different
glibc-devel builds would compute the same key for different binaries, and the fetcher would install
the producer's build as its own. Verify-before-install cannot catch this: it proves the bytes equal
the producer's `record.output`, not that they equal what this device would have built. shem and
ethosengine escaped only because their `ld` builds happen to differ too. The measured split:
`crt1.o` and `Scrt1.o` differ (glibc-devel 2.39-128.el10_2.alma.1 vs 2.39-145.el10_2); gcc
objects and `.comment` are identical. Until K1 lands, F2 reuse is safe only between devices whose
images are known identical.

**Owner:** ethosengine implements from this brief once the E4 storage closure lands; shem
reviews (astra) and integrates.

**Where.** `elohim/eprfs/epr-cli/src/artifact.rs`: `KEYED_TOOLS` (:566) and `tool_lines` (:590),
plus `put`. Lands after the E4 storage closure (E4 also edits `artifact.rs`). eprfs gate only; no
lockfile change, no alpha roll.

## Rulings

1. **Key the effective linker, not `ld` on PATH.** The default for the toolchain and target the
   method records is self-contained `rust-lld`, whose identity is already pinned by the method's
   `rustc -vV`. Key it as `tool:linker\trust-lld\t<rustc commit>`. If RUSTFLAGS or the cargo config
   names a linker (`-C linker=`, `-C linker-flavor=`, `-C link-self-contained=`,
   `-C link-arg=-fuse-ld=…`, `[target.*] linker`), key the tool it names (resolved path plus
   `--version` CID, as today). A target or toolchain this rule does not know keys `ld` exactly as
   today: no narrowing without knowledge.
2. **Key the image's link inputs.** Add one line per startup object and stub that `cc` feeds the
   link: `crt1.o Scrt1.o crti.o crtn.o crtbeginS.o crtendS.o libgcc.a libgcc_s.so
   libc_nonshared.a libc.so`, as `tool:link-input:<name>\t<canonical path>\t<byte CID>`, resolved
   through `cc -print-file-name=` under the recorded PATH. `cc --version` stays keyed.
3. **Falsifier at `put` (integrity class).** After the build, `put` reads the output's ELF
   `.comment`. If the method keyed `rust-lld` and `.comment` does not name `Linker: LLD`, or it keyed
   another linker and `.comment` names LLD, `put` refuses with `unrecorded: the method's linker
   claim does not match the built bytes`. Nothing is recorded under a key that mis-describes its
   linker.
4. **Old keys stop matching.** Every recorded method key changes once. The F2r producer record
   (`bafyreibz2qaq…`) becomes a miss, and shem re-records with one rebuild and re-put. Say so in the
   commit.

## Tests

- `a_system_ld_change_does_not_move_the_key_when_rust_lld_links` (fixture PATH with two `ld`
  stubs of different `--version`).
- `an_explicit_linker_override_keys_that_linker`.
- `a_changed_startup_object_moves_the_key`.
- `put_refuses_a_binary_whose_comment_names_another_linker` (fixture ELF with a crafted `.comment`).
- `an_unknown_target_keys_ld_as_before`.

## Decides F2r

ethosengine was asked (2026-11-09 ~20:50Z) for os-release, the `rpm -q` package set, its binary's
`.comment` and the sha256 of the ten objects above. shem's values are in STATE.md. If the objects
match, K1 alone makes the keys converge. If they differ, the keys are right to differ, and the
devfile images must align (an operator item, with the exact package diff).
