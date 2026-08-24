---
id: "backlog-deprecation-doorway-strips-schema-version-header"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The doorway silently strips X-Schema-Version on every proxied /db write — conformant clients read to storage as legacy clients, and version negotiation is defeated rather than merely warned about"
slug: "deprecation-doorway-strips-schema-version-header"
written: "2026-08-24"
author: "deprecation-triage"
status: "wip"
priority: "medium"
deprecation_status: in-progress
severity: medium
fingerprints: ["db68b690b3cd"]
relatedNodeIds: []
tags: [deprecation, doorway, elohim-storage, schema-negotiation, http-contract, header-allowlist, cors]
cites:
  - doorway/doorway-service/src/routes/storage_proxy.rs
  - doorway/doorway-service/src/cors.rs
  - elohim/elohim-storage/src/http.rs
  - elohim/elohim-views/src/shared.rs
  - genesis/seeder/src/doorway-client.ts
  - genesis/data/timeline/backlog/deprecation-storage-bulk-x-schema-version-header.md
---

## What is deprecated

The captured warning is the same first-party deprecation line the sibling entry
owns (`elohim/elohim-storage/src/http.rs`):

```rust
None => {
    warn!("Bulk request missing X-Schema-Version header (deprecated: clients should send this header)");
    Ok(None)
}
```

**But the root cause is a different one, and it is a defect rather than a
tolerance question.** The sibling entry closed its client-conformance work on
2026-08-21 with the claim "every in-repo client is now conformant", and that
claim is *true at the client*. The warning still fired on 2026-08-24 at
`20:35:35.470283Z` in `matthew.log` during the local-mesh prologue seed, because
the header does not survive the hop between a conformant client and storage:

`forward_to_storage` in `doorway/doorway-service/src/routes/storage_proxy.rs`
rebuilds the outbound request from an explicit **header allowlist** —
`content-type`, `authorization`, `x-observation-id`, plus two ctx-injected
headers (`X-Agent-Cid`, `x-elohim-verified-performer`). `X-Schema-Version` was
not on that list, so every proxied `/db/**` write had it dropped.

The forwarder's own doc comment already named the hazard, two lines above the gap:

> The forwarder rebuilds the outbound request from an allowlist, so this must
> ride ctx — a header injected on the inbound hyper request would be silently
> dropped here.

**This is worse than a spurious log line.** `X-Schema-Version` is a
*negotiation* header, and `validate_schema_version_header` is
advisory-on-absence / **strict-on-presence**. Dropping it does not merely
downgrade a conformant client to the default — it converts the strict branch
into the advisory branch, so a client requesting an **unsupported** version is
silently served the default instead of receiving the `400` that protects it.
Version negotiation across a doorway is not weakened; it is absent.

A second, independent defect in the same contract: `X-Schema-Version` was also
missing from the doorway's CORS `Access-Control-Allow-Headers`
(`doorway/doorway-service/src/cors.rs`), while storage's own CORS already
advertises it. A **cross-origin** browser client that correctly sends the header
on a bulk write has the whole request refused at preflight — a harder failure
than the dropped-header case, because the request never leaves the browser.
Latent so far only because every in-repo browser caller reaches a doorway
same-origin (served by the doorway in prod; Angular dev-server proxy locally). A
Tauri origin or any third-party client would have hit it.

## Usage inventory

The gap is at one hop, not many call sites — which is why the sibling entry's
client-by-client inventory could be complete and still not extinguish the warning.

| File | Site | Defect |
|---|---|---|
| `doorway/doorway-service/src/routes/storage_proxy.rs` | `forward_to_storage` header allowlist | `X-Schema-Version` not forwarded — **the live emitter** |
| `doorway/doorway-service/src/cors.rs` | `ALLOWED_HEADERS` | `X-Schema-Version` not advertised at preflight |

`forward_blob_to_storage` (same file) is **out of scope**: it is GET-only for
blob reads and reaches no guarded bulk route.

Affected clients are every caller that reaches storage *through a doorway* —
notably `genesis/seeder/src/doorway-client.ts`, whose five bulk methods
(content / relationships / presences / events / mastery) all set the header
correctly and all had it stripped. The 2026-08-24 emission came from one of the
five silent handlers (only `handle_db_content_bulk` logs on entry), consistent
with the prologue's presences/events traffic.

