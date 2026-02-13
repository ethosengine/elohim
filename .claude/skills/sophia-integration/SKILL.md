---
name: sophia-integration
description: Reference for integrating the Sophia assessment engine into Angular. Covers the sophia-question web component API, Recognition callbacks, and three assessment modes (mastery, discovery, reflection). Use when someone asks "how do I render a quiz", "add an assessment", "handle Recognition callbacks", "implement mastery scoring", or works with psychometric instruments and session management.
metadata:
  author: elohim-protocol
  version: 1.0.0
---

# Sophia Integration Reference

Sophia is the **rendering layer only** for assessments. It produces Recognition callbacks. Session management, aggregation, and interpretation belong in the consuming app's services (lamad pillar).

## Architecture

```
RENDERING LAYER (Sophia packages)              CONSUMER LAYER (Lamad/Elohim-App)

sophia-core (types)                             QuizSessionService
  -> perseus-score (mastery scoring)            PracticeService
  -> psyche-survey (discovery/reflection)       DiscoveryAttestationService
  -> sophia (widget rendering)                  PathAdaptationService
    -> sophia-element (<sophia-question>)        ContentMasteryService
      -> sophia-plugin (Angular wrapper)         StreakTrackerService
```

### Separation of Concerns

| Responsibility | Where |
|---------------|-------|
| Render widgets | Sophia |
| Score user input | Sophia (perseus-score, psyche-survey) |
| Produce Recognition callbacks | Sophia |
| Aggregate across questions | Consumer (lamad services) |
| Manage sessions | Consumer (QuizSessionService) |
| Interpret results | Consumer (PathAdaptationService) |
| Define instruments | Consumer (application code) |
| Persist mastery | Consumer (ContentMasteryService -> storage API) |

---

## `<sophia-question>` Web Component API

### Configuration (once at app startup)

```typescript
import { Sophia } from '@ethosengine/sophia-element';

Sophia.configure({
  theme: 'auto',              // 'light' | 'dark' | 'auto'
  detectThemeFrom: 'class',   // 'system' | 'class' | 'attribute'
  colors: { primary: '#673ab7' },  // optional overrides
});
```

### Properties

| Property | Type | Description |
|----------|------|-------------|
| `moment` | `Moment` | The assessment content to render |
| `onRecognition` | `(Recognition) => void` | Callback when user completes response |

### Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `getRecognition()` | `Recognition` | Get current recognition state |

### Usage

```typescript
const el = document.querySelector('sophia-question');
el.moment = myMoment;
el.onRecognition = (recognition: Recognition) => {
  if (recognition.mastery) {
    console.log('Correct?', recognition.mastery.demonstrated);
    console.log('Score:', recognition.mastery.score, '/', recognition.mastery.total);
  }
  if (recognition.resonance) {
    console.log('Subscales:', recognition.resonance.subscaleContributions);
  }
  if (recognition.reflection) {
    console.log('Text:', recognition.reflection.textContent);
  }
};
```

---

## Core Types

### Moment

A unit of assessment content. Named "Moment" because not all are questions.

```typescript
interface Moment {
  id: string;
  purpose: 'mastery' | 'discovery' | 'reflection' | 'invitation';
  content: PerseusRenderer;     // Widget tree for rendering
  hints?: Hint[];               // Optional hints
  subscaleContributions?: SubscaleMappings;  // For discovery/reflection
}
```

### Recognition

The result of processing a learner's response.

```typescript
interface Recognition {
  momentId: string;
  purpose: AssessmentPurpose;
  userInput: UserInputMap;
  mastery?: MasteryResult;       // For graded assessments
  resonance?: ResonanceResult;   // For discovery/psychometric
  reflection?: ReflectionResult; // For open-ended reflection
  timestamp?: number;
}
```

### Result Types

```typescript
interface MasteryResult {
  demonstrated: boolean;     // Correct?
  score: number;            // Points earned
  total: number;            // Points possible
  message?: string;         // Feedback text
}

interface ResonanceResult {
  subscaleContributions: Record<string, number>;  // Subscale -> score
}

interface ReflectionResult {
  userInput: UserInputMap;
  textContent?: string;     // Free-text response
  timestamp: number;
}
```

