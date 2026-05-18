---
name: sdk-scaffolding-generators
description: "The 2026-05-18 manifest-modularization + SDK-boundary sprint surfaced 6+ repeatable patterns that consuming-developer tooling should mechanize (ng-generate-style). Candidates: scaffold a new content-type, add a new View/InputView pair, add a JSON schema + matching Rust struct, split a monolithic manifest into modular form, add a new domain manifest, add a new signal-kind. Operator's framing: 'utility methods for SDK help in future app-manifest implementations for consuming developers.' Audience is app authors building on the protocol, not protocol maintainers."
metadata:
  node_type: memory
  type: project
---

The manifest-modularization sprint (2026-05-18, see `genesis/docs/plans/2026-05-18-app-manifest-modularization.md`) made 28 identical content-type extractions visible. The SDK boundary work (Plan 2, in progress) repeats the same shape 7 times for per-domain View moves. The CLAUDE.md at `elohim/elohim-storage/CLAUDE.md` already documents a 5-step ritual for adding a new entity (model → View → InputView → schema → handler → codegen). These are not one-offs; they are the *protocol's surface area as an SDK*, and right now every consuming developer (us included) is hand-walking the steps.

## The framing — what tooling looks like

Audience: a developer building an app **on top of** the Elohim Protocol. They have a vision for their app's content types, governance affinities, signal kinds. The SDK should let them go from "I want a new contentType called `recipe`" to "it works end-to-end" with one command — not 28 manual edits.

Inspirations: Angular's `ng generate` (`ng generate component foo` scaffolds .ts + .html + .css + .spec.ts + module registration + barrel export); Nx generators; Rails scaffolds; cargo-generate.

## The recognized repeatable patterns (discovered 2026-05-18)

| # | Pattern | Today's effort | Proposed CLI command |
|---|---|---|---|
| 1 | Add a new content type to a domain manifest | Hand-write JSON file under `manifest/content-types/<name>.json`, add `$ref` to shell manifest, regenerate types, verify byte-identical | `elohim-sdk add content-type <domain> <name> --renderer=<X>` |
| 2 | Add a new View + InputView pair (HTTP boundary) | 5-step ritual per `elohim/elohim-storage/CLAUDE.md` — Diesel model, View, InputView, schema, route handler | `elohim-sdk add view <domain> <Name> --writes=<bool>` |
| 3 | Add a JSON schema with matching Rust struct (schema-first contract) | Write schema, write struct, add to `INTERFACE_FILES` in codegen-ts.mjs, run codegen, add schema_contract test | `elohim-sdk add schema <name>` |
| 4 | Split a monolithic manifest into modular form | What Plan 1 just did, 30+ times for lamad | `elohim-sdk split-manifest <domain>` (one-shot migration) |
| 5 | Add a new domain manifest | Scaffold `manifest.json`, `scripts/codegen.mjs`, register in workspace | `elohim-sdk new-domain <name>` |
| 6 | Add a new signal-kind | Extend `feedback_signal.rs` whitelist, add entry to manifest's signalKinds, write schema, document | `elohim-sdk add signal-kind <name> --target-kinds=<...>` |

Each of these has the same shape: **template file(s) + manifest insertion(s) + codegen + verification**. The SDK CLI is the "ng generate" that bundles them.

## Where it should live

Two reasonable homes; pick after a small spike:

- **`elohim/sdk/cli/`** as a Node.js CLI bundled with `@elohim/storage-client`. Pro: same toolchain as codegen scripts, easy to share with browser-side consumers. Con: anything that needs to touch Rust source needs to shell out or template carefully.
- **`crates/elohim-sdk-cli/`** as a Rust binary. Pro: type-safe templates, can use `serde_json` + cargo metadata directly, friendly to Rust-only consumers. Con: distribution to JS-only consumers is harder.

A `cargo install elohim-sdk-cli` + a thin `npx @elohim/sdk` Node wrapper that shells out to the Rust binary is the best of both worlds.

## Why this is load-bearing for the mission

The protocol's claim is "subsume web2/walled-garden by being a substrate, not a platform" — but a substrate without tooling is a library, and libraries-without-tooling are how protocols stall. Foster's framing (see `[[project_mission_platform_for_collective_biography]]`): the protocol's audience-of-first-resort is the personas in `genesis/data/humans/`. Tomorrow's audience is the developer who wants to build an app for those personas. If that developer needs to hand-walk 28 contentType insertions to add a domain manifest, the friction kills the experiment before it starts.

## How to apply

1. **Don't ad-hoc extract-and-paste these patterns again.** When the next domain (shefa, qahal, etc.) needs the manifest split, stop and write the CLI command instead.
2. **The Plan 2 per-domain View moves (T5-T10) are themselves a repetition signal.** If after T4's pilot the recipe is mechanical, the move itself is a candidate generator: `elohim-sdk move-views-to-sdk <domain>` — the spec for that generator is essentially Plan 2.
3. **The codegen scripts already exist** (`elohim/sdk/schemas/scripts/codegen-ts.mjs`, `lamad/scripts/codegen.mjs`, `codegen-manifest.mjs`). The CLI is glue + templating on top of those, not a from-scratch tool.
4. **Track the patterns** in `genesis/docs/specs/2026-05-18-sdk-scaffolding-cli-spec.md` (write the spec next, after Plan 2 T4 pilot validates the per-domain view-move recipe).

Related: `[[feedback_design_for_a_generation_no_shortcuts]]` (prefer standards even at higher cost); `[[project_elohim_dna_as_sdk_boundary]]` (the SDK is a first-class contract); `[[project_mission_platform_for_collective_biography]]` (the audience is the personas, served via apps built by developers who should not be hand-walking 28 inserts).

## When this becomes its own sprint

After Plan 2 (SDK boundary) lands, and ideally during/after Plan 3's monolithic decomp — because the decomp surfaces what the SDK's stable surface area actually IS. The CLI is the user-facing layer over that boundary. Sequence: boundaries first, then tools.

Sprint shape (when commissioned):
1. Write the spec at `genesis/docs/specs/2026-05-18-sdk-scaffolding-cli-spec.md`
2. Pilot one generator end-to-end (probably #1 — add content-type — smallest blast radius)
3. Generalize the templating layer
4. Add the remaining generators
5. Document in `elohim/sdk/CLAUDE.md` and the `@elohim/storage-client` README
