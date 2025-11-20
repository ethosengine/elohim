import { Component, Input, OnChanges, SimpleChanges, Output, EventEmitter, ViewChild, ElementRef, AfterViewInit, OnDestroy } from '@angular/core';
import { CommonModule } from '@angular/common';

export interface HexNode {
  id: string;
  title: string;
  affinity: number;
  affinityLevel: 'unseen' | 'low' | 'medium' | 'high';
  [key: string]: any;
  // Canvas computed props
  x?: number;
  y?: number;
}

@Component({
  selector: 'lamad-hexagon-grid',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './hexagon-grid.component.html',
  styleUrls: ['./hexagon-grid.component.css']
})
export class HexagonGridComponent implements OnChanges, AfterViewInit, OnDestroy {
  @Input() nodes: HexNode[] = [];
  @Input() itemsPerRow: number = 12;
  @Output() nodeClick = new EventEmitter<HexNode>();

  @ViewChild('hexCanvas') canvasRef!: ElementRef<HTMLCanvasElement>;
  @ViewChild('wrapper') wrapperRef!: ElementRef<HTMLDivElement>;

  private ctx!: CanvasRenderingContext2D;
  private resizeObserver!: ResizeObserver;
  
  // Configuration
  private readonly hexRadius = 16; // Size of hex (center to corner)
  private readonly hexGap = 2;     // Gap between hexes
  
  // Math helpers
  // Pointy top: width = sqrt(3) * size, height = 2 * size
  // Horiz distance = width
  // Vert distance = 3/4 * height
  private readonly hexWidth = Math.sqrt(3) * this.hexRadius;
  private readonly hexHeight = 2 * this.hexRadius;
  private readonly xStep = this.hexWidth + this.hexGap;
  private readonly yStep = (this.hexHeight * 0.75) + this.hexGap;

  hoveredNode: HexNode | null = null;
  private computedNodes: HexNode[] = [];

  // Tooltip state
  tooltipX = 0;
  tooltipY = 0;

  ngAfterViewInit(): void {
    const canvas = this.canvasRef.nativeElement;
    this.ctx = canvas.getContext('2d')!;
    
    // Handle resizing
    this.resizeObserver = new ResizeObserver(() => this.resizeCanvas());
    this.resizeObserver.observe(this.wrapperRef.nativeElement);
    
    // Initial draw triggers
    // this.resizeCanvas(); // handled by observer
  }

  ngOnChanges(changes: SimpleChanges): void {
    if (this.ctx && (changes['nodes'] || changes['itemsPerRow'])) {
      // If data changes, we need to recalculate positions and potentially height
      this.resizeCanvas();
    }
  }

  ngOnDestroy(): void {
    if (this.resizeObserver) {
      this.resizeObserver.disconnect();
    }
  }

  private resizeCanvas(): void {
    const wrapper = this.wrapperRef.nativeElement;
    const canvas = this.canvasRef.nativeElement;
    
    if (!wrapper) return;

    // 1. Get available width
    const wrapperWidth = wrapper.offsetWidth;
    
    // 2. Calculate positions based on this width to find needed height
    // We need to temporarily set canvas width to logical width to ensure drawing context is valid?
    // Actually, calculations don't depend on canvas size, just wrapper width.
    
    const { totalHeight } = this.calculatePositions(wrapperWidth);
    
    // 3. Set canvas dimensions
    const dpr = window.devicePixelRatio || 1;
    
    // Canvas logical size (CSS pixels)
    // We add a little bottom padding
    const height = Math.max(totalHeight + 40, 200); // Min height 200
    
    canvas.width = wrapperWidth * dpr;
    canvas.height = height * dpr;
    
    canvas.style.width = `${wrapperWidth}px`;
    canvas.style.height = `${height}px`;
    
    // 4. Scale context
    this.ctx.resetTransform(); // Reset before scaling
    this.ctx.scale(dpr, dpr);
    
    // 5. Draw
    this.draw();
  }

