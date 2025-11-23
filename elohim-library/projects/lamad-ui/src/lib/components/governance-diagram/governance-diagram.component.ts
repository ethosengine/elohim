/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import { Component, ViewChild, ElementRef, AfterViewInit, OnDestroy, Input } from '@angular/core';

interface GovernanceLayer {
  x: number;
  y: number;
  width: number;
  height: number;
  color: string;
  label: string;
  sublabel: string;
  icon: string;
  scale: number;
  targetScale: number;
  offsetY: number;
  targetOffsetY: number;
}

@Component({
  selector: 'lamad-governance-diagram',
  standalone: true,
  imports: [],
  templateUrl: './governance-diagram.component.html',
  styleUrls: ['./governance-diagram.component.css']
})
export class GovernanceDiagramComponent implements AfterViewInit, OnDestroy {
  @ViewChild('canvas') canvasRef?: ElementRef<HTMLCanvasElement>;
  @Input() autoPlay = true;
  @Input() width = 800;
  @Input() height = 600;

  private ctx: CanvasRenderingContext2D | null = null;
  private animationFrame: number | null = null;
  private resizeObserver: ResizeObserver | null = null;

  private layers: GovernanceLayer[] = [];
  private time = 0;
  private connectionProgress = 0;

  ngAfterViewInit(): void {
    if (!this.canvasRef) {
      console.warn('GovernanceDiagram: Canvas ref not available');
      return;
    }

    const canvas = this.canvasRef.nativeElement;
    const context = canvas.getContext('2d');

    if (!context) {
      console.warn('GovernanceDiagram: Could not get 2D context');
      return;
    }

    this.ctx = context;
    this.setupCanvas();
    this.initializeLayers();

    if (this.autoPlay) {
      this.animate();
    }

    // Handle window resize
    const parent = canvas.parentElement;
    if (parent) {
      this.resizeObserver = new ResizeObserver(() => {
        this.setupCanvas();
        if (!this.animationFrame) {
          this.draw();
        }
      });
      this.resizeObserver.observe(parent);
    }
  }

  ngOnDestroy(): void {
    if (this.animationFrame !== null) {
      cancelAnimationFrame(this.animationFrame);
      this.animationFrame = null;
    }
    if (this.resizeObserver) {
      this.resizeObserver.disconnect();
      this.resizeObserver = null;
    }
  }

  private setupCanvas(): void {
    if (!this.canvasRef || !this.ctx) return;

    const canvas = this.canvasRef.nativeElement;
    const parent = canvas.parentElement;
    if (!parent) return;

    const dpr = window.devicePixelRatio || 1;
    const rect = parent.getBoundingClientRect();

    canvas.width = rect.width * dpr;
    canvas.height = this.height * dpr;

    canvas.style.width = `${rect.width}px`;
    canvas.style.height = `${this.height}px`;

    this.ctx.scale(dpr, dpr);
  }

  private initializeLayers(): void {
    const w = this.canvasRef?.nativeElement.parentElement?.clientWidth || this.width;
    const h = this.height;

    const padding = 40;
    const layerHeight = 100;
    const verticalSpacing = 30;

    const startY = (h - (layerHeight * 3 + verticalSpacing * 2)) / 2;

    this.layers = [
      {
        x: w / 2,
        y: startY,
        width: Math.min(w * 0.5, 400),
        height: layerHeight,
        color: '#c9a961',
        label: 'Constitutional Layer',
        sublabel: 'Immutable Principles',
        icon: '⚖️',
        scale: 0,
        targetScale: 1,
        offsetY: 100,
        targetOffsetY: 0
      },
      {
        x: w / 2,
        y: startY + layerHeight + verticalSpacing,
        width: Math.min(w * 0.65, 520),
        height: layerHeight,
        color: '#6366f1',
        label: 'Guardian AI',
        sublabel: 'Autonomous Stewards',
        icon: '🛡️',
        scale: 0,
        targetScale: 1,
        offsetY: 100,
        targetOffsetY: 0
      },
      {
        x: w / 2,
        y: startY + (layerHeight + verticalSpacing) * 2,
        width: Math.min(w * 0.8, 640),
        height: layerHeight,
        color: '#34d399',
        label: 'Local Communities',
        sublabel: 'Flexible Adaptation',
        icon: '🌍',
        scale: 0,
        targetScale: 1,
        offsetY: 100,
        targetOffsetY: 0
      }
    ];
  }

  private animate = (): void => {
    this.time += 0.016; // ~60fps

    this.update();
    this.draw();

    this.animationFrame = requestAnimationFrame(this.animate);
  };

  private update(): void {
    // Update layer animations with staggered timing
    this.layers.forEach((layer, i) => {
      const delay = i * 0.3;
      if (this.time > delay) {
        layer.scale += (layer.targetScale - layer.scale) * 0.1;
        layer.offsetY += (layer.targetOffsetY - layer.offsetY) * 0.08;
      }
    });

    // Animate connection lines
    if (this.time > 1.2 && this.connectionProgress < 1) {
      this.connectionProgress += 0.02;
    }
  }

