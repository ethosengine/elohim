import {
  AttestationType,
  Attestation,
  AttestationJourney,
  Endorsement,
  AttestationRequirement,
  AttestationAccessRequirement,
  UserAttestations,
  AttestationProgress
} from './attestations.model';

describe('Attestations Model', () => {
  describe('AttestationType', () => {
    it('should include all attestation types', () => {
      const types: AttestationType[] = [
        'educational',
        'skill',
        'relational',
        'civic',
        'professional',
        'emotional',
        'time-based',
        'social-proof'
      ];

      types.forEach(type => {
        const attestation: { type: AttestationType } = { type };
        expect(attestation.type).toBe(type);
      });
    });
  });

  describe('Attestation interface', () => {
    it('should create valid attestation', () => {
      const attestation: Attestation = {
        id: 'att-1',
        name: '4th Grade Math',
        description: 'Mastery of 4th grade mathematics',
        type: 'educational',
        earnedAt: '2025-01-01T00:00:00.000Z',
        revocable: false
      };

      expect(attestation.id).toBe('att-1');
      expect(attestation.name).toBe('4th Grade Math');
      expect(attestation.type).toBe('educational');
      expect(attestation.revocable).toBe(false);
    });

    it('should support optional fields', () => {
      const attestation: Attestation = {
        id: 'att-1',
        name: 'Ham Radio Technician',
        description: 'FCC Ham Radio Technician License',
        type: 'skill',
        earnedAt: '2025-01-01T00:00:00.000Z',
        expiresAt: '2030-01-01T00:00:00.000Z',
        issuedBy: 'FCC',
        revocable: true,
        metadata: { licenseNumber: 'KA1ABC' }
      };

      expect(attestation.expiresAt).toEqual('2030-01-01T00:00:00.000Z');
      expect(attestation.issuedBy).toBe('FCC');
      expect(attestation.metadata?.['licenseNumber']).toBe('KA1ABC');
    });
  });

  describe('AttestationJourney interface', () => {
    it('should create valid journey', () => {
      const journey: AttestationJourney = {
        nodesVisited: ['node-1', 'node-2', 'node-3'],
        startingAffinity: { 'node-1': 0.0, 'node-2': 0.0 },
        endingAffinity: { 'node-1': 0.8, 'node-2': 0.9 },
        startDate: '2025-01-01T00:00:00.000Z',
        endDate: '2025-02-01T00:00:00.000Z'
      };

      expect(journey.nodesVisited.length).toBe(3);
      expect(journey.startingAffinity['node-1']).toBe(0.0);
      expect(journey.endingAffinity['node-1']).toBe(0.8);
    });

    it('should support optional journey fields', () => {
      const journey: AttestationJourney = {
        nodesVisited: ['node-1'],
        startingAffinity: {},
        endingAffinity: {},
        exercisesCompleted: ['ex-1', 'ex-2'],
        applicationsCompleted: ['app-1'],
        endorsements: [],
        timeInvested: 3600000,
        startDate: '2025-01-01T00:00:00.000Z',
        endDate: '2025-01-15T00:00:00.000Z'
      };

      expect(journey.exercisesCompleted?.length).toBe(2);
      expect(journey.applicationsCompleted?.length).toBe(1);
      expect(journey.timeInvested).toBe(3600000);
    });
  });

  describe('Endorsement interface', () => {
    it('should create valid endorsement', () => {
      const endorsement: Endorsement = {
        endorserId: 'user-2',
        endorserName: 'John Doe',
        endorsedAt: '2025-01-15T00:00:00.000Z',
        reason: 'Demonstrated competence',
        weight: 1.0
      };

      expect(endorsement.endorserId).toBe('user-2');
      expect(endorsement.endorserName).toBe('John Doe');
      expect(endorsement.weight).toBe(1.0);
    });

    it('should work with minimal fields', () => {
      const endorsement: Endorsement = {
        endorserId: 'user-2',
        endorsedAt: '2025-01-15T00:00:00.000Z'
      };

      expect(endorsement.endorserId).toBe('user-2');
      expect(endorsement.endorserName).toBeUndefined();
    });
  });

  describe('AttestationRequirement interface', () => {
    it('should create comprehensive requirement', () => {
      const requirement: AttestationRequirement = {
        requiredAffinity: { 'node-1': 0.8 },
        prerequisiteAttestations: ['att-1'],
        minimumExercises: 10,
        minimumApplications: 5,
        minimumEndorsements: 3,
        minimumTimeInvested: 3600000,
        minimumDuration: 2592000000
      };

      expect(requirement.minimumExercises).toBe(10);
      expect(requirement.prerequisiteAttestations).toContain('att-1');
    });

    it('should work with partial requirements', () => {
      const requirement: AttestationRequirement = {
        requiredAffinity: { 'node-1': 0.7 }
      };

      expect(requirement.requiredAffinity?.['node-1']).toBe(0.7);
      expect(requirement.prerequisiteAttestations).toBeUndefined();
    });
  });

  describe('AttestationAccessRequirement interface', () => {
    it('should create access requirement with OR logic', () => {
      const access: AttestationAccessRequirement = {
        contentNodeId: 'advanced-content',
        requiredAttestations: ['att-1', 'att-2'],
        steward: 'steward-1',
        revocable: false
      };

      expect(access.requiredAttestations?.length).toBe(2);
      expect(access.steward).toBe('steward-1');
    });

    it('should create access requirement with AND logic', () => {
      const access: AttestationAccessRequirement = {
        contentNodeId: 'advanced-content',
        requiredAllAttestations: ['att-1', 'att-2'],
        steward: 'steward-1',
        revocable: false
      };

      expect(access.requiredAllAttestations?.length).toBe(2);
    });

    it('should support alternative endorsements', () => {
      const access: AttestationAccessRequirement = {
        contentNodeId: 'advanced-content',
        alternativeEndorsements: {
          count: 5,
          fromAttestationHolders: ['att-1']
        },
        steward: 'steward-1',
        revocable: false
      };

      expect(access.alternativeEndorsements?.count).toBe(5);
    });

    it('should include reason and explanation', () => {
      const access: AttestationAccessRequirement = {
        contentNodeId: 'sensitive-content',
        requiredAttestations: ['trauma-support'],
        steward: 'community',
        revocable: true,
        reason: 'Requires emotional maturity',
        explanation: 'This content discusses sensitive topics requiring preparation'
      };

      expect(access.reason).toBe('Requires emotional maturity');
      expect(access.explanation).toBeDefined();
    });
  });

  describe('UserAttestations interface', () => {
    it('should create user attestations collection', () => {
      const userAttestations: UserAttestations = {
        userId: 'user-1',
        attestations: [],
        lastUpdated: '2025-01-01T00:00:00.000Z'
      };

      expect(userAttestations.userId).toBe('user-1');
      expect(userAttestations.attestations.length).toBe(0);
    });

    it('should hold multiple attestations', () => {
      const attestation1: Attestation = {
        id: 'att-1',
        name: 'Attestation 1',
        description: 'First',
        type: 'educational',
        earnedAt: '2025-01-01T00:00:00.000Z',
        revocable: false
      };

      const attestation2: Attestation = {
        id: 'att-2',
        name: 'Attestation 2',
        description: 'Second',
        type: 'skill',
        earnedAt: '2025-01-01T00:00:00.000Z',
        revocable: false
      };

      const userAttestations: UserAttestations = {
        userId: 'user-1',
        attestations: [attestation1, attestation2],
        lastUpdated: '2025-01-01T00:00:00.000Z'
      };

      expect(userAttestations.attestations.length).toBe(2);
    });
  });

  describe('AttestationProgress interface', () => {
    it('should track progress toward attestation', () => {
      const progress: AttestationProgress = {
        attestationId: 'att-1',
        requirement: {
          minimumExercises: 10,
          minimumTimeInvested: 3600000
        },
        currentProgress: {
          affinityProgress: { 'node-1': 0.5 },
          exercisesCompleted: 5,
          applicationsCompleted: 2,
          endorsementsReceived: 1,
          timeInvested: 1800000,
          durationSoFar: 604800000
        },
        percentComplete: 50
      };

      expect(progress.percentComplete).toBe(50);
      expect(progress.currentProgress.exercisesCompleted).toBe(5);
    });

    it('should include estimated completion', () => {
      const progress: AttestationProgress = {
        attestationId: 'att-1',
        requirement: {},
        currentProgress: {
          affinityProgress: {},
          exercisesCompleted: 0,
          applicationsCompleted: 0,
          endorsementsReceived: 0,
          timeInvested: 0,
          durationSoFar: 0
        },
        percentComplete: 25,
        estimatedCompletion: '2025-03-01T00:00:00.000Z'
      };

      expect(progress.estimatedCompletion).toEqual('2025-03-01T00:00:00.000Z');
    });
  });
});
