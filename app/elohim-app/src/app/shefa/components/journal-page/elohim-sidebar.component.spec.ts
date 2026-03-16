import { ComponentFixture, TestBed } from '@angular/core/testing';
import { HttpClient } from '@angular/common/http';
import { of } from 'rxjs';
import { describe, it, expect, vi, beforeEach } from 'vitest';

import { ElohimSidebarComponent } from './elohim-sidebar.component';

describe('ElohimSidebarComponent', () => {
  let fixture: ComponentFixture<ElohimSidebarComponent>;
  let component: ElohimSidebarComponent;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [ElohimSidebarComponent],
      providers: [
        {
          provide: HttpClient,
          useValue: { post: vi.fn().mockReturnValue(of({})) },
        },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(ElohimSidebarComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('is collapsed by default', () => {
    expect(component.expanded()).toBe(false);

    const panel = fixture.nativeElement.querySelector(
      '[data-testid="sidebar-panel"]',
    );
    expect(panel).toBeNull();

    const toggle = fixture.nativeElement.querySelector(
      '[data-testid="sidebar-toggle"]',
    );
    expect(toggle).toBeTruthy();
  });

  it('expands when toggle is clicked', () => {
    const toggle: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-testid="sidebar-toggle"]',
    );
    toggle.click();
    fixture.detectChanges();

    expect(component.expanded()).toBe(true);

    const panel = fixture.nativeElement.querySelector(
      '[data-testid="sidebar-panel"]',
    );
    expect(panel).toBeTruthy();

    const messageList = fixture.nativeElement.querySelector(
      'app-elohim-message-list',
    );
    expect(messageList).toBeTruthy();

    const card = fixture.nativeElement.querySelector(
      'app-gate-artifact-card',
    );
    expect(card).toBeTruthy();
  });

  it('collapses when close button is clicked', () => {
    component.expanded.set(true);
    fixture.detectChanges();

    const close: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-testid="sidebar-close"]',
    );
    close.click();
    fixture.detectChanges();

    expect(component.expanded()).toBe(false);

    const panel = fixture.nativeElement.querySelector(
      '[data-testid="sidebar-panel"]',
    );
    expect(panel).toBeNull();
  });

  it('appends human message and canned response on post', () => {
    component.expanded.set(true);
    fixture.detectChanges();

    // Type into the textarea
    const textarea: HTMLTextAreaElement =
      fixture.nativeElement.querySelector(
        '[data-testid="artifact-textarea"]',
      );
    textarea.value = 'I need help with this';
    textarea.dispatchEvent(new Event('input'));
    fixture.detectChanges();

    // Click submit
    const submit: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-testid="artifact-submit"]',
    );
    submit.click();
    fixture.detectChanges();

    const messages = component.messages();
    expect(messages.length).toBe(2);
    expect(messages[0]).toEqual({
      role: 'human',
      text: 'I need help with this',
    });
    expect(messages[1].role).toBe('elohim');
    expect(messages[1].text.length).toBeGreaterThan(0);
  });

  it('grows message count with each exchange', () => {
    component.expanded.set(true);
    fixture.detectChanges();

    const postMessage = (text: string) => {
      const textarea: HTMLTextAreaElement =
        fixture.nativeElement.querySelector(
          '[data-testid="artifact-textarea"]',
        );
      textarea.value = text;
      textarea.dispatchEvent(new Event('input'));
      fixture.detectChanges();

      const submit: HTMLButtonElement =
        fixture.nativeElement.querySelector(
          '[data-testid="artifact-submit"]',
        );
      submit.click();
      fixture.detectChanges();
    };

    postMessage('First message');
    expect(component.messages().length).toBe(2);

    postMessage('Second message');
    expect(component.messages().length).toBe(4);

    postMessage('Third message');
    expect(component.messages().length).toBe(6);
  });
});
