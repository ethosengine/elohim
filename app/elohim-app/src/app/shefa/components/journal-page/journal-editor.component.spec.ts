import { ComponentFixture, TestBed } from '@angular/core/testing';
import { HttpClient } from '@angular/common/http';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { of } from 'rxjs';

import { JournalEditorComponent } from './journal-editor.component';

describe('JournalEditorComponent', () => {
  let component: JournalEditorComponent;
  let fixture: ComponentFixture<JournalEditorComponent>;
  let mockHttp: { patch: ReturnType<typeof vi.fn>; get: ReturnType<typeof vi.fn>; post: ReturnType<typeof vi.fn> };

  beforeEach(async () => {
    mockHttp = {
      patch: vi.fn().mockReturnValue(of({})),
      get: vi.fn().mockReturnValue(of({})),
      post: vi.fn().mockReturnValue(of({})),
    };

    await TestBed.configureTestingModule({
      imports: [JournalEditorComponent],
      providers: [{ provide: HttpClient, useValue: mockHttp }],
    }).compileComponents();

    fixture = TestBed.createComponent(JournalEditorComponent);
    component = fixture.componentInstance;
    fixture.componentRef.setInput('contentId', 'journal-1');
    fixture.detectChanges();
  });

  it('should render title input with data-testid', () => {
    const el = fixture.nativeElement.querySelector('[data-testid="journal-title"]');
    expect(el).toBeTruthy();
    expect(el.tagName).toBe('INPUT');
  });

  it('should render body textarea with data-testid', () => {
    const el = fixture.nativeElement.querySelector('[data-testid="journal-body"]');
    expect(el).toBeTruthy();
    expect(el.tagName).toBe('TEXTAREA');
  });

  it('should show save status indicator with data-testid', () => {
    const el = fixture.nativeElement.querySelector('[data-testid="save-status"]');
    expect(el).toBeTruthy();
  });

  it('should update title signal on input event', () => {
    const input = fixture.nativeElement.querySelector('[data-testid="journal-title"]') as HTMLInputElement;
    input.value = 'My Journal';
    input.dispatchEvent(new Event('input'));
    expect(component.title()).toBe('My Journal');
  });

  it('should update body signal on input event', () => {
    const textarea = fixture.nativeElement.querySelector('[data-testid="journal-body"]') as HTMLTextAreaElement;
    textarea.value = 'Some thoughts';
    textarea.dispatchEvent(new Event('input'));
    expect(component.body()).toBe('Some thoughts');
  });

  it('should save title via PATCH on blur', () => {
    component.title.set('New Title');
    const input = fixture.nativeElement.querySelector('[data-testid="journal-title"]') as HTMLInputElement;
    input.dispatchEvent(new Event('blur'));
    fixture.detectChanges();

    expect(mockHttp.patch).toHaveBeenCalledWith(
      expect.stringContaining('/db/content/journal-1'),
      { title: 'New Title' },
    );
  });

  it('should debounce body autosave by 1.5s', async () => {
    vi.useFakeTimers();
    const textarea = fixture.nativeElement.querySelector('[data-testid="journal-body"]') as HTMLTextAreaElement;

    textarea.value = 'Draft 1';
    textarea.dispatchEvent(new Event('input'));
    vi.advanceTimersByTime(500);
    expect(mockHttp.patch).not.toHaveBeenCalledWith(
      expect.stringContaining('/db/content/journal-1'),
      expect.objectContaining({ contentBody: 'Draft 1' }),
    );

    textarea.value = 'Draft 2';
    textarea.dispatchEvent(new Event('input'));
    vi.advanceTimersByTime(1500);

    expect(mockHttp.patch).toHaveBeenCalledWith(
      expect.stringContaining('/db/content/journal-1'),
      { contentBody: 'Draft 2' },
    );
    vi.useRealTimers();
  });

  it('should show Saved status after successful save', async () => {
    vi.useFakeTimers();
    const textarea = fixture.nativeElement.querySelector('[data-testid="journal-body"]') as HTMLTextAreaElement;
    textarea.value = 'Content';
    textarea.dispatchEvent(new Event('input'));
    vi.advanceTimersByTime(1500);

    expect(component.saveStatus()).toBe('Saved');
    vi.useRealTimers();
  });

  it('should populate fields via loadContent', () => {
    component.loadContent('Loaded Title', 'Loaded Body');
    expect(component.title()).toBe('Loaded Title');
    expect(component.body()).toBe('Loaded Body');
  });
});
