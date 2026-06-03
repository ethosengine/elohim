# SDK Scaffolding CLI Spec

**Audience:** Developers building apps on top of the Elohim Protocol.
**Motivation:** The manifest-modularization sprint made 28 identical content-type extractions visible as a single repeatable pattern. The SDK-boundary work repeated the same shape 7 times for per-domain View moves. The `elohim/elohim-storage/CLAUDE.md` already documents a 5-step ritual for adding a new entity. These are not one-offs — they are the *protocol's surface area as an SDK*, and every consuming developer (us included) is hand-walking the steps.

The protocol's claim is "subsume web2/walled-garden by being a substrate, not a platform" — a substrate without tooling is a library, and libraries-without-tooling are how protocols stall. If a developer needs to hand-walk 28 `contentType` insertions to add a domain manifest, that friction kills the experiment before it starts.

---

## Recognized Repeatable Patterns

The following patterns were discovered in the 2026-05-18 modularization sprint and the SDK-boundary clarification work:

| # | Pattern | Today's effort | Proposed CLI command |
|---|---|---|---|
| 1 | Add a new content type to a domain manifest | Hand-write JSON file under `manifest/content-types/<name>.json`, add `$ref` to shell manifest, regenerate types, verify byte-identical | `elohim-sdk add content-type <domain> <name> --renderer=<X>` |
| 2 | Add a new View + InputView pair (HTTP boundary) | 5-step ritual per `elohim/elohim-storage/CLAUDE.md` — Diesel model, View, InputView, schema, route handler | `elohim-sdk add view <domain> <Name> --writes=<bool>` |
| 3 | Add a JSON schema with matching Rust struct (schema-first contract) | Write schema, write struct, add to `INTERFACE_FILES` in codegen-ts.mjs, run codegen, add schema_contract test | `elohim-sdk add schema <name>` |
| 4 | Split a monolithic manifest into modular form | What the lamad modularization did, 30+ times | `elohim-sdk split-manifest <domain>` (one-shot migration) |
| 5 | Add a new domain manifest | Scaffold `manifest.json`, `scripts/codegen.mjs`, register in workspace | `elohim-sdk new-domain <name>` |
| 6 | Add a new signal-kind | Extend `feedback_signal.rs` whitelist, add entry to manifest's `signalKinds`, write schema, document | `elohim-sdk add signal-kind <name> --target-kinds=<...>` |

Each has the same shape: **template file(s) + manifest insertion(s) + codegen + verification**. The CLI bundles them, Angular `ng generate`-style.

---

## Inspiration

- Angular's `ng generate` — scaffolds `.ts` + `.html` + `.css` + `.spec.ts` + module registration + barrel export in one command.
- Nx generators, Rails scaffolds, `cargo-generate`.

---

## Recommended CLI Location

Two homes; decide after a small pilot spike:

- **`elohim/sdk/cli/`** as a Node.js CLI bundled with `@elohim/storage-client`. Same toolchain as the existing codegen scripts; easy to share with browser-side consumers. Anything touching Rust source must template carefully or shell out.
- **`crates/elohim-sdk-cli/`** as a Rust binary. Type-safe templates, can use `serde_json` + cargo-metadata directly, friendly to Rust-only consumers. Distribution to JS-only consumers is harder.

Preferred resolution: `cargo install elohim-sdk-cli` + a thin `npx @elohim/sdk` Node wrapper that shells out to the Rust binary — best of both worlds.

---

## Existing Codegen to Compose On Top Of

The CLI is glue + templating over existing scripts, not from scratch:

- `elohim/sdk/schemas/scripts/codegen-ts.mjs` — protocol type generation
- `elohim/sdk/domains/lamad/scripts/codegen.mjs` — domain type generation
- `elohim/sdk/domains/lamad/scripts/codegen-manifest.mjs` — manifest validation + resolution

---

## Sequencing

**Boundaries first, then tools.** The CLI is the user-facing layer over the SDK boundary; the boundary must be stable before the CLI is worth writing.

Trigger: after Plan 2 (SDK boundary) lands, and ideally during/after Plan 3's monolithic decomposition — because the decomp surfaces what the SDK's stable surface area actually IS.

Sprint shape (when commissioned):

1. Write/refine this spec based on Plan 2 + Plan 3 discoveries
2. Pilot generator #1 (`add content-type`) end-to-end — smallest blast radius
3. Generalize the templating layer
4. Add remaining generators
5. Document in `elohim/sdk/CLAUDE.md` and the `@elohim/storage-client` README

**Repetition signal:** The Plan 2 per-domain View moves (T5–T10) are themselves a repetition signal. If after the T4 pilot the recipe is mechanical, the move is a candidate generator: `elohim-sdk move-views-to-sdk <domain>` — the spec for that generator is essentially Plan 2.

**Stop re-hand-walking patterns.** When the next domain (shefa, qahal, etc.) needs the manifest split, pause and write the CLI command instead.

---

## Related

- `genesis/docs/plans/2026-05-18-app-manifest-modularization.md` — the sprint whose 28 identical extractions surfaced these patterns
- `genesis/docs/plans/2026-05-18-sdk-boundary-clarification.md` — per-domain View moves that repeat the same shape
- `elohim/sdk/CLAUDE.md` — canonical SDK boundary definition + modular manifest pattern (lamad as example)
- `elohim/elohim-storage/CLAUDE.md` — the 5-step entity-addition ritual that generator #2 would replace
