REFUTED — TS address admission and signature verification diverge from the kernel contract.

Read-only review of the specified worktree, with in-memory Node probes. No files changed; no Git or Cargo run. Rust conclusions below are source-based, not executed test results.

1. **High — TS accepts signatures Rust rejects.**  
   [proof.ts:17](/projects/elohim/.worktrees/rg1-kernel/elohim/sdk/epr-ts/src/proof.ts:17), versus [signature.rs:301](/projects/elohim/.worktrees/rg1-kernel/elohim/epr/src/signature.rs:301).  
   `zip215: false` does **not** make installed Noble 2.3.0 equivalent to Dalek’s `verify_strict`. Noble uses a cofactored verification equation and does not reject every small-order `R`.

   **Evidence:** choose public key `A = B` (the standard base point), `R = (0,-1)` encoded as `ec ff×30 7f`, and `S = SHA512(R‖A‖transcript) mod L`. The actual TS `verifyEpr` returned `{ok:true}`. Dalek explicitly rejects `R.is_small_order()`. This establishes divergent acceptance, not forgery against an arbitrary victim key.

   **Exact fix:** implement one precisely shared verification policy: reject small-order `A` and `R`, use the uncofactored equation, and align point-decoding rules. Do not rely on the Noble option alone.

2. **High — TS address verification remains on the old profile and admits noncanonical CID encodings.**  
   [address.ts:43](/projects/elohim/.worktrees/rg1-kernel/elohim/sdk/epr-ts/src/address.ts:43), [address.ts:74](/projects/elohim/.worktrees/rg1-kernel/elohim/sdk/epr-ts/src/address.ts:74), [address.ts:104](/projects/elohim/.worktrees/rg1-kernel/elohim/sdk/epr-ts/src/address.ts:104).  
   **Evidence:** the pinned BLAKE3 `"hello world"` CID returns `false`; both `sha256-<digest>` and `sha256:<digest>` return `true`. A matching raw/SHA-256 CID with version encoded nonminimally as `81 00` also returns `true`. `CID.parse` performs decoding, but this implementation does not enforce canonical reconstruction equality.

   **Exact fix:** remove legacy-marker admission; dispatch on exactly the three permitted triples; implement BLAKE3 verification; reconstruct canonical CID bytes/text and compare them with the input. Byte-only admission must require exactly `01 55 1e 20 ‖ digest`.

3. **Medium — The generic transcript is not injective.**  
   [signature.rs:193](/projects/elohim/.worktrees/rg1-kernel/elohim/epr/src/signature.rs:193).  
   Let `K` be a valid Ed25519 multikey, `P = 00000000‖K`, `D = "elohim:test:v0:"`, and `B = "abc"`. At epoch/scheme zero, these distinct `(domain,key,body)` triples produce identical bytes:

   - `(D, K, ":"‖P‖B)`
   - `(D‖P‖":", K, B)`

   Both domains end in `:`. Rust accepts arbitrary domain bytes, so the documented suffix convention does not prevent this. Independently, unrestricted key lengths permit `(D,K‖x,B)` and `(D,K,x‖B)` to collide.

   **Exact fix:** introduce a reviewed transcript version with explicit domain and key lengths, identically encoded in Rust and TS. Current fixed EPR/carrier domains and scheme-0 key lengths prevent these particular collisions in their normal entry points; I found no replay between those two registered domains.

4. **Medium — Malformed byte CID can panic in debug builds.**  
   [lib.rs:237](/projects/elohim/.worktrees/rg1-kernel/crates/elohim-address/src/lib.rs:237).  
   **Evidence:** base32-encode `81 00 55 1e 20 ‖ 31 zero bytes`. The result is 59 characters, decodes to 36 bytes, and passes `check_header` because its local varint reader permits nonminimal encoding. The subsequent `debug_assert_eq!` panics before the fallible CID decoder rejects it.

   **Exact fix:** replace the assertion with a runtime header comparison returning `NonCanonical`, or require minimal varints before reaching it. Untrusted parsing must return an error in every build mode.

