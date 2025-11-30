# Lamad Renderers: Implementation Guide

*Content rendering system for the Lamad learning platform. All renderers are complete and integrated.*

**Last updated:** 2025-11-27

## Architecture Overview

The renderer system decouples content display from content types using a registry pattern.

```
ContentViewerComponent
         │
         ▼
RendererRegistryService.getRenderer(node)
         │
         ▼
ViewContainerRef.createComponent(RendererComponent)
         │
         ▼
AffinityTrackingService (for interactive content)
```

---

## File Inventory

| File | Status | Description |
|------|--------|-------------|
| `renderer-registry.service.ts` | ✅ Complete | Format→Component mapping with priority |
| `renderer-initializer.service.ts` | ✅ Complete | Registers built-in renderers on injection |
| `index.ts` | ✅ Complete | Barrel exports |
| `markdown-renderer/` | ✅ Complete | Markdown via `marked` library |
| `gherkin-renderer/` | ✅ Complete | Gherkin syntax display with keyword coloring |
| `iframe-renderer/` | ✅ Complete | HTML5 apps, video embeds |
| `quiz-renderer/` | ✅ Complete | Interactive assessments with scoring |

---

## Format Mappings

| ContentFormat | Renderer | Priority |
|---------------|----------|----------|
| `markdown` | MarkdownRendererComponent | 10 |
| `html5-app` | IframeRendererComponent | 10 |
| `video-embed` | IframeRendererComponent | 10 |
| `quiz-json` | QuizRendererComponent | 10 |
| `gherkin` | GherkinRendererComponent | 5 |
| (fallback) | Plaintext inline | 0 |

---

## Renderer Interfaces

### ContentRenderer (Base)
All renderers implement this interface:

```typescript
interface ContentRenderer {
  node: ContentNode;  // Set via @Input()
}
```

### InteractiveRenderer (Extended)
For renderers that track completion:

```typescript
interface InteractiveRenderer extends ContentRenderer {
  complete: EventEmitter<RendererCompletionEvent>;
}

interface RendererCompletionEvent {
  type: 'quiz' | 'simulation' | 'video' | 'exercise';
  passed: boolean;
  score: number;  // 0-100
  details?: Record<string, unknown>;
}
```

---

## Initialization Flow

```
LamadLayoutComponent (injects RendererInitializerService)
         ↓
RendererInitializerService.constructor()
         ↓
RendererRegistryService.register() for each format
         ↓
ContentViewerComponent.loadRenderer()
         ↓
ViewContainerRef.createComponent(renderer)
```

The RendererInitializerService must be injected somewhere in the component tree to register the built-in renderers. Currently injected in LamadLayoutComponent.

---

## Affinity Integration

When an interactive renderer emits `complete`, ContentViewerComponent calculates affinity delta:

```typescript
const affinityDelta = event.passed
  ? 0.3 + (event.score / 100) * 0.2  // 0.3-0.5 for passing
  : 0.1;                              // 0.1 for attempting

affinityService.incrementAffinity(nodeId, affinityDelta);
```

---

## Extending the System

### Adding a New Renderer

1. Create component in `renderers/my-renderer/`
2. Implement with `@Input() node: ContentNode`
3. Register in `renderer-initializer.service.ts`:

```typescript
this.registry.register(['my-format'], MyRendererComponent, 10);
```

### Example: VR Scene Renderer (Future)

```typescript
@Component({
  selector: 'app-vr-scene-renderer',
  standalone: true,
  template: `<a-scene>...</a-scene>`
})
export class VrSceneRendererComponent {
  @Input() node!: ContentNode;
}

// In renderer-initializer.service.ts
this.registry.register(['vr-scene', 'aframe'], VrSceneRendererComponent, 15);
```

---

## Notes for Agents

**The renderer system is complete.** All four content formats are supported.

### Terminology
- Use "human" not "user" in all code and comments
- Use "contributor" not "creator" for content authors
- Interactive content tracks "engagement" not "consumption"

### Do NOT:
- Hardcode format checks in other components
- Import renderer components directly (use registry)
- Modify ContentNode structure
- Skip security considerations for iframe content

### Dependencies
- `marked` library for markdown parsing
- `DomSanitizer` for trusted HTML/URLs
- `FormsModule` for quiz inputs

### Security Notes
- IframeRenderer uses sandboxing
- External URLs sanitized via DomSanitizer
- Quiz inputs validated before submission

---

## Lessons Learned

### CSS Variables vs Media Queries (2025-11-30)

**Problem:** Headers displayed wrong color in light mode when OS was set to dark mode.

**Root Cause:** The `@media (prefers-color-scheme: dark)` block hard-coded `color: #e6edf3` for headers, overriding the CSS variable `var(--lamad-text-primary)`. When the app has a manual theme toggle that updates CSS variables, the media query fought against it.

**Solution:** Remove hard-coded colors from `prefers-color-scheme` media queries. Let CSS variables control colors in both modes.

```css
/* WRONG - hard-coded color overrides CSS variable */
@media (prefers-color-scheme: dark) {
  :host ::ng-deep .markdown-content h1 {
    color: #e6edf3;  /* This ignores manual theme toggle */
  }
}

/* CORRECT - let CSS variable handle theming */
:host ::ng-deep .markdown-content h1 {
  color: var(--lamad-text-primary, #1f2328);
}

@media (prefers-color-scheme: dark) {
  :host ::ng-deep .markdown-content h1 {
    border-bottom-color: #21262d;  /* Only non-variable properties */
  }
}
```

**Rule:** When an app has manual theme control via CSS variables, avoid hard-coding colors in `prefers-color-scheme` media queries - let the variables do the work.

### Styling innerHTML with Angular View Encapsulation (2025-11-30)

**Problem:** Styles don't apply to dynamically injected `[innerHTML]` content.

**Solution:** Use `:host ::ng-deep` selector pattern to penetrate view encapsulation while keeping styles scoped to the component:

```css
/* Styles for innerHTML content */
:host ::ng-deep .markdown-content p {
  color: var(--lamad-text-primary, #1f2328);
}
```

**Note:** `ViewEncapsulation.None` is an alternative but leaks styles globally - avoid it.
