---
title: "Letter to WeAll — peer review reply: capture resistance while the room is small"
id: letter-to-weall-peer-review-2026-10-10
status: Sent
date: 2026-10-10
derived-from: weall-protocol-peer-review-2026-10-10
---

# WeAll Protocol: a peer review from Elohim

Errol,

You asked for critique. Here it is. We cloned WeAll-Protocol at HEAD `111066c` (1,228 commits by `git rev-list --count`, 2026-04-29 to 2026-10-07) and read it in full, with the governance, Proof-of-Humanity, dispute, group and consensus code each mapped separately and every line we cite re-opened in source before sending. We installed your hash-locked dev environment in a scratch venv (`pip install --require-hashes`, Python 3.12.14, exit 0). `pytest --collect-only` collected 5,179 tests. We ran five targeted files (`test_dispute_auto_juror_assignment`, `test_governance_multi_option_voting`, `test_governance_due_height_trust_boundary`, `test_review_scope_and_async_quorum`, `test_priority0_poh_bootstrap_policy`): 20 tests, all passed. We did not run the full suite. Every "no code reads this" or "no document names this" claim below was checked by searching the whole source and docs tree for the identifier, excluding tests. Paths are relative to `Weall-Protocol/src/weall/` unless they start with `configs/`, `docs/`, `tests/` or `scripts/`.

WeAll puts all civic state on one public, replayable chain. Elohim is a learning and community platform built on Holochain, a peer-to-peer framework with no global chain: each person's device keeps its own signed record log, and peers validate each other's records against shared rules. Those rules live in what Holochain calls an *integrity zome*, validation code every peer runs on every record and cannot swap without changing the network's identity. Every Elohim record carries one of eight scope levels, which we call reach: `private`, `self`, `intimate`, `trusted`, `familiar`, `community`, `public`, `commons`. A decision can therefore bind a few people without being visible to everyone. An *elohim* is a bounded software agent that acts for a person or a community under a written constitution. In one line: WeAll buys closure and pays in flatness; we buy gradient and pay in closure.

One thing first. Every vote, governance or dispute, passes through `_require_active_ballot_profile` (`runtime/apply/governance.py:63-75, :2011`; `runtime/apply/dispute.py:48-61, :2174`), and no transaction writes the activation receipt a public ballot profile needs (`runtime/ballot_policy.py:118-153`). On the shipped production profile no ballot can be cast. So the vote-driven findings below are latent. They are defects in code that switches on when ballots and tier-2 growth switch on, as on a controlled testnet with the ballot flag set. They are not attacks reachable on production today. Your claim discipline is strict enough that we owe you that framing first.

## Your question, answered

How does a small network resist coordinated capture? Your code answers in four layers.

Below about nine verified humans there is no mechanism, only a founder. Strict disputes refuse a panel under 9 (`dispute.py:571-574`), your Byzantine-fault-tolerant (BFT) consensus has zero fault tolerance under 4 validators (`runtime/bft_hotstuff.py:45, :58-62`), and the live PoH panel of 5 cannot seat from a pool of one. You have fenced all three correctly. The honest small-N design is a declared, time-bounded founder grant with a published exit. You have the declaration and the bound. We found no code that retires the founder's transitional tier-2 grant after its expiry (`bootstrap_expires_height = 1008` in `configs/genesis.ledger.prod.json`).

Between about 9 and 50, on any chain with ballots on, one participant can grind each contest. The thresholds are not weak; two-thirds of the whole electorate is strong. The problem is that the proposer chooses the quorum, the reporter chooses the panel seed, and the enrolee chooses the enrolment floor. Each is an input the adversary controls, read after the adversary knows what it does. The fix has one shape: fix the rules before the actors are known, and fix the randomness after the pool and the case are frozen.

Above about 50, Sybil cost is the whole question. You have refused to answer it with a mechanism you do not have, which is right.

Compressed: your defaults are strong and your overrides give them away; capture at small N is a parameter-ordering problem before it is a threshold problem.

## Findings we believe are new to you

