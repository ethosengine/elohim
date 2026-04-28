# Doorway Server — Route Registry Anti-Pattern Gate

**Before adding ANY route to `http.rs`**, answer this question:

> Does this route need doorway-specific logic (auth gating, path rewriting, WebSocket upgrade, non-storage target)?

- **NO** → Do NOT add it here. Add the endpoint to elohim-storage and register it in `build_manifest()`. The RouteRegistry auto-discovers it via the wildcard arm at the bottom of the dispatch — no doorway code change required.
- **YES** → Add a match arm ABOVE the wildcard arm. Document why the registry can't handle it.

We deleted 13 identical proxy files that violated this rule. See `doorway/CLAUDE.md` for the full anti-pattern catalog.

> **Path-prefix guards are forbidden in the wildcard arm.** Earlier versions of this dispatch gated the registry by `p.starts_with("/api/v1/") || p.starts_with("/account/")`, which silently broke every new manifest path family added since (`blob_proxy`, `stream_proxy`, …). The wildcard arm now delegates unconditionally to `classify_dispatch`. If you find yourself wanting to add a prefix check there, stop — the registry already knows its own prefixes.

## How Routes Work

```
Request → http.rs match block
  ├─ Built-in routes (health, auth, admin, conductor, bootstrap, signal, cache)
  ├─ Special routes with doorway-specific logic (collectives, elohim-agent, identity)
  ├─ Wildcard arm → classify_dispatch(...)
  │   ├─ Registry match + StorageProxy target  → forward_to_storage()
  │   ├─ Registry match + other target type    → 404 (until that target's
  │   │                                            handler is wired —
  │   │                                            BlobProxy, StreamProxy,
  │   │                                            ZomeCall, AgentProxy)
  │   ├─ No registry match + GET + slug set    → SPA bootstrap
  │   └─ No registry match otherwise           → 404
```

The wildcard arm is unconditional — it consults the RouteRegistry on every request that didn't match an explicit arm above. Any path elohim-storage declares in its manifest (routes, blob_proxy, stream_proxy, …) becomes routable without a doorway code change. Adding a new path-family-prefix to the dispatch is no longer required and is no longer a regression vector.

The dispatch tail used to be a hand-maintained list of prefixes (`/api/v1/`, `/account/`). Every new manifest path family (blob_proxy → `/blob/`, stream_proxy → `/stream/`) silently fell through to the SPA bootstrap until someone noticed thumbnails breaking. The `classify_dispatch` helper exists specifically so that pattern cannot recur.

New storage endpoints become routable automatically when declared in storage's `build_manifest()`. No doorway code changes needed.
