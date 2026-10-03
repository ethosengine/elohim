---
epr-meta-version: 1
id: doorway-service-governance
covers: subtree
purpose: >
  The Rust web2-projection gateway for the elohim substrate — HTTP/WS ingress, the manifest-driven
  route registry, single-target storage proxy + projection cache, OAuth/JWKS federation, and the
  conductor pool for hosted identity. This manifest CLAIMS full responsibility for the subtree
  (covers: subtree): the coverage walk terminates here, integrity by construction — the way the core
  never re-audits an app-manifest's vocabulary (seam-map §3.7). It carries two author-time advisories,
  both born from the 2026-10-03 hosted-registration root cause: signing-credential issuance and
  hosted provisioning. The two older traps recorded below still carry no rule, by considered
  decision, not by omission.
rules:
  - id: authority-one-grant-per-relationship
    class: inject
    when:
      write: "*.rs"
      contains-any: ["authorize_signing_credentials", "grant_zome_call_capability", "GrantedFunctions::All", "functions: None"]
    dedupe-of: genesis/docs/superpowers/specs/2026-10-03-plane-separation-design.md
    retire-when: >
      when the conductor's grant is idempotent by entry hash and the doorway derives its credential
      from a root secret, so a second grant for one (grantee, cell) cannot be authored from this crate
      by construction, and a test forces a mint timeout and reads one live grant.
    why: >
      A GRANT IS A PERMANENT CHAIN ACTION ON A PERSON'S CHAIN — plane-separation §3. Three traps live
      on this path, all measured on alpha: (1) a mint wrapped in a timeout is abandoned by the doorway
      but finished by the conductor (the grant landed 4 s after the 10 s deadline), and because the
      keypair and secret are generated per call the next attempt authors another — never re-roll a
      credential on retry; present the same one. (2) `functions: None` / `GrantedFunctions::All` is an
      all-function credential; first-party grants are assigned and function-listed. (3) A grant is
      never the answer to "this is my own conductor": the DNA's invocation gates separate the person
      from a process under grant, and the author path would pass every one silently. Do not add a
      revoke-per-session or a timed rotation either: a grant has no expiry field, so both only mint
      more. Advisory only.
  - id: provisioning-is-not-a-zome-call
    class: inject
    when:
      write: "*.rs"
      contains-any: ["provision_on", "is_transport_fault", "PROVISION_TRANSPORT_ATTEMPTS", "IDENTITY_CREATION_FAILED", "find_existing_app", "DEFAULT_ZOME_CALL_TIMEOUT_MS"]
    dedupe-of: genesis/docs/superpowers/specs/2026-10-03-plane-separation-design.md
    retire-when: >
      when hosted registration is an asynchronous saga with typed sent/unsent faults and a
      conductor-reported capacity signal, and the falsifier reads zero enabled apps without an
      account per day and never two apps for one identifier.
    why: >
      CREATING A HOSTED HUMAN IS A SAGA, NOT A CALL — plane-separation §4. The first grant on a fresh
      cell runs zome init and two write transactions on a database every hosted agent of that DNA
      shares; the conductor waits 15 s for that writer, so a 10 s caller always gives up first.
      Raising the deadline is the wrong fix (a limit raise is a design signal): take the person off
      the path instead. Before you change a retry or a fault matcher here: each attempt mints a FRESH
      agent key, so re-offering an install whose outcome is unknown gives one person two identities —
      only "never sent" may be re-offered. Do not classify faults by substring of rendered error text
      (the live `Websocket closed: ConnectionClosed` matched no marker). Do not auto-uninstall after
      `create_human` may have landed. Do not count capacity from this doorway's own persisted
      mappings (it read 94 on conductors holding five cells). Advisory only.
---

# doorway/doorway-service/ — the web2 projection seam (atlas §3.9, Track 4)

A doorway is the "porch" of the P2P network: it makes substrate truth legible to browsers and the
traditional internet, and is **not itself a P2P participant**. Almost nothing is authored here — routes
are manifest-driven (a peer's storage declares them via `build_manifest()`, the registry compiles them,
the doorway serves them; this is why 13 identical per-domain proxy files were deleted and must never
return). The crate's full orientation lives, gospel-tier and co-located, in `CLAUDE.md` and `../CLAUDE.md`.

## The two advisories

Both rules above inject their reason when a matching line is written; neither blocks. They exist
because the 2026-10-03 hosted-registration 503 was five defects on one path, each of which looked
reasonable in isolation, and the enforcing halves (an idempotent conductor grant, typed faults, the
saga) are not built yet. Each carries its own `retire-when`.

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
