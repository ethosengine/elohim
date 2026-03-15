import { ComponentFixture, TestBed } from '@angular/core/testing';
import { HttpClient } from '@angular/common/http';
import { of } from 'rxjs';
import { vi } from 'vitest';

import { GateFeedbackTriggerComponent } from './gate-feedback-trigger.component';

describe('GateFeedbackTriggerComponent', () => {
  let component: GateFeedbackTriggerComponent;
  let fixture: ComponentFixture<GateFeedbackTriggerComponent>;
  let httpMock: { post: ReturnType<typeof vi.fn> };

  beforeEach(async () => {
    httpMock = { post: vi.fn().mockReturnValue(of({})) };

    await TestBed.configureTestingModule({
      imports: [GateFeedbackTriggerComponent],
      providers: [{ provide: HttpClient, useValue: httpMock }],
    }).compileComponents();

    fixture = TestBed.createComponent(GateFeedbackTriggerComponent);
    component = fixture.componentInstance;
    fixture.componentRef.setInput('contentId', 'content-1');
    fixture.detectChanges();
  });

  // --- Trigger button ---

  it('should render the trigger button', () => {
    const btn = fixture.nativeElement.querySelector(
      '[data-testid="feedback-trigger-btn"]',
    );
    expect(btn).toBeTruthy();
  });

  // --- Menu visibility ---

  it('should not show menu initially', () => {
    const menu = fixture.nativeElement.querySelector(
      '[data-testid="feedback-trigger-menu"]',
    );
    expect(menu).toBeFalsy();
  });

  it('should show menu when trigger clicked', () => {
    const btn: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-testid="feedback-trigger-btn"]',
    );
    btn.click();
    fixture.detectChanges();

    const menu = fixture.nativeElement.querySelector(
      '[data-testid="feedback-trigger-menu"]',
    );
    expect(menu).toBeTruthy();
  });

  // --- Menu items ---

  it('should show three menu items', () => {
    const btn: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-testid="feedback-trigger-btn"]',
    );
    btn.click();
    fixture.detectChanges();

    const items = fixture.nativeElement.querySelectorAll(
      '[data-testid^="feedback-menu-item-"]',
    );
    expect(items.length).toBe(3);
  });

  it('should show Flag, Challenge, Feedback labels', () => {
    const btn: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-testid="feedback-trigger-btn"]',
    );
    btn.click();
    fixture.detectChanges();

    const items = fixture.nativeElement.querySelectorAll(
      '[data-testid^="feedback-menu-item-"]',
    );
    const labels = Array.from(items).map(
      (el: Element) => (el as HTMLElement).textContent?.trim(),
    );
    expect(labels).toContain('Flag');
    expect(labels).toContain('Challenge');
    expect(labels).toContain('Feedback');
  });

  // --- Modal interaction ---

  it('should open modal when Flag clicked', () => {
    // Open menu
    const btn: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-testid="feedback-trigger-btn"]',
    );
    btn.click();
    fixture.detectChanges();

    // Click Flag
    const flagItem: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-testid="feedback-menu-item-flag"]',
    );
    flagItem.click();
    fixture.detectChanges();

    const backdrop = fixture.nativeElement.querySelector(
      '[data-testid="feedback-modal-backdrop"]',
    );
    expect(backdrop).toBeTruthy();
  });

  it('should close menu when item clicked', () => {
    // Open menu
    const btn: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-testid="feedback-trigger-btn"]',
    );
    btn.click();
    fixture.detectChanges();

    // Click an item
    const item: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-testid="feedback-menu-item-flag"]',
    );
    item.click();
    fixture.detectChanges();

    const menu = fixture.nativeElement.querySelector(
      '[data-testid="feedback-trigger-menu"]',
    );
    expect(menu).toBeFalsy();
  });

  it('should close modal when modal emits closed', () => {
    // Open menu then click Flag to open modal
    const btn: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-testid="feedback-trigger-btn"]',
    );
    btn.click();
    fixture.detectChanges();

    const flagItem: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-testid="feedback-menu-item-flag"]',
    );
    flagItem.click();
    fixture.detectChanges();

    // Modal should be open
    expect(
      fixture.nativeElement.querySelector(
        '[data-testid="feedback-modal-backdrop"]',
      ),
    ).toBeTruthy();

    // Click the modal close button
    const closeBtn: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-testid="feedback-modal-close"]',
    );
    closeBtn.click();
    fixture.detectChanges();

    // Modal should be gone
    expect(
      fixture.nativeElement.querySelector(
        '[data-testid="feedback-modal-backdrop"]',
      ),
    ).toBeFalsy();
  });

  it('should forward feedbackPosted from modal with feedbackType included', () => {
    const postedSpy = vi.fn();
    component.feedbackPosted.subscribe(postedSpy);

    // Open menu and click challenge to set activeFeedbackType
    const btn: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-testid="feedback-trigger-btn"]',
    );
    btn.click();
    fixture.detectChanges();

    const challengeItem: HTMLButtonElement =
      fixture.nativeElement.querySelector(
        '[data-testid="feedback-menu-item-challenge"]',
      );
    challengeItem.click();
    fixture.detectChanges();

    // Simulate the modal posting
    component.onModalPosted({ reachTier: 'community' });
    fixture.detectChanges();

    expect(postedSpy).toHaveBeenCalledWith({
      feedbackType: 'challenge',
      reachTier: 'community',
    });
  });
});
