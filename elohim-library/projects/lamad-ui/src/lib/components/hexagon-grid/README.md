# Canvas Hexagon Grid

The `HexagonGridComponent` is a responsive, high-performance honeycomb grid implemented using HTML5 Canvas.

## Features
- **Pixel-Perfect Tessellation**: Uses trigonometric calculations to place hexagons in a tight "zipper" formation, avoiding CSS sub-pixel rounding errors.
- **Responsive**: Automatically recalculates the number of columns and centers the grid based on the container width.
- **Star Glow**: Uses Canvas `shadowBlur` to create performant glow effects for high-affinity nodes.
- **Interactivity**: Built-in hit testing for hover effects and tooltips.

## Usage

```html
<lamad-hexagon-grid 
  [nodes]="myNodes" 
  [itemsPerRow]="12" 
  (nodeClick)="handleSelection($event)">
</lamad-hexagon-grid>
```

## Inputs
- `nodes`: Array of `HexNode` objects.
- `itemsPerRow`: Maximum columns (will adjust down based on screen size).

## Outputs
- `nodeClick`: Emits the clicked `HexNode`.