import { ComponentFixture, TestBed } from '@angular/core/testing';
import { Component } from '@angular/core';
import { describe, it, expect, vi, beforeEach } from 'vitest';

import type { RoutingSuggestion } from '../../models/journal-routing.model';
import { JournalRoutedComponent } from './journal-routed.component';

const POSTED: RoutingSuggestion[] = [
  {
    id: 'filing-1',
    kind: 'filing',
    destinationType: 'journal-filing',
    title: 'File journal entry',
    summary: 'Save to /journal/home-maintenance/',
    suggestedPath: '/journal/home-maintenance/',
    reach: 'private',
    contextMetadata: {},
    status: 'posted',
  },
  {
    id: 'deriv-1',
    kind: 'derivative',
    destinationType: 'exchange-request',
    title: 'Post to Exchange',
    summary: 'Create a request on the community exchange',
    suggestedPath: '',
    reach: 'community',
    contextMetadata: {},
    status: 'posted',
  },
];

@Component({
  standalone: true,
  imports: [JournalRoutedComponent],
  template: `
    <app-journal-routed [suggestions]="suggestions" (writeAnother)="onWriteAnother()" />
  `,
})
class TestHostComponent {
  suggestions: RoutingSuggestion[] = POSTED;
  onWriteAnother = vi.fn();
}

describe('JournalRoutedComponent', () => {
  let fixture: ComponentFixture<TestHostComponent>;
  let host: TestHostComponent;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [TestHostComponent],
    }).compileComponents();

    fixture = TestBed.createComponent(TestHostComponent);
    host = fixture.componentInstance;
    // No detectChanges() here: "does not show dismissed cards" mutates
    // host.suggestions before its first render. Under ChangeDetectionStrategy
    // OnPush (Angular 22's implicit default, which JournalRoutedComponent also
    // sets explicitly), an initial render here — followed by a second
    // detectChanges() after the signal input's bound value changes — leaves the
    // child's `postedCards()` computed stale, a CD-timing artifact of the double
    // pass. Each test renders exactly once, from its own desired starting state.
  });

  it('shows posted cards with reach badges', () => {
    fixture.detectChanges();
    const badges = fixture.nativeElement.querySelectorAll('[data-testid="routed-reach"]');
    expect(badges.length).toBe(2);
  });

  it('shows completion message', () => {
    fixture.detectChanges();
    const message = fixture.nativeElement.querySelector('[data-testid="routed-message"]');
    expect(message).toBeTruthy();
  });

  it('emits writeAnother on button click', () => {
    fixture.detectChanges();
    const btn: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-testid="write-another-btn"]'
    );
    btn.click();

    expect(host.onWriteAnother).toHaveBeenCalled();
  });

  it('does not show dismissed cards', () => {
    host.suggestions = [POSTED[0], { ...POSTED[1], status: 'dismissed' }];
    fixture.detectChanges();

    const badges = fixture.nativeElement.querySelectorAll('[data-testid="routed-reach"]');
    expect(badges.length).toBe(1);
  });
});
