# Lamad Renderer Registry Implementation Guide

Ref: `LAMAD_API_SPECIFICATION_v1.0.md` - Part 4: Content Rendering Strategy

## Objective
Decouple content display from content type. The system should support new content types (Video, VR, Quizzes) without modifying the core view components.

## Tasks

### 1. Create `RendererRegistry`
- [ ] Create `renderers/renderer-registry.service.ts`.
- [ ] Implement `register(renderer)` and `getRenderer(node)`.
- [ ] Define `ContentRenderer` interface:
  ```typescript
  interface ContentRenderer {
    canRender(node: ContentNode): boolean;
    render(node: ContentNode, container: HTMLElement): Promise<void>;
  }
  ```

### 2. Implement Core Renderers
- [ ] **MarkdownRenderer**: Move existing markdown logic here.
- [ ] **GherkinRenderer**: Move existing Gherkin logic here.
- [ ] **VideoRenderer**: Simple HTML5 video or YouTube embed.
- [ ] **Html5AppRenderer**: Iframe-based renderer for interactive simulations (e.g., "Evolution of Trust").

### 3. Update `ContentViewerComponent`
- [ ] Refactor `components/content-viewer/content-viewer.component.ts`.
- [ ] Remove hardcoded `ngIf="node.contentFormat === 'markdown'"` switches.
- [ ] Inject `RendererRegistry`.
- [ ] Use a dynamic container: `<div #contentContainer></div>`.
- [ ] Call `registry.getRenderer(node).render(node, container)`.

## Extensibility
- This pattern allows us to drop in a `VRSceneRenderer` later just by registering it, with zero changes to the UI components.
