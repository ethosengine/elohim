import { CommonModule } from '@angular/common';
import { Component, OnInit, computed, inject, signal } from '@angular/core';

import { DebugContextService } from '../../elohim/services/debug-context.service';
import { LoggerService, LogLevel } from '../../elohim/services/logger.service';

const LEVEL_KEY = 'elohim-log-level';
const ANGULAR_LEVELS: LogLevel[] = ['debug', 'info', 'warn', 'error'];
// Rust tracing range — displayed (read-only) in tauri; live-adjust is a follow-on
// (needs a set_log_level IPC + reloadable EnvFilter, neither exists today).
const RUST_LEVELS = ['off', 'error', 'warn', 'info', 'debug', 'trace'];

@Component({
  selector: 'app-logging-lens',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './logging-lens.component.html',
  styleUrl: './logging-lens.component.scss',
})
export class LoggingLensComponent implements OnInit {
  private readonly logger = inject(LoggerService);
  private readonly ctx = inject(DebugContextService);

  readonly angularLevels = ANGULAR_LEVELS;
  readonly rustLevels = RUST_LEVELS;
  readonly current = signal<LogLevel>('debug');
  readonly levelFilter = signal<LogLevel | 'all'>('all');
  readonly isTauri = this.ctx.isTauri;

  readonly logs = computed(() => {
    const f = this.levelFilter();
    const all = this.logger.getRecentLogs();
    return f === 'all' ? all : all.filter(e => e.level === f);
  });

  ngOnInit(): void {
    const saved = this.readSaved();
    if (saved) {
      this.logger.setMinLevel(saved);
      this.current.set(saved);
    }
  }

  setLevel(level: LogLevel): void {
    this.logger.setMinLevel(level);
    this.current.set(level);
    try {
      // eslint-disable-next-line no-restricted-syntax -- SSR-safe: inside try/catch SSR fallback
      localStorage.setItem(LEVEL_KEY, level);
    } catch {
      /* unavailable */
    }
  }

  setFilter(f: LogLevel | 'all'): void {
    this.levelFilter.set(f);
  }
  clear(): void {
    this.logger.clearRecentLogs();
  }

  private readSaved(): LogLevel | null {
    try {
      // eslint-disable-next-line no-restricted-syntax -- SSR-safe: inside try/catch SSR fallback
      const v = localStorage.getItem(LEVEL_KEY);
      return (ANGULAR_LEVELS as string[]).includes(v ?? '') ? (v as LogLevel) : null;
    } catch {
      return null;
    }
  }
}
