---
id: "backlog-content-projection-patch-cannot-express-clear"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "ContentProjectionPatch cannot say \"clear\" — a None field always means PRESERVE, so a verified head with no blob keeps the previous blob"
slug: "content-projection-patch-cannot-express-clear"
written: "2026-09-21"
author: "serving-edge failover-balance-stream campaign, 2026-09-21 review"
status: "open"
priority: "medium"
jobs: [elohim]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "habit:dataplane-convergence"
tags: [dataplane, content-projection-patch, blob-cid, torn-row, inventory-broadcaster]
---

**The fact.** `ContentProjectionPatch` (`elohim/elohim-storage/src/db/content_diesel.rs:928-946`) applies its
present fields via `apply_content_patch_fields` (`:967-...`) as targeted per-field UPDATEs, and every field is
`Option<T>` where `None` means "preserve the existing column" — documented on the struct itself ("`None`
preserves the existing column, exactly like every other field here", `:940-944`, re: `declared_head_record_json`)
and enforced per-field (`if let Some(ref v) = patch.blob_cid { ...update... }`, `:973-982`). There is no way to
express "clear this field to NULL": a verified head that legitimately carries no blob (or no size) cannot make
the row forget its previous one — the row keeps whatever it held before, silently paired with whatever the patch
DID set. The DNA entry permits exactly this unpaired shape: `content_store_integrity::Content` declares
`blob_cid: Option<String>` (`elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs:521`) and
`content_size_bytes: Option<u64>` (`:524`) with no validation rule requiring them to be set or cleared together.

**Evidence.** `p2p/inventory_broadcaster.rs`'s `gather_hints` (`:178-217`) queries rows by `blob_hash` and reads
back `content_size_bytes` from the SAME row to build the `BlobHint` it announces (`:178-181`, `:208-214`). If a
patch moves a row's `blob_cid` (hence `blob_hash`, mirrored at `:979`) to a NEW blob but leaves
`content_size_bytes: None` because the verified head carried no size, the row now holds `(new blob, stale
size)` — a self-consistent-looking SQL row that the broadcaster will publish to the network as a `BlobHint`
pairing the new hash with the OLD blob's size, with nothing to catch the mismatch.

**Why it matters.** This is the general form of the pointer/size-tearing class the T-1/T-2 work (story 1.4a) has
been closing one call-site pattern at a time: as long as the patch type itself cannot express "this field is
verified-absent, clear it," every future call site that constructs a `ContentProjectionPatch` from a real,
verified answer has to remember to special-case "verified-but-empty" vs "not touching this," and a missed case
reintroduces torn state that reads as coherent SQL.

**Smallest next step.** Give the patch a clearable variant per field (`Option<Option<T>>`, or a companion
`clear: HashSet<Field>` set) with a NULL-setting UPDATE arm, and update every patch writer to distinguish "did not
observe this field" from "observed this field as absent." Do NOT use `blob_cid.is_some()` as a stand-in for "this
row carries content evidence" anywhere in that migration — 3,770 of 3,770 corpus items carry no blob pointer
today, so that predicate would misclassify the entire corpus; T-2's actual rule (content evidence = a patch was
present at all, `StaleReason::PointerAbsent` gating a MOVE, `content_diesel.rs:1818-1830`) must stay keyed on
patch-presence, not blob-presence.

**Links.** Struct: `elohim/elohim-storage/src/db/content_diesel.rs:928-946`. DNA entry:
`elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs:490-525`. Consumer surfacing the risk:
`elohim/elohim-storage/src/p2p/inventory_broadcaster.rs:178-217`. Design context: T-2's pointer-absent refusal,
`content_diesel.rs:1818-1830`; `genesis/a2o/reports/recovery/serving-edge-20260919/story-1.4a-design.md`. Habit:
`elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md`.