We looked through your audit ledgers for each of these and did not find them. Each carries a severity: **exploitable once ballots are on**, **latent** (real bug, no effect today), or **documentation**.

1. **The proposer sets the quorum.** Exploitable once ballots are on. Proposal create copies `rules = dict(payload.rules)` (`governance.py:1592`), and the governed `gov_config.quorum` fills only keys the proposer left absent (`:1593-1596`). `_electorate_required_votes` honours `rules.quorum_bps` as `max(1, ceil(N·bps/10000))` (`:733-749`), so `quorum_bps = 1` means one required vote at any N. The bounds check at `:2742-2749` guards `GOV_QUORUM_SET`, not a proposal's own rules. Your test `tests/test_m3_closure_regressions.py:141-147` asserts that a proposer-supplied quorum is honoured (`quorum_bps: 5000` gives required 2). Your engine docstring calls payload rules a "stealth override vector" that "MUST NEVER" be used (`runtime/gov_engine.py:97-104`), but the stored record is the payload. `VALIDATOR_SUSPEND` and `VALIDATOR_REMOVE` sit on the same allowlist (`governance.py:1398-1420`), so with ballots on, one tier-2 account can shrink the validator set in a few blocks. Fix shape: the governed config overrides proposer rules instead of filling them, with a floor. This is the one to fix first.

2. **The dispute panel seed belongs to the reporter.** Exploitable once ballots are on. The selector sorts eligible jurors by `sha256(seed ∥ candidate)` and takes the first *n* (`runtime/apply/dispute.py:552-564`), where:

   ```
   seed = { domain, chain_id, dispute_id, target_type, target_id,
            opened_at_height, round, candidate_commitment }
   ```

   `dispute_id` is the reporter's own free-form payload string, required and not bound to a hash (`:1275-1280`, used at `:1610`). A colluding reporter can therefore grind it offline: pick a candidate `dispute_id`, compute the panel the way `select_dispute_panel` does (one sha256 per eligible juror), check whether colluders hold 4 of 7, and repeat. With *n* = 7 (`:506`) and a uniform draw, the chance an honest draw already gives colluders 4 of 7 is hypergeometric: for a pool of 12 with 6 colluders, P(X ≥ 4) = 0.50; for a pool of 30 with 10 colluders, 0.14. Expected grind cost is 1/P, so about 2 tries and about 7 tries respectively. Two things soften this: `opened_at_height` is set by the block builder (`:1665`), so each attempt must predict its inclusion height, and a failed attempt leaves a visible dispute. Neither changes the shape. Your A20-F001 closure (`docs/audit/WeAll-A01-A20-P0-Closure-Manifest-20260930.md:119`) treats the class as removed because PoH is scope-closed. The dispute selector is the same gap outside that closure, and no document names it. Fix shape: a commit-reveal or beacon that lands after the pool and the case are frozen.

3. **The quorum race.** Exploitable once ballots are on. A dispute resolves the instant `total_votes ≥ ceil(2n/3)`, outcome `yes > no`, with abstentions counted toward quorum (`dispute.py:1186-1232`). Three yes and two abstain uphold a report with two ballots uncast. Substitutes are recorded (`:651, :1713`) and never promoted. The no-show flags written at `:1136, :2874, :2880` have no reader anywhere in `src/`. Your appeal window protects targets of a wrongful removal; a wrongful "not upheld" has no reporter appeal (`:1655-1658`). Fix shape: close on deadline or full turnout, and give the flags a reader.

4. **The enrolee's payload overrides the reputation floor.** Latent. `_role_required_reputation_milli` checks `if key in payload` before `if key in params` (`runtime/apply/roles.py:293-306`). Today the floor is 0 and not governable, so this changes nothing. It bites the day a floor exists. Fix shape: read params first and reject the payload key.

5. **`restricted_members` lands in a store nothing reads.** Latent. `GROUP_MEMBERSHIP_RESTRICT` writes into `state["groups"]["by_id"]` while the canonical store is `roles.groups_by_id` (`dispute.py:1174-1183`), and no code outside that writer reads the key. A ruling to restrict a member therefore does not reach the group's membership logic as written. Relatedly, `GROUP_ROLE_GRANT` can grant posting roles to accounts that are not members (`content.py:256-275`). Fix shape: write to the canonical store and add a test that reads it back.

