// AUTO-GENERATED from app manifest: elohim/sdk/domains/imagodei/manifest.json
// DO NOT EDIT — regenerate with: pnpm run imagodei:codegen

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

export const IMAGODEI_COUPLING_MAP: Record<string, ContentTypeCoupling> = {
  human: {
    value: {
      onConsume: { action: 'use', resourceConformsTo: 'identity', recognition: 'presence-record' },
    },
    governance: {
      defaultReach: 'community',
      minimumReach: 'self',
      governanceModel: 'self-sovereign',
      signalTypes: ['identity-created', 'presence-established', 'agency-progressed'],
    },
  },
  role: {
    value: {
      onConsume: {
        action: 'use',
        resourceConformsTo: 'capability-grant',
        recognition: 'role-record',
      },
    },
    governance: {
      defaultReach: 'community',
      minimumReach: 'community',
      governanceModel: 'steward-consent',
      signalTypes: ['attestation-granted', 'attestation-revoked'],
    },
  },
  contributor: {
    value: {
      onContribute: {
        action: 'produce',
        resourceConformsTo: 'contribution',
        recognition: 'stewardship-standing',
      },
    },
    governance: {
      defaultReach: 'commons',
      minimumReach: 'community',
      governanceModel: 'steward-consent',
      signalTypes: ['presence-established', 'attestation-granted', 'relationship-formed'],
    },
  },
};
