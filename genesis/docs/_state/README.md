# `_state/` — the doc state-machine (pressure dirs)

The directory a doc lives in IS its lifecycle state; a transition is a `git mv`. These are the
**pressure** states — they exist to be **emptied**. At equilibrium every dir here holds only its
`CLAUDE.md` gate. Resolved work lives in the terminal homes (`../content/elohim-protocol/architecture/`
= canonical, `../content/elohim-protocol/history/` = retired). See `../PLACEMENT.md`.

Add a state = add one entry to `STATES` in `state-machine-gen.py` (anti-proliferation: one root, shallow).
Check health: `python3 .claude/scripts/memory-kit/placement-audit.py` (structural-equilibrium section).
