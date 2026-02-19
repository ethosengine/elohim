# lamad-ui

Visual components for the Elohim Protocol's learning layer. "Lamad" is Hebrew for learning -- these components give shape to the protocol's educational concepts: knowledge as a living graph, governance as nested layers, value as visible flow.

## Components

### `<lamad-hexagon-grid>`

Canvas-rendered honeycomb grid for displaying content nodes. Each hexagon represents a learning unit with an affinity level (`unseen`, `low`, `medium`, `high`) that drives visual intensity through Canvas `shadowBlur` glow effects. Responsive layout auto-adjusts columns to container width. Supports click events for navigation into content.

```html
<lamad-hexagon-grid
  [nodes]="contentNodes"
  [itemsPerRow]="12"
  (nodeClick)="onSelect($event)">
</lamad-hexagon-grid>
```

### `<lamad-observer-diagram>`

Interactive diagram demonstrating the protocol's observation model. Toggles between "witness" and "private" modes to illustrate how data visibility changes based on context -- a core concept in the protocol's graduated intimacy design.

### `<lamad-value-scanner-diagram>`

Step-through animation showing how the protocol's value scanner works: scanning items, agent negotiation, value bundling, and care token generation. Illustrates the economics of recognition described in the manifesto.

### `<lamad-governance-diagram>`

Interactive layered governance visualization showing the protocol's constitutional architecture from global principles through governing states, regional communities, and family layers. Each layer displays its consensus mechanism and scope.

## Development

```bash
cd elohim-library

# Build the library
ng build lamad-ui

# The built output goes to dist/lamad-ui
```

All components are standalone Angular components that can be imported directly.
