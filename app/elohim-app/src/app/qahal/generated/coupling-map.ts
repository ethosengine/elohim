// AUTO-GENERATED from app manifest: elohim/sdk/domains/qahal/manifest.json
// DO NOT EDIT — regenerate with: pnpm run qahal:codegen

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

export const QAHAL_COUPLING_MAP: Record<string, ContentTypeCoupling> = {
  collective: {
    value: {
      onContribute: {
        action: 'produce',
        resourceConformsTo: 'governance-structure',
        recognition: 'stewardship-standing',
      },
    },
    governance: {
      defaultReach: 'community',
      minimumReach: 'intimate',
      governanceModel: 'steward-consent',
      signalTypes: ['governance-decision', 'community-report'],
    },
  },
  proposal: {
    value: {
      onConsume: {
        action: 'use',
        resourceConformsTo: 'governance-participation',
        recognition: 'governance-credit',
      },
      onComplete: {
        action: 'produce',
        resourceConformsTo: 'governance-decision',
        recognition: 'governance-credit',
      },
    },
    governance: {
      defaultReach: 'community',
      minimumReach: 'community',
      governanceModel: 'community-vote',
      signalTypes: ['governance-decision', 'consensus-reached'],
    },
  },
  challenge: {
    value: {
      onContribute: {
        action: 'produce',
        resourceConformsTo: 'constitutional-review',
        recognition: 'governance-credit',
      },
    },
    governance: {
      defaultReach: 'community',
      minimumReach: 'community',
      governanceModel: 'constitutional',
      signalTypes: ['challenge-filed', 'governance-decision'],
    },
  },
  appeal: {
    value: {
      onContribute: {
        action: 'produce',
        resourceConformsTo: 'constitutional-review',
        recognition: 'governance-credit',
      },
    },
    governance: {
      defaultReach: 'community',
      minimumReach: 'community',
      governanceModel: 'constitutional',
      signalTypes: ['appeal-filed', 'governance-decision'],
    },
  },
  statement: {
    value: {
      onContribute: {
        action: 'produce',
        resourceConformsTo: 'deliberation-input',
        recognition: 'governance-credit',
      },
    },
    governance: {
      defaultReach: 'community',
      minimumReach: 'community',
      governanceModel: 'community-vote',
      signalTypes: ['social-engagement', 'consensus-reached'],
    },
  },
  post: {
    value: {
      onContribute: {
        action: 'produce',
        resourceConformsTo: 'social-content',
        recognition: 'community-standing',
      },
    },
    governance: {
      defaultReach: 'community',
      minimumReach: 'intimate',
      governanceModel: 'steward-consent',
      signalTypes: ['social-engagement'],
    },
  },
  event: {
    value: {
      onConsume: {
        action: 'use',
        resourceConformsTo: 'community-gathering',
        recognition: 'community-standing',
      },
      onContribute: {
        action: 'produce',
        resourceConformsTo: 'community-gathering',
        recognition: 'stewardship-standing',
      },
    },
    governance: {
      defaultReach: 'community',
      minimumReach: 'intimate',
      governanceModel: 'steward-consent',
      signalTypes: ['social-engagement', 'relationship-formed'],
    },
  },
  group: {
    value: {
      onContribute: {
        action: 'produce',
        resourceConformsTo: 'governance-structure',
        recognition: 'stewardship-standing',
      },
    },
    governance: {
      defaultReach: 'community',
      minimumReach: 'intimate',
      governanceModel: 'steward-consent',
      signalTypes: ['relationship-formed'],
    },
  },
  message: {
    value: {
      onContribute: {
        action: 'produce',
        resourceConformsTo: 'social-content',
        recognition: 'community-standing',
      },
    },
    governance: {
      defaultReach: 'intimate',
      minimumReach: 'intimate',
      governanceModel: 'steward-consent',
      signalTypes: ['social-engagement'],
    },
  },
  thread: {
    value: {
      onContribute: {
        action: 'produce',
        resourceConformsTo: 'social-content',
        recognition: 'community-standing',
      },
    },
    governance: {
      defaultReach: 'community',
      minimumReach: 'intimate',
      governanceModel: 'steward-consent',
      signalTypes: ['social-engagement'],
    },
  },
};
