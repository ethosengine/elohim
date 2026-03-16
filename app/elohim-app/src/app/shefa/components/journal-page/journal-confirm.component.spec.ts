import { ComponentFixture, TestBed } from '@angular/core/testing';
import { describe, it, expect, beforeEach, vi } from 'vitest';

import { JournalConfirmComponent } from './journal-confirm.component';

describe('JournalConfirmComponent', () => {
  let component: JournalConfirmComponent;
  let fixture: ComponentFixture<JournalConfirmComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [JournalConfirmComponent],
    }).compileComponents();

    fixture = TestBed.createComponent(JournalConfirmComponent);
    component = fixture.componentInstance;
  });

  function setInputs(overrides: { text?: string; intentSummary?: string; analyzing?: boolean } = {}) {
    fixture.componentRef.setInput('text', overrides.text ?? 'My journal entry');
    fixture.componentRef.setInput('intentSummary', overrides.intentSummary ?? 'You want to offer help');
    fixture.componentRef.setInput('analyzing', overrides.analyzing ?? false);
    fixture.detectChanges();
  }

  it('should show the journal text read-only', () => {
    setInputs({ text: 'Today I reflected on community.' });

    const el = fixture.nativeElement.querySelector('[data-testid="confirm-text"]');
    expect(el).toBeTruthy();
    expect(el.textContent).toContain('Today I reflected on community.');
  });

  it('should show intent summary', () => {
    setInputs({ intentSummary: 'You seem to want to volunteer.' });

    const el = fixture.nativeElement.querySelector('[data-testid="intent-summary"]');
    expect(el).toBeTruthy();
    expect(el.textContent).toContain('You seem to want to volunteer.');
  });

  it('should show shimmer when analyzing', () => {
    setInputs({ analyzing: true });

    const shimmer = fixture.nativeElement.querySelector('[data-testid="confirm-shimmer"]');
    expect(shimmer).toBeTruthy();

    const text = fixture.nativeElement.querySelector('[data-testid="confirm-text"]');
    expect(text).toBeFalsy();
  });

  it('should emit confirmed on "Looks good" click', () => {
    const spy = vi.fn();
    component.confirmed.subscribe(spy);
    setInputs();

    const btn: HTMLButtonElement = fixture.nativeElement.querySelector('[data-testid="confirm-btn"]');
    btn.click();

    expect(spy).toHaveBeenCalledOnce();
  });

  it('should emit editRequested on "Edit" click', () => {
    const spy = vi.fn();
    component.editRequested.subscribe(spy);
    setInputs();

    const btn: HTMLButtonElement = fixture.nativeElement.querySelector('[data-testid="edit-btn"]');
    btn.click();

    expect(spy).toHaveBeenCalledOnce();
  });

  it('should disable "Looks good" button when analyzing', () => {
    setInputs({ analyzing: true });

    const btn: HTMLButtonElement = fixture.nativeElement.querySelector('[data-testid="confirm-btn"]');
    expect(btn.disabled).toBe(true);
  });
});
