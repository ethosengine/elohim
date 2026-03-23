# Commons Reach Bypass at Storage HTTP Layer

**Date:** 2026-03-23
**Status:** Approved
**Scope:** Serve commons/public content without authentication at the storage HTTP layer

## Problem

The Angular app calls `/auth/account` before rendering content. When unauthenticated (web2 visitor via doorway), this returns 401 and blocks rendering. But commons content — ratified by the network for the widest audience — should be viewable without auth.

## Design

### Enforcement at Storage Layer

Storage is the single API boundary for all clients (doorway proxy + Tauri direct). Reach enforcement happens here so behavior is consistent everywhere.

In the content GET handler (`handle_db_content_by_id`):
- `"commons"` or `"public"` reach → serve unconditionally
- Any other reach → return 403 with `{ "error": "Authentication required", "requiredReach": "{reach}" }`

In list handlers (`handle_db_content_list`, `handle_db_paths_list`, `handle_db_paths_by_id`):
- When no auth header present, filter results to only commons/public content
- When auth header present, serve all content (full attestation-based filtering comes in a future sprint)

### Angular Side

The content viewer should attempt to load content directly, not gate on `/auth/account` success. If storage returns 403, show login prompt. If it returns content, render it.

### Future: Attestation-Based Auth

This sprint only handles the commons bypass. Restricted content (community, trusted, intimate) requires a signed reach claim verified against DHT-backed attestations. That's a separate sprint — the gate points established here will accept the attestation header when it arrives.

## Files Changed

| Action | File | What |
|--------|------|------|
| Modify | `elohim/elohim-storage/src/http.rs` | Reach check on content GET, list filtering |
| Modify | `app/elohim-app/src/app/lamad/` | Content viewer: don't block on auth/account for content loading |

No changes to doorway, seeder, P2P layer, or EPR protocol.
