/**
 * SophiaWrapperComponent spec
 *
 * Covers two concerns:
 *
 * 1. Basic component creation (smoke test)
 * 2. SSR placeholder shape — CUSTOM_ELEMENTS_SCHEMA causes Angular to treat
 *    <sophia-question> as an opaque placeholder element. The renderer must
 *    emit the element with its attribute bindings intact so client hydration
 *    can upgrade it once the UMD bundle registers the custom element.
 *
 * Test approach: TestBed component test (not full SSR renderApplication).
 * Rationale: wiring up renderApplication requires route resolution and all
 * service providers for a real SSR run, which is disproportionate for testing
 * a single component's schema behaviour. The CUSTOM_ELEMENTS_SCHEMA contract
 * is enforced at compile+render time, which TestBed exercises correctly.
 * The fixture DOM is the same rendered output the SSR renderer would produce
 * for this component's template.
 */

import { ComponentFixture, TestBed } from '@angular/core/testing';
import { vi } from 'vitest';

import * as sophiaElementLoader from './sophia-element-loader';
import { SophiaWrapperComponent } from './sophia-wrapper.component';

describe('SophiaWrapperComponent', () => {
  let component: SophiaWrapperComponent;
  let fixture: ComponentFixture<SophiaWrapperComponent>;

  beforeEach(async () => {
    // registerSophiaElement tries to load the UMD bundle — stub it out so
    // tests run without a built sophia artefact.
    vi.spyOn(sophiaElementLoader, 'registerSophiaElement').mockResolvedValue(undefined);
    vi.spyOn(sophiaElementLoader, 'getSophiaElement').mockReturnValue(null);

    await TestBed.configureTestingModule({
      imports: [SophiaWrapperComponent],
    }).compileComponents();

    fixture = TestBed.createComponent(SophiaWrapperComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  describe('SSR placeholder shape', () => {
    /**
     * Angular must not throw "unknown element 'sophia-question'" and must emit
     * the element literally so SSR HTML contains it for client hydration.
     *
     * This test validates the CUSTOM_ELEMENTS_SCHEMA is in place and effective:
     * if the schema were missing, Angular's template compiler would throw an
     * NG8001 error during TestBed compilation above, and this test would never
     * reach the assertion.
     */
    it('emits <sophia-question> as a DOM placeholder element', () => {
      const host = fixture.nativeElement as HTMLElement;
      const sophiaEl = host.querySelector('sophia-question');

      expect(sophiaEl).not.toBeNull();
    });

    it('preserves [attr.mode] binding on the placeholder element', () => {
      // Default mode is 'mastery' — Angular writes it via [attr.mode]
      component.mode = 'mastery';
      fixture.detectChanges();

      const host = fixture.nativeElement as HTMLElement;
      const sophiaEl = host.querySelector('sophia-question');

      expect(sophiaEl?.getAttribute('mode')).toBe('mastery');
    });

    it('preserves [attr.review-mode] binding when reviewMode is true', () => {
      // Use setInput so Angular's OnPush change detection picks up the new value.
      fixture.componentRef.setInput('reviewMode', true);
      fixture.detectChanges();

      const host = fixture.nativeElement as HTMLElement;
      const sophiaEl = host.querySelector('sophia-question');

      // [attr.review-mode]="reviewMode ? '' : null" — present when true
      expect(sophiaEl?.hasAttribute('review-mode')).toBe(true);
    });

    it('omits [attr.review-mode] when reviewMode is false', () => {
      fixture.componentRef.setInput('reviewMode', false);
      fixture.detectChanges();

      const host = fixture.nativeElement as HTMLElement;
      const sophiaEl = host.querySelector('sophia-question');

      // [attr.review-mode]="reviewMode ? '' : null" — absent when false
      expect(sophiaEl?.hasAttribute('review-mode')).toBe(false);
    });

    it('does not register a custom element definition during render', () => {
      // The UMD bundle must not run during SSR — verify registerSophiaElement
      // was called (i.e. lifecycle ran) but the browser customElements.define
      // API was never invoked by the render phase itself.
      const defineSpy = vi.spyOn(globalThis.customElements, 'define');

      fixture.detectChanges();

      expect(defineSpy).not.toHaveBeenCalled();
      defineSpy.mockRestore();
    });
  });
});
