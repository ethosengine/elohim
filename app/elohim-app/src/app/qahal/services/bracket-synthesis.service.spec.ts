import { describe, it, expect, vi, beforeEach } from 'vitest';
import { TestBed } from '@angular/core/testing';

import { BracketSynthesisService } from './bracket-synthesis.service';
import { GovernanceApiService } from '@app/elohim/services/governance-api.service';
import type {
  SensemakingResultView,
  ProposalView,
  ProposalOptionView,
} from '@elohim/storage-client/generated';

const mockProposal: ProposalView = {
  id: 'prop-synth-1',
  contentId: 'entity-1',
  proposerPresenceId: 'system-sensemaking',
  proposalType: 'ranked-choice',
  title: 'Community synthesis: What should we do about content entity-1?',
  body: 'Ranked-choice vote on bridging statements identified through sensemaking',
  status: 'open',
  votesFor: 0,
  votesAgainst: 0,
  votingAnonymous: false,
  createdAt: '2026-03-16T00:00:00Z',
  updatedAt: '2026-03-16T00:00:00Z',
  votingMechanism: 'ranked-choice',
  scoreMin: null,
  scoreMax: null,
  dotsPerVoter: null,
  quorumPercentage: null,
  passageThreshold: null,
};

const mockGovernanceApi = {
  createProposal: vi.fn().mockResolvedValue(mockProposal),
  createProposalOptions: vi.fn().mockResolvedValue([]),
};

function makeSensemakingResult(
  bridgingStatements: SensemakingResultView['bridgingStatements'] = []
): SensemakingResultView {
  return {
    entityType: 'content',
    entityId: 'entity-1',
    clusters: [],
    bridgingStatements,
    totalParticipants: 5,
    totalStatements: 10,
  };
}

describe('BracketSynthesisService', () => {
  let service: BracketSynthesisService;

  beforeEach(() => {
    vi.clearAllMocks();

    TestBed.configureTestingModule({
      providers: [
        BracketSynthesisService,
        { provide: GovernanceApiService, useValue: mockGovernanceApi },
      ],
    });

    service = TestBed.inject(BracketSynthesisService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should create a proposal with ranked-choice mechanism', async () => {
    const result = makeSensemakingResult();

    const proposal = await service.synthesizeBracket('content', 'entity-1', result);

    expect(mockGovernanceApi.createProposal).toHaveBeenCalledOnce();
    const input = mockGovernanceApi.createProposal.mock.calls[0][0];
    expect(input.votingMechanism).toBe('ranked-choice');
    expect(input.proposalType).toBe('ranked-choice');
    expect(input.contentId).toBe('entity-1');
    expect(input.proposerPresenceId).toBe('system-sensemaking');
    expect(input.title).toContain('content');
    expect(input.title).toContain('entity-1');
    expect(proposal).toEqual(mockProposal);
  });

  it('should convert bridging statements into proposal options', async () => {
    const bridging = [
      {
        id: 'stmt-1',
        entityType: 'content',
        entityId: 'entity-1',
        humanId: 'human-1',
        text: 'We should focus on quality over speed',
        agreeCount: 8,
        disagreeCount: 2,
        passCount: 1,
        groupId: null,
        isBridging: true,
        createdAt: '2026-03-16T00:00:00Z',
      },
      {
        id: 'stmt-2',
        entityType: 'content',
        entityId: 'entity-1',
        humanId: 'human-2',
        text: 'Community review should be mandatory',
        agreeCount: 7,
        disagreeCount: 3,
        passCount: 0,
        groupId: null,
        isBridging: true,
        createdAt: '2026-03-16T00:00:00Z',
      },
    ];

    const result = makeSensemakingResult(bridging);
    const mockOptions: ProposalOptionView[] = bridging.map((s, i) => ({
      id: `synth-opt-${i}`,
      proposalId: 'prop-synth-1',
      label: s.text.substring(0, 100),
      description: s.text,
      position: i,
      source: s.id,
      sourceJustification: `Bridging statement (agree: ${s.agreeCount}, disagree: ${s.disagreeCount})`,
      createdAt: '2026-03-16T00:00:00Z',
    }));
    mockGovernanceApi.createProposalOptions.mockResolvedValueOnce(mockOptions);

    await service.synthesizeBracket('content', 'entity-1', result);

    expect(mockGovernanceApi.createProposalOptions).toHaveBeenCalledOnce();
    const [proposalId, options] = mockGovernanceApi.createProposalOptions.mock.calls[0];
    expect(proposalId).toBe('prop-synth-1');
    expect(options).toHaveLength(2);
    expect(options[0].label).toBe('We should focus on quality over speed');
    expect(options[0].description).toBe('We should focus on quality over speed');
    expect(options[0].position).toBe(0);
    expect(options[0].source).toBe('stmt-1');
    expect(options[0].sourceJustification).toContain('agree: 8');
    expect(options[0].sourceJustification).toContain('disagree: 2');
    expect(options[1].position).toBe(1);
    expect(options[1].source).toBe('stmt-2');
  });

  it('should create proposal without options when bridging statements are empty', async () => {
    const result = makeSensemakingResult([]);

    await service.synthesizeBracket('content', 'entity-1', result);

    expect(mockGovernanceApi.createProposal).toHaveBeenCalledOnce();
    expect(mockGovernanceApi.createProposalOptions).not.toHaveBeenCalled();
  });

  it('should truncate option labels to 100 characters', async () => {
    const longText =
      'This is a very long bridging statement that exceeds one hundred characters and should be truncated when used as the label for a proposal option in the ranked-choice ballot';

    const bridging = [
      {
        id: 'stmt-long',
        entityType: 'content',
        entityId: 'entity-1',
        humanId: 'human-1',
        text: longText,
        agreeCount: 5,
        disagreeCount: 1,
        passCount: 0,
        groupId: null,
        isBridging: true,
        createdAt: '2026-03-16T00:00:00Z',
      },
    ];

    const result = makeSensemakingResult(bridging);

    await service.synthesizeBracket('content', 'entity-1', result);

    const [, options] = mockGovernanceApi.createProposalOptions.mock.calls[0];
    expect(options[0].label).toHaveLength(100);
    expect(options[0].label).toBe(longText.substring(0, 100));
    // Description retains the full text
    expect(options[0].description).toBe(longText);
  });
});
