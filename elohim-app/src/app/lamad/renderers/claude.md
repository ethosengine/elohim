# Lamad Renderers

Content rendering system using registry pattern.

## Renderers

| Format | Renderer | Priority |
|--------|----------|----------|
| `markdown` | MarkdownRendererComponent | 10 |
| `html5-app` | IframeRendererComponent | 10 |
| `video-embed` | IframeRendererComponent | 10 |
| `quiz-json` | QuizRendererComponent | 10 |
| `gherkin` | GherkinRendererComponent | 5 |

## Architecture

```
ContentViewerComponent
    → RendererRegistryService.getRenderer(node)
    → ViewContainerRef.createComponent(renderer)
```

## Adding a Renderer

1. Create component with `@Input() node: ContentNode`
2. Register in `renderer-initializer.service.ts`:

```typescript
this.registry.register(['my-format'], MyRendererComponent, 10);
```

## CSS with innerHTML

Use `:host ::ng-deep` for dynamically injected content:

```css
:host ::ng-deep .markdown-content p {
  color: var(--lamad-text-primary);
}
```

Avoid hard-coded colors in `prefers-color-scheme` - let CSS variables handle theming.
