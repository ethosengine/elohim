/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import { Component, ViewChild, ElementRef, AfterViewInit, OnDestroy, Input } from '@angular/core';

interface ObserverNode {
  x: number;
  y: number;
  radius: number;
  color: string;
  emoji: string;
  label: string;
  scale: number;
  targetScale: number;
}

interface DataParticle {
  x: number;
  y: number;
  targetX: number;
  targetY: number;
  progress: number;
  size: number;
  color: string;
}

@Component({
  selector: 'lamad-observer-diagram',
  standalone: true,
  imports: [],
  templateUrl: './observer-diagram.component.html',
  styleUrls: ['./observer-diagram.component.css']
})
export class ObserverDiagramComponent implements AfterViewInit, OnDestroy {
  @ViewChild('canvas') canvasRef?: ElementRef<HTMLCanvasElement>;
  @Input() autoPlay = true;
  @Input() width = 800;
  @Input() height = 400;

  private ctx: CanvasRenderingContext2D | null = null;
  private animationFrame: number | null = null;
  private resizeObserver: ResizeObserver | null = null;

  private nodes: ObserverNode[] = [];
  private particles: DataParticle[] = [];
  private time = 0;
  private pathProgress = 0;
  private cycleTime = 0;

  ngAfterViewInit(): void {
    if (!this.canvasRef) {
      console.warn('ObserverDiagram: Canvas ref not available');
      return;
    }

    const canvas = this.canvasRef.nativeElement;
    const context = canvas.getContext('2d');

    if (!context) {
      console.warn('ObserverDiagram: Could not get 2D context');
      return;
    }

    this.ctx = context;
    this.setupCanvas();
    this.initializeNodes();

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

  private initializeNodes(): void {
    const w = this.canvasRef?.nativeElement.parentElement?.clientWidth || this.width;
    const h = this.height;

    this.nodes = [
      {
        x: w * 0.15,
        y: h / 2,
        radius: 35,
        color: '#c9a961',
        emoji: '👁️',
        label: 'Observer',
        scale: 0,
        targetScale: 1
      },
      {
        x: w * 0.5,
        y: h / 2,
        radius: 30,
        color: '#6366f1',
        emoji: '⚡',
        label: 'Extract Value',
        scale: 0,
        targetScale: 1
      },
      {
        x: w * 0.85,
        y: h / 2,
        radius: 35,
        color: '#dc2626',
        emoji: '🔥',
        label: 'Cryptographic Destruction',
        scale: 0,
        targetScale: 1
      }
    ];
  }

  private animate = (): void => {
    this.time += 0.016; // ~60fps
    this.cycleTime += 0.016;

    // Cycle the animation every 4 seconds
    if (this.cycleTime > 4) {
      this.cycleTime = 0;
      this.pathProgress = 0;
      this.particles = [];
    }

    this.update();
    this.draw();

    this.animationFrame = requestAnimationFrame(this.animate);
  };

  private update(): void {
    const w = this.canvasRef?.nativeElement.parentElement?.clientWidth || this.width;

    // Update node scales with smooth easing
    this.nodes.forEach((node, i) => {
      const delay = i * 0.3;
      if (this.time > delay) {
        node.scale += (node.targetScale - node.scale) * 0.1;
      }
    });

    // Create particles flowing through the system
    if (this.time > 1 && this.cycleTime < 3.5 && Math.random() < 0.05) {
      this.particles.push({
        x: this.nodes[0].x,
        y: this.nodes[0].y,
        targetX: this.nodes[1].x,
        targetY: this.nodes[1].y,
        progress: 0,
        size: 3 + Math.random() * 3,
        color: `hsla(${Math.random() * 60 + 180}, 70%, 60%, 0.8)`
      });
    }

    // Update particles
    this.particles.forEach(p => {
      if (p.progress < 1) {
        p.progress += 0.02;
        const t = this.easeInOut(p.progress);
        p.x = this.nodes[0].x + (p.targetX - this.nodes[0].x) * t;
        p.y = this.nodes[0].y + (p.targetY - this.nodes[0].y) * t;
      } else if (p.targetX === this.nodes[1].x) {
        // Move to destruction node
        p.targetX = this.nodes[2].x;
        p.targetY = this.nodes[2].y;
        p.progress = 0;
      }
    });

    // Remove particles that reached the end
    this.particles = this.particles.filter(p =>
      p.progress < 1 || p.targetX !== this.nodes[2].x
    );

    // Animate path drawing
    if (this.pathProgress < 1) {
      this.pathProgress += 0.01;
    }
  }

  private draw(): void {
    if (!this.ctx || !this.canvasRef) return;

    const canvas = this.canvasRef.nativeElement;
    const w = canvas.width / (window.devicePixelRatio || 1);
    const h = canvas.height / (window.devicePixelRatio || 1);

    this.ctx.clearRect(0, 0, w, h);

    // Draw connecting paths with animation
    this.drawPath();

    // Draw particles
    this.particles.forEach(p => this.drawParticle(p));

    // Draw nodes
    this.nodes.forEach(node => this.drawNode(node));

    // Draw labels
    this.nodes.forEach(node => this.drawLabel(node));
  }

  private drawPath(): void {
    if (!this.ctx || this.pathProgress <= 0) return;

    const ctx = this.ctx;
    ctx.save();

    ctx.strokeStyle = 'rgba(201, 169, 97, 0.3)';
    ctx.lineWidth = 2;
    ctx.lineCap = 'round';

    const progress = Math.min(this.pathProgress, 1);

    // Draw line from node 0 to 1
    if (progress > 0) {
      const x1 = this.nodes[0].x + this.nodes[0].radius;
      const x2 = this.nodes[1].x - this.nodes[1].radius;
      const lineProgress = Math.min(progress * 2, 1);

      ctx.beginPath();
      ctx.moveTo(x1, this.nodes[0].y);
      ctx.lineTo(x1 + (x2 - x1) * lineProgress, this.nodes[0].y);
      ctx.stroke();
    }

    // Draw line from node 1 to 2
    if (progress > 0.5) {
      const x1 = this.nodes[1].x + this.nodes[1].radius;
      const x2 = this.nodes[2].x - this.nodes[2].radius;
      const lineProgress = Math.min((progress - 0.5) * 2, 1);

      ctx.beginPath();
      ctx.moveTo(x1, this.nodes[1].y);
      ctx.lineTo(x1 + (x2 - x1) * lineProgress, this.nodes[1].y);
      ctx.stroke();
    }

    ctx.restore();
  }

  private drawNode(node: ObserverNode): void {
    if (!this.ctx || node.scale <= 0) return;

    const ctx = this.ctx;
    const r = node.radius * node.scale;

    ctx.save();

    // Draw glow
    ctx.shadowColor = node.color;
    ctx.shadowBlur = 20 * node.scale;

    // Draw circle
    ctx.beginPath();
    ctx.arc(node.x, node.y, r, 0, Math.PI * 2);
    ctx.fillStyle = '#1c1917';
    ctx.fill();

    ctx.shadowBlur = 0;
    ctx.strokeStyle = node.color;
    ctx.lineWidth = 2;
    ctx.stroke();

    // Draw emoji
    ctx.font = `${r * 0.8}px serif`;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(node.emoji, node.x, node.y);

    ctx.restore();
  }

  private drawLabel(node: ObserverNode): void {
    if (!this.ctx || node.scale < 0.5) return;

    const ctx = this.ctx;

    ctx.save();
    ctx.font = '12px sans-serif';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'top';
    ctx.fillStyle = '#57534e';
    ctx.globalAlpha = Math.min(node.scale, 1);
    ctx.fillText(node.label, node.x, node.y + node.radius + 10);
    ctx.restore();
  }

  private drawParticle(p: DataParticle): void {
    if (!this.ctx) return;

    const ctx = this.ctx;

    ctx.save();
    ctx.beginPath();
    ctx.arc(p.x, p.y, p.size, 0, Math.PI * 2);
    ctx.fillStyle = p.color;
    ctx.shadowColor = p.color;
    ctx.shadowBlur = 8;
    ctx.fill();
    ctx.restore();
  }

  private easeInOut(t: number): number {
    return t < 0.5 ? 2 * t * t : -1 + (4 - 2 * t) * t;
  }
}