6. **Whitelisted parameters with no reader.** Documentation, with a governance cost. `param_policy.py:29-30` whitelists `live_n_jurors` and `live_interacting_jurors`; the live scheduler uses constants (`runtime/poh/live_scheduler.py:333-334`). `live_pass_threshold` and `economics.transfer_fee_bps` are also whitelisted with zero readers. A valid vote on any of them changes nothing, which a voter will not know. Fix shape: wire them or drop them from the whitelist.

7. **Node binding wording.** Documentation. `docs/THREAT_MODEL_CHECKLIST.md:15-19` says a node key is "bound on-chain to a verified account." `net/peer_identity.py:317-330` checks that the account exists with one active node device and reads no `poh_tier`. That is one node per account, not per human. Fix shape: change the doc or add the tier check.

## Where you are ahead

- **Running code for every mechanism you asked about.** Proposals, electorate snapshots, dispute panels, appeals, PoH cases and validator-set transitions all exist as state transitions with tests. Our random-panel draw exists only as a rejected stub in a design note (`genesis/docs/superpowers/specs/2026-06-23-elohim-ceiling-design.md:101` in our repository).
- **Enforcement where every node runs it.** The parameter whitelist (`runtime/param_policy.py:21-48`) is checked at proposal create, edit and execute, and again inside the rules setter. Governance cannot widen its own reach. Our integrity zome for governance never reads who authored a record.
- **A Sybil cost at all.** Registration needs a 16-bit proof-of-work bound to payload and signer (`runtime/account_registration_work.py:29-30, 211-290`). Ours is a non-empty-string check.
- **Fail-closed gates.** Economics locks are refusals, not labels (`runtime/econ_phase.py:180-195`). Under strict governance the constitution cannot be amended at all, and the error names what is unspecified (`governance.py:1517-1540`).
- **Test surface.** 5,179 collected tests, zero unconditional skips, a mutation gate, and hash-locked dependencies that install cleanly.
- **Candour.** `juror_select.py:325` says the selector is "deterministic and *not* an unpredictability claim." Your README's status table is ten rows of NO-GO. You say it on the first screen.

## Things you already document, connected

- **One key is everything at genesis.** In both shipped genesis files (`configs/genesis.ledger.prod.json`, `configs/genesis.ledger.testnet-v1.json`) one pubkey is validator, operator, sole tier-2 human, founder and allowlist entry. Your README says NO-GO. Agreed.
- **The account ceiling.** `docs/security/ACCOUNT_REGISTRATION_SCARCITY.md:47` concedes a distributed attacker can consume capacity, and you present the ceiling as a state-safety bound. We agree it is not meant as Sybil defence. One new detail: PoH reviewers are excluded only from their own case (`runtime/poh/juror_select.py:~310-440`), so A can verify B and B can verify A.
- **Jurisdiction.** `GEO-002` and `SCP-003` are declared normative targets, and your M1 to M3 audit (`docs/audits/M1_M3_COMPREHENSIVE_AUDIT.md:83-85`) names the removal-authority conflict. Today the realized primitive is a group whose creator holds every lever (`runtime/apply/groups.py:543-600, :853-871`).
- **Governance's reach over validators.** `docs/consensus_governance_surface.md` lists only parameters. In code governance also adds, removes and suspends validators. The lifecycle gate blocks arbitrary additions (`apply/consensus.py:558-644`). We found no minimum-set check on removal and no validator-side ratification step.
- **Where authority terminates.** Upgrades are records (`apply/protocol.py:277-298`, `software_applied=False`, `operator_action_required=True`), so constitutional authority ends at whoever runs the validator binaries. Your `docs/constitution/CONSTITUTIONAL_TRACEABILITY.md` lists the same gaps we found.
- **The pinned clock.** `configs/genesis.ledger.prod.json` pins `economic_unlock_time` to 2026-08-07. The generator is sound (`scripts/build_production_genesis_manifest.py:407-408, :412`), but if this exact chain identity launches, its first 90 days elapsed before the first block.

