---
title: "K10 design note: dispatch evidence bound to the complete executable"
written: "2026-11-11"
author: "gpt-6-astra (Codex), read-only design pass dispatched by claude-opus-5-5"
status: "design input; not a ruling"
---

**K10 design note — bind dispatch evidence to the complete executable**

Recommend a **post-link, proof-bearing attestation**, checked by `put` before narrowing. Function hashes establish identity; they do **not** establish guarded reachability. The certificate must cover the guarded code, the detector, and every route into that code.

One correction changes the minimum scope: K8’s `sha_ni` examples include ring **and two `sha2` implementations**. Ring plus compiler_builtins evidence alone cannot justify exactly v3. See [k8-extraction-ethosengine.json:412](../../../../../../genesis/docs/superpowers/sprints/2026-11-07-artifact-store/evidence/k8-extraction-ethosengine.json:412).

**1. What constitutes proof**

For every instruction requiring a removed feature, establish:

> Under the remaining host requirements, every realizable entry into this instruction implies that its required CPU and OS state is available.

This includes direct calls, tail calls, indirect calls, cached pointers, alternate entries and constructors. A guard may execute earlier and establish a persistent invariant; it need not dominate each invocation as a literal conditional branch.

The following costs and limitations are design assessments:

| Evidence mechanism | What it proves | Failure modes | Cost and byte binding |
|---|---|---|---|
| **Build-time attestation** | A trusted producer asserts a relationship between exact code and guards. With independently checkable certificates, the reader can establish that relationship itself. | An inventory of `cfg`, `target_feature` annotations or dispatch tables establishes intended dispatch, not absence of bypasses. Crate-local evidence misses external callers; pre-link ranges miss optimization and relocation changes. | Moderate per-version maintenance, potentially substantial initial tooling. Bind the **whole final artifact CID**, plus guard/code ranges and hashes, after linking and stripping. A signature authenticates the producer, not the assertion’s correctness. |
| **Static binary analysis** | A sound over-approximation of control flow and state proves every possible entry satisfies the guard predicate. | Missed indirect targets, jump tables, exported entries, mutable feature words, pointer escapes or undecoded code make “all callers” unjustified. Recognizing one branch is insufficient. | Highest analysis cost. Naturally byte-bound when the analysis names the complete executable CID. Use bounded supported cases; unresolved paths retain requirements. |
| **Compiler multiversioning metadata** | Resolver/clone associations identify candidate guarded implementations; resolver semantics plus complete use analysis can establish dispatch. | An ifunc can coexist with direct calls to a clone. `target_feature` permits instructions but does not prove callers checked support. Source patterns may disappear or become unconditional. | Cheapest when trustworthy metadata survives into the final output; otherwise needs compiler/linker support and post-link validation. Bind metadata, resolver, clones and final executable together. |

