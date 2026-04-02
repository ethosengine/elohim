# elohim-token

Default economic rail for the Elohim Protocol.

Value is minted from witnessed care, work, and exchange — attested by three-leg coupling (lamad + shefa + qahal). Not proof-of-work, not proof-of-stake: **proof of witnessed contribution**.

The token's structural mechanism is the **ResponsibilityDemandParam** — as you accumulate, more is demanded of you. Power coupled with responsibility. Initialized from the empirical 92% consensus on ideal wealth distribution (Ariely & Norton, 2011), evolved through qahal governance, sensed and responded to by elohim agents.

## Status

Research and specification phase.

## Directory Structure

```
elohim-token/
  research/         # Theory, references, design thinking
```

## Key References

- `research/theory-of-value.md` — Core thesis, responsibility demand primitive, empirical basis
- `genesis/docs/content/elohim-protocol/protocol-specification.md` — EPR spec (Shefa Economic Protocol companion spec TBD)
- `elohim/sdk/domains/shefa/manifest.json` — Shefa domain vocabulary
- `app/elohim-app/src/app/elohim/models/protocol-core.model.ts` — Existing token type system
- `app/elohim-app/src/app/elohim/models/rea-bridge.model.ts` — REA primitives