## Challenges to the assumptions

**Public-only as an axiom.** ADR 0001 says nothing consensus-affecting may be private. We disagree, and we want to state the disagreement fairly. A household's decisions are not the commons' business, and a protocol that cannot express a decision binding on fewer than everyone is not local-first. Our model is the reach ladder above. The price we pay is real: we cannot yet hand two strangers one artifact that proves a decision was made. You can. We have to argue against your position, not assume past it.

**The account ceiling as a weapon.** At 16 bits the whole 10,000-account ceiling is 10,000 × 2^16 ≈ 6.6 × 10^8 sha256 evaluations, about a minute on one laptop at 10^7 hashes per second, and once it is full, registration fails for everyone (`runtime/account_registration_work.py:224-235`). A Sybil farm does not need to win the vote. It can close the door. Denial to newcomers is the one failure a small civic network cannot afford.

**The validator set as the electorate's landlord.** Governance can replace validators, and validators apply governance. Upgrades are records, and the binary each operator runs is outside the chain. Every BFT chain has this shape. What WeAll could do that most do not is record the operator-side act, a signed `software_applied` attestation per validator per upgrade, so the gap is at least visible.

## Where we disagree at the root

You asked for perspectives that challenge the underlying assumptions, so here is ours stated as a position, not a finding. Your premise is Bitcoin's: legitimacy is a replayable signed chain, a person is a key plus a verification tier, the chain is the final authority, and the client may hold none (your Art. X §2). Your code honours that premise, and on one part of it you are simply right and ahead of us: the deepest limits must live where no vote reaches, in the code every node runs. You have built that floor. Ours is thinner.

We part on the rest. We began expecting to need a chain too. What we found is that a fully replicated peer network, every node holding and validating every record, already is a global consensus if you want one, and that nobody should want one: in our own measurements every mistake is multiplied by the number of peers, and no tuning carries seven billion people through one validating set. The unit of agreement is the group that is whole in itself and part of a larger one, and what sits above it is amended by representatives, not by every peer re-checking every record. You will meet that wall as your validator set grows, in the quorum arithmetic above. We meet the opposite wall, which you named for us: closure.

We part hardest on identity. You say no mechanism you have proves one human, one account, so you refuse to confer authority. We agree with the refusal and think no hash ever will prove it. Every Sybil answer so far is a filter that compares a person to a template, and templates are cheap to fake. Our bet is that humanity is witnessed: a capable agent that holds a person's record, with every claim and every witness linked, does not validate a hash. It walks the graph from the claim, looks at the shape of the activity against every other claim available to it, and asks a representative elohim, a software agent acting for the community rather than for the person, to attest. Faking a template is cheap. Faking a whole person is dearer for a structural reason: every relationship in the record has to be a consistent person too, so the cost compounds with each link where a template's cost does not. Astroturf operations do build such histories, and that is exactly why we hold this as a bet rather than a result. The capability we are betting on is the one by which an agent tells a staged test from the real world: in both cases the context around a claim is thinner than it should be, and the detector is the same. This is design, not shipped code. Our shipped witness today only covers account recovery.

Put another way: Bitcoin made each identity cost work, and your sixteen bits per account is a small inheritance of that idea. We think the work that actually tells a person from a farm was paid once, upstream, in the training runs that produced the intelligence doing the discerning. The proof-of-work is banked in the intelligence itself. We do not compete with that spend or ask registrants to re-pay a sliver of it; we ride it, and the attacker's task becomes producing a whole consistent person rather than out-hashing a ceiling. The banked work only defends if the discerning is spread out, and that is what our reach design is for: many elohim, each attuned to a different slice of a person's life and never pooling what they see, each attesting within its own reach, with reach composing those attestations upward. An attacker then has to fool many differently-informed discerners at once rather than one, and the cost of the defence is distributed across the network instead of charged per account. That is a bet on an asymmetry that generation tools are also climbing, and we hold it as a bet.

