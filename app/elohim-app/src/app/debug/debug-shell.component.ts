import { CommonModule } from '@angular/common';
import { Component, signal } from '@angular/core';
import { DebugLens } from './debug.types';
import { ConnectionLensComponent } from './lenses/connection-lens.component';

/** Hidden-but-accessible /debug surface (chrome://flags model). Always resolves
 *  by URL; the nav entry is gated separately (DebugModeService). Read-only +
 *  client-local toggles only — no backend actuation. */
@Component({
  selector: 'app-debug-shell',
  standalone: true,
  // Lens components render dynamically via *ngComponentOutlet (registry below),
  // so they are NOT template-referenced and must not be in `imports` — only
  // CommonModule (for ngComponentOutlet/ngFor/ngIf) belongs here.
  imports: [CommonModule],
  templateUrl: './debug-shell.component.html',
  styleUrl: './debug-shell.component.scss',
})
export class DebugShellComponent {
  // Registry — later tasks append Stability, Logging, Health, Flags here.
  readonly lenses: DebugLens[] = [
    { id: 'connection', title: 'Connection', icon: '🔌', component: ConnectionLensComponent },
  ];
  readonly activeId = signal(this.lenses[0].id);
  select(id: string): void { this.activeId.set(id); }
}
