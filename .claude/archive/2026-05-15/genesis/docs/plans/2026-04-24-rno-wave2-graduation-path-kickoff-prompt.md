I want to write the R&O graduation-path narrative. This is Wave 2 sub-project #9 from the R&O lessons roadmap — a small writing sprint that tells Sasha, the VF team, and R&O's existing users what it *feels* like for a Moss group running R&O to graduate its exchange history into the Elohim Protocol. Not a technical spec. A user story.

Doing this before #4 (hREA / VF-GraphQL) is deliberate — the recipe reveals what's missing in #7, #8, and especially #4, so the bigger design work doesn't start in the dark.

## Context (self-contained)

The R&O lessons roadmap handoff is at `genesis/docs/plans/2026-04-21-rno-lessons-roadmap-handoff.md`. The nine sub-projects are decomposed in §5; this sprint addresses **sub-project #9 only** — do not expand scope into #4 or #8.

Current state on `dev` (as of 2026-04-24 post-Batch D merge):
- EPR Phase 2C federation-complete ✅ (Batches A–D all green — the graph substrate is no longer theoretical).
- Sweettest #3 🟡 verifying — bodies landed, ignore-flip pending Jenkins green per DNA.
- hREA / VF-GraphQL #4 🔴 not started — this sprint teaches us what it must cover.
- Moss Weave Tool #8 🔴 not started — this sprint teaches us what it must do.
- R&O graduation path #9 🔴 not started ← **this sprint**.

### Why the narrative form

The target reader is not an elohim contributor. It's a user deciding whether to graduate a Moss group, or a Holochain-team member deciding whether elohim is the natural landing zone for the future they're building. A technical spec loses them; a lived walkthrough keeps them.

The structural pattern to borrow: "A day in the life" style — pick one concrete Moss group with real-feeling members (steward, contributor, edge case), walk through graduation from their POV, surface the protocol's commitments as incidental observations rather than section headings.

### What the narrative must cover (not as section headings — as lived moments)