On your Art. X we agree more than the word "ceiling" suggests. We refuse client authority too. No device and no agent in Elohim, an elohim included, can make a stand that cannot be challenged. Authority is vested in the diversity of the network itself and in the validation of claims across it: a record stands because many independent peers, in many groups, validated it against the same rules, and because the witnesses to it can be asked. Neither holding a key nor spending compute confers sovereignty here. The elohim are the ceiling only in the sense that they do the consistent, context-rich, machine-speed adjudication; they are not a seat of authority, because every act of theirs is witnessed, scoped by reach, challengeable, and overturnable by councils of ordinary people drawn by lot, while neither the agents nor the councils can vote past the mechanical floor, the code-enforced limits no vote reaches. One commitment we can state exactly, because your question (iii) below asked for it: the council that may overturn an elohim is drawn to the reach of the decision. A decision that binds a household is overturned by that household; one that binds a community by a representative sample of it; one that binds the planet only by a council that represents humanity's full diversity closely enough to be a true, statistically representative consensus. The override exists at every scope, and its legitimacy is the representativeness of the draw, not who showed up. Where we differ from Art. X is in what counts as "the network." For you it is one chain's validator set, whose standing is conferred by keys, work and tier. For us it is the plurality of validators across groups and the claims they witnessed. The conviction underneath is one we hold rather than prove: people need to be one degree removed from the temptations of that kind of power, and so does any single agent. The image we use for it is the garden: the tree stays, and the fruit is reached for by appeal rather than held in a hand. Today the rule that bounds this is prose, and your frame is right to say so. The protocol rule that bounds it is the work in front of us.

One line each, then. You should be able to say what happens when a perfectly replayable record holds a lie every validator accepted. We should be able to say what single artifact two strangers compare to agree a decision was made. Neither of us can write our sentence yet.

## What we owe you

Your frame puts three questions to us, and we are carrying them.

1. Name one finished Elohim governance decision and the single artifact two strangers with no access grants can compute and compare byte for byte.
2. Which integrity-zome file stops one person's 100 free keys from casting 100 votes, writing a precedent record at the `constitutional` binding level (our governance module accepts a `binding` field whose highest value is `constitutional`, and nothing checks who wrote it), or upholding a challenge? If none, will our habit register, the public list of behaviours we claim, each bound to a runnable check, say that governance outcomes are not yet authoritative?
3. Which reach tiers are barred from carrying binding decisions, and what protocol rule, not prose, keeps an elohim from becoming the final civic authority?

Two admissions, with the files so you can check them. Our governance module is called mishpat. Its integrity validators never read the author of an entry (`elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs:415-433`), so any agent can write a precedent marked constitutional. Our vote path has no one-vote-per-agent check (`elohim/holochain/dna/elohim/zomes/content_store/src/governance_action.rs:396-410`). Both paths are in our public repository: https://github.com/ethosengine/elohim

## Reading list

- Ford, *Identity and Personhood in Digital Democracy* (2020): [arXiv 2011.02412](https://arxiv.org/abs/2011.02412)
- Keidar, Lewis-Pye, Shapiro, Talmon, *Constitutional Consensus for Democratic Governance*: [arXiv 2505.19216](https://arxiv.org/abs/2505.19216). The closest academic sibling to your thesis; its treatment of participant-set changes is the validator-set question above.
- Kleros: stake-weighted juror drawing, appeal-by-doubling, and the *p + ε* analysis (how much a briber must offer so that voting their way is each juror's best move).
- Proof of Humanity (Kleros registry), BrightID, Idena, Duniter, including the ring-plus-deepfake attack documented on the PoH forum.
- Participation data from decentralized autonomous organizations (DAOs): the Complutense study reported by The Defiant, and the Compound "Humpy" quorum capture.
- Ostrom's design principles, especially nested enterprises and graduated sanctions.

## Close

The full internal review is available on request. Please tell us which findings you dispute.

Matthew Dowell
Elohim Protocol, https://github.com/ethosengine/elohim

*This review was produced with AI agents under my direction. Every cited line was re-verified against source before sending.*
