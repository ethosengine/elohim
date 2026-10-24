/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/source-state.schema.json -- DO NOT EDIT */

/**
 * Ephemeral local source view. Cargo metadata path packages supply the repository source closure; unavailable metadata conservatively selects the whole checkout. Git trees describe HEAD, and clean/status cover those same paths. No persisted authority or network head.
 */
export interface SourceState {
  head: string;
  clean: boolean;
  status: string;
  /**
   * Raw CIDv1 of sorted UTF-8 path, TAB, Git tree object id, LF lines.
   */
  closureCid: string;
  /**
   * Sorted repository-relative paths and committed tree identities; a gitlink uses its pinned commit object id. A dot path means the complete checkout.
   *
   * @minItems 1
   */
  closure: [
    {
      path: string;
      tree: string;
    },
    ...{
      path: string;
      tree: string;
    }[],
  ];
}
