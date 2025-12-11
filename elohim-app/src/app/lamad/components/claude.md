# Lamad Components: Implementation Guide

*UI components for the Lamad learning platform. Core components are complete and functional.*

**Last updated:** 2025-11-27

## Component Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                        LamadLayoutComponent                       │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │ Session Human Info │ Upgrade Banner │ Navigation           │  │
│  ├────────────────────────────────────────────────────────────┤  │
│  │                    Router Outlet                            │  │
│  │  ┌────────────────┐  ┌────────────────────────────────┐   │  │
│  │  │ LamadHome      │  │       PathNavigator             │   │  │
│  │  │ (/)            │  │ (path/:pathId/step/:stepIndex) │   │  │
│  │  ├────────────────┤  ├────────────────────────────────┤   │  │
│  │  │ PathOverview   │  │ ContentViewer                  │   │  │
│  │  │ (path/:pathId) │  │ (resource/:id)                 │   │  │
│  │  ├────────────────┤  ├────────────────────────────────┤   │  │
│  │  │ GraphExplorer  │  │ LearnerDashboard               │   │  │
│  │  │ (/explore)     │  │ (/me)                          │   │  │
│  │  └────────────────┘  └────────────────────────────────┘   │  │
│  └────────────────────────────────────────────────────────────┘  │
│  │ Upgrade Modal (when showUpgradeModal = true)               │  │
└──────────────────────────────────────────────────────────────────┘
```

---

## File Inventory

### Active Components
| Component | Status | Description |
|-----------|--------|-------------|
| `lamad-home/` | ✅ Active | Path-centric home with tabs (Paths, Explore) |
| `lamad-layout/` | ✅ Active | Shell layout with session human UI, upgrade prompts |
| `path-overview/` | ✅ Active | Path landing page with step list and progress |
| `path-navigator/` | ✅ Active | Step-by-step learning player |
| `content-viewer/` | ✅ Active | Direct content access with back-links |
| `graph-explorer/` | ✅ Active | D3.js knowledge graph visualization |
| `learner-dashboard/` | ✅ Active | Personal progress view (/me) |
| `affinity-circle/` | ✅ Active | Reusable affinity indicator |
| `meaning-map/` | ✅ Active | List/card view for content discovery |
| `search/` | ✅ Active | Content search interface |

### Pending Components (UI for new services)
| Component | Status | Description |
|-----------|--------|-------------|
| `profile-page/` | 🔮 Pending | Render HumanProfile from ProfileService |
| `timeline-view/` | 🔮 Pending | Render TimelineEvent[] from ProfileService |
| `resume-card/` | 🔮 Pending | Render ResumePoint "Continue where you left off" |
| `access-gate/` | 🔮 Pending | Content access denial UI with unlock actions |

### Future Components (REA Economic Layer)
| Component | Status | Description |
|-----------|--------|-------------|
| `contributor-card/` | 🔮 Future | Display ContributorPresence with recognition stats |
| `recognition-flow/` | 🔮 Future | Visualize accumulated recognition for content |
| `stewardship-badge/` | 🔮 Future | Show Elohim stewardship status on contributor |
| `claim-flow/` | 🔮 Future | UI for contributors claiming their presence |

### Deleted Components (Nov 2024)
| Component | Reason |
|-----------|--------|
| `epic-viewer/` | Used deprecated EpicNode model |
| `feature-viewer/` | Used deprecated FeatureNode model |
| `scenario-detail/` | Used deprecated ScenarioNode model |
| `epic-content-panes/` | Three-pane explorer, replaced by path-navigator |
| `module-viewer/` | Used deprecated node models |

---

## Route Structure

```typescript
const LAMAD_ROUTES: Routes = [
  { path: '', component: LamadHomeComponent },           // Path-centric home
  { path: 'path/:pathId', component: PathOverviewComponent },
  { path: 'path/:pathId/step/:stepIndex', component: PathNavigatorComponent },
  { path: 'resource/:resourceId', component: ContentViewerComponent },
  { path: 'explore', component: GraphExplorerComponent },
  { path: 'me', component: LearnerDashboardComponent },
];
```

---

## Key Components

### LamadLayoutComponent
Shell layout with session human integration.

**Features:**
- Session human info display (name, stats)
- "Join Network" button
- Upgrade prompt banner (contextual triggers)
- Upgrade modal with benefits and migration info
- Navigation header

**Services Used:** SessionUserService

**Session Human UI Elements:**
```html
<!-- Header shows session human state -->
<div class="session-human-info">
  <span class="human-name">{{ getDisplayName() }}</span>
  <span class="human-stats">{{ getStatsSummary() }}</span>
  <button (click)="onJoinNetwork()">Join Network</button>
</div>

<!-- Upgrade banner when prompted -->
<div *ngIf="activeUpgradePrompt" class="upgrade-banner">
  {{ activeUpgradePrompt.message }}
