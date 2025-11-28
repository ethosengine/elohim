# Lamad UI API Handoff Documentation

## Overview

This document provides UI developers with everything they need to implement Territory exploration and Journey navigation features for the upcoming sprint (1-2 weeks).

**Lamad Architecture (Hebrew לָמַד - "to learn/teach"):**
- **Territory**: Content nodes (immutable knowledge graph)
- **Journey**: Learning paths (curated sequences through Territory)
- **Traveler**: Humans navigating the Territory

**What's New:**
- ✅ **Cross-Journey content visibility** - content completed in one Journey shows in all Journeys
- ✅ **Bulk loading APIs** for efficient rendering
- ✅ **Chapter navigation** for structured Journeys
- ✅ **Human analytics** for dashboard displays
- ✅ **Open standards metadata** (DID, ActivityPub, JSON-LD, Open Graph)

**Data Coverage:**
- 3,397 total content nodes in Territory
- 98.4% coverage on all standards fields
- All data validated and ready for production

---

## Lamad Terminology (Critical!)

**Use these terms in UI, NOT Khan Academy terminology:**

| Lamad Term | Meaning | ❌ Don't Use |
|------------|---------|-------------|
| **Territory** | The content graph (ContentNodes) | "Library", "Content DB" |
| **Journey** | A curated learning path | "Course", "Unit", "Class" |
| **Traveler** / **Human** | Person navigating Territory | "User", "Student", "Learner" |
| **Step** | A point in a Journey referencing a content node | "Lesson", "Exercise" |
| **Content Node** | Single piece of content | "Skill", "Concept", "Topic" |
| **Affinity** (0.0-1.0) | Depth of engagement with content | "Mastery level", "Score" |
| **Attestation** | Earned credential/badge | "Badge", "Achievement" |
| **Meaningful Encounter** | High-affinity content interaction | "View", "Completion" |

**Critical Convention:** Use "human" NOT "user" throughout all code and UI.

---

## Table of Contents

