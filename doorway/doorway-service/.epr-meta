---
epr-meta-version: 1
id: doorway-service-governance
covers: subtree
purpose: >
  The Rust web2-projection gateway for the elohim substrate — HTTP/WS ingress, the manifest-driven
  route registry, single-target storage proxy + projection cache, OAuth/JWKS federation, and the
  conductor pool for hosted identity. This manifest CLAIMS full responsibility for the subtree
  (covers: subtree): the coverage walk terminates here, integrity by construction — the way the core
  never re-audits an app-manifest's vocabulary (seam-map §3.7). It carries NO author-time rule, by the
  considered decision recorded below, not by omission.
---

# doorway/doorway-service/ — the web2 projection seam (atlas §3.9, Track 4)

A doorway is the "porch" of the P2P network: it makes substrate truth legible to browsers and the
traditional internet, and is **not itself a P2P participant**. Almost nothing is authored here — routes
are manifest-driven (a peer's storage declares them via `build_manifest()`, the registry compiles them,
the doorway serves them; this is why 13 identical per-domain proxy files were deleted and must never
return). The crate's full orientation lives, gospel-tier and co-located, in `CLAUDE.md` and `../CLAUDE.md`.

## Considered, no enforcing rule (the deliberate opt-in, not a nag)

This directory has two real, recurring, multi-incident traps. Neither is expressible as an *enforced*
`.epr-meta` predicate, and both already carry stronger, already-mechanized backstops — so a rule here
would fire on the wrong moment, or fire on nothing (the footgun this toolkit's own `validate_meta`
guards against). Sibling precedent for this outcome on a code tree: `.claude/scripts/.epr-meta`.

1. **The `is_service_path` two-gate.** A new GET route on the 8080 main listener needs BOTH the
   `match (method, path)` arm AND an entry in `is_service_path()` — the two live ~1000 lines apart in
   `src/server/http.rs` (32 references) — or the EPR router shadows the route to the SPA bundle (the
   `/auth/portal`, `/sync`, `/metrics` shadow-incident shape; [[project_doorway_main_route_needs_is_service_path]]).
   This is an *intra-file, two-location sync invariant* over http.rs content, not a placement /
   frontmatter / dedupe concern, so no enforced declarative predicate fits. The only shaped fit,
   `validator: epr:<name>`, has no registered validator-EPR in v1 and would silently degrade to an inert
   advisory. The real gate is the co-located CLAUDE.md "Adding New Routes" discipline plus its required
   `is_service_path` unit test — caught at author-time there, not from here.

2. **The edge-bake Dockerfile COPY.** A new (or transitively pulled-in) workspace path-dep crate in
   `Cargo.toml` needs a matching `COPY elohim/<crate> ./elohim/<crate>` in `Dockerfile`, or the edge
   image fails to build — but only on `dev` (the sole branch that triggers the edge build), invisibly on
   feat/sprint ([[project_new_path_dep_needs_dockerfile_copy]]). This is a *cross-file sync invariant*
   (`Cargo.toml` ↔ `Dockerfile`), which no enforced predicate expresses (`require-sibling` is narrow —
   it fires only when a new *subtree* is born and cannot pair two files in an existing directory). The
   real backstop is the CI edge-build itself, plus the co-located CLAUDE.md "Edge-bake trap" note and
   `cargo tree -i <crate>` proof.

And the predicates that *would* parse here are actively wrong for this crate: `no-new-subdirs` would
toll its legitimate, ongoing modularization (`server/`, `projection/` were added within the last week),
and `require-sibling: ".epr-meta"` would directly contradict the `covers: subtree` claim above by
demanding every new module carry its own manifest. So this is the considered-coverage outcome:
responsibility owned, no redundant gate. Add a rule here only if a NEW recurring, mechanizable,
genuinely `.epr-meta`-shaped drift appears in this tree.
