import { TestBed } from '@angular/core/testing';
import { HttpClient } from '@angular/common/http';
import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { of } from 'rxjs';

import { JournalRoutingService } from './journal-routing.service';

describe('JournalRoutingService', () => {
  let service: JournalRoutingService;
  let mockHttp: {
    patch: ReturnType<typeof vi.fn>;
    post: ReturnType<typeof vi.fn>;
    get: ReturnType<typeof vi.fn>;
  };

  beforeEach(() => {
    vi.useFakeTimers();

    mockHttp = {
      patch: vi.fn().mockReturnValue(of({})),
      post: vi.fn().mockReturnValue(of({})),
      get: vi.fn().mockReturnValue(of({})),
    };

    TestBed.configureTestingModule({
      providers: [
        JournalRoutingService,
        { provide: HttpClient, useValue: mockHttp },
      ],
    });

    service = TestBed.inject(JournalRoutingService);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  // =========================================================================
  // Initial state
  // =========================================================================

  it('should start in writing state', () => {
    expect(service.state()).toBe('writing');
  });

  it('should have empty intent summary initially', () => {
    expect(service.intentSummary()).toBe('');
  });

  it('should have empty suggestions initially', () => {
    expect(service.suggestions()).toEqual([]);
  });

  it('should have empty journal text initially', () => {
    expect(service.journalText()).toBe('');
  });

  // =========================================================================
  // finish() — Round 1: intent analysis
  // =========================================================================

  describe('finish()', () => {
    it('should transition to confirming after timer resolves', () => {
      service.finish('I need to fix the leaky faucet');
      expect(service.state()).toBe('writing'); // still writing during timer

      vi.advanceTimersByTime(1000);

      expect(service.state()).toBe('confirming');
    });

    it('should store the journal text', () => {
      service.finish('My journal entry text');
      vi.advanceTimersByTime(1000);

      expect(service.journalText()).toBe('My journal entry text');
    });

    it('should produce intent summary for exchange keywords', () => {
      service.finish('I need help fixing the broken sink');
      vi.advanceTimersByTime(1000);

      expect(service.intentSummary()).toBeTruthy();
      expect(service.intentSummary().length).toBeGreaterThan(0);
    });

    it('should detect exchange-request intent from need/want/broken keywords', () => {
      service.finish('I need to repair the broken dishwasher');
      vi.advanceTimersByTime(1000);

      const summary = service.intentSummary();
      expect(summary).toContain('exchange');
    });

    it('should detect governance-proposal intent from policy/vote keywords', () => {
      service.finish('We should propose a new community policy about parking');
      vi.advanceTimersByTime(1000);

      const summary = service.intentSummary();
      expect(summary).toContain('governance');
    });

    it('should detect content intent from learned/tutorial keywords', () => {
      service.finish('I discovered how to explain quadratic equations');
      vi.advanceTimersByTime(1000);

      const summary = service.intentSummary();
      expect(summary).toContain('content');
    });

    it('should guess filing path for home-maintenance keywords', () => {
      service.finish('I need to fix the broken faucet in the kitchen');
      vi.advanceTimersByTime(1000);

      // After confirming, we can check suggestions in confirm() tests
      // But intent analysis should set a suggestedPath
      expect(service.state()).toBe('confirming');
    });
  });

  // =========================================================================
  // confirm() — Round 2: generate suggestions
  // =========================================================================

  describe('confirm()', () => {
    beforeEach(() => {
      service.finish('I need help finding someone to repair the broken fence');
      vi.advanceTimersByTime(1000);
      expect(service.state()).toBe('confirming');
    });

    it('should transition to routing after timer resolves', () => {
      service.confirm();
      expect(service.state()).toBe('confirming'); // still confirming during timer

      vi.advanceTimersByTime(1000);

      expect(service.state()).toBe('routing');
    });

    it('should always include a filing card', () => {
      service.confirm();
      vi.advanceTimersByTime(1000);

      const cards = service.suggestions();
      const filingCard = cards.find((c) => c.kind === 'filing');
      expect(filingCard).toBeDefined();
      expect(filingCard!.destinationType).toBe('journal-filing');
      expect(filingCard!.reach).toBe('private');
    });

    it('should include derivative card for detected exchange-request type', () => {
      service.confirm();
      vi.advanceTimersByTime(1000);

      const cards = service.suggestions();
      const derivative = cards.find(
        (c) => c.kind === 'derivative' && c.destinationType === 'exchange-request',
      );
      expect(derivative).toBeDefined();
    });

    it('should generate suggestion IDs', () => {
      service.confirm();
      vi.advanceTimersByTime(1000);

      const cards = service.suggestions();
      for (const card of cards) {
        expect(card.id).toBeTruthy();
      }
    });

    it('should set all suggestions to suggested status', () => {
      service.confirm();
      vi.advanceTimersByTime(1000);

      const cards = service.suggestions();
      for (const card of cards) {
        expect(card.status).toBe('suggested');
      }
    });
  });

  describe('confirm() with governance text', () => {
    beforeEach(() => {
      service.finish('We should vote on a new community rule about noise');
      vi.advanceTimersByTime(1000);
    });

    it('should include governance-proposal derivative card', () => {
      service.confirm();
      vi.advanceTimersByTime(1000);

      const cards = service.suggestions();
      const governance = cards.find(
        (c) => c.kind === 'derivative' && c.destinationType === 'governance-proposal',
      );
      expect(governance).toBeDefined();
    });
  });

  describe('confirm() with content text', () => {
    beforeEach(() => {
      service.finish('I learned how to build a tutorial for explaining fractions');
      vi.advanceTimersByTime(1000);
    });

    it('should include content derivative card', () => {
      service.confirm();
      vi.advanceTimersByTime(1000);

      const cards = service.suggestions();
      const content = cards.find(
        (c) => c.kind === 'derivative' && c.destinationType === 'content',
      );
      expect(content).toBeDefined();
    });
  });

  // =========================================================================
  // edit() — back to writing
  // =========================================================================

  describe('edit()', () => {
    it('should return to writing from confirming', () => {
      service.finish('Some text');
      vi.advanceTimersByTime(1000);
      expect(service.state()).toBe('confirming');

      service.edit();
      expect(service.state()).toBe('writing');
    });

    it('should return to writing from routing', () => {
      service.finish('I need to fix something broken');
      vi.advanceTimersByTime(1000);
      service.confirm();
      vi.advanceTimersByTime(1000);
      expect(service.state()).toBe('routing');

      service.edit();
      expect(service.state()).toBe('writing');
    });
  });

  // =========================================================================
  // postCard() — posting individual cards
  // =========================================================================

  describe('postCard()', () => {
    beforeEach(() => {
      service.setContentId('journal-123');
      service.finish('I need to find someone to help fix the broken fence');
      vi.advanceTimersByTime(1000);
      service.confirm();
      vi.advanceTimersByTime(1000);
      expect(service.state()).toBe('routing');
    });

    it('should PATCH for filing card', () => {
      const filingCard = service.suggestions().find((c) => c.kind === 'filing')!;
      service.postCard(filingCard.id);

      expect(mockHttp.patch).toHaveBeenCalledTimes(1);
      const [url, body] = mockHttp.patch.mock.calls[0] as [string, unknown];
      expect(url).toContain('journal-123');
      expect(body).toEqual(
        expect.objectContaining({
          metadata: expect.objectContaining({ journalFolder: expect.any(String) }),
        }),
      );
    });

    it('should POST for derivative card', () => {
      const derivativeCard = service.suggestions().find((c) => c.kind === 'derivative')!;
      service.postCard(derivativeCard.id);

      expect(mockHttp.post).toHaveBeenCalledTimes(1);
      const [, body] = mockHttp.post.mock.calls[0] as [string, Record<string, unknown>];
      expect(body['contentType']).toBe(derivativeCard.destinationType);
      expect((body['metadata'] as Record<string, unknown>)['sourceJournalId']).toBe('journal-123');
    });

    it('should mark card as posted after successful post', () => {
      const filingCard = service.suggestions().find((c) => c.kind === 'filing')!;
      service.postCard(filingCard.id);

      const updated = service.suggestions().find((c) => c.id === filingCard.id)!;
      expect(updated.status).toBe('posted');
    });

    it('should transition to routed when all cards are posted or dismissed', () => {
      const cards = service.suggestions();
      for (const card of cards) {
        service.postCard(card.id);
      }

      expect(service.state()).toBe('routed');
    });
  });

  // =========================================================================
  // dismissCard()
  // =========================================================================

  describe('dismissCard()', () => {
    beforeEach(() => {
      service.setContentId('journal-123');
      service.finish('I need to find help fixing the broken fence');
      vi.advanceTimersByTime(1000);
      service.confirm();
      vi.advanceTimersByTime(1000);
    });

    it('should mark card as dismissed', () => {
      const filingCard = service.suggestions().find((c) => c.kind === 'filing')!;
      service.dismissCard(filingCard.id);

      const updated = service.suggestions().find((c) => c.id === filingCard.id)!;
      expect(updated.status).toBe('dismissed');
    });

    it('should not make any HTTP calls', () => {
      const filingCard = service.suggestions().find((c) => c.kind === 'filing')!;
      service.dismissCard(filingCard.id);

      expect(mockHttp.patch).not.toHaveBeenCalled();
      expect(mockHttp.post).not.toHaveBeenCalled();
    });

    it('should transition to routed when all cards are dismissed', () => {
      const cards = service.suggestions();
      for (const card of cards) {
        service.dismissCard(card.id);
      }

      expect(service.state()).toBe('routed');
    });
  });

  // =========================================================================
  // setContentId()
  // =========================================================================

  describe('setContentId()', () => {
    it('should store the content ID for later use', () => {
      service.setContentId('journal-456');
      service.finish('I need help fixing something broken');
      vi.advanceTimersByTime(1000);
      service.confirm();
      vi.advanceTimersByTime(1000);

      const filingCard = service.suggestions().find((c) => c.kind === 'filing')!;
      service.postCard(filingCard.id);

      const [url] = mockHttp.patch.mock.calls[0] as [string];
      expect(url).toContain('journal-456');
    });
  });

  // =========================================================================
  // Filing path guessing
  // =========================================================================

  describe('filing path detection', () => {
    it('should suggest home-maintenance path for repair keywords', () => {
      service.finish('I need to fix the broken kitchen faucet');
      vi.advanceTimersByTime(1000);
      service.confirm();
      vi.advanceTimersByTime(1000);

      const filingCard = service.suggestions().find((c) => c.kind === 'filing')!;
      expect(filingCard.suggestedPath).toBe('home-maintenance');
    });

    it('should suggest learning path for educational keywords', () => {
      service.finish('I learned about the quadratic formula today');
      vi.advanceTimersByTime(1000);
      service.confirm();
      vi.advanceTimersByTime(1000);

      const filingCard = service.suggestions().find((c) => c.kind === 'filing')!;
      expect(filingCard.suggestedPath).toBe('learning');
    });

    it('should suggest work path for work keywords', () => {
      service.finish('Meeting notes from the project deadline discussion');
      vi.advanceTimersByTime(1000);
      service.confirm();
      vi.advanceTimersByTime(1000);

      const filingCard = service.suggestions().find((c) => c.kind === 'filing')!;
      expect(filingCard.suggestedPath).toBe('work');
    });

    it('should suggest general path when no keywords match', () => {
      service.finish('The sky was beautiful today');
      vi.advanceTimersByTime(1000);
      service.confirm();
      vi.advanceTimersByTime(1000);

      const filingCard = service.suggestions().find((c) => c.kind === 'filing')!;
      expect(filingCard.suggestedPath).toBe('general');
    });
  });

  // =========================================================================
  // Mixed card handling — post some, dismiss others
  // =========================================================================

  describe('mixed post and dismiss', () => {
    beforeEach(() => {
      service.setContentId('journal-789');
      service.finish('I need help and should propose a community rule about noise');
      vi.advanceTimersByTime(1000);
      service.confirm();
      vi.advanceTimersByTime(1000);
    });

    it('should transition to routed when all cards are resolved', () => {
      const cards = service.suggestions();
      expect(cards.length).toBeGreaterThanOrEqual(2);

      // Post the first, dismiss the rest
      service.postCard(cards[0].id);
      for (let i = 1; i < cards.length; i++) {
        service.dismissCard(cards[i].id);
      }

      expect(service.state()).toBe('routed');
    });
  });
});