GCC documents resolver-based multiversioning; Rust explicitly documents that `is_x86_feature_detected!` becomes `true` for compile-time-enabled features. Neither fact licenses blanket exemptions. [GCC multiversioning](https://gcc.gnu.org/onlinedocs/gcc/Function-Multiversioning.html), [Rust feature detection](https://doc.rust-lang.org/std/macro.is_x86_feature_detected.html).

**2. Applying this to the actual crates**

Source links below identify the inspected registry versions; a certificate must identify the version actually linked.

**Ring 0.17.14.** The proposed “known `OPENSSL_ia32cap_P` word” model needs correction. Here that name is the output-array parameter of `OPENSSL_cpuid_setup`; Rust passes a local array, converts it into capability flags, and initializes its cache. C clears AVX-dependent capabilities when XCR0 lacks the required state. [crypto/cpu_intel.c:77](/opt/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/ring-0.17.14/crypto/cpu_intel.c:77), [crypto/cpu_intel.c:118](/opt/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/ring-0.17.14/crypto/cpu_intel.c:118), [src/cpu/intel.rs:46](/opt/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/ring-0.17.14/src/cpu/intel.rs:46).

Dispatch crosses several mechanisms:

- Rust chooses the SHA assembly entry using `(Sha, Ssse3)` capability tokens; perlasm emits that separate entry. [sha2_32.rs:44](/opt/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/ring-0.17.14/src/digest/sha2/sha2_32.rs:44), [sha512-x86_64.pl:526](/opt/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/ring-0.17.14/crypto/fipsmodule/sha/asm/sha512-x86_64.pl:526).
- Montgomery multiplication selects `bn_mulx4x_mont` through `Option<(Adx, Bmi2)>`. P256 C wrappers instead branch on `adx_bmi2_available`, which Rust sets after checking both capabilities. [mont.rs:53](/opt/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/ring-0.17.14/src/arithmetic/limbs/x86_64/mont.rs:53), [p256-nistz.c:126](/opt/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/ring-0.17.14/crypto/fipsmodule/ec/p256-nistz.c:126), [intel.rs:316](/opt/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/ring-0.17.14/src/cpu/intel.rs:316).
- AES/GCM selection constructs capability-dependent key variants; the vector GCM constructor requires `(Avx2, VAesClmul)`. Thus later calls depend on object-construction provenance. [aes_gcm.rs:79](/opt/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/ring-0.17.14/src/aead/aes_gcm.rs:79), [vclmulavx2.rs:29](/opt/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/ring-0.17.14/src/aead/gcm/vclmulavx2.rs:29).

**Best fit:** version-pinned crate contracts, including generated assembly, discharged against the final link. Binary-only analysis is possible but requires capability/object dataflow, not merely branch dominance. Compiler multiversioning metadata alone does not describe these inspected mechanisms.

**Compiler_builtins FMA.** The local libm implementation selects FMA, then FMA4, otherwise software; `select_once!` stores the selected function in an `AtomicPtr`. Its own comment calls this an ifunc equivalent *without linker support*. FMA4 uses inline assembly, not a `target_feature` annotation. [libm-0.2.16/fma.rs:10](/opt/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/libm-0.2.16/src/math/arch/x86/fma.rs:10), [fma.rs:89](/opt/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/libm-0.2.16/src/math/arch/x86/fma.rs:89), [feature_detect.rs:31](/opt/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/libm-0.2.16/src/math/support/feature_detect.rs:31).

The detector enables FMA4 from AMD/Hygon extended CPUID bit 16, outside its OS-state-checking block. A certificate must state its OS-state assumptions; for this slice, retain v3’s prerequisites. [detect.rs:134](/opt/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/libm-0.2.16/src/math/arch/x86/detect.rs:134), [detect.rs:191](/opt/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/libm-0.2.16/src/math/arch/x86/detect.rs:191).

**Best fit:** a small cached-pointer proof: initial target, all pointer writers, selection predicate and all consumers. A simple branch-dominance checker fails here.

**Evidence limit:** I inspected the final binary’s corresponding initializer and FMA4 instruction, but the exact compiler_builtins source snapshot is absent locally. Local libm is supporting evidence, not a substitute for that snapshot. Upstream compiler_builtins imports libm through [src/math/mod.rs:5](https://github.com/rust-lang/compiler-builtins/blob/main/compiler-builtins/src/math/mod.rs#L5); K10 must pin the actual toolchain source/object, not this moving upstream reference.

**BLAKE3 1.8.5.** `Platform::detect()` chooses variants; matching on those variants reaches unsafe FFI wrappers. Detection uses `cpufeatures` for AVX2 and AVX512F+VL. [platform.rs:65](/opt/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/blake3-1.8.5/src/platform.rs:65), [platform.rs:255](/opt/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/blake3-1.8.5/src/platform.rs:255), [platform.rs:411](/opt/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/blake3-1.8.5/src/platform.rs:411), [ffi_avx2.rs:6](/opt/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/blake3-1.8.5/src/ffi_avx2.rs:6).

However, `platform` and its enum variants are public. The final application could construct `Platform::AVX2` directly. Moreover, `cpufeatures` returns constant `true` when compilation enables all requested features. [lib.rs:99](/opt/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/blake3-1.8.5/src/lib.rs:99), [platform.rs:44](/opt/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/blake3-1.8.5/src/platform.rs:44), [cpufeatures/x86.rs:13](/opt/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cpufeatures-0.3.0/src/x86.rs:13).

**Best fit:** build attestation plus final-program enum-provenance and FFI-entry checks. Static analysis can discharge these obligations; annotations alone cannot.

**3. Minimum honest slice**

Scope the first implementation to one supported ELF/link configuration and these exact dispatch contracts. Include both SHA implementations: their default paths check `shani_cpuid`, while SHA2 0.11 also offers a compile-time-selected direct backend. [sha2-0.10.9/x86.rs:100](/opt/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sha2-0.10.9/src/sha256/x86.rs:100), [sha2-0.11.0/sha256.rs:18](/opt/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sha2-0.11.0/src/sha256.rs:18), [sha256.rs:62](/opt/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sha2-0.11.0/src/sha256.rs:62).

Proposed evidence document, canonically serialized and stored by CID:

```text
schema: dispatch-evidence/v1
artifact: { byteCid, byteLength, elfMappingDigest }
methodCid, buildInputsCid, verifierCid, decoderCid
sourceContracts: [{ crateSourceCid, configurationCid, contractCid }]
assumptions: { executionModel, residualHostRequirements }
regions: [{
  fileOffset, length, linkedVirtualAddress, contentHash,
  instructionFeatureSites,
  guardPredicate,
  detectorRanges,
  incomingEdges,
  pointerOrObjectInvariants,
  certificateCid
}]
coverage: { examinedSites, certifiedSites, unresolvedSites }
```

Names may appear as diagnostics; acceptance uses bytes, offsets and verified relationships.

A **build finalization step** produces this document after all byte-changing operations. Per-crate contracts supply proof obligations; they do not authorize exemptions themselves. `put` checks the final artifact CID and certificates, then performs narrowing before merging caller declarations. This fits the existing extraction-before-declaration sequence; caller-supplied narrowing is already rejected. [requires.rs:44](../../../../../../elohim/eprfs/epr-cli/src/artifact/requires.rs:44), [requires.rs:62](../../../../../../elohim/eprfs/epr-cli/src/artifact/requires.rs:62).

Rescan **every occurrence**, not K8’s examples: `FeatureUse` retains only three example symbols. Remove certified instruction requirements, then recompute level closure from the remainder. SQLite’s remaining v3 instructions keep v3. [extract_isa.rs:85](../../../../../../elohim/eprfs/epr-cli/src/artifact/extract_isa.rs:85), [extract_isa.rs:25](../../../../../../elohim/eprfs/epr-cli/src/artifact/extract_isa.rs:25).

Only when coverage is complete should `removed` contain exactly:

```text
adx, aes, pclmulqdq, vpclmulqdq, vaes, sha_ni, fma4
```

Set `Narrowing.evidence` to the evidence CID. Admission remains unchanged; provenance is ignored by `admit`. If another artifact loses its entire above-baseline clause, omit that clause and attach the narrowing to Produce. [requirement.rs:31](../../../../../../crates/elohim-host-contract/src/requirement.rs:31).

A second device fetches the artifact and evidence closure, verifies all CIDs, runs its approved verifier, rescans instructions and recomputes the result. It need not execute x86 code or observe the builder’s CPU. Missing or unsupported proof retains the original requirement.

**4. Failure boundaries**

- **False narrowing:** require complete incoming-edge and state provenance. An extra unguarded caller, forged enum variant, alternate pointer writer or unsupported indirect edge defeats the affected exemption.
- **Stale evidence:** bind the complete final artifact, not merely a function hash, recipe or build ID. A new caller can invalidate unchanged guarded-function bytes.
- **Unreproducible evidence:** publish the verifier specification, pinned contracts and required source/object material by CID. A builder-only path or unsigned prose assertion cannot support independent rechecking. Reproducing the executable build is separate from reproducing verification.
- **Acceptance:** test withheld evidence, altered guards, added bypasses and stale certificates. Exactly-v3 admission is an outcome of complete coverage, not a preset target. With the currently unresolved compiler_builtins binding, this note does **not** certify the existing artifact as exactly v3.