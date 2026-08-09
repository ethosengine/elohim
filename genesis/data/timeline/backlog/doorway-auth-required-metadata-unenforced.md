---
id: "backlog-doorway-auth-required-metadata-unenforced"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "doorway route_registry auth_required metadata is declared but never read — no dispatch-path enforcement"
slug: "doorway-auth-required-metadata-unenforced"
written: "2026-08-09"
author: "convergence-serve-path shift"
status: "backlog"
priority: "medium"
tags: [doorway, auth, route-registry, security, dead-flag]
---

# doorway auth_required is a dead flag

Found during the 2026-08-09 authority-precedes-shed fix (elohim-storage
ab316cad7): doorway/doorway-service's route registry sets `auth_required()`
per route, but grep shows the flag is never read outside route_registry.rs —
no dispatch-path code enforces it. Today authority checks live in
elohim-storage (X-Agent-Cid at the handler), so nothing is currently
unprotected THAT WE KNOW OF — but a declared-unenforced auth flag is a trap:
a future route author will set it and reasonably believe it does something.
Either wire it into the dispatch path (admission-layer refusal before proxy)
or delete the field with a comment pointing at where authority actually
lives. Route through p2p-design-gate if wiring (authority is a capability
concern, not a REST concern).
