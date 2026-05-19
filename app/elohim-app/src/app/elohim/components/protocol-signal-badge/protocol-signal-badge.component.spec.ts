import { ComponentFixture, TestBed } from '@angular/core/testing';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

import { ProtocolSignalBadgeComponent } from './protocol-signal-badge.component';

describe('ProtocolSignalBadgeComponent', () => {
  let fixture: ComponentFixture<ProtocolSignalBadgeComponent>;
  let component: ProtocolSignalBadgeComponent;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [ProtocolSignalBadgeComponent],
    }).compileComponents();

    fixture = TestBed.createComponent(ProtocolSignalBadgeComponent);
    component = fixture.componentInstance;
    fixture.componentRef.setInput('contentId', 'test-content-id');
    fixture.detectChanges();
  });

  afterEach(() => {
    delete (globalThis as Record<string, unknown>)['__TAURI__'];
    delete (globalThis as Record<string, unknown>)['__elohimExtensionTakeover'];
  });

  it('renders the badge pill when no higher trust surface is present', () => {
    const pill = fixture.nativeElement.querySelector('[data-testid="protocol-signal-badge-pill"]');
    expect(pill).not.toBeNull();
  });

  it('starts collapsed (no panel visible)', () => {
    const panel = fixture.nativeElement.querySelector('[data-testid="protocol-signal-panel"]');
    expect(panel).toBeNull();
  });

  it('expands the provenance panel on pill click', () => {
    const pill: HTMLElement = fixture.nativeElement.querySelector(
      '[data-testid="protocol-signal-badge-pill"]'
    );
    pill.click();
    fixture.detectChanges();
    const panel = fixture.nativeElement.querySelector('[data-testid="protocol-signal-panel"]');
    expect(panel).not.toBeNull();
    expect(panel.textContent).toContain('test-content-id');
  });

  it('suppresses itself when window.__TAURI__ is defined (Tier 3 takeover)', () => {
    (globalThis as Record<string, unknown>)['__TAURI__'] = {};
    fixture = TestBed.createComponent(ProtocolSignalBadgeComponent);
    fixture.componentRef.setInput('contentId', 'test-content-id');
    fixture.detectChanges();
    const pill = fixture.nativeElement.querySelector('[data-testid="protocol-signal-badge-pill"]');
    expect(pill).toBeNull();
  });

  it('suppresses itself when extension takeover marker is set (Tier 2 takeover)', () => {
    (globalThis as Record<string, unknown>)['__elohimExtensionTakeover'] = true;
    fixture = TestBed.createComponent(ProtocolSignalBadgeComponent);
    fixture.componentRef.setInput('contentId', 'test-content-id');
    fixture.detectChanges();
    const pill = fixture.nativeElement.querySelector('[data-testid="protocol-signal-badge-pill"]');
    expect(pill).toBeNull();
  });
});
