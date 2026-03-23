# EPR Reach Badge — Progressive Trust Disclosure

**Date:** 2026-03-23
**Status:** Approved
**Scope:** Add a reach badge to content headers that tooltips on hover and navigates to the resource detail trust section on click. First visible surface for the distributed trust layer.

## The Problem

The backend provides full EPR trust context (reach, stewardship, attestation requirements) and the Angular model carries it through, but nothing renders to the human. Reach is buried in a hidden tooltip. Stewardship has zero UI surface. A learner browsing content has no way to know "this is community-reach content" or "this requires prerequisite mastery."

## Design Principle: Progressive Disclosure

Three surfaces, scoped from ambient to deep:
1. **Badge icons** next to content header — ambient, no text, register peripherally
2. **EPR links as contextual navigation boxes** — content graph relationships carry trust context
3. **Context menu (...)** — flag/feedback modal or deep navigation to resource detail

This sprint implements **Surface 1 only**: the reach badge.

## Reach Badge

**What:** A small icon next to the content title. Communicates reach tier visually without text.

**Where it appears:**
- Content viewer header (primary)
- Content cards in path steps and search results (secondary, smaller variant)

**Visual metaphor:** Concentric circles — more circles = wider reach. Commons is fully open (all circles), private is a single dot. Icon-only, no text label on the badge.

| Reach Tier | Visual | Tooltip |
|-----------|--------|---------|
| commons | All circles open | "Commons — accessible to everyone" |
| public | All circles open | "Public — accessible to everyone" |
| community | Three circles | "Community — requires collective membership" |
| familiar | Two circles | "Familiar — requires shared collective with steward" |
| trusted | One filled circle | "Trusted — requires relationship with steward" |
| intimate | One filled dot | "Intimate — requires mutual intimate relationship" |
| private | Lock icon | "Private — creator only" |

**Interactions:**
- **Hover:** Tooltip with reach tier name + one-line description
- **Click:** Navigate to the resource detail page's trust/attestation section

**Data source:** `ContentNode.reach` — already populated from `ContentView.reach` via `DataLoaderService`. No new API calls needed.

## Click Navigation Target

Click navigates to the content's resource detail page with the trust section visible. Two options depending on current routing:

- If already on the content viewer: scroll to and activate the Attestations tab
- If on a content card elsewhere: navigate to `/lamad/content/{id}` with a query param like `?tab=trust` that opens the Attestations tab

The Attestations tab currently shows trust badges. Future: it should also show stewardship allocations and reach details. That's a separate piece of work — this sprint just makes the badge clickable with a destination.

## Component Structure

```
ReachBadgeComponent (standalone, reusable)
  Input: reach: ContentReach
  Input: size: 'sm' | 'md' (for card vs header variants)
  Output: click event (parent handles navigation)
  Template: icon + tooltip directive
```

Standalone component so it can be used in content-viewer header, content cards, path steps, search results — anywhere content appears.

## Files

| Action | File | What |
|--------|------|------|
| Create | `app/elohim-app/src/app/lamad/components/reach-badge/reach-badge.component.ts` | Badge component |
| Create | `app/elohim-app/src/app/lamad/components/reach-badge/reach-badge.component.html` | Icon + tooltip template |
| Create | `app/elohim-app/src/app/lamad/components/reach-badge/reach-badge.component.scss` | Badge styles |
| Modify | `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.html` | Add badge to header |
| Modify | `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.ts` | Handle badge click → tab navigation |

## Not In Scope

- Resiliency badge (Surface 1, second badge — future sprint)
- EPR link navigation boxes (Surface 2 — future sprint)
- Context menu with flag/feedback (Surface 3 — future sprint)
- Stewardship display in Attestations tab (separate work)
- Content card variants (after the component exists, can be added incrementally)
