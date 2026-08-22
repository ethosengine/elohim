---
index: false
id: project-conductor-signal-msgpack-decode-class
name: conductor-signal-msgpack-decode-class
title: Conductor signal msgpack decode class
description: "holo_hash in conductor msgpack signals = raw bytes; Value pre-pass or String mirrors silently drop the signal — decode typed HoloHashB64 (closed 2026-06-13)."
metadata: 
  node_type: memory
  type: project
  originSessionId: da9eca9d-9d7f-4a87-83f4-1f4197e6beba
---

Conductor app signals are MessagePack (`ExternIO` = `rmp_serde::to_vec_named`); `AgentPubKey`/`ActionHash` serialize as raw 39-byte arrays, NOT base64 strings (base64 is only their Display/JSON form). Two failure modes, both silent-at-debug: (1) decoding via `rmp_serde::from_slice::<serde_json::Value>` fails outright (Value can't represent bytes — same class as the DNA CLAUDE.md serde_json::Value-at-zome-boundary trap); (2) a storage mirror declaring those fields `String` fails the typed parse. Either way the signal drops and the projection goes dark while emit-side succeeds (this hid the empty `peer_statuses` for the whole EPR durability arc).

**Why:** signal-projection bugs of this class look like emit-side or gossip bugs; the decode is the LAST place anyone looks because tests round-trip JSON (which proves nothing about the conductor wire).

**How to apply:** decode signals with a typed `rmp_serde::from_slice` into a mirror whose hash fields use `HoloHashB64` (`elohim/elohim-storage/src/signals.rs` — accepts bytes/seq/str, normalizes to `u`+b64url); classify misses (mirror-tag ⇒ LOUD counted WARN, foreign tag ⇒ quiet). Wire-conformance test pattern: encode the DNA-shaped enum with real holo_hash types via `to_vec_named` and decode through the subscriber's exact path (`peer_status_signal_decodes_from_conductor_msgpack_wire`). CLASS CLOSED 2026-06-13 on feat/frontend-eyes-sprint: mishpat (73b665122 — unblocked the Epic B provide projection 4626f820b; `CommitmentPayload` embedded fields are all-String, survive the wire) and REA+ElohimContent (2571fe642). The content subscriber had a WORSE bug than byte-drop: its flat `ElohimContentSignal` mirror matched a shape the DNA never emits — the real signal is the tagged `ProjectionSignal::ContentCommitted` envelope — so attestation/recovery projections were dark since wiring (decode now translates envelope→flat contract, filtering `attestation:*`/`governance-action:*`). The infra family landed on typed decoders in d33b0e1f5. All four families share `SignalDecodeMiss` (every subscriber's match must stay exhaustive across all `*ShapeMismatch` variants — union-merge trap when parallel branches extend the enum). Relates to [[mishpat-commitment-cid-is-entry-hash]].
