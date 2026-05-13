---
name: serde_json::Value breaks Holochain zome boundary — pre-stringify across WASM
description: Holochain's SerializedBytes uses MessagePack; serde_json::Value doesn't round-trip — decodes byte arrays as raw bytes instead of structured JSON; breaks DNA init for ALL tests
type: feedback
originSessionId: 4d20bf7b-4639-43d8-ad10-fccb514a7f0a
---
`serde_json::Value` as a field on a `#[derive(SerializedBytes)]` type that crosses the Holochain zome boundary **compiles fine but fails at runtime**, taking down DNA initialization for ALL sweettest scenarios in the conductor — not just the function that uses the broken type.

**Why:** Holochain's `holochain_serialized_bytes` uses MessagePack as the wire format at zome-call boundaries (zome inputs/outputs and post-commit signal payloads). `serde_json::Value` derives `Serialize`/`Deserialize` for JSON specifically — when MessagePack encodes it, the round-trip produces byte arrays where structured values were expected. Decoder errors with `SerializationError(Bytes(Deserialize("invalid type: byte array, expected any valid JSON value")))` from `sweet_conductor_handle.rs`.

**Symptom you'll see:** unrelated tests in the same DNA fail with the same byte-array error, all from inside the sweettest framework before your test even gets going. The DNA can't register the broken zome, so nothing loads.

**Why:** M5 introduced `anomaly_attestation: serde_json::Value` on `SubmitSpecialistRevocationInput` (per the schema's `$ref` to a structured type). Compiled clean. At test time it took down `epr_2b_batch_a_full_loop` and `binding_creates_and_is_readable` (Batch A tests touching unrelated zome entries) — the imagodei DNA couldn't initialize for any conductor that loaded it.

**How to apply:**

- **Never** put `serde_json::Value` on a struct that derives `SerializedBytes` and crosses the WASM boundary. Even if the schema describes structured JSON, pre-serialize at the HTTP layer and use `String` at the zome.
- Field naming convention: when the WASM type is the JSON-stringified form, append `_json` suffix (e.g., `anomaly_attestation_json: String`). Documents the contract.
- The HTTP-side InputView (storage `views.rs`) and JSON schema keep the structured shape — the storage handler does `serde_json::to_string` before forwarding. The translation is the storage bridge's responsibility, not the zome's.
- This generalizes: any "free-form structured payload" needs the same treatment. `votes_json: String` in `KeyRevocation` already follows this pattern (legacy field). Follow the precedent.
- Detection: a passing `just check`/`just pack` does NOT prove the zome boundary works. Sweettest is the only check that actually exercises serialization. If you only have local Eclipse Che: catch this when CI runs.
- Failure mode is shared: a single bad type takes down the whole DNA. Don't assume "my tests pass therefore my zome's clean" — unrelated DNA tests failing in CI are a strong signal that some recent zome change has a SerializedBytes-incompatible type.
