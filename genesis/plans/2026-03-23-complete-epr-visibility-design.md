# Complete EPR Visibility — Content Flags & Stewardship Display

**Date**: 2026-03-23
**Status**: Approved
**Extends**: EPR Reach Badge Design (2026-03-23)

## Context

The reach and resilience badges shipped (Surface 1). Two visibility gaps remain:

1. **Content flags** (`ContentFlag[]`) exist in the model but render nowhere — learners have no warning when content is disputed, outdated, or under appeal.
2. **Stewardship in trust tab** — the trust tab shows attestation badges but not "who stewards this content" despite `StewardshipAllocationService.getContentStewardship()` being fully wired.

## Design

### Part 1 — Content Flag Tags

**Location**: After the `#tags` row in content-viewer and lesson-view headers. Only renders when `node.flags?.length > 0`.

**Rendering**: Inline tags matching the existing `.tag` pattern but with flag-type-specific border colors:

| Flag type | Border color | Label |
|-----------|-------------|-------|
| `disputed` | red (#ef4444) | Disputed |
| `outdated` | amber (#f59e0b) | Outdated |
| `appeal-pending` | purple (#8b5cf6) | Appeal Pending |
| `under-review` | blue (#3b82f6) | Under Review |
| `partial-revocation` | red (#ef4444) | Partial Revocation |

- Hover tooltip shows `flag.reason`
- Click navigates to trust tab (same pattern as reach/resilience badges)
- `data-testid="viewer-content-flag"` / `data-testid="lesson-content-flag"`

### Part 2 — Stewardship Section in Trust Tab

**Location**: Between the Reach Level line and Warnings section in the trust tab.

**Data source**: `StewardshipAllocationService.getContentStewardship(nodeId)` — injected into content-viewer, called alongside `loadTrustBadge()`.

**Rendering**:
- Section header: "Stewardship"
- Empty state: "No stewardship allocations yet" (muted text)
- With allocations: Compact list using existing `.badge-card` styling:
  - Steward display name (from `ContributorPresenceView.displayName`)
  - Role tag (author/curator/translator/endorser/steward)
  - Allocation ratio as percentage
  - Recognition accumulated
  - Dispute indicator if `governanceState === 'disputed'`

### Not in scope

- No new standalone components — all inline in existing templates
- No context menu (Surface 3)
- No EPR link navigation boxes (Surface 2)
- No appeal workflow wiring

## Files to modify

| File | Change |
|------|--------|
| `content-viewer.component.html` | Flag tags after `#tags`, stewardship section in trust tab |
| `content-viewer.component.ts` | Flag helper methods, inject StewardshipAllocationService, load stewardship data |
| `content-viewer.component.css` | Flag tag styles, stewardship card styles |
| `content-viewer.component.spec.ts` | Tests for flags rendering, stewardship loading/display |
| `lesson-view.component.ts` | Flag tags in inline template, flag helper methods |
