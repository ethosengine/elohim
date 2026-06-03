---
id: feedback-dht-readback-use-to-app-option
name: feedback_dht_readback_use_to_app_option
description: "Read DHT entries back with record.entry().to_app_option::<T>(), never Entry::try_into() -> SerializedBytes — the try_into round-trip serializes the Entry::App variant TAG into the bytes, so readback fails 'missing field' even when the struct shape is identical."
metadata:
  node_type: memory
  type: feedback
  originSessionId: 2026-04-25T00-43-clear-dna-integration-holochain
cites:
  - elohim/holochain/dna/node-registry/zomes/node_registry_coordinator/src/lib.rs
---

**Read DHT entries back with `record.entry().to_app_option::<T>()`, never `Entry::try_into() -> SerializedBytes`.**

**Why:** the `node_registry_coordinator` deserialize helpers used an `Entry::try_into()` SerializedBytes round-trip. That serializes the `Entry::App(...)` *variant tag* into the bytes instead of unwrapping the inner app entry. On readback it fails with `Deserialize error: missing field 'node_id'` — even though the fixture and the integrity struct have identical (23-field) shape. The shape is fine; the **envelope is wrong**: you're trying to deserialize `T` out of bytes that actually encode `Entry::App(<T>)`. `to_app_option::<T>()` unwraps the variant first, then deserializes the inner bytes. Sibling DNAs (mishpat, imagodei) already used the correct `to_app_option` pattern. Fix landed `d68fe834`.

**How to apply:**
- On any DHT readback, go through `record.entry().to_app_option::<T>()?` (or `.to_app_option()` on the `RecordEntry`), not `Entry::try_into::<SerializedBytes>()` then `SerializedBytes::try_into::<T>()`.
- Symptom signature: a `missing field '<first field>'` deserialize error on readback when the integrity struct and the write fixture are field-for-field identical → suspect the variant-tag envelope, not the struct.
- When adding a new coordinator, copy the readback helper from mishpat/imagodei, not from any zome still carrying the try_into pattern.

**Distinct from** [[feedback_serde_json_value_breaks_zome_boundary]]: that one is `serde_json::Value` at the zome *call* boundary (pre-stringify to a `_json: String`); this is the *Entry variant envelope* on DHT readback. Both present as serde failures but the fix surface is different.