5. **Medium — TS silently coerces malformed wire values and aliases transcript integers.**  
   [signature.ts:19](/projects/elohim/.worktrees/rg1-kernel/elohim/sdk/epr-ts/src/signature.ts:19), [epr.ts:34](/projects/elohim/.worktrees/rg1-kernel/elohim/sdk/epr-ts/src/epr.ts:34), [epr.ts:61](/projects/elohim/.worktrees/rg1-kernel/elohim/sdk/epr-ts/src/epr.ts:61).  
   **Evidence:** transcript epoch `65536` equals epoch `0`; scheme `65536` likewise aliases `0`. Missing scheme, `-1`, fractions and string `"0"` become `scheme-held`, although none is a valid unknown `u16`. Adding 256 to every JSON key/signature byte still produces `{ok:true}` through `Uint8Array` coercion.

   **Exact fix:** validate mandatory fields before dispatch; require integer epoch/scheme in `0..65535` and integer bytes in `0..255`; reject malformed values without coercion. Reserve Held for structurally valid, unsupported scheme codes. Current `verifyEpr` fixes epoch and checks scheme before constructing its transcript, limiting the integer-aliasing exposure there.

6. **Medium — Fallible chunking ignores the profile’s epoch and address declaration.**  
   [profile.rs:316](/projects/elohim/.worktrees/rg1-kernel/elohim/epr/src/profile.rs:316), [profile.rs:369](/projects/elohim/.worktrees/rg1-kernel/elohim/epr/src/profile.rs:369).  
   Both entry points validate only `profile.chunker`. A deserialized profile with epoch `1`, wrong codec/multihash, or invalid erasure parameters still chunks successfully if its chunker parameters pass. This contradicts the instruction at line 306 that wire profiles can safely enter through `try_chunk_bytes`.

   **Exact fix:** call `profile.validate()?` in both fallible entry points. For the panicking wrapper, require a validated-profile type or return `Result`. Min/avg/max/order/evenness/level checks themselves are present; I found no production wire caller of the wrapper in the inspected tree.

7. **Low — Carrier unknown-scheme classification can become Invalid.**  
   [carrier.rs:181](/projects/elohim/.worktrees/rg1-kernel/crates/consent-grant/src/carrier.rs:181).  
   An unknown scheme paired with a signer mismatch, undecodable signer, or invalid statement returns `Invalid` before scheme dispatch. This violates the stated unconditional unknown-scheme → Held rule, though it grants no authority.

   **Exact fix:** dispatch unsupported schemes before authenticity checks, or represent structural/statement rejection separately from `SignatureCheck::Invalid`.

The remaining targeted checks survive inspection: Rust rejects the requested malformed header, padding, uppercase, 58/60-character, SHA-256, wrong-codec, wrong-prefix and NUL cases; canonical re-encoding equality is explicitly enforced at `lib.rs:239`. `verify_cid_any` admits only the three exact triples and never substitutes BLAKE3 for SHA-256. `check_for` compares decoded minimal-varint Multikeys, and CarrierProof still derives the expected key independently from the signer.

The actual FastCDC tables independently hash to `3bd9b879…8001`. An independent port reproduced the pinned 10 MiB boundaries and matched streaming feeds of 1 KiB, 7 KiB and 1 MiB+1. Rust tests cover those feeds. Existing chunker entries are compared before any regeneration write. Fourteen negative address vectors are present and exercised. The TS envelope transcript matches the Rust vector byte-for-byte; all three signed-EPR fixtures passed the Node probe.

Tests I would add:

- Shared small-order and mixed-torsion signature rejection vectors.
- Transcript collision regressions and exhaustive invalid integer boundaries.
- TS malformed scheme/byte-array rejection through `verifyEpr`.
- Shared Rust/TS address negatives, including NUL and nonminimal varints; parser fuzzing with no-panic assertions.
- Unknown-profile and invalid-parameter rejection through both chunking APIs.
- Regeneration refusal with an existing vector deliberately changed.
- Carrier unknown-scheme classification across signer/statement failures.