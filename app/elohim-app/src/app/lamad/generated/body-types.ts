// AUTO-GENERATED from lamad manifest + companion schemas.
// DO NOT EDIT — regenerate with: pnpm run lamad:codegen

export interface Section {
  id: string;
  title?: string;
  description?: string;
  level?: 'course' | 'unit' | 'lesson' | 'item';
  estimatedDuration?: string;
  optional?: boolean;
  items?: Item[];
  sections?: Section[];
}

export interface Item {
  /** EPR reference (e.g. 'epr:concept-id' or bare 'concept-id') */
  ref: string;
  role?: 'step' | 'checkpoint' | 'optional' | 'reflection';
  title?: string;
  narrative?: string;
  learningObjectives?: string[];
  completionCriteria?: { type?: 'view' | 'score' | 'time' | 'interaction'; threshold?: number };
}

export interface EprCompositeBody {
  schemaVersion?: number;
  pathType?: string;
  layout?: 'sequential' | 'branching' | 'exploratory';
  sections: Section[];
}
