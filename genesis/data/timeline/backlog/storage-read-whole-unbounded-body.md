---
id: "backlog-storage-read-whole-unbounded-body"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "elohim-storage's read_whole collects a whole request body before any check, so a route's own size limit runs only after the bytes are in memory"
slug: "storage-read-whole-unbounded-body"
written: "2026-10-16"
author: "graduate-onto-own-node-key review (Codex, read-only), 2026-10-16"
status: "backlog"
priority: "medium"
jobs: [elohim-edge]
shift_objective: |
  Give every read_whole caller a byte bound enforced while the body is read (413 on overflow),
  the way api/observations.rs and api/compute_tasks.rs already use http_body_util::Limited,
  without changing the exact bytes a session proof signs.
tags: [elohim-storage, http, resource-bounds]
---

**The fact.** `HttpServer::read_whole` (`elohim/elohim-storage/src/http.rs`) calls `body.collect()` and
returns the whole body. Its callers read the body first and check its size afterwards, if they check it at
all. So the size limit doesn't stop a large or chunked body from being buffered whole. Six callers predate
this row (http.rs, about lines 14252, 14367, 14863, 15033, 15259 and 15422, at the time of writing). The
account-bind sign route added on 2026-10-16 reads its body through a bounded read instead.

**Not measured.** A Codex read-only review found this. Nobody has sent an oversized body to a running
node, and this row does not establish which of the six routes can be reached before authentication.

**Fix shape.** Have `read_whole` take a byte limit, wrap the body in `Limited::new(body, limit)`, and map the
length error to 413. Each caller passes the limit it already checks.
