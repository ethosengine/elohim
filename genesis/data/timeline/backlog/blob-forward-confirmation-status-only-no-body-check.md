---
id: "backlog-blob-forward-confirmation-status-only-no-body-check"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "forwarded_to_storage:true is the deploy's only proof storage took a bundle, and the read-back behind it checks the response STATUS without consuming or hashing the body"
slug: "blob-forward-confirmation-status-only-no-body-check"
written: "2026-09-21"
author: "serving-edge stage-spa-blob readiness campaign, 2026-09-21 third review"
status: "open"
priority: "medium"
jobs: [elohim-edge, elohim]
tags: [doorway, seed, blob, deploy-evidence, storage-forward]
---

**The fact.** `scripts/ci/stage-spa-blob.sh` now refuses to call a byte-seed staged without an affirmative
top-level `forwarded_to_storage:true` in the doorway's `PUT /admin/seed/blob` answer — a deliberate tightening,
because the alternative evidence (a `GET /blob/{hash}` through the doorway) is cache-first
(`doorway/doorway-service/src/routes/blob.rs:621`) and proves only that the doorway remembers the bytes we just
sent it. That makes this one boolean the deploy's entire basis for believing storage holds a bundle, so its own
limit is worth writing down: `forward_to_storage` sets it from a storage PUT followed by a storage GET, and the
read-back at `doorway/doorway-service/src/routes/seed.rs:606` checks the response STATUS only — it neither
consumes nor hashes the returned body. It therefore confirms "storage answered both calls successfully", not
"storage returned these exact bytes". Storage does hash cached bytes before writing
(`elohim/elohim-storage/src/http.rs:3317`), which covers the write side; what is unverified is the read side of
the confirmation.

**Why it matters.** The 2026-08-22 local-mesh incident is the shape to avoid: a 69MB bundle hit a storage-side
shard-verify drop, the staging leg stamped ✓, and every peer declared a head for a bundle nobody could
materialize. The current confirmation is strictly stronger than what that incident had, but a read-back that
never looks at the bytes cannot distinguish "serves the blob" from "serves a 200 for that path".

**Not fixed here on purpose.** The readiness campaign that surfaced this is a CI-script change under an explicit
no-doorway-code rule, and the cure belongs in the doorway: have `forward_to_storage`'s read-back consume the body
and compare its sha256 to the expected hash (or read a storage-supplied digest header), then keep
`forwarded_to_storage` meaning exactly "storage serves these bytes". Until then, treat a confirmed forward as
evidence about storage's responses rather than about storage's content.

**Done when:** the read-back verifies content, not just status, and `forwarded_to_storage:true` can be cited as
proof the bytes are retrievable — or the field is renamed to say what it actually establishes.
