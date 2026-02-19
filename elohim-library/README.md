# elohim-library

Shared Angular libraries powering the Elohim Protocol's frontend. These libraries exist so that the protocol's core concerns -- content intelligence, learning visualization, assessment rendering, and interactive simulation -- live as reusable, testable modules independent of any single application.

## Projects

### elohim-service

The content intelligence layer. Transforms raw educational content (markdown, Gherkin scenarios) into structured `ContentNode` graphs, extracts relationships between concepts, and provides a mode-aware client (`ElohimClient`) that abstracts whether data comes from a doorway gateway, a local Tauri sidecar, or a Holochain conductor.

Key capabilities:
- **Import pipeline**: Markdown and Gherkin parsers, transformers (epic, scenario, archetype, resource, source), and a CLI (`npx ts-node src/cli/import.ts import`)
- **Graph database**: Kuzu-backed knowledge graph for content relationships
- **Reach-aware caching**: Content cache that respects the protocol's graduated privacy levels
- **ElohimClient**: Unified read/write interface across browser, Tauri, and Holochain modes with Angular DI integration (`provideElohimClient`)

### lamad-ui

Visual components for knowledge exploration and protocol concepts. See [lamad-ui README](projects/lamad-ui/README.md) for details.

- `<lamad-hexagon-grid>` -- Canvas-rendered honeycomb grid for content nodes with affinity-based glow
- `<lamad-observer-diagram>` -- Interactive witness/private mode data observation visualization
- `<lamad-value-scanner-diagram>` -- Step-through animation of the protocol's value scanning flow
- `<lamad-governance-diagram>` -- Interactive layered governance model (global through family)

### perseus-plugin

Bridges Khan Academy's Perseus assessment renderer into the Elohim ecosystem as a web component (`<perseus-question>`). Built as a UMD bundle that auto-registers the custom element on load. Angular apps consume it through a wrapper component that handles lazy loading and CSS injection.

### html5-app-plugin

Framework-agnostic types and utilities for loading interactive HTML5 simulations as sandboxed apps within the platform. Manages Service Worker-based caching, IndexedDB storage, and iframe sandboxing for third-party educational content.

## Development

```bash
cd elohim-library
npm install

# Run tests (elohim-service)
npx jest

# Build a library for publishing
ng build lamad-ui
ng build elohim-service
```

The workspace uses ng-packagr for library builds and Jest for testing.
