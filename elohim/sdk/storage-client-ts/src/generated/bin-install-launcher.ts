/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/bin-install-launcher.schema.json -- DO NOT EDIT */

/**
 * Device-local CLI output; reconstructed from private custody and runtime observations. No network authority.
 */
export type BinShadows = {
  name: string;
  path: string;
  target: string | null;
  superseded: boolean;
}[];

/**
 * Device-local CLI output; reconstructed from private custody and runtime observations. No network authority.
 */
export interface BinInstallLauncher {
  launcher: string;
  head: string;
  adopted: {
    [k: string]: string;
  };
  persistentAdopted: {
    [k: string]: string;
  };
  /**
   * Plain files at a launcher name that could not identify themselves, keyed by their path; each value is where their bytes are kept in the store. The launcher link replaced them.
   */
  retired: {
    [k: string]: string;
  };
  shadows: BinShadows;
}