### Type Guards

```typescript
import { hasMasteryResult, hasResonanceResult, hasReflectionResult } from '@ethosengine/sophia-core';

if (hasMasteryResult(recognition)) {
  // TypeScript knows recognition.mastery exists
  console.log(recognition.mastery.demonstrated);
}
```

---

## Three Assessment Modes

### 1. Mastery (Graded)

Purpose: Test knowledge with correct/incorrect answers.

| Aspect | Details |
|--------|---------|
| Package | `perseus-score` |
| Strategy ID | `mastery` |
| Has correct answer | Yes |
| Output | `MasteryResult { demonstrated, score, total, message }` |
| Consumer aggregation | QuizSessionService, StreakTrackerService |

### 2. Discovery (Psychometric)

Purpose: Map resonance/affinity across subscales (personality, learning style, etc.)

| Aspect | Details |
|--------|---------|
| Package | `psyche-survey` |
| Strategy ID | `discovery` |
| Has correct answer | No |
| Output | `ResonanceResult { subscaleContributions }` |
| Consumer aggregation | PsychometricSessionService, DiscoveryAttestationService |

### 3. Reflection (Open-Ended)

Purpose: Capture qualitative responses, journaling, self-assessment.

| Aspect | Details |
|--------|---------|
| Package | `psyche-survey` |
| Strategy ID | `reflection` |
| Has correct answer | No |
| Output | `ReflectionResult { userInput, textContent, timestamp }` |
| Consumer aggregation | Application-specific |

---

## Scoring Strategy Registry

Sophia uses a plugin registry for scoring strategies:

```typescript
import { getScoringStrategy, registerScoringStrategy } from '@ethosengine/sophia-core';

// Built-in strategies
const mastery = getScoringStrategy('mastery');    // From perseus-score
const discovery = getScoringStrategy('discovery'); // From psyche-survey
const reflection = getScoringStrategy('reflection'); // From psyche-survey

// Register custom strategy
registerScoringStrategy({
  id: 'custom',
  name: 'Custom Strategy',
  getEmptyWidgetIds(content, userInput, locale) { return []; },
  recognize(moment, userInput, locale) {
    return { momentId: moment.id, purpose: moment.purpose, userInput };
  },
});
```

---

## Angular Integration via sophia-plugin

The sophia Angular integration in `elohim-app/src/app/lamad/content-io/plugins/sophia/` provides:

```typescript
// Re-exports from sophia-element
export { Sophia, SophiaQuestionElement, registerSophiaElement } from '@ethosengine/sophia-element';
export type { Moment, Recognition } from '@ethosengine/sophia-element';

// Angular wrapper component
export { SophiaWrapperComponent } from './sophia-wrapper.component';
```

### Using SophiaWrapperComponent

```typescript
// In Angular template
<sophia-wrapper
  [moment]="currentMoment"
  (recognition)="onRecognition($event)">
</sophia-wrapper>
```

---

## Consumer-Side Services (Lamad Pillar)

These services live in `elohim-app/src/app/lamad/`:

| Service | Purpose |
|---------|---------|
| `QuizSessionService` | Aggregates mastery results across a quiz session |
| `PracticeService` | Manages practice mode with spaced repetition |
| `DiscoveryAttestationService` | Processes psychometric instrument results |
| `PathAdaptationService` | Adapts learning paths based on mastery/resonance |
| `ContentMasteryService` | Persists mastery to storage API |
| `StreakTrackerService` | Tracks consecutive correct answers |

---

## psyche-core Instrument Registry

Instruments register themselves. psyche-core provides the framework, not specific instruments:

