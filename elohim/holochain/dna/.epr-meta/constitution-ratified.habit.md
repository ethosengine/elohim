---
epr-habit-version: 1
id: constitution-ratified
invariant: >
  The constitution text in force at a reach carries a notarized anchor — its content address
  committed as a validated DHT entry at that reach — and a named ratification mechanism that
  produced the anchor, so that an elohim harness loading the constitution can refuse a text
  whose anchor is not the ratified one. A constitution that reads "Hash: [notarized public
  anchor]" and "Ratified: [consensus mechanism TBD]" binds agents by prose alone; nothing a
  validator runs can tell a ratified text from an edited copy.
status: unwired
active: false
refs:
  - genesis/docs/content/elohim-protocol/constitution.md
  - genesis/docs/content/elohim-protocol/values-forward.md
  - genesis/research/weall-protocol-peer-review-2026-10-10.md
retire-when: >
  when ratification is a mishpat Commitment of role floor over the global reach, the anchor is
  read by the elohim harness at load, and a text cannot be in force unanchored — at which point
  the harness's own refusal is the check and this row adds nothing
---
DELTA 2026-10-10: Declared UNWIRED from the WeAll peer review
(epr:weall-protocol-peer-review-2026-10-10 §7 challenge 5, §10 T4). WeAll hash-binds its draft
constitution into its chain manifest and exposes the hash on its status surface, and refuses
amendment until the process is specified; ours reads `constitution.md:198-199` "Hash: [notarized
public anchor] / Ratified: [consensus mechanism TBD]". The council-drawn-to-reach commitment
(governance-layers-architecture §Constitutional Councils, 2026-10-10) names who may overturn an
elohim at each reach; it does not yet name who ratifies the text the elohim reads. No runnable
check exists. The check it needs: a probe that reads the constitution's declared anchor, finds
the DHT entry at the global reach that commits that CID, and verifies the entry validates and
names its ratification Commitment — exit 1 while the anchor field reads `[TBD]`.
Credit: Errol Swaby, author of the WeAll Protocol (github.com/errol1swaby2-bit/WeAll-Protocol). His draft constitution is hash-bound into the chain manifest (`constitution_hash`, `constitution_version`) and exposed on the status surface, and his governance fails closed on amendment until the process is specified; that is the discipline this habit borrows, and the question in his frame ('which constitution can a citizen verify?') is the one it answers when it is wired.
