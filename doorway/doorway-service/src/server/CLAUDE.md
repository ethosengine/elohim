# Doorway Server — Route Registry Anti-Pattern Gate

**Before adding ANY route to `http.rs`**, answer this question:

> Does this route need doorway-specific logic (auth gating, path rewriting, WebSocket upgrade, non-storage target)?

- **NO** → Do NOT add it here. Add the endpoint to elohim-storage and register it in `build_manifest()`. The RouteRegistry auto-discovers it.
- **YES** → Add a match arm ABOVE the registry fallback. Document why the registry can't handle it.

We deleted 13 identical proxy files that violated this rule. See `doorway/CLAUDE.md` for the full anti-pattern catalog.

## How Routes Work

```
Request → http.rs match block
  ├─ Built-in routes (health, auth, admin, conductor, bootstrap, signal, cache)
  ├─ Special routes with doorway-specific logic (few, justified)
  ├─ RouteRegistry lookup → forward_to_storage()  ← THIS handles 90% of routes
  └─ 404
```

New storage endpoints become routable automatically when declared in storage's `build_manifest()`. No doorway code changes needed.
