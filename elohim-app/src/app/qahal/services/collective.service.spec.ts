/**
 * CollectiveService Tests
 *
 * Tests governance context CRUD and participation management
 * via the /api/v1/collectives and /db/participations APIs.
 */

import { TestBed } from '@angular/core/testing';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { provideHttpClient } from '@angular/common/http';

import { CollectiveService } from './collective.service';

import type {
  CollectiveParticipationView,
  CollectiveView,
  CreateCollectiveInputView,
} from '@elohim/storage-client';

describe('CollectiveService', () => {
  let service: CollectiveService;
  let httpMock: HttpTestingController;

  const mockCollective: CollectiveView = {
    id: 'household-dowell',
    name: 'Dowell Household',
    description: 'Core family + workplace',
    governanceLayer: 'family',
    constitutionalParentId: null,
    reach: 'private',
    metadata: null,
    createdBy: null,
    createdAt: '2026-03-03T00:00:00Z',
    updatedAt: '2026-03-03T00:00:00Z',
    dissolvedAt: null,
  };

  const mockParticipation: CollectiveParticipationView = {
    id: 'part-001',
    collectiveId: 'household-dowell',
    humanId: 'human-matthew-manager',
    intimacyLevel: 'intimate',
    roleContext: null,
    governanceWeight: 1.0,
    consentState: 'consented',
    metadata: null,
    joinedAt: '2026-03-03T00:00:00Z',
    departedAt: null,
  };

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [CollectiveService, provideHttpClient(), provideHttpClientTesting()],
    });

    service = TestBed.inject(CollectiveService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    httpMock.verify();
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('listCollectives', () => {
    it('should GET /api/v1/collectives and return items', () => {
      service.listCollectives().subscribe(collectives => {
        expect(collectives).toHaveLength(1);
        expect(collectives[0].id).toBe('household-dowell');
        expect(collectives[0].governanceLayer).toBe('family');
      });

      const req = httpMock.expectOne(request => request.url.endsWith('/api/v1/collectives'));
      expect(req.request.method).toBe('GET');
      req.flush({ items: [mockCollective], count: 1 });
    });

    it('should pass query parameters when filtering', () => {
      service.listCollectives({ governanceLayer: 'neighborhood', activeOnly: true }).subscribe();

      const req = httpMock.expectOne(
        request =>
          request.url.endsWith('/api/v1/collectives') &&
          request.params.get('governanceLayer') === 'neighborhood' &&
          request.params.get('activeOnly') === 'true'
      );
      expect(req.request.method).toBe('GET');
      req.flush({ items: [], count: 0 });
    });
  });

  describe('getCollective', () => {
    it('should GET /api/v1/collectives/{id}', () => {
      service.getCollective('household-dowell').subscribe(collective => {
        expect(collective.id).toBe('household-dowell');
        expect(collective.name).toBe('Dowell Household');
      });

      const req = httpMock.expectOne(request =>
        request.url.endsWith('/api/v1/collectives/household-dowell')
      );
      expect(req.request.method).toBe('GET');
      req.flush(mockCollective);
    });
  });

  describe('createCollective', () => {
    it('should POST to /api/v1/collectives', () => {
      const input: CreateCollectiveInputView = {
        id: 'comm-new',
        name: 'New Community',
        description: 'A new governance context',
        governanceLayer: 'community',
        constitutionalParentId: null,
        reach: 'neighborhood',
        metadata: null,
        createdBy: 'human-matthew-manager',
      };

      service.createCollective(input).subscribe(collective => {
        expect(collective.id).toBe('comm-new');
      });

      const req = httpMock.expectOne(request => request.url.endsWith('/api/v1/collectives'));
      expect(req.request.method).toBe('POST');
      expect(req.request.body).toEqual(input);
      req.flush({ ...mockCollective, id: 'comm-new', name: 'New Community' });
    });
  });

  describe('getParticipants', () => {
    it('should GET /api/v1/collectives/{id}/participants and return items', () => {
      service.getParticipants('household-dowell').subscribe(participants => {
        expect(participants).toHaveLength(1);
        expect(participants[0].humanId).toBe('human-matthew-manager');
        expect(participants[0].intimacyLevel).toBe('intimate');
      });

      const req = httpMock.expectOne(request =>
        request.url.endsWith('/api/v1/collectives/household-dowell/participants')
      );
      expect(req.request.method).toBe('GET');
      req.flush({ items: [mockParticipation], count: 1 });
    });
  });

  describe('addParticipant', () => {
    it('should POST to /api/v1/collectives/{id}/participants', () => {
      service
        .addParticipant('household-dowell', 'human-susan-spouse', {
          intimacyLevel: 'intimate',
          roleContext: 'spouse',
        })
        .subscribe(participation => {
          expect(participation.humanId).toBe('human-susan-spouse');
        });

      const req = httpMock.expectOne(request =>
        request.url.endsWith('/api/v1/collectives/household-dowell/participants')
      );
      expect(req.request.method).toBe('POST');
      expect(req.request.body).toEqual({
        humanId: 'human-susan-spouse',
        intimacyLevel: 'intimate',
        roleContext: 'spouse',
      });
      req.flush({ ...mockParticipation, humanId: 'human-susan-spouse', roleContext: 'spouse' });
    });

    it('should send defaults when no options provided', () => {
      service.addParticipant('household-dowell', 'human-traveler').subscribe();

      const req = httpMock.expectOne(request =>
        request.url.endsWith('/api/v1/collectives/household-dowell/participants')
      );
      expect(req.request.body).toEqual({
        humanId: 'human-traveler',
        intimacyLevel: undefined,
        roleContext: undefined,
      });
      req.flush(mockParticipation);
    });
  });

  describe('departCollective', () => {
    it('should DELETE /api/v1/collectives/{id}/participants/{humanId}', () => {
      service.departCollective('household-dowell', 'human-traveler').subscribe();

      const req = httpMock.expectOne(request =>
        request.url.endsWith('/api/v1/collectives/household-dowell/participants/human-traveler')
      );
      expect(req.request.method).toBe('DELETE');
      req.flush(null);
    });
  });

  describe('getCollectivesForHuman', () => {
    it('should GET /api/v1/humans/{humanId}/collectives and return items', () => {
      service.getCollectivesForHuman('human-matthew-manager').subscribe(participations => {
        expect(participations).toHaveLength(1);
        expect(participations[0].collectiveId).toBe('household-dowell');
      });

      const req = httpMock.expectOne(request =>
        request.url.endsWith('/api/v1/humans/human-matthew-manager/collectives')
      );
      expect(req.request.method).toBe('GET');
      req.flush({ items: [mockParticipation], count: 1 });
    });
  });
});
