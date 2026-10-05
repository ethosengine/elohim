/**
 * One step of `<elohim-imagodei-witness-trail>`: who took part in what just
 * happened, from the person's point of view. Mirrors the element's
 * `WitnessStep` contract (app/elohim-elements/elohim-imagodei/src/
 * elohim-imagodei-witness-trail.ts) so services and specs never import Lit.
 *
 * Hosts list only what they actually observed; the element adds nothing.
 */
export interface WitnessStep {
  id: string;
  act: 'checked' | 'signed' | 'recorded' | 'seen';
  relation: 'you' | 'this-device' | 'your-device' | 'your-doorway' | 'vouches-for-you' | 'others';
  /** The party's name as the person knows it. Never read for `others`. */
  label?: string;
  /** How many others took part. Only read for `others`. */
  count?: number;
  state: 'waiting' | 'working' | 'done' | 'failed';
  /** Plain reason, shown when the step failed. */
  note?: string;
}

/** The same steps with one step's state changed. */
export function withStepState(
  steps: readonly WitnessStep[],
  id: string,
  state: WitnessStep['state']
): WitnessStep[] {
  return steps.map(step => (step.id === id ? { ...step, state } : step));
}
