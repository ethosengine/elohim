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
    suggestedPath: '/shefa/exchange/',
    reach: 'community',
    contextMetadata: {},
    status: 'posted',
  },
];

@Component({
  standalone: true,
  imports: [JournalRoutedComponent],
  template: `<app-journal-routed
    [suggestions]="suggestions"
    (writeAnother)="onWriteAnother()"
  />`,
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
    fixture.detectChanges();
  });

  it('shows posted cards with reach badges', () => {
    const badges = fixture.nativeElement.querySelectorAll(
      '[data-testid="routed-reach"]',
    );
    expect(badges.length).toBe(2);
  });

  it('shows completion message', () => {
    const message = fixture.nativeElement.querySelector(
      '[data-testid="routed-message"]',
    );
    expect(message).toBeTruthy();
  });

  it('emits writeAnother on button click', () => {
    const btn: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-testid="write-another-btn"]',
    );
    btn.click();
    fixture.detectChanges();

    expect(host.onWriteAnother).toHaveBeenCalled();
  });

  it('does not show dismissed cards', () => {
    host.suggestions = [
      POSTED[0],
      { ...POSTED[1], status: 'dismissed' },
    ];
    fixture.detectChanges();

    const badges = fixture.nativeElement.querySelectorAll(
      '[data-testid="routed-reach"]',
    );
    expect(badges.length).toBe(1);
  });
});
