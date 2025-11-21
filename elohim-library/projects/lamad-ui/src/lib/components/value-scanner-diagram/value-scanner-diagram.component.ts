/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import { Component, ViewChild, ElementRef, AfterViewInit, OnDestroy, Input } from '@angular/core';
import { CommonModule } from '@angular/common';

interface ValueNode {
  x: number;
  y: number;
  radius: number;
  color: string;
  emoji: string;
  label: string;
  scale: number;
  targetScale: number;
  pulsePhase: number;
}

@Component({
  selector: 'lamad-value-scanner-diagram',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './value-scanner-diagram.component.html',
  styleUrls: ['./value-scanner-diagram.component.css']
})
export class ValueScannerDiagramComponent implements AfterViewInit, OnDestroy {
  @ViewChild('canvas') canvasRef!: ElementRef<HTMLCanvasElement>;
  @Input() autoPlay = true;
  @Input() width = 800;
  @Input() height = 600;

  private ctx!: CanvasRenderingContext2D;
  private animationFrame?: number;
  private resizeObserver?: ResizeObserver;

  private nodes: ValueNode[] = [];
  private time = 0;
  private connectionProgress = 0;

  ngAfterViewInit() {
    const canvas = this.canvasRef.nativeElement;
    this.ctx = canvas.getContext('2d')!;

    this.setupCanvas();
    this.initializeNodes();

    if (this.autoPlay) {
      this.animate();
    }

    // Handle window resize
    this.resizeObserver = new ResizeObserver(() => this.setupCanvas());
    this.resizeObserver.observe(canvas.parentElement!);
  }

  ngOnDestroy() {
    if (this.animationFrame) {
      cancelAnimationFrame(this.animationFrame);
    }
    if (this.resizeObserver) {
      this.resizeObserver.disconnect();
    }
  }

