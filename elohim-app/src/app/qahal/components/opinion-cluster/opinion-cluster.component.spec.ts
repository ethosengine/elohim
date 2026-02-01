import { ComponentFixture, TestBed, fakeAsync, tick } from '@angular/core/testing';
import { SimpleChange, SimpleChanges } from '@angular/core';

import { of } from 'rxjs';

import { GovernanceSignalService } from '@app/elohim/services/governance-signal.service';

import { OpinionClusterComponent, Statement, StatementVote } from './opinion-cluster.component';

// OpinionCluster matches the service interface
interface MockOpinionCluster {
  id: string;
  label: string;
  color: string;
  centroid: [number, number];
  memberCount: number;
  averagePosition: number;
}

describe('OpinionClusterComponent', () => {
  let component: OpinionClusterComponent;
  let fixture: ComponentFixture<OpinionClusterComponent>;
  let mockSignalService: jasmine.SpyObj<GovernanceSignalService>;

  // Mock clusters matching the service interface
  const mockClusters: MockOpinionCluster[] = [
    {
      id: 'progressive',
      label: 'Progressive',
      color: '#3498db',
      centroid: [0.5, 0.5],
      memberCount: 10,
      averagePosition: 0.8,
    },
    {
      id: 'conservative',
      label: 'Traditionalist',
      color: '#9b59b6',
      centroid: [-0.5, 0.5],
      memberCount: 8,
      averagePosition: 0.7,
    },
  ];

  beforeEach(async () => {
    mockSignalService = jasmine.createSpyObj('GovernanceSignalService', ['computeOpinionClusters']);
    mockSignalService.computeOpinionClusters.and.returnValue(of(mockClusters));

    await TestBed.configureTestingModule({
      imports: [OpinionClusterComponent],
      providers: [{ provide: GovernanceSignalService, useValue: mockSignalService }],
    }).compileComponents();

    fixture = TestBed.createComponent(OpinionClusterComponent);
    component = fixture.componentInstance;
    component.contextId = 'test-context';
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  describe('initialization', () => {
    it('should have default values', () => {
      expect(component.showLabels).toBeTrue();
      expect(component.highlightConsensus).toBeTrue();
      expect(component.interactive).toBeTrue();
      expect(component.statements).toEqual([]);
      expect(component.votes).toEqual([]);
    });

    it('should load cluster data on init', () => {
      fixture.detectChanges();

      expect(mockSignalService.computeOpinionClusters).toHaveBeenCalledWith('test-context');
    });

    it('should update cluster count after loading', fakeAsync(() => {
      fixture.detectChanges();
      tick();

      expect(component.clusterCount).toBe(2);
    }));
  });

  describe('ngOnChanges', () => {
    it('should recalculate clusters when votes change', () => {
      spyOn<any>(component, 'recalculateClusters');

      const changes: SimpleChanges = {
        votes: new SimpleChange([], [{ participantId: 'p1', statementId: 's1', value: 1 }], false),
      };
      component.ngOnChanges(changes);

      expect((component as any).recalculateClusters).toHaveBeenCalled();
    });

    it('should recalculate clusters when statements change', () => {
      spyOn<any>(component, 'recalculateClusters');

      const changes: SimpleChanges = {
        statements: new SimpleChange([], [{ id: 's1', text: 'Statement 1' }], false),
      };
      component.ngOnChanges(changes);

      expect((component as any).recalculateClusters).toHaveBeenCalled();
    });
  });

  describe('computeParticipantPositions()', () => {
    it('should return empty array when no statements', () => {
      component.statements = [];
      component.votes = [{ participantId: 'p1', statementId: 's1', value: 1 }];

      const positions = (component as any).computeParticipantPositions();

      expect(positions).toEqual([]);
    });

    it('should return empty array when no votes', () => {
      component.statements = [{ id: 's1', text: 'Test statement' }];
      component.votes = [];

      const positions = (component as any).computeParticipantPositions();

      expect(positions).toEqual([]);
    });

    it('should compute positions for participants with votes', () => {
      component.statements = [
        { id: 's1', text: 'Statement 1' },
        { id: 's2', text: 'Statement 2' },
        { id: 's3', text: 'Statement 3' },
        { id: 's4', text: 'Statement 4' },
      ];
      component.votes = [
        { participantId: 'p1', statementId: 's1', value: 1 },
        { participantId: 'p1', statementId: 's2', value: 1 },
        { participantId: 'p2', statementId: 's1', value: -1 },
        { participantId: 'p2', statementId: 's2', value: -1 },
      ];

      const positions = (component as any).computeParticipantPositions();

      expect(positions.length).toBe(2);
      expect(positions[0].participantId).toBe('p1');
      expect(positions[1].participantId).toBe('p2');
    });

    it('should normalize positions to [-1, 1] range', () => {
      component.statements = [
        { id: 's1', text: 'Statement 1' },
        { id: 's2', text: 'Statement 2' },
      ];
      component.votes = [
        { participantId: 'p1', statementId: 's1', value: 1 },
        { participantId: 'p1', statementId: 's2', value: 1 },
      ];

      const positions = (component as any).computeParticipantPositions();

      expect(positions[0].x).toBeGreaterThanOrEqual(-1);
      expect(positions[0].x).toBeLessThanOrEqual(1);
      expect(positions[0].y).toBeGreaterThanOrEqual(-1);
      expect(positions[0].y).toBeLessThanOrEqual(1);
    });

    it('should track vote count per participant', () => {
      component.statements = [
        { id: 's1', text: 'Statement 1' },
        { id: 's2', text: 'Statement 2' },
      ];
      component.votes = [
        { participantId: 'p1', statementId: 's1', value: 1 },
        { participantId: 'p1', statementId: 's2', value: 1 },
      ];

      const positions = (component as any).computeParticipantPositions();

      expect(positions[0].voteCount).toBe(2);
    });

    it('should update totalParticipants', () => {
      component.statements = [{ id: 's1', text: 'Statement 1' }];
      component.votes = [
        { participantId: 'p1', statementId: 's1', value: 1 },
        { participantId: 'p2', statementId: 's1', value: -1 },
        { participantId: 'p3', statementId: 's1', value: 0 },
      ];

      (component as any).computeParticipantPositions();

      expect(component.totalParticipants).toBe(3);
    });
  });

  describe('computeClusters()', () => {
    it('should return empty array with less than 3 participants', () => {
      component.participants = [
        { participantId: 'p1', x: 0, y: 0, cluster: null, isCurrentUser: false, voteCount: 1 },
      ];

      const clusters = (component as any).computeClusters();

      expect(clusters).toEqual([]);
    });

    it('should create clusters for 3+ participants', () => {
      component.participants = [
        { participantId: 'p1', x: 0.5, y: 0.5, cluster: null, isCurrentUser: false, voteCount: 1 },
        { participantId: 'p2', x: 0.6, y: 0.6, cluster: null, isCurrentUser: false, voteCount: 1 },
        { participantId: 'p3', x: -0.5, y: 0.5, cluster: null, isCurrentUser: false, voteCount: 1 },
      ];

      const clusters = (component as any).computeClusters();

      expect(clusters.length).toBeGreaterThan(0);
    });

    it('should assign participants to clusters', () => {
      component.participants = [
        { participantId: 'p1', x: 0.5, y: 0.5, cluster: null, isCurrentUser: false, voteCount: 1 },
        { participantId: 'p2', x: 0.6, y: 0.6, cluster: null, isCurrentUser: false, voteCount: 1 },
        { participantId: 'p3', x: -0.5, y: 0.5, cluster: null, isCurrentUser: false, voteCount: 1 },
      ];

      (component as any).computeClusters();

      expect(component.participants.every(p => p.cluster !== null)).toBeTrue();
    });

    it('should filter out empty clusters', () => {
      component.participants = [
        { participantId: 'p1', x: 0.5, y: 0.5, cluster: null, isCurrentUser: false, voteCount: 1 },
        { participantId: 'p2', x: 0.6, y: 0.6, cluster: null, isCurrentUser: false, voteCount: 1 },
        {
          participantId: 'p3',
          x: 0.55,
          y: 0.55,
          cluster: null,
          isCurrentUser: false,
          voteCount: 1,
        },
      ];

      const clusters = (component as any).computeClusters();

      expect(clusters.every((c: MockOpinionCluster) => c.memberCount > 0)).toBeTrue();
    });
  });

  describe('identifyConsensusAndDivisive()', () => {
    it('should handle empty statements', () => {
      component.statements = [];
      component.votes = [];

      (component as any).identifyConsensusAndDivisive();

      expect(component.consensusStatements).toEqual([]);
      expect(component.divisiveStatements).toEqual([]);
    });

    it('should identify consensus statements (low variance)', () => {
      component.statements = [
        { id: 's1', text: 'Everyone agrees' },
        { id: 's2', text: 'Divisive topic' },
        { id: 's3', text: 'Another consensus' },
        { id: 's4', text: 'Another divisive' },
        { id: 's5', text: 'Neutral' },
      ];
      component.votes = [
        // Everyone agrees on s1
        { participantId: 'p1', statementId: 's1', value: 1 },
        { participantId: 'p2', statementId: 's1', value: 1 },
        { participantId: 'p3', statementId: 's1', value: 1 },
        // Mixed votes on s2
        { participantId: 'p1', statementId: 's2', value: 1 },
        { participantId: 'p2', statementId: 's2', value: -1 },
        { participantId: 'p3', statementId: 's2', value: -1 },
      ];

      (component as any).identifyConsensusAndDivisive();

      expect(component.consensusStatements.length).toBeGreaterThan(0);
    });

    it('should identify divisive statements (high variance)', () => {
      component.statements = [
        { id: 's1', text: 'Consensus statement' },
        { id: 's2', text: 'Divisive statement' },
        { id: 's3', text: 'Another statement' },
        { id: 's4', text: 'Fourth statement' },
        { id: 's5', text: 'Fifth statement' },
      ];
      component.votes = [
        // Low variance - consensus
        { participantId: 'p1', statementId: 's1', value: 1 },
        { participantId: 'p2', statementId: 's1', value: 1 },
        { participantId: 'p3', statementId: 's1', value: 1 },
        // High variance - divisive
        { participantId: 'p1', statementId: 's2', value: 1 },
        { participantId: 'p2', statementId: 's2', value: -1 },
        { participantId: 'p3', statementId: 's2', value: 0 },
      ];

      (component as any).identifyConsensusAndDivisive();

      expect(component.divisiveStatements.length).toBeGreaterThan(0);
    });

    it('should calculate consensus score', () => {
      component.statements = [{ id: 's1', text: 'Test' }];
      component.votes = [
        { participantId: 'p1', statementId: 's1', value: 1 },
        { participantId: 'p2', statementId: 's1', value: 1 },
      ];

      (component as any).identifyConsensusAndDivisive();

      expect(component.consensusScore).toBeGreaterThanOrEqual(0);
      expect(component.consensusScore).toBeLessThanOrEqual(100);
    });
  });

  describe('coordinate conversions', () => {
    // The component uses readonly padding = 40

    it('should convert data coordinates to canvas coordinates', () => {
      const result = (component as any).toCanvasCoords(0, 0, 400, 400);

      // Center should be at middle of canvas
      expect(result.x).toBe(200);
      expect(result.y).toBe(200);
    });

    it('should convert corner coordinates correctly', () => {
      // Top-left data (-1, 1) should be at top-left canvas
      // Component uses padding=40, so usable width/height is 400-80=320
      const topLeft = (component as any).toCanvasCoords(-1, 1, 400, 400);
      expect(topLeft.x).toBe(40); // padding
      expect(topLeft.y).toBe(40); // padding

      // Bottom-right data (1, -1) should be at bottom-right canvas
      const bottomRight = (component as any).toCanvasCoords(1, -1, 400, 400);
      expect(bottomRight.x).toBe(360); // width - padding
      expect(bottomRight.y).toBe(360); // height - padding
    });

    it('should convert canvas coordinates to data coordinates', () => {
      const result = (component as any).toDataCoords(200, 200, 400, 400);

      expect(result.x).toBeCloseTo(0, 1);
      expect(result.y).toBeCloseTo(0, 1);
    });
  });

  describe('event emitters', () => {
    it('should emit statement when selectStatement called', () => {
      spyOn(component.statementSelected, 'emit');
      const statement: Statement = { id: 's1', text: 'Test' };

      component.selectStatement(statement);

      expect(component.statementSelected.emit).toHaveBeenCalledWith(statement);
    });
  });

  describe('getStatementType()', () => {
    it('should return consensus for consensus statements', () => {
      const statement: Statement = { id: 's1', text: 'Test' };
      component.consensusStatements = [statement];
      component.divisiveStatements = [];

      expect(component.getStatementType(statement)).toBe('consensus');
    });

    it('should return divisive for divisive statements', () => {
      const statement: Statement = { id: 's1', text: 'Test' };
      component.consensusStatements = [];
      component.divisiveStatements = [statement];

      expect(component.getStatementType(statement)).toBe('divisive');
    });

    it('should return neutral for other statements', () => {
      const statement: Statement = { id: 's1', text: 'Test' };
      component.consensusStatements = [];
      component.divisiveStatements = [];

      expect(component.getStatementType(statement)).toBe('neutral');
    });
  });

  describe('hexToRgba()', () => {
    it('should convert hex color to rgba', () => {
      const result = (component as any).hexToRgba('#ff0000', 0.5);
      expect(result).toBe('rgba(255, 0, 0, 0.5)');
    });

    it('should handle other hex colors', () => {
      const result = (component as any).hexToRgba('#00ff00', 0.8);
      expect(result).toBe('rgba(0, 255, 0, 0.8)');
    });
  });

  describe('canvas rendering', () => {
    it('should not throw when render called without canvas', () => {
      component['ctx'] = null;
      expect(() => component.render()).not.toThrow();
    });
  });

  describe('getUserCluster methods', () => {
    beforeEach(() => {
      component.clusters = [
        {
          id: 'progressive',
          label: 'Progressive',
          color: '#3498db',
          centroid: [0.5, 0.5],
          memberCount: 10,
          averagePosition: 0.8,
        },
        {
          id: 'conservative',
          label: 'Traditionalist',
          color: '#9b59b6',
          centroid: [-0.5, 0.5],
          memberCount: 8,
          averagePosition: 0.7,
        },
      ];
    });

    it('should return undefined when no current user position', () => {
      component.currentUserPosition = null;

      expect(component.getUserCluster()).toBeUndefined();
    });

    it('should return undefined when current user has no cluster', () => {
      component.currentUserPosition = {
        participantId: 'current-user',
        x: 0,
        y: 0,
        cluster: null,
        isCurrentUser: true,
        voteCount: 5,
      };

      expect(component.getUserCluster()).toBeUndefined();
    });

    it('should return the matching cluster for current user', () => {
      component.currentUserPosition = {
        participantId: 'current-user',
        x: 0.5,
        y: 0.5,
        cluster: 'progressive',
        isCurrentUser: true,
        voteCount: 5,
      };

      const cluster = component.getUserCluster();

      expect(cluster).toBeDefined();
      expect(cluster?.id).toBe('progressive');
      expect(cluster?.label).toBe('Progressive');
    });

    it('should return transparent for getUserClusterColor when no cluster', () => {
      component.currentUserPosition = null;

      expect(component.getUserClusterColor()).toBe('transparent');
    });

    it('should return cluster color for getUserClusterColor', () => {
      component.currentUserPosition = {
        participantId: 'current-user',
        x: 0.5,
        y: 0.5,
        cluster: 'progressive',
        isCurrentUser: true,
        voteCount: 5,
      };

      expect(component.getUserClusterColor()).toBe('#3498db');
    });

    it('should return empty string for getUserClusterLabel when no cluster', () => {
      component.currentUserPosition = null;

      expect(component.getUserClusterLabel()).toBe('');
    });

    it('should return cluster label for getUserClusterLabel', () => {
      component.currentUserPosition = {
        participantId: 'current-user',
        x: 0.5,
        y: 0.5,
        cluster: 'conservative',
        isCurrentUser: true,
        voteCount: 5,
      };

      expect(component.getUserClusterLabel()).toBe('Traditionalist');
    });
  });

  describe('updateStats()', () => {
    it('should update totalParticipants from participants array', () => {
      component.participants = [
        { participantId: 'p1', x: 0, y: 0, cluster: 'a', isCurrentUser: false, voteCount: 1 },
        { participantId: 'p2', x: 0, y: 0, cluster: 'b', isCurrentUser: false, voteCount: 2 },
      ];
      component.clusters = [
        {
          id: 'a',
          label: 'A',
          color: '#fff',
          centroid: [0, 0],
          memberCount: 1,
          averagePosition: 0,
        },
        {
          id: 'b',
          label: 'B',
          color: '#fff',
          centroid: [0, 0],
          memberCount: 1,
          averagePosition: 0,
        },
      ];

      (component as any).updateStats();

      expect(component.totalParticipants).toBe(2);
      expect(component.clusterCount).toBe(2);
    });

    it('should set currentUserPosition when participant is current user', () => {
      const currentUser = {
        participantId: 'current-user',
        x: 0.3,
        y: 0.4,
        cluster: 'a',
        isCurrentUser: true,
        voteCount: 10,
      };
      component.participants = [
        { participantId: 'p1', x: 0, y: 0, cluster: 'a', isCurrentUser: false, voteCount: 1 },
        currentUser,
      ];
      component.clusters = [];

      (component as any).updateStats();

      expect(component.currentUserPosition).toEqual(currentUser);
    });

    it('should set currentUserPosition to null when no current user', () => {
      component.participants = [
        { participantId: 'p1', x: 0, y: 0, cluster: 'a', isCurrentUser: false, voteCount: 1 },
      ];
      component.clusters = [];

      (component as any).updateStats();

      expect(component.currentUserPosition).toBeNull();
    });
  });

  describe('statement handling with no votes', () => {
    it('should handle statements with no votes', () => {
      component.statements = [
        { id: 's1', text: 'Statement 1' },
        { id: 's2', text: 'Statement 2' },
      ];
      component.votes = [];

      (component as any).identifyConsensusAndDivisive();

      // Should still calculate but with default values
      expect(component.consensusScore).toBeGreaterThanOrEqual(0);
    });
  });

  describe('cluster selection', () => {
    it('should handle selectedCluster being null initially', () => {
      expect(component['selectedCluster']).toBeNull();
    });
  });

  describe('recalculateClusters()', () => {
    it('should call computeParticipantPositions, computeClusters, identifyConsensusAndDivisive, and updateStats', () => {
      spyOn<any>(component, 'computeParticipantPositions').and.returnValue([]);
      spyOn<any>(component, 'computeClusters').and.returnValue([]);
      spyOn<any>(component, 'identifyConsensusAndDivisive');
      spyOn<any>(component, 'updateStats');
      spyOn<any>(component, 'render');

      (component as any).recalculateClusters();

      expect((component as any).computeParticipantPositions).toHaveBeenCalled();
      expect((component as any).computeClusters).toHaveBeenCalled();
      expect((component as any).identifyConsensusAndDivisive).toHaveBeenCalled();
      expect((component as any).updateStats).toHaveBeenCalled();
      expect((component as any).render).toHaveBeenCalled();
    });
  });

  describe('participant position calculation edge cases', () => {
    it('should identify current user position', () => {
      component.statements = [
        { id: 's1', text: 'Statement 1' },
        { id: 's2', text: 'Statement 2' },
      ];
      component.votes = [
        { participantId: 'current-user', statementId: 's1', value: 1 },
        { participantId: 'current-user', statementId: 's2', value: 1 },
      ];

      const positions = (component as any).computeParticipantPositions();

      expect(positions[0].isCurrentUser).toBeTrue();
    });

    it('should handle participants with missing votes for some statements', () => {
      component.statements = [
        { id: 's1', text: 'Statement 1' },
        { id: 's2', text: 'Statement 2' },
        { id: 's3', text: 'Statement 3' },
      ];
      component.votes = [
        { participantId: 'p1', statementId: 's1', value: 1 },
        // p1 didn't vote on s2 or s3
      ];

      const positions = (component as any).computeParticipantPositions();

      expect(positions.length).toBe(1);
      expect(positions[0].voteCount).toBe(1);
    });
  });

  describe('cluster assignment with near-boundary participants', () => {
    it('should assign participant to nearest cluster even at boundary', () => {
      // Participant exactly at origin should go to center cluster
      component.participants = [
        { participantId: 'p1', x: 0, y: 0, cluster: null, isCurrentUser: false, voteCount: 1 },
        {
          participantId: 'p2',
          x: 0.01,
          y: 0.01,
          cluster: null,
          isCurrentUser: false,
          voteCount: 1,
        },
        {
          participantId: 'p3',
          x: -0.01,
          y: 0.01,
          cluster: null,
          isCurrentUser: false,
          voteCount: 1,
        },
      ];

      const clusters = (component as any).computeClusters();

      // All should be assigned to center cluster (closest to 0,0)
      expect(component.participants[0].cluster).toBe('center');
    });
  });

  describe('consensus/divisive identification edge cases', () => {
    it('should handle all participants passing on a statement', () => {
      component.statements = [{ id: 's1', text: 'All pass' }];
      component.votes = [
        { participantId: 'p1', statementId: 's1', value: 0 },
        { participantId: 'p2', statementId: 's1', value: 0 },
        { participantId: 'p3', statementId: 's1', value: 0 },
      ];

      (component as any).identifyConsensusAndDivisive();

      // No variance in votes should result in consensus
      expect(component.consensusScore).toBeGreaterThanOrEqual(0);
    });
  });

  describe('loadClusterData()', () => {
    it('should call signalService.computeOpinionClusters', fakeAsync(() => {
      fixture.detectChanges();
      tick();

      expect(mockSignalService.computeOpinionClusters).toHaveBeenCalledWith('test-context');
    }));

    it('should update clusterCount after service returns data', fakeAsync(() => {
      component.clusterCount = 0;
      fixture.detectChanges();
      tick();

      // After loadClusterData completes, clusterCount should be updated
      expect(component.clusterCount).toBe(2); // mockClusters has 2 items
    }));
  });

  describe('full data flow', () => {
    it('should compute positions and clusters from real votes', () => {
      // Setup full data flow
      component.statements = [
        { id: 's1', text: 'Statement 1' },
        { id: 's2', text: 'Statement 2' },
        { id: 's3', text: 'Statement 3' },
        { id: 's4', text: 'Statement 4' },
      ];
      component.votes = [
        // Participant 1 - agrees with first half
        { participantId: 'p1', statementId: 's1', value: 1 },
        { participantId: 'p1', statementId: 's2', value: 1 },
        { participantId: 'p1', statementId: 's3', value: -1 },
        { participantId: 'p1', statementId: 's4', value: -1 },
        // Participant 2 - similar to p1
        { participantId: 'p2', statementId: 's1', value: 1 },
        { participantId: 'p2', statementId: 's2', value: 1 },
        { participantId: 'p2', statementId: 's3', value: 0 },
        { participantId: 'p2', statementId: 's4', value: -1 },
        // Participant 3 - opposite
        { participantId: 'p3', statementId: 's1', value: -1 },
        { participantId: 'p3', statementId: 's2', value: -1 },
        { participantId: 'p3', statementId: 's3', value: 1 },
        { participantId: 'p3', statementId: 's4', value: 1 },
      ];

      // Trigger recalculation
      (component as any).recalculateClusters();

      // Verify results
      expect(component.participants.length).toBe(3);
      expect(component.clusters.length).toBeGreaterThan(0);
      expect(component.totalParticipants).toBe(3);
    });

    it('should handle edge case of single statement', () => {
      component.statements = [{ id: 's1', text: 'Only statement' }];
      component.votes = [
        { participantId: 'p1', statementId: 's1', value: 1 },
        { participantId: 'p2', statementId: 's1', value: 1 },
        { participantId: 'p3', statementId: 's1', value: 1 },
      ];

      (component as any).recalculateClusters();

      expect(component.participants.length).toBe(3);
    });
  });
});
