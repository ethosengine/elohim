import { HttpClientTestingModule, HttpTestingController } from '@angular/common/http/testing';
import { TestBed, fakeAsync, tick } from '@angular/core/testing';

import {
  CommunityDirectoryComponent,
  type HumanEntry,
  type CollectiveEntry,
  type ParticipantEntry,
} from './community-directory.component';

const MOCK_HUMANS: HumanEntry[] = [
  {
    id: 'human-matthew',
    agentPubKey: null,
    displayName: 'Matthew Dowell',
    bio: null,
    affinities: [],
    profileReach: 'local',
    location: null,
    profilePhotoUrl: null,
    appId: 'lamad',
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
  },
  {
    id: 'human-jessica',
    agentPubKey: null,
    displayName: 'Jessica Dowell',
    bio: null,
    affinities: [],
    profileReach: 'local',
    location: null,
    profilePhotoUrl: null,
    appId: 'lamad',
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
  },
  {
    id: 'human-frank',
    agentPubKey: null,
    displayName: 'Frank',
    bio: null,
    affinities: [],
    profileReach: 'local',
    location: null,
    profilePhotoUrl: null,
    appId: 'lamad',
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
  },
];

const MOCK_HOUSEHOLD: CollectiveEntry = {
  id: 'household-dowell',
  name: 'Dowell Household',
  description: null,
  governanceLayer: 'family',
  constitutionalParentId: null,
  reach: 'local',
  metadata: null,
  createdBy: null,
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
  dissolvedAt: null,
};

const MOCK_GROUP: CollectiveEntry = {
  id: 'group-study',
  name: 'Bible Study',
  description: 'Weekly study group',
  governanceLayer: 'faith',
  constitutionalParentId: null,
  reach: 'local',
  metadata: null,
  createdBy: null,
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
  dissolvedAt: null,
};

const MOCK_PARTICIPANTS_HOUSEHOLD: ParticipantEntry[] = [
  {
    id: 'part-1',
    collectiveId: 'household-dowell',
    humanId: 'human-matthew',
    intimacyLevel: 'intimate',
    roleContext: 'leader',
    governanceWeight: 1,
    consentState: 'consented',
    metadata: null,
    joinedAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
    departedAt: null,
  },
  {
    id: 'part-2',
    collectiveId: 'household-dowell',
    humanId: 'human-jessica',
    intimacyLevel: 'intimate',
    roleContext: 'member',
    governanceWeight: 1,
    consentState: 'consented',
    metadata: null,
    joinedAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
    departedAt: null,
  },
];

const MOCK_PARTICIPANTS_GROUP: ParticipantEntry[] = [
  {
    id: 'part-3',
    collectiveId: 'group-study',
    humanId: 'human-matthew',
    intimacyLevel: 'connection',
    roleContext: null,
    governanceWeight: 1,
    consentState: 'consented',
    metadata: null,
    joinedAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
    departedAt: null,
  },
];

