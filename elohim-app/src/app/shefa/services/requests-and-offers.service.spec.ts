/**
 * Requests-and-offers Service Tests
 */

import { TestBed, fakeAsync, tick } from '@angular/core/testing';

import { RequestsAndOffersService } from './requests-and-offers.service';
import { EconomicService } from './economic.service';
import { of } from 'rxjs';

describe('RequestsAndOffersService', () => {
  let service: RequestsAndOffersService;
  let mockEconomic: jasmine.SpyObj<EconomicService>;

  beforeEach(() => {
    mockEconomic = jasmine.createSpyObj('EconomicService', [
      'createEvent',
      'getEventsForAgent',
    ]);
    mockEconomic.createEvent.and.returnValue(of({ id: 'event-123', hasPointInTime: new Date().toISOString() } as any));
    mockEconomic.getEventsForAgent.and.returnValue(of([]));

    TestBed.configureTestingModule({
      providers: [
        RequestsAndOffersService,
        { provide: EconomicService, useValue: mockEconomic },
      ],
    });
    service = TestBed.inject(RequestsAndOffersService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should have service instance', () => {
    expect(service).toBeDefined();
  });

  describe('createRequest', () => {
    it('should have createRequest method', () => {
      expect(service.createRequest).toBeDefined();
      expect(typeof service.createRequest).toBe('function');
    });

    it('should throw error when title is empty', fakeAsync(async () => {
      const result = service.createRequest('user-1', {
        title: '',
        description: 'Test description for request',
        serviceTypeIds: ['service-1'],
        mediumOfExchangeIds: ['exchange-1'],
        requesterId: 'user-1',
        status: 'active',
        links: [],
        action: 'take',
        interactionType: 'virtual',
        contactPreference: 'email',
        contactValue: 'user@example.com',
        dateRange: { flexibleDates: true },
        timeZone: 'UTC',
        timePreference: 'flexible',
        requiredSkills: [],
        isPublic: true
      } as any);

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
      expect(thrownError.message).toContain('required');
    }));

    it('should throw error when description is too short', fakeAsync(async () => {
      const result = service.createRequest('user-1', {
        title: 'Request Title',
        description: 'Short',
        serviceTypeIds: ['service-1'],
        mediumOfExchangeIds: ['exchange-1'],
        requesterId: 'user-1',
        status: 'active',
        links: [],
        action: 'take',
        interactionType: 'virtual',
        contactPreference: 'email',
        contactValue: 'user@example.com',
        dateRange: { flexibleDates: true },
        timeZone: 'UTC',
        timePreference: 'flexible',
        requiredSkills: [],
        isPublic: true
      } as any);

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
      expect(thrownError.message).toContain('20 characters');
    }));

    it('should throw error when serviceTypeIds is empty', fakeAsync(async () => {
      const result = service.createRequest('user-1', {
        title: 'Request Title',
        description: 'Test description for request',
        serviceTypeIds: [],
        mediumOfExchangeIds: ['exchange-1'],
        requesterId: 'user-1',
        status: 'active',
        links: [],
        action: 'take',
        interactionType: 'virtual',
        contactPreference: 'email',
        contactValue: 'user@example.com',
        dateRange: { flexibleDates: true },
        timeZone: 'UTC',
        timePreference: 'flexible',
        requiredSkills: [],
        isPublic: true
      } as any);

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
      expect(thrownError.message).toContain('service type');
    }));

    it('should throw error when mediumOfExchangeIds is empty', fakeAsync(async () => {
      const result = service.createRequest('user-1', {
        title: 'Request Title',
        description: 'Test description for request',
        serviceTypeIds: ['service-1'],
        mediumOfExchangeIds: [],
        requesterId: 'user-1',
        status: 'active',
        links: [],
        action: 'take',
        interactionType: 'virtual',
        contactPreference: 'email',
        contactValue: 'user@example.com',
        dateRange: { flexibleDates: true },
        timeZone: 'UTC',
        timePreference: 'flexible',
        requiredSkills: [],
        isPublic: true
      } as any);

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
      expect(thrownError.message).toContain('payment method');
    }));
  });

  describe('updateRequest', () => {
    it('should have updateRequest method', () => {
      expect(service.updateRequest).toBeDefined();
      expect(typeof service.updateRequest).toBe('function');
    });

    it('should throw error when request not found', fakeAsync(async () => {
      const result = service.updateRequest('request-1', 'user-1', {});

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
      expect(thrownError.message).toContain('not found');
    }));
  });

  describe('archiveRequest', () => {
    it('should have archiveRequest method', () => {
      expect(service.archiveRequest).toBeDefined();
      expect(typeof service.archiveRequest).toBe('function');
    });

    it('should throw error when request not found', fakeAsync(async () => {
      const result = service.archiveRequest('request-1', 'user-1');

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
      expect(thrownError.message).toContain('not found');
    }));
  });

  describe('deleteRequest', () => {
    it('should have deleteRequest method', () => {
      expect(service.deleteRequest).toBeDefined();
      expect(typeof service.deleteRequest).toBe('function');
    });

    it('should throw error when request not found', fakeAsync(async () => {
      const result = service.deleteRequest('request-1', 'user-1');

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
      expect(thrownError.message).toContain('not found');
    }));
  });

  describe('getRequest', () => {
    it('should have getRequest method', () => {
      expect(service.getRequest).toBeDefined();
      expect(typeof service.getRequest).toBe('function');
    });

    it('should return null', fakeAsync(async () => {
      const result = await service.getRequest('request-1');
      expect(result).toBeNull();
    }));
  });

  describe('getUserRequests', () => {
    it('should have getUserRequests method', () => {
      expect(service.getUserRequests).toBeDefined();
      expect(typeof service.getUserRequests).toBe('function');
    });

    it('should return empty array', fakeAsync(async () => {
      const result = await service.getUserRequests('user-1');
      expect(result).toEqual([]);
    }));
  });

  describe('createOffer', () => {
    it('should have createOffer method', () => {
      expect(service.createOffer).toBeDefined();
      expect(typeof service.createOffer).toBe('function');
    });

    it('should throw error when title is empty', fakeAsync(async () => {
      const result = service.createOffer('user-1', {
        title: '',
        description: 'Test description for offer',
        serviceTypeIds: ['service-1'],
        mediumOfExchangeIds: ['exchange-1'],
        offererId: 'user-1',
        status: 'active',
        links: [],
        action: 'give',
        interactionType: 'virtual',
        contactPreference: 'email',
        contactValue: 'user@example.com',
        dateRange: { flexibleDates: true },
        timeZone: 'UTC',
        timePreference: 'flexible',
        offeredSkills: [],
        isPublic: true
      } as any);

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
      expect(thrownError.message).toContain('required');
    }));

    it('should throw error when description is too short', fakeAsync(async () => {
      const result = service.createOffer('user-1', {
        title: 'Offer Title',
        description: 'Short',
        serviceTypeIds: ['service-1'],
        mediumOfExchangeIds: ['exchange-1'],
        offererId: 'user-1',
        status: 'active',
        links: [],
        action: 'give',
        interactionType: 'virtual',
        contactPreference: 'email',
        contactValue: 'user@example.com',
        dateRange: { flexibleDates: true },
        timeZone: 'UTC',
        timePreference: 'flexible',
        offeredSkills: [],
        isPublic: true
      } as any);

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
      expect(thrownError.message).toContain('20 characters');
    }));
  });

  describe('updateOffer', () => {
    it('should have updateOffer method', () => {
      expect(service.updateOffer).toBeDefined();
      expect(typeof service.updateOffer).toBe('function');
    });

    it('should throw error when offer not found', fakeAsync(async () => {
      const result = service.updateOffer('offer-1', 'user-1', {});

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
      expect(thrownError.message).toContain('not found');
    }));
  });

  describe('archiveOffer', () => {
    it('should have archiveOffer method', () => {
      expect(service.archiveOffer).toBeDefined();
      expect(typeof service.archiveOffer).toBe('function');
    });

    it('should throw error when offer not found', fakeAsync(async () => {
      const result = service.archiveOffer('offer-1', 'user-1');

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
      expect(thrownError.message).toContain('not found');
    }));
  });

  describe('deleteOffer', () => {
    it('should have deleteOffer method', () => {
      expect(service.deleteOffer).toBeDefined();
      expect(typeof service.deleteOffer).toBe('function');
    });

    it('should throw error when offer not found', fakeAsync(async () => {
      const result = service.deleteOffer('offer-1', 'user-1');

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
      expect(thrownError.message).toContain('not found');
    }));
  });

  describe('getOffer', () => {
    it('should have getOffer method', () => {
      expect(service.getOffer).toBeDefined();
      expect(typeof service.getOffer).toBe('function');
    });

    it('should return null', fakeAsync(async () => {
      const result = await service.getOffer('offer-1');
      expect(result).toBeNull();
    }));
  });

  describe('getUserOffers', () => {
    it('should have getUserOffers method', () => {
      expect(service.getUserOffers).toBeDefined();
      expect(typeof service.getUserOffers).toBe('function');
    });

    it('should return empty array', fakeAsync(async () => {
      const result = await service.getUserOffers('user-1');
      expect(result).toEqual([]);
    }));
  });

  describe('searchRequests', () => {
    it('should have searchRequests method', () => {
      expect(service.searchRequests).toBeDefined();
      expect(typeof service.searchRequests).toBe('function');
    });

    it('should return paginated result', fakeAsync(async () => {
      const result = await service.searchRequests({});
      expect(result).toEqual(jasmine.objectContaining({
        requests: jasmine.any(Array),
        totalCount: 0,
        page: 1,
        pageSize: 20,
      }));
    }));
  });

  describe('searchOffers', () => {
    it('should have searchOffers method', () => {
      expect(service.searchOffers).toBeDefined();
      expect(typeof service.searchOffers).toBe('function');
    });

    it('should return paginated result', fakeAsync(async () => {
      const result = await service.searchOffers({});
      expect(result).toEqual(jasmine.objectContaining({
        offers: jasmine.any(Array),
        totalCount: 0,
        page: 1,
        pageSize: 20,
      }));
    }));
  });

  describe('getTrendingRequests', () => {
    it('should have getTrendingRequests method', () => {
      expect(service.getTrendingRequests).toBeDefined();
      expect(typeof service.getTrendingRequests).toBe('function');
    });

    it('should return empty array', fakeAsync(async () => {
      const result = await service.getTrendingRequests();
      expect(result).toEqual([]);
    }));
  });

  describe('getTrendingOffers', () => {
    it('should have getTrendingOffers method', () => {
      expect(service.getTrendingOffers).toBeDefined();
      expect(typeof service.getTrendingOffers).toBe('function');
    });

    it('should return empty array', fakeAsync(async () => {
      const result = await service.getTrendingOffers();
      expect(result).toEqual([]);
    }));
  });

  describe('findMatchesForRequest', () => {
    it('should have findMatchesForRequest method', () => {
      expect(service.findMatchesForRequest).toBeDefined();
      expect(typeof service.findMatchesForRequest).toBe('function');
    });

    it('should return empty array', fakeAsync(async () => {
      const result = await service.findMatchesForRequest('request-1');
      expect(result).toEqual([]);
    }));
  });

  describe('findMatchesForOffer', () => {
    it('should have findMatchesForOffer method', () => {
      expect(service.findMatchesForOffer).toBeDefined();
      expect(typeof service.findMatchesForOffer).toBe('function');
    });

    it('should return empty array', fakeAsync(async () => {
      const result = await service.findMatchesForOffer('offer-1');
      expect(result).toEqual([]);
    }));
  });

  describe('createMatch', () => {
    it('should have createMatch method', () => {
      expect(service.createMatch).toBeDefined();
      expect(typeof service.createMatch).toBe('function');
    });

    it('should reject with not implemented error', fakeAsync(async () => {
      const result = service.createMatch('request-1', 'offer-1', 'Good match');

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
    }));
  });

  describe('getMatch', () => {
    it('should have getMatch method', () => {
      expect(service.getMatch).toBeDefined();
      expect(typeof service.getMatch).toBe('function');
    });

    it('should reject with not implemented error', fakeAsync(async () => {
      const result = service.getMatch('match-1');

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
    }));
  });

  describe('updateMatchStatus', () => {
    it('should have updateMatchStatus method', () => {
      expect(service.updateMatchStatus).toBeDefined();
      expect(typeof service.updateMatchStatus).toBe('function');
    });

    it('should reject with not implemented error', fakeAsync(async () => {
      const result = service.updateMatchStatus('match-1', 'contacted');

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
    }));
  });

  describe('proposeOfferToRequest', () => {
    it('should have proposeOfferToRequest method', () => {
      expect(service.proposeOfferToRequest).toBeDefined();
      expect(typeof service.proposeOfferToRequest).toBe('function');
    });

    it('should reject with not implemented error', fakeAsync(async () => {
      const result = service.proposeOfferToRequest('offer-1', 'request-1');

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
    }));
  });

  describe('proposeRequestToOffer', () => {
    it('should have proposeRequestToOffer method', () => {
      expect(service.proposeRequestToOffer).toBeDefined();
      expect(typeof service.proposeRequestToOffer).toBe('function');
    });

    it('should reject with not implemented error', fakeAsync(async () => {
      const result = service.proposeRequestToOffer('request-1', 'offer-1');

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
    }));
  });

  describe('acceptProposal', () => {
    it('should have acceptProposal method', () => {
      expect(service.acceptProposal).toBeDefined();
      expect(typeof service.acceptProposal).toBe('function');
    });

    it('should reject with not implemented error', fakeAsync(async () => {
      const result = service.acceptProposal('proposal-1', 'user-1');

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
    }));
  });

  describe('rejectProposal', () => {
    it('should have rejectProposal method', () => {
      expect(service.rejectProposal).toBeDefined();
      expect(typeof service.rejectProposal).toBe('function');
    });

    it('should reject with not implemented error', fakeAsync(async () => {
      const result = service.rejectProposal('proposal-1', 'user-1');

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
    }));
  });

  describe('markWorkComplete', () => {
    it('should have markWorkComplete method', () => {
      expect(service.markWorkComplete).toBeDefined();
      expect(typeof service.markWorkComplete).toBe('function');
    });

    it('should reject with not implemented error', fakeAsync(async () => {
      const result = service.markWorkComplete('commitment-1', 'user-1');

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
    }));
  });

  describe('settlePayment', () => {
    it('should have settlePayment method', () => {
      expect(service.settlePayment).toBeDefined();
      expect(typeof service.settlePayment).toBe('function');
    });

    it('should reject with not implemented error', fakeAsync(async () => {
      const result = service.settlePayment('match-1', {
        amount: { hasNumericalValue: 100, hasUnit: 'token' },
        mediumOfExchangeId: 'exchange-1',
        paymentMethod: 'mutual-credit',
      });

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
    }));
  });

  describe('setUserPreferences', () => {
    it('should have setUserPreferences method', () => {
      expect(service.setUserPreferences).toBeDefined();
      expect(typeof service.setUserPreferences).toBe('function');
    });

    it('should reject with not implemented error', fakeAsync(async () => {
      const result = service.setUserPreferences('user-1', {
        contactPreference: 'email',
        contactValue: 'user@example.com',
        timeZone: 'UTC',
        timePreference: 'flexible',
        interactionType: 'virtual',
        languages: [],
        skillsToLearn: [],
        skillsToShare: []
      } as any);

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
    }));
  });

  describe('getUserPreferences', () => {
    it('should have getUserPreferences method', () => {
      expect(service.getUserPreferences).toBeDefined();
      expect(typeof service.getUserPreferences).toBe('function');
    });

    it('should reject with not implemented error', fakeAsync(async () => {
      const result = service.getUserPreferences('user-1');

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
    }));
  });

  describe('getRecommendedRequests', () => {
    it('should have getRecommendedRequests method', () => {
      expect(service.getRecommendedRequests).toBeDefined();
      expect(typeof service.getRecommendedRequests).toBe('function');
    });

    it('should reject with not implemented error', fakeAsync(async () => {
      const result = service.getRecommendedRequests('user-1');

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
    }));
  });

  describe('getRecommendedOffers', () => {
    it('should have getRecommendedOffers method', () => {
      expect(service.getRecommendedOffers).toBeDefined();
      expect(typeof service.getRecommendedOffers).toBe('function');
    });

    it('should reject with not implemented error', fakeAsync(async () => {
      const result = service.getRecommendedOffers('user-1');

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
    }));
  });

  describe('saveRequest', () => {
    it('should have saveRequest method', () => {
      expect(service.saveRequest).toBeDefined();
      expect(typeof service.saveRequest).toBe('function');
    });

    it('should reject with not implemented error', fakeAsync(async () => {
      const result = service.saveRequest('user-1', 'request-1');

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
    }));
  });

  describe('saveOffer', () => {
    it('should have saveOffer method', () => {
      expect(service.saveOffer).toBeDefined();
      expect(typeof service.saveOffer).toBe('function');
    });

    it('should reject with not implemented error', fakeAsync(async () => {
      const result = service.saveOffer('user-1', 'offer-1');

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
    }));
  });

  describe('getSavedRequests', () => {
    it('should have getSavedRequests method', () => {
      expect(service.getSavedRequests).toBeDefined();
      expect(typeof service.getSavedRequests).toBe('function');
    });

    it('should reject with not implemented error', fakeAsync(async () => {
      const result = service.getSavedRequests('user-1');

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
    }));
  });

  describe('getSavedOffers', () => {
    it('should have getSavedOffers method', () => {
      expect(service.getSavedOffers).toBeDefined();
      expect(typeof service.getSavedOffers).toBe('function');
    });

    it('should reject with not implemented error', fakeAsync(async () => {
      const result = service.getSavedOffers('user-1');

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
    }));
  });

  describe('unsaveRequest', () => {
    it('should have unsaveRequest method', () => {
      expect(service.unsaveRequest).toBeDefined();
      expect(typeof service.unsaveRequest).toBe('function');
    });

    it('should reject with not implemented error', fakeAsync(async () => {
      const result = service.unsaveRequest('user-1', 'saved-1');

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
    }));
  });

  describe('unsaveOffer', () => {
    it('should have unsaveOffer method', () => {
      expect(service.unsaveOffer).toBeDefined();
      expect(typeof service.unsaveOffer).toBe('function');
    });

    it('should reject with not implemented error', fakeAsync(async () => {
      const result = service.unsaveOffer('user-1', 'saved-1');

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
    }));
  });

  describe('createServiceType', () => {
    it('should have createServiceType method', () => {
      expect(service.createServiceType).toBeDefined();
      expect(typeof service.createServiceType).toBe('function');
    });

    it('should reject with not implemented error', fakeAsync(async () => {
      const result = service.createServiceType('user-1', {
        name: 'Test Service',
        description: 'A test service type',
        isTechnical: false,
        creatorId: 'user-1',
        isAuthorOnly: false
      } as any);

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
    }));
  });

  describe('updateServiceType', () => {
    it('should have updateServiceType method', () => {
      expect(service.updateServiceType).toBeDefined();
      expect(typeof service.updateServiceType).toBe('function');
    });

    it('should reject with not implemented error', fakeAsync(async () => {
      const result = service.updateServiceType('type-1', 'user-1', {});

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
    }));
  });

  describe('getServiceTypes', () => {
    it('should have getServiceTypes method', () => {
      expect(service.getServiceTypes).toBeDefined();
      expect(typeof service.getServiceTypes).toBe('function');
    });

    it('should reject with not implemented error', fakeAsync(async () => {
      const result = service.getServiceTypes();

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
    }));
  });

  describe('getServiceType', () => {
    it('should have getServiceType method', () => {
      expect(service.getServiceType).toBeDefined();
      expect(typeof service.getServiceType).toBe('function');
    });

    it('should reject with not implemented error', fakeAsync(async () => {
      const result = service.getServiceType('type-1');

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
    }));
  });

  describe('createMediumOfExchange', () => {
    it('should have createMediumOfExchange method', () => {
      expect(service.createMediumOfExchange).toBeDefined();
      expect(typeof service.createMediumOfExchange).toBe('function');
    });

    it('should reject with not implemented error', fakeAsync(async () => {
      const result = service.createMediumOfExchange('user-1', {
        code: 'TEST',
        name: 'Test Currency',
        creatorId: 'user-1',
        isAuthorOnly: false,
        exchangeType: 'currency'
      } as any);

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
    }));
  });

  describe('updateMediumOfExchange', () => {
    it('should have updateMediumOfExchange method', () => {
      expect(service.updateMediumOfExchange).toBeDefined();
      expect(typeof service.updateMediumOfExchange).toBe('function');
    });

    it('should reject with not implemented error', fakeAsync(async () => {
      const result = service.updateMediumOfExchange('medium-1', 'user-1', {});

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
    }));
  });

  describe('getMediumsOfExchange', () => {
    it('should have getMediumsOfExchange method', () => {
      expect(service.getMediumsOfExchange).toBeDefined();
      expect(typeof service.getMediumsOfExchange).toBe('function');
    });

    it('should reject with not implemented error', fakeAsync(async () => {
      const result = service.getMediumsOfExchange();

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
    }));
  });

  describe('getMediumOfExchange', () => {
    it('should have getMediumOfExchange method', () => {
      expect(service.getMediumOfExchange).toBeDefined();
      expect(typeof service.getMediumOfExchange).toBe('function');
    });

    it('should reject with not implemented error', fakeAsync(async () => {
      const result = service.getMediumOfExchange('medium-1');

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
    }));
  });

  describe('getPendingRequests', () => {
    it('should have getPendingRequests method', () => {
      expect(service.getPendingRequests).toBeDefined();
      expect(typeof service.getPendingRequests).toBe('function');
    });

    it('should reject with not implemented error', fakeAsync(async () => {
      const result = service.getPendingRequests();

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
    }));
  });

  describe('getPendingOffers', () => {
    it('should have getPendingOffers method', () => {
      expect(service.getPendingOffers).toBeDefined();
      expect(typeof service.getPendingOffers).toBe('function');
    });

    it('should reject with not implemented error', fakeAsync(async () => {
      const result = service.getPendingOffers();

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
    }));
  });

  describe('approveRequest', () => {
    it('should have approveRequest method', () => {
      expect(service.approveRequest).toBeDefined();
      expect(typeof service.approveRequest).toBe('function');
    });

    it('should reject with not implemented error', fakeAsync(async () => {
      const result = service.approveRequest('request-1', 'admin-1');

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
    }));
  });

  describe('approveOffer', () => {
    it('should have approveOffer method', () => {
      expect(service.approveOffer).toBeDefined();
      expect(typeof service.approveOffer).toBe('function');
    });

    it('should reject with not implemented error', fakeAsync(async () => {
      const result = service.approveOffer('offer-1', 'admin-1');

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
    }));
  });

  describe('rejectRequest', () => {
    it('should have rejectRequest method', () => {
      expect(service.rejectRequest).toBeDefined();
      expect(typeof service.rejectRequest).toBe('function');
    });

    it('should reject with not implemented error', fakeAsync(async () => {
      const result = service.rejectRequest('request-1', 'admin-1', 'Not appropriate');

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
    }));
  });

  describe('rejectOffer', () => {
    it('should have rejectOffer method', () => {
      expect(service.rejectOffer).toBeDefined();
      expect(typeof service.rejectOffer).toBe('function');
    });

    it('should reject with not implemented error', fakeAsync(async () => {
      const result = service.rejectOffer('offer-1', 'admin-1', 'Not appropriate');

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
    }));
  });

  describe('suspendRequest', () => {
    it('should have suspendRequest method', () => {
      expect(service.suspendRequest).toBeDefined();
      expect(typeof service.suspendRequest).toBe('function');
    });

    it('should reject with not implemented error', fakeAsync(async () => {
      const result = service.suspendRequest('request-1', 'admin-1', 'Violation');

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
    }));
  });

  describe('suspendOffer', () => {
    it('should have suspendOffer method', () => {
      expect(service.suspendOffer).toBeDefined();
      expect(typeof service.suspendOffer).toBe('function');
    });

    it('should reject with not implemented error', fakeAsync(async () => {
      const result = service.suspendOffer('offer-1', 'admin-1', 'Violation');

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
    }));
  });

  describe('getActivityStats', () => {
    it('should have getActivityStats method', () => {
      expect(service.getActivityStats).toBeDefined();
      expect(typeof service.getActivityStats).toBe('function');
    });

    it('should reject with not implemented error', fakeAsync(async () => {
      const result = service.getActivityStats('7-days');

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
    }));
  });

  describe('getUserActivitySummary', () => {
    it('should have getUserActivitySummary method', () => {
      expect(service.getUserActivitySummary).toBeDefined();
      expect(typeof service.getUserActivitySummary).toBe('function');
    });

    it('should reject with not implemented error', fakeAsync(async () => {
      const result = service.getUserActivitySummary('user-1');

      let thrownError: any;
      result.catch((err) => {
        thrownError = err;
      });

      tick();
      expect(thrownError).toBeDefined();
    }));
  });
});
