/**
 * One step of `<elohim-imagodei-witness-trail>`: who took part in what just
 * happened, from the person's point of view.
 *
 * Framework-free and Lit-free, so a host's services and specs can build and
 * pass steps without loading the element. The element re-exports these types;
 * this file is their one home.
 *
 * Hosts list only what they actually observed; the element adds nothing.
 */

/** What happened at one step. */
export type WitnessAct = 'checked' | 'signed' | 'recorded' | 'seen';

/**
 * Who did it, from the person's point of view. `this-device` is the node on
 * the machine the person is using right now.
 */
export type WitnessRelation =
  | 'you'
  | 'this-device'
  | 'your-device'
  | 'your-doorway'
  | 'vouches-for-you'
  | 'others';

export type WitnessStepState = 'waiting' | 'working' | 'done' | 'failed';

/** One step of the trail, exactly as the host observed it. */
export interface WitnessStep {
  id: string;
  act: WitnessAct;
  relation: WitnessRelation;
  /** The party's name as the person knows it. Never read for `others`. */
  label?: string;
  /** How many others took part. Only read for `others`. */
  count?: number;
  state: WitnessStepState;
  /** Plain reason, shown when the step failed. */
  note?: string;
}

/** The same steps with one step's state changed. */
export function withStepState(
  steps: readonly WitnessStep[],
  id: string,
  state: WitnessStepState
): WitnessStep[] {
  return steps.map(step => (step.id === id ? { ...step, state } : step));
}
