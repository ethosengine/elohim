// AUTO-GENERATED from app manifest: elohim/sdk/domains/lamad/manifest.json
// DO NOT EDIT — regenerate with: pnpm run lamad:codegen

export interface ValueFlow {
  action: string;
  resourceConformsTo: string;
  recognition: string;
}

export interface ContentTypeCoupling {
  value: {
    onConsume?: ValueFlow;
    onComplete?: ValueFlow;
    onContribute?: ValueFlow;
  };
  governance: {
    defaultReach: string;
    minimumReach: string;
    governanceModel: string;
    signalTypes: string[];
  };
}

export const LAMAD_COUPLING_MAP: Record<string, ContentTypeCoupling> = {
  'concept': {
    value: {
      onConsume: { action: 'use', resourceConformsTo: 'learning-content', recognition: 'mastery-credit' },
      onComplete: { action: 'produce', resourceConformsTo: 'mastery-attestation', recognition: 'mastery-credit' },
    },
    governance: {
      defaultReach: 'commons',
      minimumReach: 'community',
      governanceModel: 'steward-consent',
      signalTypes: ['learning-signal', 'mastery-achieved'],
    },
  },
  'lesson': {
    value: {
      onConsume: { action: 'use', resourceConformsTo: 'learning-content', recognition: 'mastery-credit' },
      onComplete: { action: 'produce', resourceConformsTo: 'mastery-attestation', recognition: 'mastery-credit' },
    },
    governance: {
      defaultReach: 'commons',
      minimumReach: 'community',
      governanceModel: 'steward-consent',
      signalTypes: ['learning-signal', 'mastery-achieved'],
    },
  },
  'assessment': {
    value: {
      onConsume: { action: 'use', resourceConformsTo: 'evaluation', recognition: 'mastery-credit' },
      onComplete: { action: 'produce', resourceConformsTo: 'mastery-attestation', recognition: 'mastery-credit' },
    },
    governance: {
      defaultReach: 'commons',
      minimumReach: 'community',
      governanceModel: 'steward-consent',
      signalTypes: ['assessment-completed', 'mastery-achieved'],
    },
  },
  'exercise': {
    value: {
      onConsume: { action: 'use', resourceConformsTo: 'practice', recognition: 'mastery-credit' },
    },
    governance: {
      defaultReach: 'commons',
      minimumReach: 'community',
      governanceModel: 'steward-consent',
      signalTypes: ['practice-engagement', 'learning-signal'],
    },
  },
  'experience-story': {
    value: {

    },
    governance: {
      defaultReach: 'self',
      minimumReach: 'community',
      governanceModel: 'steward-consent',
      signalTypes: [],
    },
  },
  'experience-moment': {
    value: {

    },
    governance: {
      defaultReach: 'self',
      minimumReach: 'community',
      governanceModel: 'agent-scoped',
      signalTypes: [],
    },
  },
  'reflection': {
    value: {
      onConsume: { action: 'use', resourceConformsTo: 'learning-content', recognition: 'mastery-credit' },
    },
    governance: {
      defaultReach: 'commons',
      minimumReach: 'community',
      governanceModel: 'steward-consent',
      signalTypes: ['learning-signal'],
    },
  },
  'discussion': {
    value: {
      onConsume: { action: 'use', resourceConformsTo: 'learning-content', recognition: 'mastery-credit' },
      onContribute: { action: 'produce', resourceConformsTo: 'contribution', recognition: 'contribution-record' },
    },
    governance: {
      defaultReach: 'community',
      minimumReach: 'intimate',
      governanceModel: 'community-vote',
      signalTypes: ['learning-signal', 'contribution-created'],
    },
  },
  'article': {
    value: {
      onConsume: { action: 'use', resourceConformsTo: 'learning-content', recognition: 'mastery-credit' },
      onContribute: { action: 'produce', resourceConformsTo: 'contribution', recognition: 'stewardship-standing' },
    },
    governance: {
      defaultReach: 'commons',
      minimumReach: 'community',
      governanceModel: 'steward-consent',
      signalTypes: ['learning-signal', 'contribution-created'],
    },
  },
  'path': {
    value: {
      onConsume: { action: 'use', resourceConformsTo: 'learning-content', recognition: 'mastery-credit' },
      onComplete: { action: 'produce', resourceConformsTo: 'path-completion', recognition: 'mastery-credit' },
      onContribute: { action: 'produce', resourceConformsTo: 'contribution', recognition: 'stewardship-standing' },
    },
    governance: {
      defaultReach: 'commons',
      minimumReach: 'community',
      governanceModel: 'steward-consent',
      signalTypes: ['path-completed', 'learning-signal'],
    },
  },
  'epic': {
    value: {
      onConsume: { action: 'use', resourceConformsTo: 'learning-content', recognition: 'mastery-credit' },
    },
    governance: {
      defaultReach: 'commons',
      minimumReach: 'community',
      governanceModel: 'steward-consent',
      signalTypes: ['learning-signal'],
    },
  },
  'scenario': {
    value: {
      onConsume: { action: 'use', resourceConformsTo: 'learning-content', recognition: 'mastery-credit' },
    },
    governance: {
      defaultReach: 'commons',
      minimumReach: 'community',
      governanceModel: 'steward-consent',
      signalTypes: ['learning-signal'],
    },
  },
  'discovery-assessment': {
    value: {
      onConsume: { action: 'use', resourceConformsTo: 'evaluation', recognition: 'mastery-credit' },
      onComplete: { action: 'produce', resourceConformsTo: 'self-knowledge', recognition: 'mastery-credit' },
    },
    governance: {
      defaultReach: 'commons',
      minimumReach: 'community',
      governanceModel: 'steward-consent',
      signalTypes: ['assessment-completed', 'learning-signal'],
    },
  },
  'instrument': {
    value: {
      onContribute: { action: 'produce', resourceConformsTo: 'contribution', recognition: 'stewardship-standing' },
    },
    governance: {
      defaultReach: 'commons',
      minimumReach: 'community',
      governanceModel: 'steward-consent',
      signalTypes: ['contribution-created'],
    },
  },
  'quiz': {
    value: {
      onConsume: { action: 'use', resourceConformsTo: 'evaluation', recognition: 'mastery-credit' },
      onComplete: { action: 'produce', resourceConformsTo: 'mastery-attestation', recognition: 'mastery-credit' },
    },
    governance: {
      defaultReach: 'commons',
      minimumReach: 'community',
      governanceModel: 'steward-consent',
      signalTypes: ['assessment-completed', 'mastery-achieved'],
    },
  },
  'course-module': {
    value: {
      onConsume: { action: 'use', resourceConformsTo: 'learning-content', recognition: 'mastery-credit' },
      onComplete: { action: 'produce', resourceConformsTo: 'mastery-attestation', recognition: 'mastery-credit' },
    },
    governance: {
      defaultReach: 'commons',
      minimumReach: 'community',
      governanceModel: 'steward-consent',
      signalTypes: ['learning-signal', 'mastery-achieved'],
    },
  },
  'module': {
    value: {
      onConsume: { action: 'use', resourceConformsTo: 'learning-content', recognition: 'mastery-credit' },
    },
    governance: {
      defaultReach: 'commons',
      minimumReach: 'community',
      governanceModel: 'steward-consent',
      signalTypes: ['learning-signal'],
    },
  },
  'community': {
    value: {
      onContribute: { action: 'produce', resourceConformsTo: 'contribution', recognition: 'stewardship-standing' },
    },
    governance: {
      defaultReach: 'community',
      minimumReach: 'intimate',
      governanceModel: 'community-vote',
      signalTypes: ['contribution-created'],
    },
  },
  'tool': {
    value: {
      onConsume: { action: 'use', resourceConformsTo: 'learning-content', recognition: 'mastery-credit' },
    },
    governance: {
      defaultReach: 'commons',
      minimumReach: 'community',
      governanceModel: 'steward-consent',
      signalTypes: ['learning-signal'],
    },
  },
  'placeholder': {
    value: {

    },
    governance: {
      defaultReach: 'private',
      minimumReach: 'private',
      governanceModel: 'steward-consent',
      signalTypes: [],
    },
  },
  'simulation': {
    value: {
      onConsume: { action: 'use', resourceConformsTo: 'learning-content', recognition: 'mastery-credit' },
      onComplete: { action: 'produce', resourceConformsTo: 'mastery-attestation', recognition: 'mastery-credit' },
    },
    governance: {
      defaultReach: 'commons',
      minimumReach: 'community',
      governanceModel: 'steward-consent',
      signalTypes: ['practice-engagement', 'learning-signal'],
    },
  },
  'feature': {
    value: {
      onConsume: { action: 'use', resourceConformsTo: 'learning-content', recognition: 'mastery-credit' },
    },
    governance: {
      defaultReach: 'commons',
      minimumReach: 'community',
      governanceModel: 'steward-consent',
      signalTypes: ['learning-signal'],
    },
  },
  'practice': {
    value: {
      onConsume: { action: 'use', resourceConformsTo: 'practice', recognition: 'mastery-credit' },
      onComplete: { action: 'produce', resourceConformsTo: 'mastery-attestation', recognition: 'mastery-credit' },
    },
    governance: {
      defaultReach: 'commons',
      minimumReach: 'community',
      governanceModel: 'steward-consent',
      signalTypes: ['practice-engagement', 'mastery-achieved'],
    },
  },
  'gate-process-declaration': {
    value: {

    },
    governance: {
      defaultReach: 'commons',
      minimumReach: 'community',
      governanceModel: 'steward-consent',
      signalTypes: ['attestation-granted', 'attestation-revoked'],
    },
  },
  'universal-band-declaration': {
    value: {

    },
    governance: {
      defaultReach: 'commons',
      minimumReach: 'commons',
      governanceModel: 'protocol-ratification',
      signalTypes: ['attestation-granted'],
    },
  },
  'gate-rules-declaration': {
    value: {

    },
    governance: {
      defaultReach: 'commons',
      minimumReach: 'community',
      governanceModel: 'steward-consent',
      signalTypes: ['attestation-granted', 'attestation-revoked'],
    },
  },
  'aggregation-spec': {
    value: {

    },
    governance: {
      defaultReach: 'commons',
      minimumReach: 'community',
      governanceModel: 'steward-consent',
      signalTypes: ['attestation-granted', 'attestation-revoked'],
    },
  },
  'escalation-target-spec': {
    value: {

    },
    governance: {
      defaultReach: 'commons',
      minimumReach: 'community',
      governanceModel: 'steward-consent',
      signalTypes: ['attestation-granted', 'attestation-revoked'],
    },
  },
};
