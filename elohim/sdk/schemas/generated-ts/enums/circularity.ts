/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: enums/circularity.schema.json -- DO NOT EDIT */

/**
 * Lifecycle design intent — how value flows back after primary use. linear=single-use (generates circularity obligation), reusable=glass jar, cascading=EV battery to grid storage, circular=food to compost to soil to food.
 */
export type Circularity = 'linear' | 'reusable' | 'cascading' | 'circular';
