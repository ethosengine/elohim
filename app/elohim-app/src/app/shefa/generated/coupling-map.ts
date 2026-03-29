// AUTO-GENERATED from app manifest: elohim/sdk/domains/shefa/manifest.json
// DO NOT EDIT — regenerate with: pnpm run shefa:codegen

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

export const SHEFA_COUPLING_MAP: Record<string, ContentTypeCoupling> = {
  'stewardship-context': {
    value: {
      onContribute: { action: 'produce', resourceConformsTo: 'stewardship-standing', recognition: 'stewardship-standing' },
    },
    governance: {
      defaultReach: 'community',
      minimumReach: 'intimate',
      governanceModel: 'steward-consent',
      signalTypes: ['stewardship-allocated', 'custodian-attestation'],
    },
  },
};