describe('CommunityDirectoryComponent', () => {
  let component: CommunityDirectoryComponent;
  let httpMock: HttpTestingController;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [CommunityDirectoryComponent, HttpClientTestingModule],
    }).compileComponents();

    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    httpMock.verify();
  });

  function createAndInit(): void {
    const fixture = TestBed.createComponent(CommunityDirectoryComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  }

  function flushInitialRequests(): void {
    // Flush humans + collectives
    const humansReq = httpMock.expectOne('/db/humans');
    humansReq.flush({ items: MOCK_HUMANS, count: MOCK_HUMANS.length });

    const collectivesReq = httpMock.expectOne('/api/v1/collectives');
    collectivesReq.flush({
      items: [MOCK_HOUSEHOLD, MOCK_GROUP],
      count: 2,
    });

    // Flush participant requests for each collective
    const householdParticipantsReq = httpMock.expectOne(
      '/api/v1/collectives/household-dowell/participants'
    );
    householdParticipantsReq.flush({
      items: MOCK_PARTICIPANTS_HOUSEHOLD,
      count: MOCK_PARTICIPANTS_HOUSEHOLD.length,
    });

    const groupParticipantsReq = httpMock.expectOne(
      '/api/v1/collectives/group-study/participants'
    );
    groupParticipantsReq.flush({
      items: MOCK_PARTICIPANTS_GROUP,
      count: MOCK_PARTICIPANTS_GROUP.length,
    });
  }

  it('should create', () => {
    createAndInit();
    flushInitialRequests();
    expect(component).toBeTruthy();
  });

  it('should default to "all" view', () => {
    createAndInit();
    flushInitialRequests();
    expect(component.activeView()).toBe('all');
  });

  it('should start in loading state', () => {
    createAndInit();
    expect(component.loading()).toBe(true);
    flushInitialRequests();
  });

  it('should set loading to false after data loads', () => {
    createAndInit();
    flushInitialRequests();
    expect(component.loading()).toBe(false);
  });

  it('should load humans on init', () => {
    createAndInit();
    flushInitialRequests();
    expect(component.humans().length).toBe(3);
  });

  it('should separate collectives into households and groups', () => {
    createAndInit();
    flushInitialRequests();
    expect(component.households().length).toBe(1);
    expect(component.households()[0].governanceLayer).toBe('family');
    expect(component.groups().length).toBe(1);
    expect(component.groups()[0].governanceLayer).toBe('faith');
  });

  it('should load participants by collective', () => {
    createAndInit();
    flushInitialRequests();
    const map = component.participantsByCollective();
    expect(map.get('household-dowell')?.length).toBe(2);
    expect(map.get('group-study')?.length).toBe(1);
  });

  it('should compute humanHousehold map', () => {
    createAndInit();
    flushInitialRequests();
    const householdMap = component.humanHousehold();
    expect(householdMap.get('human-matthew')).toBe('Dowell Household');
    expect(householdMap.get('human-jessica')).toBe('Dowell Household');
    expect(householdMap.has('human-frank')).toBe(false);
  });

  it('should compute individualsNotInHousehold', () => {
    createAndInit();
    flushInitialRequests();
    const individuals = component.individualsNotInHousehold();
    expect(individuals.length).toBe(1);
    expect(individuals[0].id).toBe('human-frank');
  });

  it('should switch views', () => {
    createAndInit();
    flushInitialRequests();

    component.activeView.set('households');
    expect(component.activeView()).toBe('households');

    component.activeView.set('groups');
    expect(component.activeView()).toBe('groups');

    component.activeView.set('all');
    expect(component.activeView()).toBe('all');
  });

  it('should handle empty collectives response', () => {
    createAndInit();

    httpMock.expectOne('/db/humans').flush({ items: MOCK_HUMANS, count: 3 });
    httpMock.expectOne('/api/v1/collectives').flush({ items: [], count: 0 });

    expect(component.loading()).toBe(false);
    expect(component.households().length).toBe(0);
    expect(component.groups().length).toBe(0);
  });

  it('should handle HTTP error gracefully', () => {
    createAndInit();

    // forkJoin subscribes to both simultaneously. When humans errors,
    // forkJoin cancels the collectives request. We must match it to
    // satisfy verify(), but it's already cancelled so we can't flush it.
    const collectivesReq = httpMock.expectOne('/api/v1/collectives');

    httpMock
      .expectOne('/db/humans')
      .error(new ProgressEvent('error'), { status: 500, statusText: 'Internal Server Error' });

    // collectivesReq is already cancelled by forkJoin teardown — just verify
    expect(collectivesReq.cancelled).toBe(true);
    expect(component.loading()).toBe(false);
  });

  it('should build humanNameMap from humans signal', () => {
    createAndInit();
    flushInitialRequests();
    const nameMap = component.humanNameMap();
    expect(nameMap.get('human-matthew')).toBe('Matthew Dowell');
    expect(nameMap.get('human-jessica')).toBe('Jessica Dowell');
  });
});
