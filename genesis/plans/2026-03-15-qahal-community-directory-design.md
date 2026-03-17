# Qahal Community Directory Design

**Date**: 2026-03-15
**Scope**: Sprint 1 of the Governance Immune System — the community body that governance serves

## Problem

The qahal pillar has governance machinery (challenges, proposals, votes, precedents) but no community body. UI components exist (Loomio-style voting, reaction bar, SLA timer) but persist only to localStorage. Backend CRUD functions exist but aren't exposed via HTTP. No a2o scenarios exist.

Before wiring the immune system, the community needs a body — faces, names, households, relationships. People need to see each other before they can govern together.

## Design Decisions

### The directory is the entry point

Qahal starts as a digital church directory — a photo grid of faces organized by household and small group. Not a feed. Not an algorithm. A lobby directory where you see people and learn their names.

This is the "Facebook of the elohim protocol" — but not crap. No engagement optimization, no ads, no algorithmic feed. Just the relational fabric of a community.

### Household = Collective(family)

No new entities. A household is a `Collective` with `governanceLayer: 'family'`. Couples and children are participants with `roleContext` (spouse, child, parent). The existing collective model handles this without modification.

### Context-aware face cards

The face card is the atomic UI unit. What metadata surfaces depends on context:
- **All Members view**: name + avatar + household label (subtle)
- **Household view**: cards grouped under household headers
- **Collective/group view**: name + avatar + role tags (leader, member)

The view context shapes the card, not a one-size-fits-all layout. This maps naturally to EPR — the Human is the content, the collective context determines presentation.

### Governance attaches to collectives, not individual content (for now)

The governance immune system activates at the collective level first. A collective's governance posture creates a signal that flows down to content within a computed boundary. That boundary is a research problem — it emerges from real usage, not upfront architecture.

Sprint 1 builds the body. Sprint 2 wires collective-level governance. Sprint 3 activates the immune system. The computed boundary question answers itself through use.

### Seed data models the simulacra

The seed creates a living simulation — not placeholder data, but a realistic community that feels alive before real people arrive. Constrained to current infrastructure:

- **2 households**: Matthew & Jessica + 1 child (3 people), one other family (2 people)
- **3 individual peers**: Pete, Timothy, Frank (not yet in household collectives)
- **1-2 small groups**: drawn from the pool of ~7 people
- **Total: ~7 people**

Scales to ~20 when more compute arrives. The directory grid works the same at 7 or 70.

## Data Model Changes

### Human model — one new field

**Rust** (`elohim-storage/src/db/models.rs`):
```rust
pub struct Human {
    // ... existing fields ...
    pub profile_photo_url: Option<String>,  // NEW
}
```

**View** (`views.rs`):
```rust
pub struct HumanView {
    // ... existing fields ...
    pub profile_photo_url: Option<String>,  // NEW — #[derive(TS)] auto-generates to TypeScript
}
```

Migration adds `profile_photo_url TEXT` column to `humans` table.

### No new tables or entities

- Households are `Collective(family)`
- Small groups are `Collective(faith|interest|education)`
- People are `Human`
- Relationships are collective participation with role context

### Avatar strategy

Deterministic placeholder images generated from initials or name hash. No external service dependency. Profile photo upload is a future feature — `profilePhotoUrl` points to `/blob/{cid}` when a real photo exists, otherwise the frontend renders initials.

## Architecture

### Backend (elohim-storage)

- Add `profile_photo_url` column (migration)
- Add field to `HumanView` with `#[derive(TS)]`
- **No new routes** — existing `GET /api/v1/humans` and `GET /api/v1/collectives` + participants endpoints serve the directory

### Frontend (Angular)

- **New component**: `CommunityDirectoryComponent` in `qahal/components/community-directory/`
- **Consumes existing services**: `HumanService` (imagodei) for people, `CollectiveService` (qahal) for households/groups + participants
- **No new services** — just a new view composing existing data
- **Route**: `/qahal/directory`

### Directory views

Three view modes, toggled by tabs at the top:

1. **All Members** — flat grid of face cards
2. **Households** — cards grouped under household headers, "Individuals" section for non-household members
3. **Groups** — pick a collective, see its members with role tags

### Face card component

Atomic unit: `FaceCardComponent`
- Initials-based avatar (colored circle derived from name hash)
- Display name
- Context-dependent subtitle (household name, role tag, or nothing)
- Click/tap → profile detail view (bio, groups, household members)

### Seed data (genesis)

- Add humans with `profilePhotoUrl: null` (initials avatars) to seed JSON
- Add 2 household collectives with participant links
- Add 1-2 small group collectives
- Existing seeder handles humans and collectives — just new data, not new seeder logic

## What This Does NOT Touch

- No governance write paths (Sprint 2)
- No profile editing or photo upload (future)
- No content governance or computed boundaries (emerges from use)
- No feed or activity stream
- No algorithm

## Sprint Roadmap

| Sprint | Scope |
|--------|-------|
| **1 (this design)** | Directory grid, face cards, household collectives, seed data |
| **2** | Wire governance CRUD to POST routes, replace localStorage MVP, collective-level proposals/votes/discussions, a2o scenarios |
| **3** | Immune system activation — challenges, appeals, precedents, elohim resolve function, content governance boundaries emerge |

## Relationship to CLAUDE-PICKS.md #2

This design reframes "The Governance Immune System (Qahal Write Path)" as a three-sprint progression. The original scope (wire CRUD to POST routes) is Sprint 2. This design adds Sprint 1 (the community body) as the prerequisite — you can't have an immune system without a body to protect.