</div>
```

### LamadHomeComponent
Path-centric landing page with tabbed navigation.

**Features:**
- "Learning Paths" tab showing available paths
- "Explore" tab for graph-based discovery
- Path cards with progress indicators
- "Continue" button for active paths

**Services Used:** PathService, AgentService

### PathNavigatorComponent
The primary learning experience - step-by-step content player.

**Features:**
- Step narrative sidebar (collapsible)
- Dynamic content rendering via RendererRegistry
- Prev/Next navigation with fog-of-war
- Mark Complete with attestation grants
- Time-based affinity tracking

**Services Used:** PathService, AgentService, AffinityTrackingService

### ContentViewerComponent
Direct content access with Wikipedia-style back-links.

**Features:**
- Dynamic renderer selection
- "Appears in Learning Paths" section
- Trust badge display (ready for UI)
- Access control messages for gated content

**Services Used:** ContentService, TrustBadgeService

### GraphExplorerComponent
D3.js force-directed knowledge graph.

**Features:**
- Hierarchical zoom (click to expand)
- Node type coloring
- Relationship visualization
- Click to navigate to content

**Services Used:** ExplorationService, DataLoaderService

---

## Remaining UI Work

### Priority 1: Profile UI (Services Ready)
ProfileService provides:
- `getProfile()` - HumanProfile with journey stats
- `getCurrentFocus()` - Active paths
- `getTimeline()` - Significant events
- `getResumePoint()` - Smart continuation

UI needs:
- [ ] Profile page component
- [ ] Journey timeline visualization
- [ ] Resume point card
- [ ] Paths overview with progress bars

### Priority 2: Content Access UI (Services Ready)
SessionUserService provides:
- `checkContentAccess()` - Detailed denial reasons
- `canAccessContent()` - Boolean check

UI needs:
- [ ] Gated content lock indicator
- [ ] Access denial modal with unlock actions
- [ ] "Join Network" flow placeholder

### Priority 3: Trust Badges (Service Complete, UI Pending)
TrustBadgeService provides:
- `getBadge()` - Full badge with warnings and actions
- `getIndicators()` - Unified badges + flags with polarity

UI needs: Badge component to render TrustIndicator data

### Priority 4: Search UI Enhancement
SearchService provides:
- Relevance scoring (0-100)
- Highlighted match snippets
- Faceted filtering

UI needs: Facet sidebar, highlight rendering

### Priority 5: Mobile Responsiveness
- Content viewer exceeds CSS budget (6kb limit)
- Lamad layout exceeds CSS budget (6kb limit)
- Path navigator needs responsive breakpoints
- Graph explorer needs touch gestures

---

## Design Guidelines

### Human-Centered Terminology
- Use "human" not "user" in all UI text and code
- Use "journey" not "consumption"
- Use "meaningful" not "popular" or "trending"

### Fog of War Visual Language
- **Completed steps**: Full color, checkmark icon
- **Current step**: Highlighted, "current" badge
- **Available steps**: Normal color, clickable
- **Locked steps**: Grayed out, lock icon, tooltip with reason

### Access Level Visual Language
- **Open content**: No indicator needed
- **Gated content**: Lock icon with "Join to access"
- **Protected content**: Lock icon with path requirement

### Sacred Attention Principle
- Minimal chrome/distraction
- Focus on content + narrative
- No infinite scroll
- Clear navigation (where am I? where can I go?)

### Data Attributes for Testing
```html
data-cy="prev-button"
data-cy="next-button"
data-cy="complete-button"
data-cy="completed-badge"
data-cy="step-list"
data-cy="step-item-N"
data-cy="back-button"
data-cy="session-human-info"
data-cy="upgrade-banner"
data-cy="join-network-button"
```

---

## Import Pattern

```typescript
// Use active services
import { PathService } from '../../services/path.service';
import { ContentService } from '../../services/content.service';
import { AgentService } from '../../services/agent.service';
import { TrustBadgeService } from '../../services/trust-badge.service';
import { SessionUserService } from '../../services/session-user.service';
import { ProfileService } from '../../services/profile.service';

// Use models from barrel
import {
  LearningPath,
  PathStep,
  ContentNode,
  TrustBadge,
  SessionUser,
  HumanProfile,
  TimelineEvent,
  ResumePoint
} from '../../models';
```

---

## Notes for Agents

**Core components are functional.** Remaining work is Profile UI, Access UI, and polish.

### Terminology
- Use "human" not "user" in all UI text
- Use "journey" not "consumption"
- Use "meaningful encounters" not "views"

### Do NOT:
- Use deprecated services (DocumentGraphService, NavigationService)
- Load full graphs or all content
- Create new routed components without instruction
- Skip data-cy attributes for testing
- Use "user" terminology - use "human" instead
- Use "creator" terminology - use "contributor" instead

### CSS Budget
Content-viewer and lamad-layout exceed the 6kb limit. Priority cleanup needed.

### Accessibility Checklist
- [ ] All interactive elements keyboard navigable
- [ ] ARIA labels for screen readers
- [ ] Sufficient color contrast
- [ ] Focus indicators visible
