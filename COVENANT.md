# Covenant

This file sets out what the Elohim Protocol repository owes the people whose work it rests on, what holds each term today, and what is still only promised. It is how we mean to live by "A Covenant From Below: Citing What Was Taken", Part IV-B of our [manifesto](genesis/docs/content/elohim-protocol/manifesto.md). That section argues that two bodies of work entered the commons with the link to their makers cut, everything AI models were trained on and the software libraries every program rests on, and that the honest response is to cite them again and hold a place for their makers. The Elohim Protocol is an open-source project building infrastructure households and communities run for themselves. If you maintain something we depend on and want to know whether we keep our word, start here.

As counted from our lockfiles on 2026-10-15, our Rust code rests on 1,211 distinct crates across 35 `Cargo.lock` files, and our Node code on about 5,900 package versions resolved by pnpm; the counts drift as the lockfiles do. Behind those numbers are people we have mostly never named. Where something checks a term below, we name it. Where nothing does yet, we say "promised", and you should read it as nothing stronger.

## 1. We cite what our work rests on

**Today:** The lockfiles pin each package to an exact version and checksum. That records what we resolved, not how our work relies on it, and nothing records reliance yet.

**Promised:** A reliance record for every workspace, Rust first and npm after, in which each build lists everything it rests on as a citation addressed by the package's own published checksum. A check in our local gate will raise a notice when the record falls behind the lockfile.

## 2. We name our ancestors

A work derived from another whole work says so, as a typed record, and keeps the ancestor's notice.

**Today:** We know of four such derivations, credited in prose where they are credited at all:

- `brit` is a fork of gitoxide, Sebastian Thiel's work. Its README credits him, and it still carries his own Drips `FUNDING.json`, which he committed upstream on 2023-11-05.
- `sophia` is built from Khan Academy's Perseus and keeps Khan's MIT `LICENSE`.
- `elohim/holochain-conductor` is a fork of Holochain, under CAL-1.0. Its README does not yet say that it is our fork.
- `vendor/iroh-quinn-proto-0.13.0` is vendored so that it can carry the upstream Quinn fix 434c3586 for a crash on our fleet. It has a `VENDOR-NOTE.md` and its licences.

**Promised:** A `DERIVED_FROM` declaration for each, kept in this repository's own `.epr-meta` metadata, so the ancestry is a record a program can read rather than a sentence someone has to find.

## 3. We carry the notices

**Today:** Where we hold upstream source in our own tree, its licence comes with it, as in `sophia` and the vendored Quinn crate. No check yet confirms that licence and NOTICE text stays attached to everything we depend on.

**Promised:** Each dependency's record will carry its licence and NOTICE text by content address, pointing at the exact text its makers published. Notices are never stripped.

## 4. We hold the makers' seats

A presence is a place in the network held in trust for a project's makers, where the record of what their work has meant collects until they choose to claim it.

**Today:** Eight upstream projects have hand-written presences: Automerge, gitoxide, Holochain, Eclipse Che, iroh, libp2p, Khan Academy and ValueFlows. Each is a small record in `genesis/data/presences/`. gitoxide's, for example, gives its name, marks it as an organization whose work we compose (`standing: prior-art`), identifies it by its GitHub URL, and notes who recorded it, when, and why. Nobody has claimed one yet, and the claiming process is not built.

**Promised:** Every dependency in a reliance record will name an unclaimed presence for its makers, and each presence will gather the record of what relies on that work. Claiming it will mean the makers showing they are who its external identifier says, and taking up the place. Identity is claimed, never inferred: we will not mint a person from commit email addresses. A presence stands for the project, and who speaks for it is for its makers to say.

## 5. We record facts and defer valuation

**Today:** We have published no allocation, share or ranking of what our dependencies are owed, and the reliance records being built hold facts of reliance only. Nothing but this document, and the people keeping it, stops us from publishing one.

**Promised:** We will publish no default allocation. Any valuation is a claim made with its method cited. What a work was worth is settled with its makers, at a table where their seat is already held, and never announced to them. A funding channel a maker has declared, such as Sebastian Thiel's Drips file, is recorded as a fact about them; how anything is settled through it is a question the protocol's steward (Matthew Dowell, who founded it and makes its decisions until its councils exist) has not yet ruled on.

