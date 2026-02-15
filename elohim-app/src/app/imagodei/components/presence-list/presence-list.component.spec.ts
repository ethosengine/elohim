import { ComponentFixture, TestBed } from '@angular/core/testing';
import { signal } from '@angular/core';

import { PresenceListComponent } from './presence-list.component';
import { PresenceService } from '../../services/presence.service';
import { IdentityService } from '../../services/identity.service';
import type { ContributorPresenceView } from '../../models/presence.model';

describe('PresenceListComponent', () => {
  let component: PresenceListComponent;
  let fixture: ComponentFixture<PresenceListComponent>;
  let mockPresenceService: jasmine.SpyObj<PresenceService>;
  let mockIdentityService: jasmine.SpyObj<IdentityService>;

  const mockPresences: ContributorPresenceView[] = [
    {
      id: 'presence-1',
      displayName: 'Test User',
      presenceState: 'unclaimed',
      externalIdentifiers: [],
      establishingContentIds: [],
      establishedAt: '2024-01-01T00:00:00Z',
      affinityTotal: 0,
      uniqueEngagers: 0,
      citationCount: 0,
      recognitionScore: 0,
      accumulatingSince: '2024-01-01T00:00:00Z',
      lastRecognitionAt: '2024-01-01T00:00:00Z',
      stewardId: null,
      stewardshipStartedAt: null,
      stewardshipQualityScore: null,
      claimInitiatedAt: null,
      claimVerifiedAt: null,
      claimVerificationMethod: null,
      claimedAgentId: null,
      note: null,
      image: null,
      createdAt: '2024-01-01T00:00:00Z',
      updatedAt: '2024-01-01T00:00:00Z',
    },
    {
      id: 'presence-2',
      displayName: 'Stewarded User',
      presenceState: 'stewarded',
      externalIdentifiers: [],
      establishingContentIds: [],
      establishedAt: '2024-01-02T00:00:00Z',
      affinityTotal: 0,
      uniqueEngagers: 0,
      citationCount: 0,
      recognitionScore: 0,
      accumulatingSince: '2024-01-02T00:00:00Z',
      lastRecognitionAt: '2024-01-02T00:00:00Z',
      stewardId: 'steward-123',
      stewardshipStartedAt: '2024-01-02T00:00:00Z',
      stewardshipQualityScore: 95,
      claimInitiatedAt: null,
      claimVerifiedAt: null,
      claimVerificationMethod: null,
      claimedAgentId: null,
      note: null,
      image: null,
      createdAt: '2024-01-02T00:00:00Z',
      updatedAt: '2024-01-02T00:00:00Z',
    },
  ];

  beforeEach(async () => {
    mockPresenceService = jasmine.createSpyObj(
      'PresenceService',
      [
        'getPresencesByState',
        'getMyStewardedPresences',
        'createPresence',
        'beginStewardship',
      ],
      {
        isLoading: signal(false),
        myStewardedPresences: signal([]),
      }
    );

    mockIdentityService = jasmine.createSpyObj(
      'IdentityService',
      [],
      {
        isAuthenticated: signal(true),
        agentPubKey: signal('agent-123'),
      }
    );

    mockPresenceService.getPresencesByState.and.returnValue(Promise.resolve([]));
    mockPresenceService.getMyStewardedPresences.and.returnValue(Promise.resolve([]));

    await TestBed.configureTestingModule({
      imports: [PresenceListComponent],
      providers: [
        { provide: PresenceService, useValue: mockPresenceService },
        { provide: IdentityService, useValue: mockIdentityService },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(PresenceListComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  describe('Filtering', () => {
    it('should change filter state', () => {
      component.setFilter('unclaimed');

      expect(component.filter()).toBe('unclaimed');
    });
  });

  describe('Create Form', () => {
    it('should open create form', () => {
      component.openCreateForm();

      expect(component.showCreateForm()).toBe(true);
    });

    it('should close create form', () => {
      component.showCreateForm.set(true);

      component.closeCreateForm();

      expect(component.showCreateForm()).toBe(false);
    });

    it('should require display name', async () => {
      component.createForm.displayName = '';

      await component.createPresence();

      expect(component.error()).toContain('Display name is required');
    });
  });

  describe('Formatters', () => {
    it('should format date', () => {
      const formatted = component.formatDate('2024-06-15T12:00:00Z');

      expect(formatted).toContain('2024');
    });

    it('should handle invalid date', () => {
      const formatted = component.formatDate('invalid');

      expect(formatted).toBe('Invalid Date');
    });
  });

  describe('Message Clearing', () => {
    it('should clear error and success messages', () => {
      component.error.set('Error');
      component.successMessage.set('Success');

      component.clearMessages();

      expect(component.error()).toBeNull();
      expect(component.successMessage()).toBeNull();
    });
  });
});
