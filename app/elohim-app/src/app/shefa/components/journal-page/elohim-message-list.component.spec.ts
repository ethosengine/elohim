import { ComponentFixture, TestBed } from '@angular/core/testing';
import { describe, it, expect, beforeEach } from 'vitest';

import { ElohimMessageListComponent, type ChatMessage } from './elohim-message-list.component';

describe('ElohimMessageListComponent', () => {
  let component: ElohimMessageListComponent;
  let fixture: ComponentFixture<ElohimMessageListComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [ElohimMessageListComponent],
    }).compileComponents();

    fixture = TestBed.createComponent(ElohimMessageListComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should render empty state when no messages', () => {
    const el = fixture.nativeElement;
    expect(el.querySelectorAll('[data-testid^="chat-message-"]').length).toBe(0);
  });

  it('should render human messages with human class', () => {
    fixture.componentRef.setInput('messages', [
      { role: 'human', text: 'Hello there' },
    ] satisfies ChatMessage[]);
    fixture.detectChanges();

    const msg = fixture.nativeElement.querySelector('[data-testid="chat-message-0"]');
    expect(msg).toBeTruthy();
    expect(msg.textContent).toContain('Hello there');
    expect(msg.classList.contains('human')).toBe(true);
  });

  it('should render elohim messages with elohim class', () => {
    fixture.componentRef.setInput('messages', [
      { role: 'elohim', text: "I'm here" },
    ] satisfies ChatMessage[]);
    fixture.detectChanges();

    const msg = fixture.nativeElement.querySelector('[data-testid="chat-message-0"]');
    expect(msg).toBeTruthy();
    expect(msg.textContent).toContain("I'm here");
    expect(msg.classList.contains('elohim')).toBe(true);
  });

  it('should render multiple messages in order', () => {
    fixture.componentRef.setInput('messages', [
      { role: 'human', text: 'First' },
      { role: 'elohim', text: 'Second' },
      { role: 'human', text: 'Third' },
    ] satisfies ChatMessage[]);
    fixture.detectChanges();

    const msgs = fixture.nativeElement.querySelectorAll('[data-testid^="chat-message-"]');
    expect(msgs.length).toBe(3);
    expect(msgs[0].textContent).toContain('First');
    expect(msgs[2].textContent).toContain('Third');
  });
});
