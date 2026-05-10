/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: enums/observation-polarity.schema.json -- DO NOT EDIT */

/**
 * Polarity of a feedback observation. Positive observations extend claim validity horizons. Negative observations shorten them. The protocol requires at least one negative-polarity observation per manifest — systems cannot ship positive-only feedback.
 */
export type ObservationPolarity = 'positive' | 'negative';