1. **Before graduation** — a Moss group running R&O with three months of exchange history, a few dozen members, and a growing "why does this stop at the group boundary" frustration. What do they *want* that they can't get today?
2. **The moment of graduation decision** — is this a group vote? A single steward acting? Per-member opt-in? (Handoff §5 #9 "Brainstorm questions" raises this as open; pick a defensible default and justify it briefly in an author's aside, not as a bullet list.)
3. **Identity handoff** — how does an R&O member's agent key relate to their imagodei identity? Same key linked? Separate keys with a bridge? A graduation ceremony that signs a linkage claim? The steward recovery memories (`project_graduated_recovery_authority.md`, `project_peer_native_account_canonical_surface.md`) are load-bearing here.
4. **History carries** — exchange events become EPRs. What does the VF vocabulary look like on the wire (stub level — this is where #4 inherits requirements, not where you design #4)? What doesn't carry (R&O's group-private signals? Archived states? Scoring?) — those gaps are the honest part of the story.
5. **What the R&O group keeps** — the DHT continues or sunsets; the Moss group continues or decommissions; members who don't graduate still see the group's historical state. Make the answer concrete with a specific scenario.
6. **What the protocol gains** — the group's graph presence. New visibility. New composability. A concrete example: a member who graduated runs a search across R&O-origin EPRs + non-R&O EPRs and actually gets a useful result. Don't hand-wave.
7. **The pitch, made concrete** — by this point the reader should feel the difference between "R&O as a walled hApp" and "R&O graduated into the protocol's graph." Sasha's read of this should be: *yeah, that's the substrate I wanted but couldn't build*.

### What this document must NOT do

- Not a technical spec. No tables of types, no wire formats, no sequence diagrams (one concrete illustration fine, more than one is too many).
- Not a decision log. Decisions the author made appear as asides, not debates.
- Not a promise. Nothing here commits elohim to ship any of it on any timeline. The document is a **recipe** and a **vision artifact**.
- Not a pitch deck. Don't set it up for slides. Prose flow.

## Sprint scope

**In scope:**
1. One prose document at `genesis/docs/plans/2026-04-24-rno-graduation-path.md` (plan dir, not superpowers — this is audience-facing, not execution-shaped).
2. 1500–3000 words. Under 1500 and it reads thin. Over 3000 and it's a spec in narrative clothing.
3. One concrete illustrative example embedded in the flow. Pick a realistic Moss group archetype — a mutual-aid cooperative, a craft guild, a research circle. Name names (fictional) so it reads alive.
4. A closing "what this tells us about #4 and #8" section (max 300 words) — explicit handoff to the downstream sub-projects. This is how the graduation-path informs hREA alignment and Moss Tool packaging without designing either.

**Out of scope:**
- Designing #4 (hREA / VF-GraphQL mapping). Narrative only surfaces the requirements; the design sprint happens after.
- Designing #8 (Moss Weave Tool packaging). Same.
- Defining the identity-handoff protocol. Narrative can describe what it feels like; the cryptographic design is Phase-separate work.
- Contacting Sasha / the VF team / anyone at R&O about the doc. This is an internal artifact first. A later sprint decides whether/how to share it.

## Known unknowns the author should resolve before drafting

Brief think-through at session start (~15 minutes, not a full brainstorm):

1. **Narrative POV.** First person (a member's voice), omniscient narrator, or an author-to-reader direct address? The handoff memory bank (`project_elohim_vision_fruit_back_on_tree.md`, `project_stewardship_philosophy.md`, `project_ungrudging_service.md`) leans toward direct address — *here is what graduation looks like, told like a story you can verify*. Pick one and commit.
2. **Identity handoff default.** Pick the most defensible answer from the handoff §5 #9 brainstorm questions: same key linked (simplest, identity already self-sovereign), separate keys with attestation bridge (safer, preserves R&O's historical agent), or the ceremony in between. The memory `project_peer_native_account_canonical_surface.md` on OAuth-pattern graduation is the relevant scaffolding. Whatever you pick, the narrative must make it feel natural, not cryptographic.
3. **What does "Moss group running R&O" look like concretely?** Describe the baseline — which Moss UI, which R&O features active, how many members, what the frustrations are. Lean on the R&O drift analysis in §2 of the handoff for real features (active/archived listings, markdown descriptions, organization contacts).
4. **Graduation governance.** The handoff raises: is graduation a group vote, a steward's call, or per-member? Reconcile against `project_social_compute_collective_is_stewardship_unit.md` — design for collective-as-stewardship-unit, not household-specific. Default (justify in text): a qahal-style call with individual opt-in for history portage; the group's DHT continues for non-graduating members.
5. **What stays with the R&O DHT.** Be specific. Does graduation mean archive + sunset? Continue alongside? Hybrid — R&O for ongoing group-internal coordination, elohim for the graph-visible layer? A single concrete choice beats a "depends" — pick one.
6. **The concrete example's scope.** A mutual-aid group doing time-banking. A craft guild coordinating commissions. A neighborhood food network. Pick one that lets you show exchange events (VF-shaped), identity continuity, and a payoff at the protocol-graph level. Don't describe the group in a paragraph — let them come through in the scenes.

Write the resolutions as a ~150-word preamble at the top of the plan (author's framing note), then draft.

## How to run this session

1. Check branch. Default: fresh branch `wave2-rno-graduation-path` off `dev` (Wave 1 sweettest ignore-flip is still running on its own branch; don't entangle).
2. Resolve the six known-unknowns above in ~15 minutes. Write the 150-word framing note.
3. Draft the narrative — one pass, don't optimize per paragraph. Target 2000 words for the first draft.
4. Second pass: compress. Cut the asides. Make the concrete example sharper. Add the closing "what this tells us about #4 and #8" section.
5. Self-review against the §"What the narrative must cover" checklist — did lived moments 1–7 actually land? If a moment is in a section header rather than in the flow, rewrite.
6. Read it aloud. Narrative docs live or die on cadence; silent reading hides bad rhythm.
7. Commit as `docs(rno): graduation-path narrative (wave 2 #9)`. One commit for the whole doc. Husky runs docs-only lint; don't bypass.
8. Update the handoff doc `genesis/docs/plans/2026-04-21-rno-lessons-roadmap-handoff.md` §0 — flip sub-project #9 from 🔴 to ✅ with a pointer to the new file.

## Constraints & conventions

- **No sovereignty, no ownership language.** Stewardship, not ownership. Graduated capability, not claims. Memory: `project_no_sovereignty_stewardship_over_ownership.md`. If you find yourself writing "the group owns its data," stop — the group stewards it.
- **Ungrudging service.** The protocol's gift flows whether R&O acknowledges it or not. The narrative should not read as "R&O validates elohim"; it reads as "elohim makes R&O's work more than R&O alone could be." Memory: `project_ungrudging_service.md`.
- **Collective as stewardship unit.** Design for Moss groups but think collective-general. The Moss-groups specifics are one instance of a broader pattern. Memory: `project_social_compute_collective_is_stewardship_unit.md`.
- **The pitch is for Sasha, made concrete.** Not "here's what elohim does." "Here is what your users can do when they graduate." Memory: `project_elohim_vision_fruit_back_on_tree.md`.
- **Schema-first is irrelevant here.** This is prose. Don't write a JSON schema in passing.
- **No new memories.** This sprint consumes memories and writes one narrative. It does not modify memory unless the author surfaces a new framing constraint that needs to be preserved — and even then, a memory is written only if the constraint will recur.

## Plan location

`genesis/docs/plans/2026-04-24-rno-graduation-path.md` — plan dir, not superpowers. This is vision-shaped, audience-facing. Memory: `reference_superpowers_docs_location.md`.

## Definition of done

- [ ] `genesis/docs/plans/2026-04-24-rno-graduation-path.md` exists, 1500–3000 words, narrative-shaped.
- [ ] Opens with a ~150-word author framing note resolving the six known-unknowns.
- [ ] Contains one concrete Moss-group example threaded through the document, not bolted on.
- [ ] Covers lived moments 1–7 from §"What the narrative must cover" — in flow, not as headers.
- [ ] Closes with a "what this tells us about #4 and #8" section under 300 words.
- [ ] No table of types, no sequence diagram, no decision log (at most ONE concrete illustration).
- [ ] Handoff doc §0 updated — #9 🔴 → ✅ with pointer.
- [ ] Committed on `wave2-rno-graduation-path` branch with husky passing.

## Memories worth checking on start

- `project_elohim_vision_fruit_back_on_tree.md` — the "why we exist" framing that the pitch rests on.
- `project_no_sovereignty_stewardship_over_ownership.md` — language hygiene throughout.
- `project_stewardship_philosophy.md` — graduated capability is the frame for identity handoff.
- `project_social_compute_collective_is_stewardship_unit.md` — design for collective-general, not household-specific.
- `project_ungrudging_service.md` — tone hygiene; no gating on R&O's acknowledgment.
- `project_peer_native_account_canonical_surface.md` — the OAuth-pattern graduation gives the identity-handoff shape.
- `project_graduated_recovery_authority.md` — informs the "what does graduation change about recoverability" aside.
- `project_epr_substrate_vs_vf_graphql.md` — EPR is the substrate, VF-GraphQL is app-layer; don't conflate in the narrative.
- `reference_superpowers_docs_location.md` — plans go in `genesis/docs/plans/`, superpowers in `genesis/docs/superpowers/`.

Go.