  private calculatePositions(availableWidth: number): { totalHeight: number } {
    if (!this.nodes.length) return { totalHeight: 0 };

    // Calculate how many items fit per row dynamically
    const maxItemsPerRow = Math.floor((availableWidth - this.hexWidth) / this.xStep);
    const safeItemsPerRow = Math.max(3, Math.min(this.itemsPerRow, maxItemsPerRow));
    
    // Calculate total width of the grid content
    const totalGridWidth = (safeItemsPerRow * this.xStep) + (this.hexWidth / 2);
    
    // Center horizontally
    const startX = (availableWidth - totalGridWidth) / 2 + (this.hexWidth / 2);
    const startY = 40; // Top padding

    let maxRow = 0;

    this.computedNodes = this.nodes.map((node, index) => {
      let r = 0;
      let c = 0;
      let remaining = index;
      let currentRow = 0;
      
      while (true) {
        const isOdd = currentRow % 2 !== 0;
        const capacity = isOdd ? safeItemsPerRow - 1 : safeItemsPerRow;
        
        if (remaining < capacity) {
          r = currentRow;
          c = remaining;
          break;
        }
        remaining -= capacity;
        currentRow++;
      }

      if (r > maxRow) maxRow = r;

      const xOffset = (r % 2) * (this.xStep / 2);
      const x = startX + (c * this.xStep) + xOffset;
      const y = startY + (r * this.yStep);

      return { ...node, x, y };
    });

    // Calculate used height
    // Last row Y + hex half height
    const totalHeight = startY + (maxRow * this.yStep) + (this.hexHeight / 2);
    return { totalHeight };
  }

  private draw(): void {
    if (!this.ctx || !this.computedNodes.length) return;

    const width = this.canvasRef.nativeElement.width / (window.devicePixelRatio || 1);
    const height = this.canvasRef.nativeElement.height / (window.devicePixelRatio || 1);
    
    this.ctx.clearRect(0, 0, width, height);

    this.computedNodes.forEach(node => {
      this.drawHexNode(node, node === this.hoveredNode);
    });
  }

  private drawHexNode(node: HexNode, isHovered: boolean): void {
    if (node.x === undefined || node.y === undefined) return;

    const ctx = this.ctx;
    const size = isHovered ? this.hexRadius * 1.4 : this.hexRadius;
    
    ctx.beginPath();
    for (let i = 0; i < 6; i++) {
      // Pointy topped: angle starts at 30 degrees (PI/6)
      const angle_deg = 60 * i - 30;
      const angle_rad = Math.PI / 180 * angle_deg;
      const px = node.x + size * Math.cos(angle_rad);
      const py = node.y + size * Math.sin(angle_rad);
      if (i === 0) ctx.moveTo(px, py);
      else ctx.lineTo(px, py);
    }
    ctx.closePath();

    // Fill
    const color = this.getAffinityColor(node.affinityLevel);
    ctx.fillStyle = color;
    
    // Glow effect for high affinity or hover
    if (node.affinityLevel === 'high' || node.affinityLevel === 'medium' || isHovered) {
        ctx.shadowColor = color;
        ctx.shadowBlur = isHovered ? 20 : (node.affinityLevel === 'high' ? 15 : 8);
    } else {
        ctx.shadowBlur = 0;
    }

    ctx.fill();
    
    // Reset shadow for performance/cleanliness
    ctx.shadowBlur = 0;
  }

  private getAffinityColor(level: string): string {
    switch (level) {
      case 'high': return '#34d399'; // Green
      case 'medium': return '#fbbf24'; // Gold
      case 'low': return '#fca5a5'; // Red
      case 'unseen': return 'rgba(148, 163, 184, 0.8)'; // Slate
      default: return '#ccc';
    }
  }

  onMouseMove(event: MouseEvent): void {
    const rect = this.canvasRef.nativeElement.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    
    const hit = this.computedNodes.find(node => {
      if (node.x === undefined || node.y === undefined) return false;
      const dx = node.x - x;
      const dy = node.y - y;
      return Math.sqrt(dx*dx + dy*dy) < this.hexRadius;
    });

    if (hit !== this.hoveredNode) {
      this.hoveredNode = hit || null;
      this.draw(); 
    }
    
    // Update tooltip position even if node doesn't change, 
    // so it follows mouse or stays fixed? 
    // Let's just update on hit change or if we want it to follow mouse.
    // Usually following mouse is better.
    if (this.hoveredNode) {
        this.tooltipX = x;
        this.tooltipY = y;
    }
  }

  getAffinityPercentage(affinity: number): number {
    return Math.round((affinity || 0) * 100);
  }


  onMouseLeave(): void {
    if (this.hoveredNode) {
      this.hoveredNode = null;
      this.draw();
    }
  }

  onClick(event: MouseEvent): void {
    if (this.hoveredNode) {
      this.nodeClick.emit(this.hoveredNode);
    }
  }
}
