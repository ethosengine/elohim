---
id: feedback-diesel-migration-timestamp-collision
name: Diesel migration timestamp collisions silently drop migrations
title: Diesel migration timestamp collisions
description: "Two migration dirs sharing a YYYY-MM-DD-HHMMSS prefix make embed_migrations! keep only one — runtime 'no such table' while cargo build passes."
type: feedback
originSessionId: 53d58b9b-be66-4db2-bfb9-75f7f377aed9
cites:
  - elohim/elohim-storage/src/db/mod.rs
---

Fixture snapshot of this entry's frontmatter; the body is deliberately not copied.
