/**
 * Type-level smoke test for coordination envelope types.
 *
 * This file is NOT a runtime test — it verifies that the types
 * compose correctly at compile time. If this file compiles,
 * the type relationships are sound.
 *
 * Excluded from test runner via naming convention (.typetest.ts).
 */

import type {
  InvocationEnvelope,
  CoordinationPayload,
  CoordinationScope,
  CoordinationVerb,
  ScopeTarget,
  InvocationRequest,
  ResponseShape,
} from './coordination-envelope.model';
import type { Agreement, Commitment, Intent } from './rea-bridge.model';
import type { DeliberationProposal } from '@app/qahal/models/governance-deliberation.model';
import type { MutualAidContext } from '@app/qahal/models/mutual-aid.model';

// -- Verify discriminated union exhaustiveness --

function _assertPayloadVerb(p: CoordinationPayload): CoordinationVerb {
  return p.verb; // This proves verb field exists on all variants
}

// -- Verify ScopeTarget discriminated union --

function _handleTarget(t: ScopeTarget): string {
  switch (t.kind) {
    case 'agent':
      return t.agentId;
    case 'layer':
      return t.layer;
    case 'affinity':
      return t.scope;
    case 'broadcast':
      return t.reach;
    case 'council':
      return t.councilId;
  }
}

// -- Verify ResponseShape discriminated union --

function _handleShape(s: ResponseShape): string {
  switch (s.kind) {
    case 'categorical':
      return s.options[0];
    case 'graduated':
      return s.selector.targetId;
    case 'numeric':
      return `${s.min}-${s.max}`;
    case 'ranked':
      return s.options[0];
    case 'freeform':
      return 'freeform';
  }
}

// -- Verify reused REA types compose in payloads --

const _delegatePayload: CoordinationPayload = {
  verb: 'delegate' as const,
  data: {} as Agreement,
};

const _provisionPayload: CoordinationPayload = {
  verb: 'provision' as const,
  data: {} as Commitment,
};

const _ratifyPayload: CoordinationPayload = {
  verb: 'ratify' as const,
  data: {} as DeliberationProposal,
};

const _recallPayload: CoordinationPayload = {
  verb: 'recall' as const,
  data: {} as Commitment,
};

// -- Verify InvocationRequest can carry optional Intent --

const _invokeWithIntent: InvocationRequest = {
  capability: ['knowledge-map-synthesis'],
  params: { contentId: 'epr:123' },
  intent: {} as Intent,
};

const _invokeWithoutIntent: InvocationRequest = {
  capability: ['content-safety-review'],
  params: { text: 'hello' },
};

// -- Verify MutualAidContext uses CoordinationScope --

const _aidContext: MutualAidContext = {
  id: 'aid-1',
  mode: 'emergency',
  scope: {} as CoordinationScope,
  declaredBy: { agentId: 'elohim-cascadia' },
  constitutionalBasis: 'epr:constitution/4.2',
  activatedAt: '2026-03-09T00:00:00Z',
  resourceType: 'compute',
  commitmentRefs: [],
  reviewStatus: 'active',
};

// -- Verify full envelope construction --

const _envelope: InvocationEnvelope = {
  id: 'env-1',
  verb: 'sense',
  scope: {
    visibility: { reach: 'municipal', affinity: 'open', federated: false },
    layer: 'municipal',
    target: { kind: 'layer', layer: 'family' },
  },
  routing: { urgency: 'async', fallback: 'escalate' },
  payload: {
    verb: 'sense',
    data: {
      question: 'Should we expand the compute pool?',
      responseShape: { kind: 'categorical', options: ['yes', 'no', 'need-more-info'] },
      quorum: 50,
    },
  },
  sender: { agentId: 'elohim-portland', layer: 'municipal' },
  timestamp: '2026-03-09T00:00:00Z',
  constitutionalBasis: 'epr:constitution/sense-authority',
};
