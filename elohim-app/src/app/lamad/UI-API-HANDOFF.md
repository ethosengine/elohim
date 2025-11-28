# Lamad UI API Handoff Documentation

## Overview

This document provides UI developers with everything they need to implement learning path navigation and content display features for the upcoming sprint (1-2 weeks).

**What's New:**
- ✅ **Khan Academy-style shared content completion** across paths
- ✅ **Bulk loading APIs** for efficient rendering
- ✅ **Chapter navigation** for structured learning journeys
- ✅ **Learning analytics** for dashboard displays
- ✅ **Open standards metadata** (DID, ActivityPub, JSON-LD, Open Graph)

**Data Coverage:**
- 3,397 total content nodes
- 98.4% coverage on all standards fields (DID, ActivityPub, JSON-LD, Open Graph)
- All data validated and ready for production

---

## Table of Contents

1. [Core Services](#core-services)
2. [Shared Content Completion](#shared-content-completion-khan-academy-style)
3. [Bulk Loading & Performance](#bulk-loading--performance)
4. [Chapter Navigation](#chapter-navigation)
5. [Learning Analytics](#learning-analytics)
6. [Open Standards Metadata](#open-standards-metadata)
7. [Data Migration](#data-migration)
8. [Code Examples](#code-examples)

---

## Core Services

### AgentService
**Location:** `src/app/lamad/services/agent.service.ts`

Manages current user (agent) and their progress across all learning paths.

**Key Methods:**
- `getCurrentAgent()` - Get current user profile
- `getProgressForPath(pathId)` - Get progress for specific path
- `completeStep(pathId, stepIndex, resourceId?)` - Mark step as complete
- `getLearningAnalytics()` - **NEW:** Get dashboard metrics

### PathService
**Location:** `src/app/lamad/services/path.service.ts`

Manages learning path navigation with fog-of-war access control.

**Key Methods:**
- `getPathStep(pathId, stepIndex)` - Get single step (lazy loading)
- `getBulkSteps(pathId, startIndex, count)` - **NEW:** Load multiple steps at once
- `getPathCompletionByContent(pathId)` - **NEW:** Content-based completion
- `getChapterSteps(pathId, chapterId)` - **NEW:** Load entire chapter

### ContentService
**Location:** `src/app/lamad/services/content.service.ts`

Direct access to content nodes (outside of path context).

**Key Methods:**
- `getContent(resourceId)` - Get content by ID
- `getOpenGraphMetadata(resourceId)` - **NEW:** Get social sharing metadata
- `getActivityPubObject(resourceId)` - **NEW:** Get ActivityPub representation
- `getJsonLd(resourceId)` - **NEW:** Get JSON-LD for SEO

---

## Shared Content Completion (Khan Academy-style)

> **Note:** Khan Academy is used as an **analogy** for explaining the feature. The Lamad ontology uses different terminology.

### Lamad Ontology Mapping

In Lamad (Territory + Learning Paths):
- **Content Nodes** = Individual pieces of content (concepts, scenarios, features, videos, etc.)
- **Paths** = Curated learning journeys (e.g., "The Elohim Protocol", "Foundations for Christian Technology")
- **Steps** = References to content nodes within a path (same content can appear in multiple paths)
- **Completion Status** = Whether a content node has been mastered (completed in ANY path)

Khan Academy Analogy:
- Content Nodes ≈ Skills/Concepts
- Paths ≈ Units/Courses
- Steps ≈ Exercises
- Completion Status ≈ Mastery Levels (practiced → level 1 → level 2 → mastered)

### The Problem: Step-Based vs Content-Based Completion

Traditional path completion counts **steps**, not **unique content**:

**Example with your actual paths:**
- "**The Elohim Protocol**": 85 steps, 60 unique content nodes
- "**Foundations for Christian Technology**": 120 steps, 80 unique content nodes
  - 30 content nodes are **shared scaffolding concepts** (e.g., "Bidirectional Trust", "REA Ontology", "Constitutional Alignment")

**Problem:** If a learner completes "The Elohim Protocol" (100%), traditional view shows "Foundations for Christian Technology" as 0% complete, even though they've already mastered 37.5% of the content (30 shared / 80 total)!

### The Solution: Cross-Path Content Visibility

Content completed in **any path** shows as completed in **all paths** that reference the same content.

**How it works:**
1. Learner completes "Bidirectional Trust" content node in "The Elohim Protocol"
2. That content node is marked as `completedContentIds` in global progress
3. When viewing "Foundations for Christian Technology", that same content node shows as ✓ completed
4. Path completion calculated by **unique content mastered**, not step indices

### Visual Pattern: "Chips" Interface (Legacy Khan Academy Reference)

Khan Academy historically showed concept mastery with colored "chips":
- 🔵 **Practiced** (attempted, not mastered)
- 🟡 **Level 1** (partial mastery)
- 🟢 **Level 2** (strong mastery)
- ⭐ **Mastered** (fully mastered)

**In Lamad, you can implement a similar pattern:**

```typescript
// Get completion status for all steps in a path
this.pathService.getAllStepsWithCompletionStatus(pathId).subscribe(steps => {
  steps.forEach(step => {
    let chipStyle = 'not-started';  // ⚪ Not started

    if (step.isCompleted) {
      chipStyle = 'completed-this-path';  // ✓ Completed in this path
    } else if (step.completedInOtherPath) {
      chipStyle = 'completed-other-path';  // ✓ Mastered elsewhere
    } else if (step.affinity > 0) {
      chipStyle = 'in-progress';  // 🔵 Started/practicing
    }

    // Render chip with appropriate styling
  });
});
```

**Suggested UI for Shared Content:**

When content is completed in another path, show:
- ✅ **Green checkmark** on the step
- 💡 **Badge**: "Mastered in [Path Name]"
- 🎯 **Tooltip**: "You completed this in 'The Elohim Protocol' on [date]"

This creates "**at-a-glance completion**" - learners immediately see which concepts they've already mastered across all paths.

**Implementation:**
```typescript
// Example: Viewing "Foundations for Christian Technology" after completing "The Elohim Protocol"
this.pathService.getPathCompletionByContent('foundations-for-christian-technology').subscribe(completion => {
  console.log(completion);
  // {
  //   totalSteps: 120,
  //   completedSteps: 15,  // Steps completed in THIS path
  //   totalUniqueContent: 80,
  //   completedUniqueContent: 30,  // 30 shared scaffolding concepts from Elohim Protocol
  //   contentCompletionPercentage: 38%,  // (30 / 80) - already mastered!
  //   stepCompletionPercentage: 13%,     // (15 / 120)
  //   sharedContentCompleted: 30  // Concepts like "Bidirectional Trust", "REA Ontology", etc.
  // }
});
```

**UI Recommendations:**
- Show **content completion** percentage prominently: "**38% Content Mastered**"
- Show step completion as secondary metric: "15 of 120 steps in this path"
- Add badge: "✓ 30 concepts already mastered from The Elohim Protocol"
- Use progress bars that distinguish:
  - 🟢 Content mastered in this path
  - 🔵 Content mastered in other paths (shared scaffolding)
  - ⚪ Content not yet started

### Global Completion Status: Per-Step Visibility

Check if specific content is completed globally (across all paths):

```typescript
// Example: Checking if "Bidirectional Trust" concept is already mastered
this.pathService.getStepWithCompletionStatus('foundations-for-christian-technology', 42).subscribe(stepView => {
  if (stepView.completedInOtherPath) {
    console.log('Already mastered in another path!');
    // Show badge: "✓ Mastered in 'The Elohim Protocol'"
    // Add tooltip: "You completed this concept on Jan 15, 2025"
  }
});

// Get ALL steps with global completion status (for path overview)
this.pathService.getAllStepsWithCompletionStatus('foundations-for-christian-technology').subscribe(steps => {
  steps.forEach((step, index) => {
    if (step.isCompletedGlobally) {
      // Render with checkmark chip/badge
    }
    if (step.completedInOtherPath) {
      // Show "already mastered" indicator
      // Example: Step 42 "Bidirectional Trust" shows ✓ even though learner
      // hasn't started FCT yet, because they completed it in Elohim Protocol
    }
  });
});
```

**Visual Example for UI:**

```
Foundations for Christian Technology - Path Overview
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
38% Content Mastered  |  15 of 120 steps  |  ✓ 30 shared concepts

Chapter 1: Constitutional Foundations
  ✓ Step 1: Introduction                    [Completed in this path]
  ✓ Step 2: Bidirectional Trust             [Mastered in "The Elohim Protocol"]
  ○ Step 3: Trust Attestations              [Not started]
  ✓ Step 4: REA Ontology                    [Mastered in "The Elohim Protocol"]
  ○ Step 5: Resource-Event-Agent Model      [Not started]

Chapter 2: Technical Implementation
  ○ Step 6: Holochain Architecture          [Not started]
  ✓ Step 7: Constitutional Alignment        [Mastered in "The Elohim Protocol"]
  ...
```

---

## Bulk Loading & Performance

### Problem: Lazy Loading is Slow for Overviews

Loading 50 steps one-at-a-time for a path overview page = 50 HTTP requests. Slow!

### Solution: Bulk Loading APIs

```typescript
// Load 10 steps at once
this.pathService.getBulkSteps('path-id', 0, 10).subscribe(steps => {
  // Render first 10 steps immediately
});

// Prefetch next 3 steps while user reads current step
this.pathService.getNextNSteps('path-id', currentIndex, 3).subscribe(nextSteps => {
  // Preload for smooth navigation
});
```

**Performance Tips:**
- Load 5-10 steps at a time for list views
- Prefetch 2-3 steps ahead for navigation
- Use virtual scrolling for paths with 50+ steps

---

## Chapter Navigation

Paths can be structured with chapters (thematic groupings).

### Check if Path Uses Chapters

```typescript
this.pathService.getPath(pathId).subscribe(path => {
  if (path.chapters && path.chapters.length > 0) {
    // Path uses chapters - show chapter-based navigation
  } else {
    // Path uses flat steps - show simple navigation
  }
});
```

### Get Chapter Summaries

```typescript
this.pathService.getChapterSummaries(pathId).subscribe(summaries => {
  summaries.forEach(summary => {
    console.log(summary.chapter.title);
    console.log(`${summary.completedSteps} / ${summary.totalSteps} steps`);
    console.log(`${summary.completionPercentage}% complete`);
  });
});
```

### Navigate by Chapter

```typescript
// Get all steps in a chapter
this.pathService.getChapterSteps(pathId, chapterId).subscribe(steps => {
  // Render entire chapter
});

// Jump to first step of a chapter
this.pathService.getChapterFirstStep(pathId, chapterId).subscribe(firstStep => {
  // Navigate to step
});

// Get next chapter
this.pathService.getNextChapter(pathId, currentChapterId).subscribe(nextChapterSteps => {
  if (nextChapterSteps) {
    // Show "Start Next Chapter" button
  } else {
    // Show "Path Complete!" message
  }
});
```

---

## Learning Analytics

Display rich dashboard metrics using `getLearningAnalytics()`.

```typescript
this.agentService.getLearningAnalytics().subscribe(analytics => {
  console.log(analytics);
  // {
  //   // Overall progress
  //   totalPathsStarted: 5,
  //   totalPathsCompleted: 2,
  //   totalContentNodesCompleted: 147,
  //   totalStepsCompleted: 203,
  //
  //   // Engagement metrics
  //   totalLearningTime: 42,  // days
  //   lastActivityDate: "2025-01-15T10:30:00Z",
  //   firstActivityDate: "2024-12-04T09:00:00Z",
  //   currentStreak: 7,  // consecutive days
  //   longestStreak: 12,
  //
  //   // Path insights
  //   mostActivePathId: "elohim-protocol",
  //   mostRecentPathId: "governance-basics",
  //
  //   // Affinity insights
  //   averageAffinity: 0.75,
  //   highAffinityPaths: ["elohim-protocol", "value-flows"],
  //
  //   // Attestations
  //   totalAttestationsEarned: 3,
  //   attestationIds: ["path-completion:elohim-protocol", ...]
  // }
});
```

**Dashboard UI Ideas:**
- **Hero Stat Cards:** Paths Completed, Content Mastered, Learning Streak
- **Activity Graph:** Show `currentStreak` with flame icon 🔥
- **Progress Bars:** Completion percentages for in-progress paths
- **Affinity Badges:** Highlight `highAffinityPaths` (learner loves these!)
- **Attestations:** Display earned credentials with icons

---

## Open Standards Metadata

All content nodes now have standards-aligned metadata for sharing, SEO, and federation.

### Coverage Stats
- **98.4%** of content has DID (Decentralized Identifiers)
- **98.4%** has ActivityPub types
- **98.4%** has JSON-LD for semantic web
- **98.4%** has Open Graph for social sharing

### Usage Examples

#### Open Graph (Social Sharing)

```typescript
// Get metadata for social share buttons
this.contentService.getOpenGraphMetadata(contentId).subscribe(og => {
  if (og) {
    // Update meta tags for rich previews
    this.metaService.updateTag({ property: 'og:title', content: og.ogTitle });
    this.metaService.updateTag({ property: 'og:description', content: og.ogDescription });
    this.metaService.updateTag({ property: 'og:image', content: og.ogImage });
    this.metaService.updateTag({ property: 'og:url', content: og.ogUrl });
  }
});
```

**Open Graph Fields:**
- `ogTitle` - Content title
- `ogDescription` - Brief description (max 200 chars)
- `ogType` - "article", "video", "website"
- `ogUrl` - Canonical URL
- `ogImage` - Preview image URL (if available)
- `articleAuthor`, `articlePublishedTime`, `articleModifiedTime`, `articleTag[]`

#### JSON-LD (SEO / Semantic Web)

```typescript
// Embed structured data for search engines
this.contentService.getJsonLd(contentId).subscribe(jsonLd => {
  if (jsonLd) {
    // Inject into page as <script type="application/ld+json">
    const script = this.renderer.createElement('script');
    script.type = 'application/ld+json';
    script.text = JSON.stringify(jsonLd);
    this.renderer.appendChild(document.head, script);
  }
});
```

**JSON-LD Fields:**
- `@context`: "https://schema.org/"
- `@type`: "Article", "VideoObject", "Book", etc.
- `@id`: Canonical identifier
- `identifier`: W3C DID
- `name`, `description`, `dateCreated`, `dateModified`, `author`, `keywords`

#### ActivityPub (Federated Social Web)

```typescript
// Get ActivityPub representation for federation
this.contentService.getActivityPubObject(contentId).subscribe(apObject => {
  if (apObject) {
    // Publish to Mastodon, Pleroma, etc.
    this.activityPubService.publish(apObject);
  }
});
```

**ActivityPub Fields:**
- `@context`: "https://www.w3.org/ns/activitystreams"
- `type`: "Note", "Article", "Video", etc.
- `id`: Unique identifier (DID)
- `name`, `content`, `published`, `updated`, `url`
- `attributedTo`: Author
- `tag[]`: Hashtags

#### Get All Standards at Once

```typescript
// More efficient than separate calls
this.contentService.getStandardsMetadata(contentId).subscribe(standards => {
  console.log(standards.did);  // W3C DID
  console.log(standards.openGraph);  // Open Graph object
  console.log(standards.jsonLd);  // JSON-LD object
  console.log(standards.activityPubObject);  // ActivityPub object
});
```

---

## Data Migration

A migration service is available to populate global content completion for existing users.

### Running the Migration

**Option 1: Browser Console (Quick Test)**

```javascript
// Get service from Angular injector
const injector = ng.probe(document.querySelector('app-root')).injector;
const migrationService = injector.get('ProgressMigrationService');

// Preview what will be migrated (dry run)
migrationService.previewMigration().subscribe(
  result => console.log('Preview:', result)
);

// Run migration
migrationService.migrateAllProgress().subscribe(
  result => console.log('Complete:', result)
);

// Verify success
migrationService.verifyMigration().subscribe(
  result => console.log('Verification:', result)
);
```

**Option 2: One-Time Startup (Recommended)**

Add to your `AppComponent`:

```typescript
export class AppComponent implements OnInit {
  constructor(private migrationService: ProgressMigrationService) {}

  ngOnInit() {
    const migrationKey = 'lamad-migration-v1-completed';
    if (!localStorage.getItem(migrationKey)) {
      this.migrationService.migrateAllProgress().subscribe(
        result => {
          console.log('Migration complete:', result);
          localStorage.setItem(migrationKey, 'true');
        }
      );
    }
  }
}
```

**What It Does:**
1. Scans all progress records in localStorage
2. Extracts resourceIds from completed steps
3. Creates `__global__` progress record with shared completion data
4. Safe to run multiple times (idempotent)

**Documentation:** See `src/app/lamad/services/MIGRATION.md` for details.

---

## Code Examples

### Example 1: Path Overview Page

```typescript
@Component({
  selector: 'app-path-overview',
  template: `
    <div class="path-header">
      <h1>{{ path?.title }}</h1>
      <div class="completion-stats">
        <div class="primary-stat">
          <strong>{{ completion?.contentCompletionPercentage }}%</strong>
          <span>Content Mastered</span>
        </div>
        <div class="secondary-stat">
          {{ completion?.completedSteps }} of {{ completion?.totalSteps }} steps
        </div>
        <div class="badge" *ngIf="completion?.sharedContentCompleted > 0">
          ✓ {{ completion.sharedContentCompleted }} steps mastered from other paths
        </div>
      </div>
    </div>

    <div class="chapters" *ngIf="chapterSummaries?.length">
      <div *ngFor="let summary of chapterSummaries" class="chapter-card">
        <h3>{{ summary.chapter.title }}</h3>
        <progress [value]="summary.completionPercentage" max="100"></progress>
        <span>{{ summary.completedSteps }} / {{ summary.totalSteps }} steps</span>
        <button (click)="startChapter(summary.chapter.id)">
          {{ summary.completedSteps > 0 ? 'Continue' : 'Start' }} Chapter
        </button>
      </div>
    </div>
  `
})
export class PathOverviewComponent implements OnInit {
  path: LearningPath;
  completion: any;
  chapterSummaries: any[];

  constructor(
    private route: ActivatedRoute,
    private pathService: PathService
  ) {}

  ngOnInit() {
    const pathId = this.route.snapshot.params['pathId'];

    // Load path metadata
    this.pathService.getPath(pathId).subscribe(path => this.path = path);

    // Load content-based completion
    this.pathService.getPathCompletionByContent(pathId).subscribe(
      completion => this.completion = completion
    );

    // Load chapter summaries if path uses chapters
    this.pathService.getChapterSummaries(pathId).subscribe(
      summaries => this.chapterSummaries = summaries
    );
  }

  startChapter(chapterId: string) {
    this.pathService.getChapterFirstStep(this.path.id, chapterId).subscribe(
      firstStep => {
        if (firstStep) {
          this.router.navigate(['/lamad/path', this.path.id, 'step', firstStep.step.order]);
        }
      }
    );
  }
}
```

### Example 2: Learning Dashboard

```typescript
@Component({
  selector: 'app-learning-dashboard',
  template: `
    <div class="dashboard">
      <div class="stats-grid">
        <div class="stat-card">
          <div class="value">{{ analytics?.totalContentNodesCompleted }}</div>
          <div class="label">Content Mastered</div>
        </div>
        <div class="stat-card">
          <div class="value">{{ analytics?.totalPathsCompleted }}</div>
          <div class="label">Paths Completed</div>
        </div>
        <div class="stat-card streak">
          <div class="value">🔥 {{ analytics?.currentStreak }}</div>
          <div class="label">Day Streak</div>
        </div>
        <div class="stat-card">
          <div class="value">{{ analytics?.totalAttestationsEarned }}</div>
          <div class="label">Credentials Earned</div>
        </div>
      </div>

      <div class="activity-summary">
        <p>Learning for {{ analytics?.totalLearningTime }} days</p>
        <p>Last active: {{ analytics?.lastActivityDate | date }}</p>
        <p *ngIf="analytics?.longestStreak > analytics?.currentStreak">
          Longest streak: {{ analytics.longestStreak }} days
        </p>
      </div>
    </div>
  `
})
export class LearningDashboardComponent implements OnInit {
  analytics: any;

  constructor(private agentService: AgentService) {}

  ngOnInit() {
    this.agentService.getLearningAnalytics().subscribe(
      analytics => this.analytics = analytics
    );
  }
}
```

### Example 3: Step Navigation with Prefetching

```typescript
@Component({
  selector: 'app-step-view',
  // ... template
})
export class StepViewComponent implements OnInit {
  currentStep: PathStepView;
  prefetchedSteps: PathStepView[] = [];

  ngOnInit() {
    const pathId = this.route.snapshot.params['pathId'];
    const stepIndex = +this.route.snapshot.params['stepIndex'];

    // Load current step
    this.pathService.getPathStep(pathId, stepIndex).subscribe(
      step => {
        this.currentStep = step;

        // Prefetch next 2 steps for smooth navigation
        this.pathService.getNextNSteps(pathId, stepIndex, 2).subscribe(
          nextSteps => this.prefetchedSteps = nextSteps
        );
      }
    );
  }

  navigateNext() {
    if (this.prefetchedSteps.length > 0) {
      // Already prefetched - instant navigation!
      this.currentStep = this.prefetchedSteps[0];
      this.prefetchedSteps.shift();
    } else {
      // Fallback to lazy load
      this.pathService.getPathStep(pathId, stepIndex + 1).subscribe(...);
    }
  }
}
```

---

## Additional Resources

- **Migration Guide:** `src/app/lamad/services/MIGRATION.md`
- **Models:** `src/app/lamad/models/`
- **Services:** `src/app/lamad/services/`
- **Validation Script:** `scripts/validate_standards_alignment.py`

---

## Questions?

Reach out to the backend team with any questions about:
- API usage
- Data structure
- Performance optimization
- Standards implementation

Happy coding! 🚀
