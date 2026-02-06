# Lamad Components

UI components for learning experience.

## Components

| Component | Route | Purpose |
|-----------|-------|---------|
| `lamad-home/` | `/lamad` | Path-centric home with tabs |
| `lamad-layout/` | Shell | Session human UI, upgrade prompts |
| `path-overview/` | `/lamad/path/:id` | Path landing with step list |
| `path-navigator/` | `/lamad/path/:id/step/:idx` | Step-by-step learning player |
| `content-viewer/` | `/lamad/resource/:id` | Direct content access |
| `graph-explorer/` | `/lamad/explore` | D3.js knowledge graph |
| `profile-page/` | `/lamad/me` | Human profile view |
| `meaning-map/` | - | Card view for discovery |
| `search/` | - | Content search interface |

## Design Guidelines

- Use "human" not "user" in all UI text
- Use "journey" not "consumption"
- Fog of War: completed/current/next step only

### Access Level Indicators

| Level | Visual |
|-------|--------|
| Open | No indicator |
| Gated | Lock icon + "Join to access" |
| Protected | Lock icon + path requirement |

### Test Attributes

```html
data-cy="prev-button"
data-cy="next-button"
data-cy="complete-button"
data-cy="step-list"
```

## CSS Budget

Content-viewer and lamad-layout exceed 6kb limit - cleanup needed.
