import { TestBed } from '@angular/core/testing';
import { LoggerService } from '../../elohim/services/logger.service';
import { DebugContextService } from '../../elohim/services/debug-context.service';
import { signal } from '@angular/core';
import { vi } from 'vitest';
import { LoggingLensComponent } from './logging-lens.component';

describe('LoggingLensComponent', () => {
  it('setLevel() calls LoggerService.setMinLevel and persists', () => {
    const logger = TestBed.configureTestingModule({
      providers: [
        LoggerService,
        { provide: DebugContextService, useValue: { isTauri: signal(false) } },
      ],
    }).inject(LoggerService);
    const spy = vi.spyOn(logger, 'setMinLevel');
    const fixture = TestBed.createComponent(LoggingLensComponent);
    fixture.componentInstance.setLevel('warn');
    expect(spy).toHaveBeenCalledWith('warn');
    expect(localStorage.getItem('elohim-log-level')).toBe('warn');
  });

  it('renders the level buttons (template smoke under JIT)', () => {
    TestBed.configureTestingModule({
      providers: [
        LoggerService,
        { provide: DebugContextService, useValue: { isTauri: signal(false) } },
      ],
    });
    const fixture = TestBed.createComponent(LoggingLensComponent);
    fixture.detectChanges();
    const el = fixture.nativeElement as HTMLElement;
    // One button per native Angular level, each with a per-level data-testid.
    expect(el.querySelectorAll('.level-btn').length).toBe(4);
    expect(el.querySelector('[data-testid="level-btn-warn"]')).toBeTruthy();
    expect(el.querySelector('[data-testid="log-level-filter"]')).toBeTruthy();
  });
});
