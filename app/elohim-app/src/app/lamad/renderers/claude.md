# Lamad Renderers

Content rendering system using unified plugin architecture.

## Unified Plugin System (Preferred)

Quiz/assessment content uses the ContentFormatPlugin system in `content-io/`.

| Format | Plugin | Renderer |
|--------|--------|----------|
| `perseus-quiz-json` | PerseusFormatPlugin | PerseusRendererComponent |
| `markdown` | MarkdownFormatPlugin | MarkdownRendererComponent |
| `gherkin` | GherkinFormatPlugin | GherkinRendererComponent |

See `content-io.module.ts` for plugin registration and format aliases.

## Legacy Registry (Deprecated)

| Format | Renderer | Priority |
|--------|----------|----------|
| `markdown` | MarkdownRendererComponent | 10 |
| `html5-app` | IframeRendererComponent | 10 |
| `video-embed` | IframeRendererComponent | 10 |
| `gherkin` | GherkinRendererComponent | 5 |

## Architecture

```
ContentViewerComponent
    → ContentFormatRegistryService.getRenderer(node)  // Unified system
    → ViewContainerRef.createComponent(renderer)
```

## CSS with innerHTML

Use `:host ::ng-deep` for dynamically injected content:

```css
:host ::ng-deep .markdown-content p {
  color: var(--lamad-text-primary);
}
```

Avoid hard-coded colors in `prefers-color-scheme` - let CSS variables handle theming.