  private setupCanvas() {
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

  private initializeNodes() {
    const w = this.canvasRef.nativeElement.parentElement?.clientWidth || this.width;
    const h = this.height;

    const centerY = h / 2;
    const spacing = Math.min(w * 0.25, 180);
    const centerX = w / 2;

    this.nodes = [
      {
        x: centerX - spacing,
        y: centerY,
        radius: 60,
        color: '#34d399',
        emoji: '💰',
        label: 'Economic Value',
        scale: 0,
        targetScale: 1,
        pulsePhase: 0
      },
      {
        x: centerX,
        y: centerY - spacing * 0.866, // sqrt(3)/2 for equilateral triangle
        radius: 60,
        color: '#fbbf24',
        emoji: '🤝',
        label: 'Social Value',
        scale: 0,
        targetScale: 1,
        pulsePhase: Math.PI * 2 / 3
      },
      {
        x: centerX + spacing,
        y: centerY,
        radius: 60,
        color: '#c9a961',
        emoji: '❤️',
        label: 'Emotional Value',
        scale: 0,
        targetScale: 1,
        pulsePhase: Math.PI * 4 / 3
      }
    ];
  }

  private animate = () => {
    this.time += 0.016; // ~60fps

    this.update();
    this.draw();

    this.animationFrame = requestAnimationFrame(this.animate);
  };

  private update() {
    // Update node scales with smooth easing
    this.nodes.forEach((node, i) => {
      const delay = i * 0.2;
      if (this.time > delay) {
        node.scale += (node.targetScale - node.scale) * 0.08;
      }
      node.pulsePhase += 0.02;
    });

    // Animate connection lines
    if (this.time > 0.8 && this.connectionProgress < 1) {
      this.connectionProgress += 0.015;
    }
  }

  private draw() {
    const canvas = this.canvasRef.nativeElement;
    const w = canvas.width / (window.devicePixelRatio || 1);
    const h = canvas.height / (window.devicePixelRatio || 1);

    this.ctx.clearRect(0, 0, w, h);

    // Draw center convergence point with glow
    this.drawConvergencePoint();

    // Draw connecting lines
    this.drawConnections();

    // Draw value nodes
    this.nodes.forEach(node => this.drawValueNode(node));

    // Draw labels
    this.nodes.forEach(node => this.drawLabel(node));
  }

  private drawConvergencePoint() {
    if (this.connectionProgress < 0.8) return;

    const ctx = this.ctx;
    const w = this.canvasRef.nativeElement.parentElement?.clientWidth || this.width;
    const h = this.height;

    const centerX = w / 2;
    const centerY = h / 2;

    const alpha = (this.connectionProgress - 0.8) * 5;
    const radius = 8 * alpha;

    ctx.save();
    ctx.globalAlpha = Math.min(alpha, 0.8);

    // Draw glow
    const gradient = ctx.createRadialGradient(centerX, centerY, 0, centerX, centerY, radius * 3);
    gradient.addColorStop(0, 'rgba(201, 169, 97, 0.6)');
    gradient.addColorStop(0.5, 'rgba(201, 169, 97, 0.2)');
    gradient.addColorStop(1, 'rgba(201, 169, 97, 0)');

    ctx.fillStyle = gradient;
    ctx.fillRect(centerX - radius * 3, centerY - radius * 3, radius * 6, radius * 6);

    // Draw center point
    ctx.beginPath();
    ctx.arc(centerX, centerY, radius, 0, Math.PI * 2);
    ctx.fillStyle = '#c9a961';
    ctx.fill();

    ctx.restore();
  }

  private drawConnections() {
    if (this.connectionProgress <= 0 || this.nodes.length < 3) return;

    const ctx = this.ctx;
    const progress = Math.min(this.connectionProgress, 1);

    ctx.save();
    ctx.strokeStyle = 'rgba(201, 169, 97, 0.4)';
    ctx.lineWidth = 2;
    ctx.lineCap = 'round';
    ctx.globalAlpha = progress;

    // Draw triangle connecting all three nodes
    const w = this.canvasRef.nativeElement.parentElement?.clientWidth || this.width;
    const h = this.height;
    const centerX = w / 2;
    const centerY = h / 2;

    // Draw lines from each node to center
    this.nodes.forEach((node, i) => {
      const lineProgress = Math.max(0, Math.min(1, (progress - i * 0.15) * 1.5));

      if (lineProgress > 0) {
        const startX = node.x + (centerX - node.x) * 0.3;
        const startY = node.y + (centerY - node.y) * 0.3;
        const endX = centerX;
        const endY = centerY;

        ctx.beginPath();
        ctx.moveTo(startX, startY);
        ctx.lineTo(
          startX + (endX - startX) * lineProgress,
          startY + (endY - startY) * lineProgress
        );
        ctx.stroke();
      }
    });

    ctx.restore();
  }

  private drawValueNode(node: ValueNode) {
    if (node.scale <= 0) return;

    const ctx = this.ctx;
    const pulse = Math.sin(node.pulsePhase) * 0.1 + 1;
    const r = node.radius * node.scale * pulse;

    ctx.save();

    // Draw outer glow
    ctx.shadowColor = node.color;
    ctx.shadowBlur = 25 * node.scale;

    // Draw circle with gradient
    const gradient = ctx.createRadialGradient(node.x, node.y, 0, node.x, node.y, r);
    gradient.addColorStop(0, this.lightenColor(node.color, 20));
    gradient.addColorStop(1, node.color);

    ctx.beginPath();
    ctx.arc(node.x, node.y, r, 0, Math.PI * 2);
    ctx.fillStyle = gradient;
    ctx.fill();

    ctx.shadowBlur = 0;

    // Draw border
    ctx.strokeStyle = this.darkenColor(node.color, 20);
    ctx.lineWidth = 2;
    ctx.stroke();

    // Draw emoji
    ctx.font = `${r * 0.6}px serif`;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillStyle = '#1c1917';
    ctx.fillText(node.emoji, node.x, node.y);

    ctx.restore();
  }

  private drawLabel(node: ValueNode) {
    if (node.scale < 0.5) return;

    const ctx = this.ctx;

    ctx.save();
    ctx.font = 'bold 13px sans-serif';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'top';
    ctx.fillStyle = '#292524';
    ctx.globalAlpha = Math.min(node.scale, 1);
    ctx.fillText(node.label, node.x, node.y + node.radius + 15);
    ctx.restore();
  }

  private lightenColor(color: string, percent: number): string {
    // Simple color lightening
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