1. [Core Services](#core-services)
2. [Cross-Journey Content Visibility](#cross-journey-content-visibility)
3. [Bulk Loading & Performance](#bulk-loading--performance)
4. [Chapter Navigation](#chapter-navigation)
5. [Human Analytics](#human-analytics)
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

## Cross-Journey Content Visibility

**Lamad Architecture:** The Territory (content graph) is immutable and shared across all Journeys. When a Traveler (human) encounters a Content Node in one Journey, that meaningful encounter is visible across all Journeys that reference the same node.

### Core Concept

In Lamad's three-layer architecture:
- **Territory Layer** = Immutable content graph (ContentNodes)
- **Journey Layer** = Curated paths through Territory (LearningPaths with Steps)
- **Traveler Layer** = Humans navigating the Territory (Agents with Progress)

**Key Insight:** Content nodes exist in the Territory, independent of any Journey. A Journey's Steps are references/pointers to Territory nodes, not the content itself.

**What This Means:**
- When a Traveler completes "Bidirectional Trust" in "The Elohim Protocol" Journey, they've mastered that Territory node
- If "Foundations for Christian Technology" also references "Bidirectional Trust", it shows as completed there too
- Progress is tracked both ways:
  - **Global**: Which Territory nodes have been mastered (stored in `__global__` progress)
  - **Journey-Specific**: Which Steps in this particular Journey have been completed

### The Problem: Step-Based vs Territory-Based Mastery

Traditional Journey completion counts **Steps** (linear progression), not **Territory mastery** (content knowledge):

**Example with our actual Journeys:**
- "**The Elohim Protocol**" Journey: 85 Steps, 60 unique Territory nodes
- "**Foundations for Christian Technology**" Journey: 120 Steps, 80 unique Territory nodes
  - 30 Territory nodes are **shared foundational concepts** (e.g., "Bidirectional Trust", "REA Ontology", "Constitutional Alignment")

**Problem:** If a Traveler completes "The Elohim Protocol" (100%), traditional view shows "Foundations for Christian Technology" as 0% complete, even though they've already mastered 37.5% of the Territory content (30 shared nodes / 80 total)!

### The Solution: Territory-First Progress Tracking

Territory nodes mastered in **any Journey** are visible in **all Journeys** that reference those same nodes.

**How it works:**
1. Traveler encounters "Bidirectional Trust" Territory node while navigating "The Elohim Protocol" Journey
2. That Territory node is marked in global progress (`completedContentIds` in `__global__` pathId)
3. When viewing "Foundations for Christian Technology" Journey, that same Territory node shows as ✓ mastered
4. Journey completion calculated by **unique Territory mastery**, not just Step indices

### UI Inspiration: Visual Progress Patterns

> 📸 **UI Reference:** See `src/app/lamad/khan-inspiration/` for visual examples
>
> **IMPORTANT:** These screenshots show Khan Academy's UI patterns as **visual inspiration only**. Use Lamad terminology (Territory/Journey/Traveler/Affinity) in all implementation, NOT Khan's ontology (skills/courses/mastery).

**Visual Pattern:** Khan Academy uses colored "chips" to show progress at-a-glance. This is a useful UI pattern for showing Territory mastery in Lamad:

**Khan's Visual System (for reference):**
- ⚪ Gray chip = "Not Started"
- 🔵 Light blue = "Practiced" (attempted but not mastered)
- 🟡 Teal = "Level 1" (partial mastery)
- 🟢 Dark teal = "Level 2" (strong mastery)
- 🟦 Dark blue = "Mastered" (fully mastered)

**Key Visual Patterns from Reference Images:**

1. **Progress Circle** (`mathematics_1_overview.png`): Large circular progress indicator with breakdown by status
2. **Cross-Course Visibility** (`explorer-navigation-with-completion-peeks.png`): Shows completion across multiple courses - this is analogous to our Territory visibility across Journeys
3. **Category Groupings**: Content organized into categories with chip-based progress indicators

**Mapping to Lamad UI:**

```typescript
// Get Territory mastery status for all Steps in a Journey
this.pathService.getAllStepsWithCompletionStatus(pathId).subscribe(steps => {
  steps.forEach(step => {
    let chipStyle = 'not-encountered';  // ⚪ Gray
    let chipLabel = '';

    if (step.isCompleted && !step.completedInOtherPath) {
      chipStyle = 'mastered-here';  // 🟦 Dark blue - mastered in THIS Journey
      chipLabel = 'Mastered';
    } else if (step.completedInOtherPath) {
      chipStyle = 'mastered-elsewhere';  // 🟢 Green - mastered in OTHER Journey
      chipLabel = 'Mastered in other Journey';
    } else if (step.affinity >= 0.7) {
      chipStyle = 'high-affinity';  // 🟢 Teal - strong engagement
      chipLabel = 'High affinity';
    } else if (step.affinity >= 0.4) {
      chipStyle = 'medium-affinity';  // 🟡 Light teal - moderate engagement
      chipLabel = 'Growing affinity';
    } else if (step.affinity > 0) {
      chipStyle = 'encountered';  // 🔵 Light blue - encountered but low affinity
      chipLabel = 'Encountered';
    }

    // Render chip with appropriate styling
    // Visual pattern inspired by Khan Academy's compact chips
  });
});
```

**Lamad Progress States:**

| Lamad State | Visual | Color | Meaning |
|-------------|--------|-------|---------|
| Not encountered | ⚪ Chip | Gray | Territory node not yet visited |
| Encountered (Affinity 0-0.4) | 🔵 Chip | Light blue | Meaningful encounter begun |
| Growing Affinity (0.4-0.7) | 🟡 Chip | Teal | Moderate engagement depth |
| High Affinity (0.7-1.0) | 🟢 Chip | Dark teal | Strong engagement depth |
| Mastered (this Journey) | 🟦 Chip | Dark blue | Completed in THIS Journey |
| Mastered (other Journey) | 🟢 Chip + badge | Green | Completed in OTHER Journey |

**Suggested UI for Cross-Journey Territory Mastery:**

When a Territory node was mastered in another Journey, show:
- ✅ **Green checkmark** on the Step
- 💡 **Badge**: "Mastered in [Journey Name]"
- 🎯 **Tooltip**: "You encountered this in 'The Elohim Protocol' on [date]"

This creates **Territory-at-a-glance** - Travelers immediately see which Territory nodes they've already mastered across all Journeys.

**Implementation:**
```typescript
// Example: Viewing "Foundations for Christian Technology" Journey after completing "The Elohim Protocol"
this.pathService.getPathCompletionByContent('foundations-for-christian-technology').subscribe(completion => {
  console.log(completion);
  // {
  //   totalSteps: 120,
  //   completedSteps: 15,  // Steps completed in THIS Journey
  //   totalUniqueContent: 80,
  //   completedUniqueContent: 30,  // 30 Territory nodes already mastered
  //   contentCompletionPercentage: 38%,  // (30 / 80) Territory mastery!
  //   stepCompletionPercentage: 13%,     // (15 / 120) Journey progression
  //   sharedContentCompleted: 30  // Nodes like "Bidirectional Trust", "REA Ontology", etc.
  // }
});
```

**UI Recommendations:**
- Show **Territory mastery** prominently: "**38% Territory Mastered**"
- Show Journey progression as secondary: "15 of 120 Steps in this Journey"
- Add badge: "✓ 30 Territory nodes mastered from The Elohim Protocol"
- Use progress bars that distinguish:
  - 🟢 Territory mastered in this Journey
  - 🔵 Territory mastered in other Journeys (shared foundation)
  - ⚪ Territory not yet encountered

### Territory Mastery: Per-Step Visibility

Check if a specific Territory node is mastered globally (across all Journeys):

```typescript
// Example: Checking if "Bidirectional Trust" Territory node is already mastered
this.pathService.getStepWithCompletionStatus('foundations-for-christian-technology', 42).subscribe(stepView => {
  if (stepView.completedInOtherPath) {
    console.log('Territory node already mastered in another Journey!');
    // Show badge: "✓ Mastered in 'The Elohim Protocol'"
    // Add tooltip: "You encountered this Territory node on Jan 15, 2025"
  }
});

// Get ALL Steps with Territory mastery status (for Journey overview)
this.pathService.getAllStepsWithCompletionStatus('foundations-for-christian-technology').subscribe(steps => {
  steps.forEach((step, index) => {
    if (step.isCompletedGlobally) {
      // Render with checkmark chip/badge
    }
    if (step.completedInOtherPath) {
      // Show "Territory mastered elsewhere" indicator
      // Example: Step 42 "Bidirectional Trust" shows ✓ even though Traveler
      // hasn't started FCT Journey yet, because they mastered it in Elohim Protocol
    }
  });
});
```

**Visual Example for UI:**

```
Foundations for Christian Technology - Journey Overview
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
38% Territory Mastered  |  15 of 120 Steps  |  ✓ 30 nodes from other Journeys

Chapter 1: Constitutional Foundations
  ✓ Step 1: Introduction                    [Mastered in this Journey]
  ✓ Step 2: Bidirectional Trust             [Mastered in "The Elohim Protocol"]
  ○ Step 3: Trust Attestations              [Not encountered]
  ✓ Step 4: REA Ontology                    [Mastered in "The Elohim Protocol"]
  ○ Step 5: Resource-Event-Agent Model      [Not encountered]

Chapter 2: Technical Implementation
  ○ Step 6: Holochain Architecture          [Not encountered]
  ✓ Step 7: Constitutional Alignment        [Mastered in "The Elohim Protocol"]
  ...
```

---

## Content Viewer Pattern (Step Navigation)

> 📸 **UI Reference:** `src/app/lamad/khan-inspiration/algeba-basics-content-view-navigation.png`
>
> **Note:** This shows Khan Academy's navigation pattern as visual inspiration. Use Lamad terminology in implementation.

When a Traveler is viewing a specific Territory node within a Journey, a **compact left navigation** pattern provides Journey context and quick navigation.

### Key UI Elements (inspired by reference image)

**Left Navigation Panel:**
1. **Journey Title + Icon** - Journey identity
2. **Breadcrumb** - "JOURNEY: [JOURNEY NAME] > CHAPTER [N]" (context)
3. **Chapter Title** - Current chapter/section
4. **Prev/Next Arrows** - Quick navigation between Steps
5. **Compact Step List:**
   - Chapter headers (▶ Chapter name)
   - Individual Steps with Territory mastery indicators (✓ Mastered)
   - Current Step highlighted
   - Only Step titles shown (not full Territory node content)
6. **Full Breadcrumb Trail** - Territory > Journey > Chapter > Step (bottom)

**Main Content Area:**
1. Territory node title with standards tags
2. Share/export buttons
3. Full Territory node content body
4. "Up next: [Next Step]" button

### Implementation with Lamad Services

```typescript
@Component({
  selector: 'app-step-viewer',
  template: `
    <!-- Left Navigation Panel -->
    <aside class="step-nav">
      <!-- 1. Journey Title -->
      <div class="journey-header">
        <img [src]="journey?.iconUrl" />
        <h2>{{ journey?.title }}</h2>
      </div>

      <!-- 2. Breadcrumb Context -->
      <div class="breadcrumb">
        <span class="label">JOURNEY:</span>
        <span>{{ journey?.title | uppercase }}</span>
        <span *ngIf="currentChapter"> > {{ currentChapter.title | uppercase }}</span>
      </div>

      <!-- 3. Current Chapter Title -->
      <div class="chapter-header">
        <button (click)="navigatePrevious()" [disabled]="!currentStep?.hasPrevious">
          <
        </button>
        <h3>{{ currentChapter?.title || 'Chapter ' + (currentStepIndex + 1) }}</h3>
        <button (click)="navigateNext()" [disabled]="!currentStep?.hasNext">
          >
        </button>
      </div>

      <!-- 4. Compact Step List (bulk loaded for performance) -->
      <nav class="step-list">
        <div *ngFor="let step of visibleSteps; let i = index">
          <!-- Chapter header if chapter changes -->
          <div *ngIf="step.chapter?.id !== prevStep?.chapter?.id" class="section-header">
            ▶ {{ step.chapter.title }}
          </div>

          <!-- Step item -->
          <a [routerLink]="['/lamad/journey', journeyId, 'step', step.step.order]"
             [class.active]="step.step.order === currentStepIndex"
             [class.mastered]="step.isCompletedGlobally">

            <!-- Territory mastery badge -->
            <span *ngIf="step.isCompletedGlobally" class="badge">
              {{ step.completedInOtherPath ? '✓ From other Journey' : '✓ Mastered' }}
            </span>

            <!-- Step title (truncated) -->
            <span class="title">{{ step.step.stepTitle | truncate:40 }}</span>
          </a>
        </div>
      </nav>

      <!-- 5. Full breadcrumb trail (bottom) -->
      <div class="breadcrumb-trail">
        <a routerLink="/lamad">Territory</a>
        <span> > </span>
        <a [routerLink]="['/lamad/journey', journeyId]">{{ journey?.title }}</a>
        <span *ngIf="currentChapter"> > {{ currentChapter.title }}</span>
        <span> > {{ currentStep?.step.stepTitle }}</span>
      </div>
    </aside>

    <!-- Main Content Area -->
    <main class="content-viewer">
      <!-- Territory node title with standards -->
      <header>
        <h1>{{ currentStep?.content.title }}</h1>
        <div class="metadata">
          <span *ngFor="let tag of currentStep?.content.tags" class="tag">
            {{ tag }}
          </span>
          <!-- Share buttons -->
          <button (click)="shareContent()">📋 Share</button>
        </div>
      </header>

      <!-- Territory node content body -->
      <article [innerHTML]="currentStep?.content.content | safeHtml">
      </article>

      <!-- Next Step button -->
      <footer *ngIf="currentStep?.hasNext">
        <button (click)="navigateNext()" class="next-step">
          Up next: {{ nextStepTitle }}
        </button>
      </footer>
    </main>
  `
})
export class StepViewerComponent implements OnInit {
  journeyId: string;
  currentStepIndex: number;
  currentStep: PathStepView;
  visibleSteps: Array<PathStepView & { isCompletedGlobally: boolean }>;
  journey: LearningPath;
  currentChapter: PathChapter;

  ngOnInit() {
    this.journeyId = this.route.snapshot.params['journeyId'];
    this.currentStepIndex = +this.route.snapshot.params['stepIndex'];

    // Load Journey metadata for left nav
    this.pathService.getPath(this.journeyId).subscribe(journey => {
      this.journey = journey;

      // Determine current chapter
      if (journey.chapters) {
        let stepCount = 0;
        for (const chapter of journey.chapters) {
          if (this.currentStepIndex < stepCount + chapter.steps.length) {
            this.currentChapter = chapter;
            break;
          }
          stepCount += chapter.steps.length;
        }
      }
    });

    // Load current Step (lazy - just this one)
    this.pathService.getPathStep(this.journeyId, this.currentStepIndex).subscribe(
      step => {
        this.currentStep = step;

        // Prefetch next Step for smooth navigation
        if (step.hasNext) {
          this.pathService.getNextNSteps(this.journeyId, this.currentStepIndex, 1).subscribe();
        }
      }
    );

    // Load compact Step list for left nav (bulk for performance)
    // Only load metadata, not full Territory node content
    this.loadCompactStepList();
  }

  loadCompactStepList() {
    // Option 1: Load all Steps (if Journey has < 50 Steps)
    this.pathService.getAllStepsWithCompletionStatus(this.journeyId).subscribe(
      steps => this.visibleSteps = steps
    );

    // Option 2: Load only current chapter (if chapters exist)
    // if (this.currentChapter) {
    //   this.pathService.getChapterSteps(this.journeyId, this.currentChapter.id).subscribe(
    //     steps => this.visibleSteps = steps
    //   );
    // }
  }

  navigateNext() {
    if (this.currentStep?.hasNext) {
      this.router.navigate(['/lamad/journey', this.journeyId, 'step', this.currentStep.nextStepIndex]);
    }
  }

  navigatePrevious() {
    if (this.currentStep?.hasPrevious) {
      this.router.navigate(['/lamad/journey', this.journeyId, 'step', this.currentStep.previousStepIndex]);
    }
  }
}
```

### At-a-Glance Territory Mastery in Left Nav

The compact Step list shows Territory mastery at-a-glance (visual pattern inspired by Khan Academy):
- **✓ Mastered** - Badge for Territory node mastered in THIS Journey
- **✓ From other Journey** - Badge for Territory node mastered in OTHER Journey
- **Current Step** - Highlighted with left border
- **Chapter headers** - Chapter titles (collapsible in advanced UIs)

### Performance Optimization

**Problem:** Loading full Territory node content for 50+ Steps just to show titles = slow!

**Solution:** Use `getAllStepsWithCompletionStatus()` which:
1. Loads Journey metadata (lightweight)
2. Loads all Territory node metadata (titles, IDs) in bulk
3. Checks Territory mastery status from localStorage
4. Does NOT load full Territory node content bodies
5. Result: Fast left nav even for 100+ Step Journeys

---

## Bulk Loading & Performance

### Problem: Lazy Loading is Slow for Journey Overviews

Loading 50 Steps one-at-a-time for a Journey overview page = 50 HTTP requests. Slow!

### Solution: Bulk Loading APIs

```typescript
// Load 10 Steps at once
this.pathService.getBulkSteps('journey-id', 0, 10).subscribe(steps => {
  // Render first 10 Steps immediately
});

// Prefetch next 3 Steps while Traveler reads current Territory node
this.pathService.getNextNSteps('journey-id', currentIndex, 3).subscribe(nextSteps => {
  // Preload for smooth navigation
});
```

**Performance Tips:**
- Load 5-10 Steps at a time for list views
- Prefetch 2-3 Steps ahead for navigation
- Use virtual scrolling for Journeys with 50+ Steps

---

## Chapter Navigation

Journeys can be structured with chapters (thematic groupings).

### Check if Journey Uses Chapters

```typescript
this.pathService.getPath(journeyId).subscribe(journey => {
  if (journey.chapters && journey.chapters.length > 0) {
    // Journey uses chapters - show chapter-based navigation
  } else {
    // Journey uses flat Steps - show simple navigation
  }
});
```

### Get Chapter Summaries (Territory-Based)

**NEW:** `getChapterSummariesWithContent()` shows **Territory mastery** at the chapter level, including Territory nodes mastered in other Journeys.

```typescript
// Example: "Foundations for Christian Technology" after completing "The Elohim Protocol"
this.pathService.getChapterSummariesWithContent('foundations-for-christian-technology').subscribe(summaries => {
  summaries.forEach(summary => {
    console.log(summary.chapter.title);
    console.log(`Territory: ${summary.contentCompletionPercentage}% mastered`);
    console.log(`  ${summary.completedUniqueContent} of ${summary.totalUniqueContent} Territory nodes`);
    console.log(`  ✓ ${summary.sharedContentCompleted} mastered in other Journeys`);
    console.log(`Steps: ${summary.stepCompletionPercentage}% (${summary.completedSteps}/${summary.totalSteps})`);
  });
});

// Example output:
// Chapter 1: Constitutional Foundations
// Territory: 75% mastered
//   9 of 12 Territory nodes
//   ✓ 6 mastered in other Journeys (Bidirectional Trust, REA Ontology, etc.)
// Steps: 45% (5/11 Steps in this Journey)
```

**Visual Pattern - Khan Academy-Inspired Categories:**

Khan Academy shows skill categories with chip bars (see `mathematics_1_overview.png` for reference).

**In Lamad, chapters use similar visual pattern:**

```typescript
// Example: Render chapters with Territory mastery visualization
this.pathService.getChapterSummariesWithContent(journeyId).subscribe(summaries => {
  summaries.forEach(summary => {
    // Render chapter title
    console.log(summary.chapter.title);

    // Render chip bar showing Territory mastery distribution
    // Get individual Step statuses to render chips
    this.pathService.getChapterSteps(journeyId, summary.chapter.id).subscribe(steps => {
      // Now you have all Steps in chapter with their Territory mastery status
      // Render horizontal chip bar (visual pattern inspired by Khan Academy)
    });

    // Show summary stats
    console.log(`${summary.contentCompletionPercentage}% Territory mastered`);
    if (summary.sharedContentCompleted > 0) {
      console.log(`✓ ${summary.sharedContentCompleted} from other Journeys`);
    }
  });
});
```

**Result looks like:**
```
Chapter 1: Constitutional Foundations
[🟦🟦🟢🟢🟢🟢🟡🔵⚪⚪⚪⚪]  75% Territory mastered
✓ 6 Territory nodes from "The Elohim Protocol"

Chapter 2: Technical Implementation
[🟦🟢🟡⚪⚪⚪⚪⚪⚪⚪]  30% Territory mastered
✓ 2 Territory nodes from "The Elohim Protocol"
```

Where:
- 🟦 = Mastered in this Journey
- 🟢 = Mastered in other Journey (shared Territory)
- 🟡 = Growing affinity (0.4-0.7)
- 🔵 = Encountered (affinity 0-0.4)
- ⚪ = Not encountered
```

**Legacy Method** (Step-based only):
```typescript
// getChapterSummaries() still available for backward compatibility
this.pathService.getChapterSummaries(journeyId).subscribe(summaries => {
  // Returns Step-based completion only
});
```

### Navigate by Chapter

```typescript
// Get all Steps in a chapter
this.pathService.getChapterSteps(journeyId, chapterId).subscribe(steps => {
  // Render entire chapter
});

// Jump to first Step of a chapter
this.pathService.getChapterFirstStep(journeyId, chapterId).subscribe(firstStep => {
  // Navigate to Step
});

// Get next chapter
this.pathService.getNextChapter(journeyId, currentChapterId).subscribe(nextChapterSteps => {
  if (nextChapterSteps) {
    // Show "Start Next Chapter" button
  } else {
    // Show "Journey Complete!" message
  }
});
```

---

## Human Analytics

Display rich dashboard metrics using `getLearningAnalytics()`.

```typescript
this.agentService.getLearningAnalytics().subscribe(analytics => {
  console.log(analytics);
  // {
  //   // Overall progress
  //   totalPathsStarted: 5,           // Journeys begun
  //   totalPathsCompleted: 2,         // Journeys completed
  //   totalContentNodesCompleted: 147, // Territory nodes mastered
  //   totalStepsCompleted: 203,       // Total Steps completed across all Journeys
  //
  //   // Engagement metrics
  //   totalLearningTime: 42,  // days navigating Territory
  //   lastActivityDate: "2025-01-15T10:30:00Z",
  //   firstActivityDate: "2024-12-04T09:00:00Z",
  //   currentStreak: 7,  // consecutive days
  //   longestStreak: 12,
  //
  //   // Journey insights
  //   mostActivePathId: "elohim-protocol",     // Most active Journey
  //   mostRecentPathId: "governance-basics",   // Most recent Journey
  //
  //   // Affinity insights
  //   averageAffinity: 0.75,
  //   highAffinityPaths: ["elohim-protocol", "value-flows"],  // Journeys with deep engagement
  //
  //   // Attestations
  //   totalAttestationsEarned: 3,
  //   attestationIds: ["path-completion:elohim-protocol", ...]
  // }
});
```

**Dashboard UI Ideas:**
- **Hero Stat Cards:** Journeys Completed, Territory Mastered, Engagement Streak
- **Activity Graph:** Show `currentStreak` with flame icon 🔥
- **Progress Bars:** Completion percentages for in-progress Journeys
- **Affinity Badges:** Highlight `highAffinityPaths` (Traveler has deep affinity with these!)
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

### Example 1: Journey Overview Page

```typescript
@Component({
  selector: 'app-journey-overview',
  template: `
    <div class="journey-header">
      <h1>{{ journey?.title }}</h1>
      <div class="completion-stats">
        <div class="primary-stat">
          <strong>{{ completion?.contentCompletionPercentage }}%</strong>
          <span>Territory Mastered</span>
        </div>
        <div class="secondary-stat">
          {{ completion?.completedSteps }} of {{ completion?.totalSteps }} Steps
        </div>
        <div class="badge" *ngIf="completion?.sharedContentCompleted > 0">
          ✓ {{ completion.sharedContentCompleted }} Territory nodes mastered in other Journeys
        </div>
      </div>
    </div>

    <div class="chapters" *ngIf="chapterSummaries?.length">
      <div *ngFor="let summary of chapterSummaries" class="chapter-card">
        <h3>{{ summary.chapter.title }}</h3>

        <!-- Territory mastery (primary) -->
        <div class="territory-progress">
          <progress [value]="summary.contentCompletionPercentage" max="100"></progress>
          <span class="primary">{{ summary.contentCompletionPercentage }}% Territory Mastered</span>
          <span class="detail">{{ summary.completedUniqueContent }} of {{ summary.totalUniqueContent }} Territory nodes</span>
        </div>

        <!-- Shared Territory badge -->
        <div class="shared-badge" *ngIf="summary.sharedContentCompleted > 0">
          ✓ {{ summary.sharedContentCompleted }} Territory nodes from other Journeys
        </div>

        <!-- Step-based progress (secondary) -->
        <div class="step-progress">
          <span class="secondary">{{ summary.completedSteps }} / {{ summary.totalSteps }} Steps in this Journey</span>
        </div>

        <button (click)="startChapter(summary.chapter.id)">
          {{ summary.completedSteps > 0 ? 'Continue' : 'Begin' }} Chapter
        </button>
      </div>
    </div>
  `
})
export class JourneyOverviewComponent implements OnInit {
  journey: LearningPath;
  completion: any;
  chapterSummaries: any[];

  constructor(
    private route: ActivatedRoute,
    private pathService: PathService
  ) {}

  ngOnInit() {
    const journeyId = this.route.snapshot.params['journeyId'];

    // Load Journey metadata
    this.pathService.getPath(journeyId).subscribe(journey => this.journey = journey);

    // Load Territory-based completion
    this.pathService.getPathCompletionByContent(journeyId).subscribe(
      completion => this.completion = completion
    );

    // Load chapter summaries with Territory-based completion
    this.pathService.getChapterSummariesWithContent(journeyId).subscribe(
      summaries => this.chapterSummaries = summaries
    );
  }

  startChapter(chapterId: string) {
    this.pathService.getChapterFirstStep(this.journey.id, chapterId).subscribe(
      firstStep => {
        if (firstStep) {
          this.router.navigate(['/lamad/journey', this.journey.id, 'step', firstStep.step.order]);
        }
      }
    );
  }
}
```

### Example 2: Traveler Dashboard

```typescript
@Component({
  selector: 'app-traveler-dashboard',
  template: `
    <div class="dashboard">
      <div class="stats-grid">
        <div class="stat-card">
          <div class="value">{{ analytics?.totalContentNodesCompleted }}</div>
          <div class="label">Territory Mastered</div>
        </div>
        <div class="stat-card">
          <div class="value">{{ analytics?.totalPathsCompleted }}</div>
          <div class="label">Journeys Completed</div>
        </div>
        <div class="stat-card streak">
          <div class="value">🔥 {{ analytics?.currentStreak }}</div>
          <div class="label">Day Streak</div>
        </div>
        <div class="stat-card">
          <div class="value">{{ analytics?.totalAttestationsEarned }}</div>
          <div class="label">Attestations Earned</div>
        </div>
      </div>

      <div class="activity-summary">
        <p>Navigating Territory for {{ analytics?.totalLearningTime }} days</p>
        <p>Last encounter: {{ analytics?.lastActivityDate | date }}</p>
        <p *ngIf="analytics?.longestStreak > analytics?.currentStreak">
          Longest streak: {{ analytics.longestStreak }} days
        </p>
      </div>
    </div>
  `
})
export class TravelerDashboardComponent implements OnInit {
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
    const journeyId = this.route.snapshot.params['journeyId'];
    const stepIndex = +this.route.snapshot.params['stepIndex'];

    // Load current Step (Territory node within Journey context)
    this.pathService.getPathStep(journeyId, stepIndex).subscribe(
      step => {
        this.currentStep = step;

        // Prefetch next 2 Steps for smooth navigation
        this.pathService.getNextNSteps(journeyId, stepIndex, 2).subscribe(
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
      const journeyId = this.route.snapshot.params['journeyId'];
      const stepIndex = +this.route.snapshot.params['stepIndex'];
      this.pathService.getPathStep(journeyId, stepIndex + 1).subscribe(...);
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