Clients that reach storage **directly** (a2o `postRaw` against `:8090`, the
Rust SDK's `schema_conformant_http_client`) were never affected — which is
exactly why the sibling entry's 2026-08-21 runtime A/B passed: it probed
storage directly and never crossed the hop that drops the header.

## Migration path

No upstream guide — the contract is ours. Two edits, both landed 2026-08-24:

1. **Forward the header.** Add an `x-schema-version` arm to
   `forward_to_storage`'s allowlist, mirroring the adjacent `x-observation-id`
   block verbatim.
2. **Advertise it at preflight.** Append `X-Schema-Version` to `ALLOWED_HEADERS`
   in `cors.rs`, closing the doorway-vs-storage CORS skew, and guard it with a
   regression test (`preflight_allows_schema_version_header`).

**Generalization deliberately NOT taken in this pass.** The real class here is
*allowlist drift*: storage reads a set of `X-*` request headers, the doorway
forwards a hand-maintained subset, and nothing reconciles the two — so any
future storage-side request header is dropped by default, silently, exactly as
this one was. A shared constant or a contract test asserting
"every header storage reads is forwarded or explicitly excluded" would make the
class impossible. That is a design change with its own blast radius (it must
decide which headers are deliberately *not* forwarded — a security boundary, not
a convenience), so it is named here rather than smuggled into a deprecation fix.

## Current decision

**Fix WRITTEN and root-cause-PROVEN at runtime, but COMPILE-UNVERIFIED — this
entry is `wip`, not closed. Do not re-dispatch triage for this warning; the
remaining work is the verification run, not re-discovery.**

Both edits are landed in the tree. They were deliberately **not** compiled this
run: the live two-peer mesh was mid-measurement (load 13.6/24, doorway and
storage both running out of the `dev` pool slot at
`/projects/.cargo-target-pool/family/doorway/`), and the dispatch constraint
forbade cargo builds that would overwrite a pool-slot binary in use. Disk was at
85%, so a scratch target dir for a ~19G parallel doorway build was also the
wrong call.

**The exact verification the next run must perform**, once the mesh is free:

```bash
# 1. compile + unit gate (doorway-service; RUSTFLAGS must be cleared — root CLAUDE.md)
RUSTFLAGS="" cargo test  -p doorway-service --lib cors::
RUSTFLAGS="" cargo clippy -p doorway-service --all-targets -- -D warnings
cargo fmt --check

# 2. runtime A/B — re-run the probe below; T1 must flip 200 → 400
```

Only then may this entry be deleted and fingerprint `db68b690b3cd` be removed
from the ledger.

## Verification

**Root cause — PROVEN at runtime, 2026-08-24, against the live local mesh**
(doorway A `:8888` → matthew storage `:8090`). One request distinguishes
"forwarded" from "dropped" by *status code alone*, because
`validate_schema_version_header` runs before the body is read: an unsupported
version must 400. Body `[]` makes the write a no-op either way.

| Probe | Request | HTTP | Response | Warn delta |
|---|---|---|---|---|
| C1 — direct to storage `:8090` | `POST /db/presences/bulk`, `X-Schema-Version: 99` | **400** | `{"error":"Unsupported schema version: 99. Supported: [1]"}` | 0 |
| T1 — same, via doorway `:8888` | identical request | **200** | `{"created":0,"errors":[]}` | **+1** |

C1 proves storage enforces the header when it arrives. T1 proves it does not
arrive through the doorway — the 200 shows the strict branch never ran, and the
`+1` warning shows storage took the absent-header path for a request that
carried the header. This is not an inference from reading the allowlist; it is
the defect reproduced on demand.

**CORS gap — PROVEN, same session:**

```
OPTIONS /db/presences/bulk  (Origin: http://example.test,
                             Access-Control-Request-Headers: content-type, x-schema-version)
→ HTTP 204
→ Access-Control-Allow-Headers: Content-Type, Authorization, Accept, X-Op, X-Requested-With, X-Observation-Id
```

`x-schema-version` is absent from the response, so a cross-origin browser would
refuse the write before issuing it.

**Compile/unit verification: NOT RUN this pass** (see Current decision). No
green run is claimed, and this entry must not be closed until one exists.

**Method note carried forward.** The sibling entry's 2026-08-21 pass replaced a
structural claim with a runtime A/B and still missed this, because the A/B
probed the wrong topology — direct-to-storage, not through the doorway the
seeder actually uses. The durable lesson is narrower than "prove it at runtime":
**prove it at runtime over the same path the reporting client takes.** A probe
that skips a hop cannot see a defect that lives in that hop.