```typescript
import { registerInstrument, interpretReflection } from '@ethosengine/psyche-core';

// Application code registers instruments
registerInstrument({
  id: 'my-instrument',
  name: 'My Assessment',
  category: 'personality',
  subscales: [
    { id: 'openness', name: 'Openness', description: '...' },
    { id: 'focus', name: 'Focus', description: '...' },
  ],
  scoringConfig: { method: 'highest-subscale' },
});

// Interpret aggregated responses
const interpretation = interpretReflection('my-instrument', aggregatedData);
```

### Scoring Methods

| Method | Description |
|--------|-------------|
| `highest-subscale` | Result is subscale with highest score |
| `threshold-based` | Types determined by meeting thresholds |
| `profile-matching` | Cosine similarity to predefined profiles |
| `dimensional` | Multi-dimensional profile without typing |

---

## Widget Placeholder Syntax

In Sophia content JSON, widgets are referenced with placeholder syntax:

```
[[snowman widget-type index]]

Example: [[☃ radio 1]]   -> First radio widget
         [[☃ input-number 2]]  -> Second input-number widget
```

This syntax is parsed by the sophia renderer to place interactive widgets inline.

---

## Build Order

Sophia packages must build in dependency order:

```
1. sophia-core         (foundation types)
2. psyche-core         (reflection infrastructure) -- NO Perseus deps
3. psyche-survey       (discovery/reflection)
4. perseus-core        (widget types, mastery types)
5. perseus-score       (mastery scoring -> Recognition)
6. sophia-linter       (mode-aware linting)
7. sophia-editor       (mode-aware editing)
8. sophia              (main rendering - widgets)
9. sophia-element      (Web Component - UMD/ESM/CJS)
10. Angular wrapper     (in elohim-app/src/app/lamad/content-io/plugins/sophia/)
```

### Build Commands

```bash
# Build everything (from sophia root)
cd sophia
pnpm install
pnpm build

# Build UMD bundle for Angular consumption
pnpm build:umd

# Build specific package
pnpm build --filter=sophia-element

# Angular wrapper is part of elohim-app, no separate build needed
# UMD bundle is copied as asset via angular.json config
```

---

## Gotchas

1. **Don't read UMD files** - `sophia-element.umd.js` is 3.4MB minified. Reading it crashes AI assistants. Use `ls -la` to check size.

2. **Submodule commits** - sophia is a git submodule. Changes require commit in sophia repo, then pointer update in elohim repo.

3. **pnpm not npm** - Sophia uses pnpm workspaces. `npm install` will break things.

4. **`.test.ts` not `.spec.ts`** - Sophia uses Jest convention (`.test.ts`), not Angular's Jasmine convention (`.spec.ts`).

5. **psyche-core must NEVER depend on perseus** - This is a hard architectural constraint. psyche operates independently.

6. **sophia-element UMD must be pre-built** - The `prebuild` script in elohim-app checks for it. Build with:
   ```bash
   cd sophia && pnpm install && pnpm build && pnpm build:umd
   ```

7. **Sophia is rendering only** - Never add session management, persistence, or aggregation to sophia packages. Those belong in lamad services.

8. **Package naming** - sophia packages use `@ethosengine/*`. Math utilities stay `@khanacademy/*`.

---

## Key Files

| File | Purpose |
|------|---------|
| `sophia/CLAUDE.md` | Primary sophia development guide |
| `sophia/.claude/skills/sophia-moment/SKILL.md` | Moment content authoring |
| `sophia/.claude/skills/sophia-discovery/SKILL.md` | Discovery instrument authoring |
| `sophia/.claude/skills/sophia-mastery/SKILL.md` | Mastery assessment authoring |
| `sophia/packages/sophia-element/` | Web component distribution |
| `sophia/packages/sophia-core/` | Foundation types |
| `sophia/packages/psyche-core/` | Psychometric infrastructure |
| `sophia/packages/perseus-score/` | Mastery scoring |
| `elohim-app/src/app/lamad/content-io/plugins/sophia/` | Angular wrapper (renderer, loader, wrapper component) |

## External References

- Khan Academy Perseus (original fork): `https://github.com/Khan/perseus`
- Sophia Storybook: run `cd sophia && pnpm storybook`
