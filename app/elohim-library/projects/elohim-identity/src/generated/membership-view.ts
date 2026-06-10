/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/membership-view.schema.json -- DO NOT EDIT */

/**
 * Source of truth: Cat-A Membership DHT entry's member_kind field in imagodei DNA (DNA-notarized). Category C — wire-shape projection of that enum field. Polymorphic membership subject type for Collective/Qahal members. Per spec §2.1.
 */
export type MemberKind = 'Person' | 'Collective' | 'ElohimAgent';

/**
 * Source of truth: Cat-A Membership DHT entry in imagodei DNA (DNA-notarized). Category C — HTTP wire-shape projection; reconstructible at any time from DHT state. Wire shape for Membership entries with memberKind discriminating polymorphic subject.
 */
export interface MembershipView {
  cid: string;
  memberCid: string;
  memberKind: MemberKind;
  collectiveCid: string;
  role: 'Steward' | 'Contributor' | 'Observer';
  sponsorCid?: string | null;
  joinedAtBlockHeight: number;
  withdrawnAtBlockHeight?: number | null;
}
