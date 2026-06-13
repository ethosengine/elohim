import { TestBed } from '@angular/core/testing';

import { DebugShellComponent } from './debug-shell.component';

// Smoke test: the AOT build can't run in this container (no wasm-pack for the
// build:wasm prebuild), so this exercises the shell + active-lens TEMPLATES via
// vitest's JIT compiler — catching binding errors tsc cannot. ConnectionLens
// (the default active lens) injects only DebugContextService (providedIn root).
describe('DebugShellComponent', () => {
  it('renders the tab list and the active Connection lens', () => {
    TestBed.configureTestingModule({ imports: [DebugShellComponent] });
    const fixture = TestBed.createComponent(DebugShellComponent);
    fixture.detectChanges();

    const el = fixture.nativeElement as HTMLElement;
    expect(el.querySelector('.debug-tabs')).toBeTruthy();
    expect(el.textContent).toContain('Connection');
    // Active lens (ConnectionLens) rendered via *ngComponentOutlet.
    expect(el.querySelector('.debug-kv')).toBeTruthy();
    expect(el.textContent).toContain('Connection mode');
  });
});