The same term holds for the protocol's own maker. Under the licence's account of value (`EPR-LICENSE-1.0.md` §1, §2.4), the steward's attention, time and direction are labour, and what the commons owes for them is owed in labour's kind. It is recorded here as owed and unaccounted, in no denominator the outside world has, with the seat held like every maker's: no allocation is published for it, no valuation announced, and it is settled at the table or not at all; the steward has said so, and that it is good. The steward's own words, 2026-11-07: "my attention and time is my labor; the elohim protocol owes me that; not sure what outside of our network it's worth, in what denominator, but it's owed, unaccounted for now, and I'm ok with that."

## 6. What an AI contributes is cited too

**Today:** Code here is co-written by people and AI agents, and commits record this with trailers such as `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

**Promised:** A model cannot recall whom it learned from. When an AI agent finds a source for something it wrote, the citation is marked as support found afterwards, never as the model's memory of where an idea came from. That rule belongs at the gate every contribution passes before it travels further, and it exempts no author. The direction and stewardship of the work belong to the person who gave them.

## 7. How to check us, and how to correct us

**Today:** What exists can be checked in the repository itself: the lockfiles, each crate's `license` field, the READMEs and licence files named above. The eight presences are in `genesis/data/presences/`.

**Promised:** As the work is built, the records will live here:

- The design: `genesis/docs/superpowers/specs/2026-10-15-reliance-as-cite-shared-provenance-primitives-design.md`.
- The reliance records, produced by a new `bridges/cargo` crate and later `bridges/pnpm`. They will first land in the protocol's flow store (`.eprfs/status/flows.jsonl`), which today stays local to each checkout; a form an outside maintainer can read is part of the commons-pool design that follows.
- The derivation declarations: this repository's `.epr-meta`, beside each derived work.
- Whether we keep term 1: `genesis/orchestrator/.epr-meta/cites-own-dependencies.habit.md`, which shows its status and the evidence for each change. It begins `unwired`, meaning we have committed to it before we can observe whether we keep it.

If we have credited you wrongly, left you out, or got a licence wrong, tell us: open an issue at <https://github.com/ethosengine/elohim/issues>, or contact the protocol's steward. A correction is welcomed as a contribution in its own right.

## Where we stand today

- **Three licence tiers, ruled 2026-11-06.** The root `LICENSE` is Apache-2.0 and covers everything not marked otherwise. Primitives (the addressing kernel, the EPR crates, the wire-shape and lens crates, the SDK domain types, every DNA zome) are MPL-2.0; services that serve people (`elohim-storage` and its internals, `doorway-service`, `steward/node`) are AGPL-3.0; CAL-1.0 remains only on the conductor fork, whose terms come with it. Every tracked first-party crate and package now carries a `license` field, checked by habit `licence-tiers-declared`. The canon prose under `genesis/docs/content/elohim-protocol/` is CC BY-SA 4.0, ruled 2026-11-07. Its constitution (I.1) says the commons is owned by no one, including its authors; it is owed to the constitution and held in trust by the elohim and the people who tend it, never by a human person as owner. Because a licence needs a signatory, the steward is the copyright's holder of record; no trust deed exists. What binds the holder is the irrevocable grant on everything already shared, and this commitment, made 2026-11-07: the steward will offer the canon under no other terms and will add to it only under this one, so no holder, now or by succession, can dual-license it or grow it privately. A public pledge answerable to the constitution, not a legal instrument. `LICENSING.md` states the tiers; the reasoning is in `genesis/docs/superpowers/specs/2026-11-06-licence-tiers-decision.md`. Not legal advice; counsel confirms before the first published release.
- **Eight of about 1,200 upstreams are held.** Every other maker we rely on has no seat yet.
- **Derivations are credited in prose, not as typed records.**
- **The reliance records are being built, not finished.**

## A covenant, not a treaty

A treaty is negotiated by the parties present and holds for as long as they choose to stay. A covenant is taken up by those who choose to live by it, and it keeps a place for those who have not yet arrived. In the manifesto's words, it "binds no one who has not entered it, and it carries only as much consequence as the number of people living by it. The work is therefore to demonstrate it: to cite, to hold the seat, and to be found keeping the terms when the people named come to look."

Nothing here is special to software. The way we mean to record what our code rests on is the way a co-op grocer could record where its food came from, or a network of community gardens its seed lines.

---

*Drafted by Claude (Anthropic's model) at the direction of Matthew Dowell, the protocol's steward, on 2026-10-15.*
