# Licensing

The Elohim Protocol is offered under three licences, chosen by what each part of the tree
does rather than by where it sits. Each crate and package declares its own licence in its
`Cargo.toml` or `package.json`; the full texts are in `LICENSES/`. The root `LICENSE`
(Apache-2.0) covers everything that does not declare otherwise.

| Tier | What it covers | Licence | Why |
|---|---|---|---|
| **Primitives** | The addressing kernel (`elohim-address`), the EPR codec and its REA and index layers (`elohim-epr`, `elohim-epr-rea`, `elohim-epr-index`), the wire-shape and lens crates (`elohim-views`, `elohim-facings`), the TypeScript codec (`elohim/sdk/epr-ts`), the SDK domain type crates, and every DNA zome | [MPL-2.0](LICENSES/MPL-2.0.txt) | What others build *on*. File-level share-alike: a change to the primitive itself must be published; code that merely depends on it stays its owner's. |
| **Services that serve people** | `elohim-storage` and the crates carved out of it, `doorway-service`, `steward/node` | [AGPL-3.0](LICENSES/AGPL-3.0.txt) | Whoever runs a modified service for others over a network must give those people its source, so the network can always rebuild the service. The duty protects the commons, not an individual's exit. |
| **Tooling, apps, bridges, patterns** | eprfs, `epr-cli`, `ark`, the bridges, the Angular apps and elements, the a2o suite, the orchestrator, `genesis/` scripts, the `.epr-meta` conventions and the habit register, and anything else not marked | [Apache-2.0](LICENSE) | A gift to the commons with a patent grant. Any organisation can pin a release and depend on it. |

| **The canon** | The manifesto, constitution, `stewardship-over-sovereignty` and the rest of the protocol's foundational prose under `genesis/docs/content/elohim-protocol/` | [CC BY-SA 4.0](LICENSES/CC-BY-SA-4.0.txt) | Owned by no one, including its authors (constitution I.1); held in trust under the constitution by the elohim and the people who tend it. Share-alike for prose: anyone may share, translate and build on it, and every adaptation names its source and carries the same licence, so no holder can enclose it. The steward is the copyright's holder of record because a licence needs a signatory; no trust deed exists. What binds the holder: the grant is irrevocable on everything already shared, and `COVENANT.md` records the commitment to offer the canon under no other terms and add to it only under this one. |

One thing is deliberately **not** covered by any of these:

- **Upstream code held in the tree.** `elohim/holochain-conductor` is a fork of Holochain under
  CAL-1.0, `sophia` keeps Khan Academy's MIT licence, and `vendor/` carries each vendored crate's
  own licence. Their terms come with them and are not changed here.

Why not CAL-1.0 on our own code: CAL's distinctive clause protects one person's ability to leave
with their data. The protocol holds that a person's standing comes from the network that holds
them, not from keys they hold alone, and no copyright licence can say that. CAL is not a
contradiction of that view, only a smaller claim than it, but the licence a reader sees first is
the thesis they assume, so our own crates do not carry it. The conductor fork keeps it, and the
duties it places on anyone hosting people on that conductor (hand them their data; never use key
custody to lock them in) are duties the protocol's doorways already meet by design.

The decision and its reasoning: `genesis/docs/superpowers/specs/2026-11-06-licence-tiers-decision.md`.
This file is not legal advice; counsel should confirm it before the first published release.
