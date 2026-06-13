import { TestBed } from '@angular/core/testing';

import { FlagsLensComponent } from './flags-lens.component';

// Render smoke under vitest JIT (the AOT build can't run in this container).
describe('FlagsLensComponent', () => {
  it('renders the build-time feature flags + the drift note', () => {
    TestBed.configureTestingModule({ imports: [FlagsLensComponent] });
    const fixture = TestBed.createComponent(FlagsLensComponent);
    fixture.detectChanges();
    const el = fixture.nativeElement as HTMLElement;
    expect(el.querySelector('.debug-kv')).toBeTruthy();
    // showDebug + useGraphqlTopology are set in the (dev) environment under test.
    expect(el.textContent).toContain('useGraphqlTopology');
    expect(el.textContent).toContain('showDebug');
    expect(el.querySelector('.flags-note')).toBeTruthy();
  });
});
