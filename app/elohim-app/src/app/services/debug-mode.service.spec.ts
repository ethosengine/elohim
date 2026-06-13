import { TestBed } from '@angular/core/testing';
import { DebugModeService } from './debug-mode.service';

describe('DebugModeService', () => {
  beforeEach(() => {
    localStorage.removeItem('elohim-debug');
    TestBed.configureTestingModule({ providers: [DebugModeService] });
  });

  it('enable() persists and disable() clears the sticky flag', () => {
    const svc = TestBed.inject(DebugModeService);
    svc.enable();
    expect(localStorage.getItem('elohim-debug')).toBe('on');
    expect(svc.navVisible()).toBe(true);
    svc.disable();
    expect(localStorage.getItem('elohim-debug')).toBeNull();
  });
});
