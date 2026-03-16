import { TestBed } from '@angular/core/testing';
import { HttpClient } from '@angular/common/http';
import { describe, it, expect, beforeEach, vi } from 'vitest';
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
    mockHttp = {
      patch: vi.fn().mockReturnValue(of({})),
      post: vi.fn().mockImplementation((url: string) => {
        if (url.includes('/journal/analyze')) {
          return of({
            summary: 'This sounds like a need or request.',
            detectedTypes: ['exchange-request'],
            suggestedPath: 'home-maintenance',
          });
        }
        if (url.includes('/journal/suggest')) {
          return of([
            {
              id: 'filing-1',
              kind: 'filing',
              destinationType: 'journal-filing',
              title: 'File in journal',
              summary: 'File this entry under home-maintenance',
              suggestedPath: 'home-maintenance',
              reach: 'private',
              contextMetadata: {},
              status: 'suggested',
            },
            {
              id: 'deriv-1',
              kind: 'derivative',
              destinationType: 'exchange-request',
              title: 'Post to exchange',
              summary: 'Post a request on the community exchange.',
              suggestedPath: '/shefa/exchange/',
              reach: 'community',
              contextMetadata: {},
              status: 'suggested',
            },
          ]);
        }
        // Storage API calls (createContent)
        return of({});
      }),
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
  // finish() — Round 1: intent analysis via backend
  // =========================================================================

  describe('finish()', () => {
    it('should transition to confirming after backend responds', () => {
      service.finish('I need to fix the leaky faucet');
      expect(service.state()).toBe('confirming');
    });

    it('should store the journal text', () => {
      service.finish('My journal entry text');
      expect(service.journalText()).toBe('My journal entry text');
    });

    it('should populate intentSummary from backend response', () => {
      service.finish('I need help fixing the broken sink');
      expect(service.intentSummary()).toBe('This sounds like a need or request.');
    });

    it('should POST to /api/v1/journal/analyze', () => {
      service.setContentId('journal-100');
      service.finish('Some text about needs');

      expect(mockHttp.post).toHaveBeenCalledWith('/api/v1/journal/analyze', {
        text: 'Some text about needs',
        contentId: 'journal-100',
      });
    });
  });

  // =========================================================================
  // edit() — back to writing
  // =========================================================================

  describe('edit()', () => {
    it('should return to writing from confirming', () => {
      service.finish('Some text');
      expect(service.state()).toBe('confirming');

      service.edit();
      expect(service.state()).toBe('writing');
    });

    it('should return to writing from routing', () => {
      service.finish('I need to fix something broken');
      service.confirm();
      expect(service.state()).toBe('routing');

      service.edit();
      expect(service.state()).toBe('writing');
    });
  });

  // =========================================================================
  // confirm() — Round 2: generate suggestions via backend
  // =========================================================================

  describe('confirm()', () => {
    beforeEach(() => {
      service.finish('I need help finding someone to repair the broken fence');
      expect(service.state()).toBe('confirming');
    });

    it('should transition to routing after backend responds', () => {
      service.confirm();
      expect(service.state()).toBe('routing');
    });

    it('should include a filing card from backend suggestions', () => {
      service.confirm();

      const cards = service.suggestions();
      const filingCard = cards.find(c => c.kind === 'filing');
      expect(filingCard).toBeDefined();
      expect(filingCard!.destinationType).toBe('journal-filing');
      expect(filingCard!.reach).toBe('private');
    });

    it('should include derivative card from backend suggestions', () => {
      service.confirm();

      const cards = service.suggestions();
      const derivative = cards.find(
        c => c.kind === 'derivative' && c.destinationType === 'exchange-request',
      );
      expect(derivative).toBeDefined();
    });

    it('should have IDs on all suggestion cards', () => {
      service.confirm();

      const cards = service.suggestions();
      for (const card of cards) {
        expect(card.id).toBeTruthy();
      }
    });

    it('should set all suggestions to suggested status', () => {
      service.confirm();

      const cards = service.suggestions();
      for (const card of cards) {
        expect(card.status).toBe('suggested');
      }
    });

    it('should POST to /api/v1/journal/suggest with intent', () => {
      service.setContentId('journal-200');
      service.confirm();

      const suggestCall = mockHttp.post.mock.calls.find(
        (call: unknown[]) => (call[0] as string).includes('/journal/suggest'),
      );
      expect(suggestCall).toBeDefined();
      expect(suggestCall![1]).toEqual(
        expect.objectContaining({
          text: 'I need help finding someone to repair the broken fence',
          contentId: 'journal-200',
          intent: expect.objectContaining({
            summary: 'This sounds like a need or request.',
            detectedTypes: ['exchange-request'],
          }),
        }),
      );
    });
  });

  // =========================================================================
  // postCard() — posting individual cards
  // =========================================================================

  describe('postCard()', () => {
    beforeEach(() => {
      service.setContentId('journal-123');
      service.finish('I need to find someone to help fix the broken fence');
      service.confirm();
      expect(service.state()).toBe('routing');
    });

    it('should PATCH for filing card', () => {
      const filingCard = service.suggestions().find(c => c.kind === 'filing')!;
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
      const derivativeCard = service.suggestions().find(c => c.kind === 'derivative')!;
      service.postCard(derivativeCard.id);

      // Find the createContent POST (not the journal/analyze or journal/suggest calls)
      const createCall = mockHttp.post.mock.calls.find(
        (call: unknown[]) => (call[0] as string).includes('/db/content'),
      );
      expect(createCall).toBeDefined();
      const body = createCall![1] as Record<string, unknown>;
      expect(body['contentType']).toBe(derivativeCard.destinationType);
      expect((body['metadata'] as Record<string, unknown>)['sourceJournalId']).toBe('journal-123');
    });

    it('should mark card as posted after successful post', () => {
      const filingCard = service.suggestions().find(c => c.kind === 'filing')!;
      service.postCard(filingCard.id);

      const updated = service.suggestions().find(c => c.id === filingCard.id)!;
      expect(updated.status).toBe('posted');
    });

    it('should transition to routed when all cards are posted', () => {
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
      service.confirm();
    });

    it('should mark card as dismissed', () => {
      const filingCard = service.suggestions().find(c => c.kind === 'filing')!;
      service.dismissCard(filingCard.id);

      const updated = service.suggestions().find(c => c.id === filingCard.id)!;
      expect(updated.status).toBe('dismissed');
    });

    it('should not make any storage HTTP calls', () => {
      mockHttp.patch.mockClear();
      const postCallsBefore = mockHttp.post.mock.calls.length;

      const filingCard = service.suggestions().find(c => c.kind === 'filing')!;
      service.dismissCard(filingCard.id);

      expect(mockHttp.patch).not.toHaveBeenCalled();
      // No new POST calls beyond analyze+suggest
      expect(mockHttp.post.mock.calls.length).toBe(postCallsBefore);
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
    it('should store the content ID for later use in PATCH', () => {
      service.setContentId('journal-456');
      service.finish('I need help fixing something broken');
      service.confirm();

      const filingCard = service.suggestions().find(c => c.kind === 'filing')!;
      service.postCard(filingCard.id);

      const [url] = mockHttp.patch.mock.calls[0] as [string];
      expect(url).toContain('journal-456');
    });
  });

  // =========================================================================
  // Mixed card handling — post some, dismiss others
  // =========================================================================

  describe('mixed post and dismiss', () => {
    beforeEach(() => {
      service.setContentId('journal-789');
      service.finish('I need help and should propose a community rule about noise');
      service.confirm();
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
