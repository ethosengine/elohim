// AUTO-GENERATED from app manifest: app/avodah/manifest.json
// DO NOT EDIT — regenerate with: pnpm run avodah:codegen

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

export const AVODAH_COUPLING_MAP: Record<string, ContentTypeCoupling> = {
  'work-story': {
    value: {
      onConsume: { action: 'use', resourceConformsTo: 'work-capacity', recognition: 'work-credit' },
      onComplete: { action: 'produce', resourceConformsTo: 'work-output', recognition: 'work-credit' },
    },
    governance: {
      defaultReach: 'community',
      minimumReach: 'private',
      governanceModel: 'steward-consent',
      signalTypes: ['task-completed', 'cadence-reset'],
    },
  },
  'work-project': {
    value: {
      onContribute: { action: 'produce', resourceConformsTo: 'project-milestone', recognition: 'stewardship-standing' },
    },
    governance: {
      defaultReach: 'community',
      minimumReach: 'private',
      governanceModel: 'steward-consent',
      signalTypes: ['sprint-completed'],
    },
  },
};