  private draw(): void {
    if (!this.ctx || !this.canvasRef) return;

    const canvas = this.canvasRef.nativeElement;
    const w = canvas.width / (window.devicePixelRatio || 1);
    const h = canvas.height / (window.devicePixelRatio || 1);

    this.ctx.clearRect(0, 0, w, h);

    // Draw connecting lines between layers
    this.drawConnections();

    // Draw layers
    this.layers.forEach(layer => this.drawLayer(layer));
  }

  private drawConnections(): void {
    if (!this.ctx || this.connectionProgress <= 0 || this.layers.length < 2) return;

    const ctx = this.ctx;

    ctx.save();
    ctx.strokeStyle = 'rgba(120, 113, 108, 0.3)';
    ctx.lineWidth = 2;
    ctx.setLineDash([5, 5]);
    ctx.lineCap = 'round';

    for (let i = 0; i < this.layers.length - 1; i++) {
      const layer1 = this.layers[i];
      const layer2 = this.layers[i + 1];

      if (layer1.scale < 0.5 || layer2.scale < 0.5) continue;

      const progress = Math.max(0, Math.min(1, (this.connectionProgress - i * 0.2) * 2));

      if (progress > 0) {
        const startX = layer1.x;
        const startY = layer1.y + layer1.offsetY + layer1.height / 2;
        const endX = layer2.x;
        const endY = layer2.y + layer2.offsetY - layer2.height / 2;

        ctx.globalAlpha = progress;
        ctx.beginPath();
        ctx.moveTo(startX, startY);
        ctx.lineTo(
          startX + (endX - startX) * progress,
          startY + (endY - startY) * progress
        );
        ctx.stroke();
      }
    }

    ctx.restore();
  }

  private drawLayer(layer: GovernanceLayer): void {
    if (!this.ctx || layer.scale <= 0) return;

    const ctx = this.ctx;
    const x = layer.x - (layer.width * layer.scale) / 2;
    const y = layer.y + layer.offsetY - (layer.height * layer.scale) / 2;
    const w = layer.width * layer.scale;
    const h = layer.height * layer.scale;
    const radius = 12;

    ctx.save();

    // Draw shadow/glow
    ctx.shadowColor = 'rgba(0, 0, 0, 0.15)';
    ctx.shadowBlur = 20 * layer.scale;
    ctx.shadowOffsetY = 5 * layer.scale;

    // Draw rounded rectangle background
    ctx.beginPath();
    ctx.moveTo(x + radius, y);
    ctx.lineTo(x + w - radius, y);
    ctx.quadraticCurveTo(x + w, y, x + w, y + radius);
    ctx.lineTo(x + w, y + h - radius);
    ctx.quadraticCurveTo(x + w, y + h, x + w - radius, y + h);
    ctx.lineTo(x + radius, y + h);
    ctx.quadraticCurveTo(x, y + h, x, y + h - radius);
    ctx.lineTo(x, y + radius);
    ctx.quadraticCurveTo(x, y, x + radius, y);
    ctx.closePath();

    // Gradient fill
    const gradient = ctx.createLinearGradient(x, y, x, y + h);
    gradient.addColorStop(0, this.lightenColor(layer.color, 10));
    gradient.addColorStop(1, layer.color);
    ctx.fillStyle = gradient;
    ctx.fill();

    ctx.shadowBlur = 0;
    ctx.shadowOffsetY = 0;

    // Border
    ctx.strokeStyle = this.darkenColor(layer.color, 15);
    ctx.lineWidth = 2;
    ctx.stroke();

    // Draw icon
    if (layer.scale > 0.3) {
      ctx.font = `${32 * layer.scale}px serif`;
      ctx.textAlign = 'left';
      ctx.textBaseline = 'middle';
      ctx.globalAlpha = Math.min(layer.scale, 1);
      ctx.fillText(layer.icon, x + 20, y + h / 2);
    }

    // Draw text
    if (layer.scale > 0.5) {
      ctx.globalAlpha = Math.min(layer.scale, 1);

      // Main label
      ctx.font = `bold ${16 * layer.scale}px sans-serif`;
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillStyle = '#ffffff';
      ctx.fillText(layer.label, layer.x, y + h / 2 - 10 * layer.scale);

      // Sublabel
      ctx.font = `${12 * layer.scale}px sans-serif`;
      ctx.fillStyle = 'rgba(255, 255, 255, 0.8)';
      ctx.fillText(layer.sublabel, layer.x, y + h / 2 + 12 * layer.scale);
    }

    ctx.restore();
  }

  private lightenColor(color: string, percent: number): string {
    const num = parseInt(color.replace('#', ''), 16);
    const amt = Math.round(2.55 * percent);
    const R = Math.min(255, (num >> 16) + amt);
    const G = Math.min(255, ((num >> 8) & 0x00FF) + amt);
    const B = Math.min(255, (num & 0x0000FF) + amt);
    return `rgb(${R}, ${G}, ${B})`;
  }

  private darkenColor(color: string, percent: number): string {
    const num = parseInt(color.replace('#', ''), 16);
    const amt = Math.round(2.55 * percent);
    const R = Math.max(0, (num >> 16) - amt);
    const G = Math.max(0, ((num >> 8) & 0x00FF) - amt);
    const B = Math.max(0, (num & 0x0000FF) - amt);
    return `rgb(${R}, ${G}, ${B})`;
  }
}
