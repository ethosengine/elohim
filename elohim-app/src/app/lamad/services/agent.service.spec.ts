import { TestBed } from '@angular/core/testing';
import { of } from 'rxjs';
import { AgentService } from './agent.service';
import { DataLoaderService } from './data-loader.service';
import { SessionUserService } from './session-user.service';
import { Agent } from '../models/agent.model';

describe('AgentService', () => {
  let service: AgentService;
  let dataLoaderMock: jasmine.SpyObj<DataLoaderService>;
  let sessionUserMock: jasmine.SpyObj<SessionUserService>;

  const mockAgents: Agent[] = [
    {
      id: 'agent-1',
      name: 'Test User',
      type: 'human',
      createdAt: '2025-01-01T00:00:00Z'
    },
    {
      id: 'agent-2',
      name: 'Test Organization',
      type: 'organization',
      createdAt: '2025-01-01T00:00:00Z'
    }
  ];

  beforeEach(() => {
    dataLoaderMock = jasmine.createSpyObj('DataLoaderService', ['getAgents', 'getAttestations']);
    sessionUserMock = jasmine.createSpyObj('SessionUserService', ['getCurrentSession', 'getSessionId']);

    dataLoaderMock.getAgents.and.returnValue(of(mockAgents));
    dataLoaderMock.getAttestations.and.returnValue(of([]));
    sessionUserMock.getCurrentSession.and.returnValue({
      sessionId: 'session-123',
      displayName: 'Test',
      isAnonymous: true,
      accessLevel: 'visitor'
    });
    sessionUserMock.getSessionId.and.returnValue('session-123');

    TestBed.configureTestingModule({
      providers: [
        AgentService,
        { provide: DataLoaderService, useValue: dataLoaderMock },
        { provide: SessionUserService, useValue: sessionUserMock }
      ]
    });
    service = TestBed.inject(AgentService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('getAgents', () => {
    it('should return all agents', (done) => {
      service.getAgents().subscribe(agents => {
        expect(agents.length).toBe(2);
        done();
      });
    });
  });

  describe('getAgent', () => {
    it('should return agent by ID', (done) => {
      service.getAgent('agent-1').subscribe(agent => {
        expect(agent).toBeTruthy();
        expect(agent?.id).toBe('agent-1');
        expect(agent?.name).toBe('Test User');
        done();
      });
    });

    it('should return undefined for nonexistent agent', (done) => {
      service.getAgent('nonexistent').subscribe(agent => {
        expect(agent).toBeUndefined();
        done();
      });
    });
  });

  describe('getAgentsByType', () => {
    it('should filter agents by type', (done) => {
      service.getAgentsByType('human').subscribe(agents => {
        expect(agents.length).toBe(1);
        expect(agents[0].type).toBe('human');
        done();
      });
    });

    it('should return empty array for type with no agents', (done) => {
      service.getAgentsByType('ai-agent').subscribe(agents => {
        expect(agents.length).toBe(0);
        done();
      });
    });
  });

  describe('getCurrentAgent', () => {
    it('should return current session agent', (done) => {
      service.getCurrentAgent().subscribe(agent => {
        expect(agent).toBeDefined();
        done();
      });
    });
  });

  describe('getAgentReach', () => {
    it('should return agent reach level', (done) => {
      service.getAgentReach('agent-1').subscribe(reach => {
        expect(reach).toBeDefined();
        done();
      });
    });
  });

  describe('isAgentType', () => {
    it('should check agent type', (done) => {
      service.isAgentType('agent-1', 'human').subscribe(result => {
        expect(result).toBe(true);
        done();
      });
    });

    it('should return false for wrong type', (done) => {
      service.isAgentType('agent-1', 'organization').subscribe(result => {
        expect(result).toBe(false);
        done();
      });
    });
  });
});
