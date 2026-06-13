import { Type } from '@angular/core';

/** Per-block availability for honest rendering across contexts. */
export type BlockAvailability = 'real' | 'pending' | 'na' | 'loading' | 'error';

/** A debug block's value plus how to render its availability. */
export interface BlockState<T> {
  state: BlockAvailability;
  value?: T;
  /** Why a non-'real' state (e.g. "doorway-role — N/A on this node"). */
  note?: string;
}

/** A registered debug lens (one tab in the shell). */
export interface DebugLens {
  id: string;
  title: string;
  icon: string;
  component: Type<unknown>;
}
