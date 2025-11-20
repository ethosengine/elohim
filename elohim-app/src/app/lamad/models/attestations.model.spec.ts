import {
  AttestationType,
  Attestation,
  AttestationJourney,
  Endorsement,
  AttestationRequirement,
  ContentAccessRequirement,
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
        earnedAt: new Date('2025-01-01'),
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
        earnedAt: new Date('2025-01-01'),
        expiresAt: new Date('2030-01-01'),
        issuedBy: 'FCC',
        revocable: true,
        metadata: { licenseNumber: 'KA1ABC' }
      };

      expect(attestation.expiresAt).toEqual(new Date('2030-01-01'));
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
        startDate: new Date('2025-01-01'),
        endDate: new Date('2025-02-01')
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
        startDate: new Date('2025-01-01'),
        endDate: new Date('2025-01-15')
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
        endorsedAt: new Date('2025-01-15'),
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
        endorsedAt: new Date('2025-01-15')
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

  describe('ContentAccessRequirement interface', () => {
    it('should create access requirement with OR logic', () => {
      const access: ContentAccessRequirement = {
        contentNodeId: 'advanced-content',
        requiredAttestations: ['att-1', 'att-2'],
        steward: 'steward-1',
        revocable: false
      };

      expect(access.requiredAttestations?.length).toBe(2);
      expect(access.steward).toBe('steward-1');
    });

    it('should create access requirement with AND logic', () => {
      const access: ContentAccessRequirement = {
        contentNodeId: 'advanced-content',
        requiredAllAttestations: ['att-1', 'att-2'],
        steward: 'steward-1',
        revocable: false
      };

      expect(access.requiredAllAttestations?.length).toBe(2);
    });

    it('should support alternative endorsements', () => {
      const access: ContentAccessRequirement = {
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
      const access: ContentAccessRequirement = {
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
        lastUpdated: new Date('2025-01-01')
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
        earnedAt: new Date(),
        revocable: false
      };

      const attestation2: Attestation = {
        id: 'att-2',
        name: 'Attestation 2',
        description: 'Second',
        type: 'skill',
        earnedAt: new Date(),
        revocable: false
      };

      const userAttestations: UserAttestations = {
        userId: 'user-1',
        attestations: [attestation1, attestation2],
        lastUpdated: new Date()
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
        estimatedCompletion: new Date('2025-03-01')
      };

      expect(progress.estimatedCompletion).toEqual(new Date('2025-03-01'));
    });
  });
});
