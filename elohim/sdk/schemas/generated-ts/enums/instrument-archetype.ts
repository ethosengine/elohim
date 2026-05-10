/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: enums/instrument-archetype.schema.json -- DO NOT EDIT */

/**
 * Protocol-level instrument archetypes for feedback observation. Each archetype is a named pattern for how observations are produced — apps reference these in their observation declarations.
 */
export type InstrumentArchetype =
  | 'retention-check'
  | 'outcome-correlation'
  | 'distribution-health'
  | 'cost-accumulation'
  | 'outcome-divergence'
  | 'community-report';
