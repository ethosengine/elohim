import { ComponentFixture, TestBed } from '@angular/core/testing';
import { describe, it, expect, beforeEach, vi } from 'vitest';

import type { RoutingSuggestion } from '../../models/journal-routing.model';

import { JournalRoutingCardsComponent } from './journal-routing-cards.component';

const MOCK_SUGGESTIONS: RoutingSuggestion[] = [
  {
    id: 'filing-1',
    kind: 'filing',
    destinationType: 'journal-filing',
    title: 'File journal entry',
    summary: 'Save to /journal/home-maintenance/',
    suggestedPath: '/journal/home-maintenance/',
    reach: 'private',
    contextMetadata: {},
    status: 'suggested',
  },
  {
    id: 'deriv-1',
    kind: 'derivative',
    destinationType: 'exchange-request',
    title: 'Post to Exchange',
    summary: 'Create a request on the community exchange',
    suggestedPath: '/shefa/exchange/',
    reach: 'community',
    contextMetadata: {},
    status: 'suggested',
  },
];

describe('JournalRoutingCardsComponent', () => {
  let component: JournalRoutingCardsComponent;
  let fixture: ComponentFixture<JournalRoutingCardsComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [JournalRoutingCardsComponent],
    }).compileComponents();

    fixture = TestBed.createComponent(JournalRoutingCardsComponent);
    component = fixture.componentInstance;
    fixture.componentRef.setInput('suggestions', MOCK_SUGGESTIONS);
    fixture.componentRef.setInput('journalText', 'First line title\nSecond line\nThird line\nFourth line hidden');
    fixture.detectChanges();
  });

  it('renders a card for each suggestion', () => {
    const cards = fixture.nativeElement.querySelectorAll('[data-testid="routing-card"]');
    expect(cards.length).toBe(2);
  });

  it('renders filing card first', () => {
    const titles = fixture.nativeElement.querySelectorAll('[data-testid="card-title"]');
    expect(titles[0].textContent).toContain('File journal entry');
  });

  it('shows reach badge on each card', () => {
    const badges = fixture.nativeElement.querySelectorAll('[data-testid="card-reach"]');
    expect(badges.length).toBe(2);
  });

  it('emits postCard with card id on post click', () => {
    const spy = vi.fn();
    component.postCard.subscribe(spy);

    const postBtns = fixture.nativeElement.querySelectorAll('[data-testid="card-post-btn"]');
    // Click the derivative card's post button (second card)
    postBtns[1].click();
    fixture.detectChanges();

    expect(spy).toHaveBeenCalledWith('deriv-1');
  });

  it('emits dismissCard with card id on dismiss click', () => {
    const spy = vi.fn();
    component.dismissCard.subscribe(spy);

    const dismissBtns = fixture.nativeElement.querySelectorAll('[data-testid="card-dismiss-btn"]');
    dismissBtns[0].click();
    fixture.detectChanges();

    expect(spy).toHaveBeenCalledWith('filing-1');
  });

  it('shows posted state for posted cards', () => {
    const updated: RoutingSuggestion[] = [
      { ...MOCK_SUGGESTIONS[0], status: 'posted' },
      MOCK_SUGGESTIONS[1],
    ];
    fixture.componentRef.setInput('suggestions', updated);
    fixture.detectChanges();

    const badges = fixture.nativeElement.querySelectorAll('[data-testid="card-posted-badge"]');
    expect(badges.length).toBe(1);
  });

  it('emits editRequested on edit button click', () => {
    const spy = vi.fn();
    component.editRequested.subscribe(spy);

    const editBtn = fixture.nativeElement.querySelector('[data-testid="routing-edit-btn"]');
    editBtn.click();
    fixture.detectChanges();

    expect(spy).toHaveBeenCalled();
  });
});
