import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';
import { provideRouter } from '@angular/router';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { ProtocolOmniComponent } from './protocol-omni.component';
import { ProtocolNavigationService } from '@app/elohim/services/protocol-navigation.service';

describe('ProtocolOmniComponent', () => {
  let fixture: ComponentFixture<ProtocolOmniComponent>;
  let component: ProtocolOmniComponent;
  let nav: {
    back: ReturnType<typeof vi.fn>;
    forward: ReturnType<typeof vi.fn>;
    context: ReturnType<typeof vi.fn>;
    activate: ReturnType<typeof vi.fn>;
  };

  beforeEach(async () => {
    nav = {
      back: vi.fn(() => null),
      forward: vi.fn(() => null),
      context: vi.fn(() => null),
      activate: vi.fn(async () => undefined),
    };

    await TestBed.configureTestingModule({
      imports: [ProtocolOmniComponent],
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        provideRouter([]),
        { provide: ProtocolNavigationService, useValue: nav },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(ProtocolOmniComponent);
    component = fixture.componentInstance;
    fixture.componentRef.setInput('contentId', 'test-cid');
    fixture.detectChanges();
  });

  afterEach(() => {
    delete (globalThis as Record<string, unknown>)['__TAURI__'];
    delete (globalThis as Record<string, unknown>)['__elohimExtensionTakeover'];
  });

  it('renders the collapsed chip when no higher trust surface owns chrome', () => {
    const chip = fixture.nativeElement.querySelector('[data-testid="protocol-omni-chip"]');
    expect(chip).not.toBeNull();
  });

  it('does not render the toolbar by default', () => {
    const toolbar = fixture.nativeElement.querySelector('[data-testid="protocol-omni-toolbar"]');
    expect(toolbar).toBeNull();
  });

  it('expands the toolbar when the chip is clicked', () => {
    const chip: HTMLElement = fixture.nativeElement.querySelector(
      '[data-testid="protocol-omni-chip"]'
    );
    chip.click();
    fixture.detectChanges();
    const toolbar = fixture.nativeElement.querySelector('[data-testid="protocol-omni-toolbar"]');
    expect(toolbar).not.toBeNull();
  });

  it('shows the EPR identifier in the toolbar', () => {
    component.expanded.set(true);
    fixture.detectChanges();
    const epr = fixture.nativeElement.querySelector('[data-testid="protocol-omni-epr"]');
    expect(epr).not.toBeNull();
    expect(epr.textContent).toContain('test-cid');
  });

  it('hides the back affordance when nav.back() is null', () => {
    component.expanded.set(true);
    fixture.detectChanges();
    const back = fixture.nativeElement.querySelector('[data-testid="protocol-omni-back"]');
    expect(back).toBeNull();
  });

  it('shows the back affordance when nav.back() returns a ref', () => {
    nav.back.mockReturnValue({ cid: 'prev-cid', label: 'Prev' });
    component.expanded.set(true);
    fixture.detectChanges();
    const back = fixture.nativeElement.querySelector('[data-testid="protocol-omni-back"]');
    expect(back).not.toBeNull();
  });

  it('hides the forward affordance when nav.forward() is null', () => {
    component.expanded.set(true);
    fixture.detectChanges();
    const fwd = fixture.nativeElement.querySelector('[data-testid="protocol-omni-forward"]');
    expect(fwd).toBeNull();
  });

  it('shows the forward affordance when nav.forward() returns a ref', () => {
    nav.forward.mockReturnValue({ cid: 'next-cid', label: 'Next' });
    component.expanded.set(true);
    fixture.detectChanges();
    const fwd = fixture.nativeElement.querySelector('[data-testid="protocol-omni-forward"]');
    expect(fwd).not.toBeNull();
  });

  it('hides the account link when not authenticated', () => {
    component.expanded.set(true);
    fixture.detectChanges();
    const account = fixture.nativeElement.querySelector('[data-testid="protocol-omni-account"]');
    expect(account).toBeNull();
  });

  it('shows the account link when authenticated', () => {
    fixture.componentRef.setInput('authenticated', true);
    component.expanded.set(true);
    fixture.detectChanges();
    const account = fixture.nativeElement.querySelector('[data-testid="protocol-omni-account"]');
    expect(account).not.toBeNull();
  });

  it('calls nav.activate(contentId, router.url) on init', () => {
    expect(nav.activate).toHaveBeenCalled();
    const call = nav.activate.mock.calls[0];
    expect(call?.[0]).toBe('test-cid');
    expect(typeof call?.[1]).toBe('string');
  });

  it('suppresses itself entirely under Tauri (Tier 3 takeover)', () => {
    (globalThis as Record<string, unknown>)['__TAURI__'] = {};
    fixture = TestBed.createComponent(ProtocolOmniComponent);
    fixture.componentRef.setInput('contentId', 'test-cid');
    fixture.detectChanges();
    const chip = fixture.nativeElement.querySelector('[data-testid="protocol-omni-chip"]');
    expect(chip).toBeNull();
  });

  it('suppresses itself under extension takeover (Tier 2)', () => {
    (globalThis as Record<string, unknown>)['__elohimExtensionTakeover'] = true;
    fixture = TestBed.createComponent(ProtocolOmniComponent);
    fixture.componentRef.setInput('contentId', 'test-cid');
    fixture.detectChanges();
    const chip = fixture.nativeElement.querySelector('[data-testid="protocol-omni-chip"]');
    expect(chip).toBeNull();
  });

  it('does NOT suppress when window.__TAURI__ is explicitly false', () => {
    (globalThis as Record<string, unknown>)['__TAURI__'] = false;
    fixture = TestBed.createComponent(ProtocolOmniComponent);
    fixture.componentRef.setInput('contentId', 'test-cid');
    fixture.detectChanges();
    const chip = fixture.nativeElement.querySelector('[data-testid="protocol-omni-chip"]');
    expect(chip).not.toBeNull();
  });
});
