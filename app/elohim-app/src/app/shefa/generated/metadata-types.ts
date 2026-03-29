// AUTO-GENERATED from shefa manifest + companion schemas.
// DO NOT EDIT — regenerate with: pnpm run shefa:codegen

export interface StewardshipMetadata {
  /** How stewardship standing is distributed: equal (even split), weighted (by contribution), custodial (original creator priority) */
  allocationStrategy?: 'equal' | 'weighted' | 'custodial';
  /** 0.0-1.0 stewardship affinity score. Accrues from curation acts, decays via demurrage. Attention does NOT increase this — only active curation work. */
  affinityScore?: number;
  /** The steward's role in relation to the content */
  custodianRole?: 'original-creator' | 'curator' | 'reviewer' | 'community';
  /** What the steward can do with the content — governs reach expansion and derivative works */
  circulationRights?: string;
  /** Rate at which standing decays without activity (0.0-1.0 per period). Ensures stewardship requires ongoing engagement, preventing dormant capture. */
  demurrageRate?: number;
  /** Record of curation acts that built this steward's standing */
  curationHistory?: { action?: string; timestamp?: string; eprId?: string }[];
  [key: string]: unknown;
}

export interface ExchangeMetadata {
  /** What is being offered — maps to REA resource specification */
  offerType?: string;
  /** What is being requested in return — maps to REA resource specification */
  requestType?: string;
  /** Human-readable terms of the exchange */
  terms?: string;
  /** ISO 8601 datetime when this offer/request expires */
  expiresAt?: string;
  /** Whether reciprocal action is expected. False for gifts, true for trades. */
  reciprocityExpected?: boolean;
  /** REA resource nature dimensions for the exchanged resource */
  resourceNature?: { rivalry?: string; excludability?: string; depletability?: string; fungibility?: string };
  [key: string]: unknown;
}

export interface AgreementMetadata {
  /** Agent IDs of agreement parties */
  parties?: string[];
  /** Individual obligations within the agreement */
  obligations?: { obligor?: string; action?: string; resourceConformsTo?: string; dueDate?: string; fulfilled?: boolean }[];
  /** Human-readable description of what constitutes fulfillment */
  fulfillmentCriteria?: string;
  /** Current agreement lifecycle state */
  state?: 'proposed' | 'active' | 'fulfilled' | 'cancelled' | 'disputed';
  /** REA Measure representing maximum coverage amount for insurance agreements */
  coverageLimit?: { hasNumericalValue?: number; hasUnit?: string };
  [key: string]: unknown;
}
