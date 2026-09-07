---
id: "backlog-prologue-seed-step-relationship-type-invalid"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "just mesh prologue: seed-base-corpus-via-A writes 1/5 content and 0/8 paths — the seeder posts relationship_type 'step' which relationship_service.rs:314 rejects (HTTP 400), so downstream propagate-to-B and the B-side serverBlobHash stamp fail on a fresh mesh"
slug: "prologue-seed-step-relationship-type-invalid"
written: "2026-09-07"
author: "overnight shift 2026-09-07"
status: "open"
priority: "medium"
jobs: [elohim-genesis]
cluster: "arch-dataplane-refactor-backlog"
tags: [seeder, prologue, mesh, vocabulary-drift]
---

**Evidence (2026-09-07 09:1xZ, fresh mesh from dev 2025afc9c):** prologue leg `seed-base-corpus-via-A` EXIT=1 with "Step relationships bulk create failed: HTTP 400: relationship_type 'step' is not valid. Valid types: [CONTAINS, BELONGS_TO, DESCRIBES, IMPLEMENTS, …]" and post-flight "No paths were written! Expected 8 new paths, got 0"; then `propagate-lamad-spa-to-B`, `propagate-landing-to-B-*` and `stamp-server-projection-peers` (404 on jessica/james) failed. Not introduced by today's merge (the 'step' type predates it) but it makes a fresh-mesh prologue red. Two homes disagree on the relationship vocabulary: the seeder's path-step edges vs `elohim/elohim-storage/src/services/relationship_service.rs` (and the lamad manifest). **Done when:** the seeder and storage agree on the path-step relationship type (schema-owned), and a fresh `just mesh start && just mesh prologue` ends EXIT=0.

**Root cause found 10:2xZ (fixtures-clone leg):** the 0/8 paths and 1/5 content are not the 'step' vocabulary alone — `create_content` returns `WasmError Deserialize` because `genesis/seeder/src/seed-production.ts:427-448` never sends `reach`, which `lamad_types::CreateContentInput` (`elohim/sdk/domains/lamad/types/src/lib.rs:39`) requires with no serde default. Seeder↔DNA input drift; the 'step' 400 is a second, independent drift. Both must close for a fresh-mesh prologue to end green.
