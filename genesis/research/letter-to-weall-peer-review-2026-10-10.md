---
title: "Letter to WeAll — peer review reply: capture resistance while the room is small"
id: letter-to-weall-peer-review-2026-10-10
status: Sent
date: 2026-10-10
derived-from: weall-protocol-peer-review-2026-10-10
---

# WeAll Protocol: a peer review from Elohim

Errol,

You asked for critique. Here it is. We cloned WeAll-Protocol at HEAD `111066c` (1,228 commits, 2026-04-29 to 2026-10-07) and read it in full, with the governance, Proof-of-Humanity, dispute, group and consensus code each mapped separately and every line we cite re-opened in source before sending. We installed your hash-locked dev environment in a scratch venv outside the clone (`pip install --require-hashes`, Python 3.12.14, exit 0). `pytest --collect-only` collected 5,179 tests, and five targeted governance, dispute and PoH test files ran 20 passed. We did not run the full suite, and we executed nothing from inside the clone. Paths are relative to `Weall-Protocol/src/weall/` unless they start with `configs/`, `docs/`, `tests/` or `scripts/`.

Two sentences of frame. WeAll puts all civic state on one public, replayable chain. Elohim puts every record on an eight-level reach ladder, from `private` through `intimate`, `trusted`, `familiar` and `community` to `commons`, so a decision can bind a few people without being visible to everyone. In one line: WeAll buys closure and pays in flatness; we buy gradient and pay in closure.

One thing you should hear before any finding. Every vote, governance or dispute, passes through `_require_active_ballot_profile` (`runtime/apply/governance.py:63-75, :2011`; `runtime/apply/dispute.py:48-61, :2174`), and no transaction writes the activation receipt a public ballot profile needs (`runtime/ballot_policy.py:118-153`). On the shipped production profile no ballot can be cast. So the vote-driven findings below are latent. They are defects in code that switches on when ballots and tier-2 growth switch on, as on a controlled testnet with the ballot flag set. They are not attacks reachable on production today. Your claim discipline is strict enough that we owe you that framing first.

## Your question, answered

How does a small network resist coordinated capture? Your code answers in four layers.

Below about nine verified humans there is no mechanism, only a founder. Strict disputes refuse a panel under 9 (`dispute.py:571-574`), BFT has zero fault tolerance under 4 validators (`runtime/bft_hotstuff.py:45, :58-62`), and the live PoH panel of 5 cannot seat from a pool of one. You have fenced all three correctly. The honest small-N design is a declared, time-bounded founder grant with a published exit. You have the declaration and the bound. We found no code that retires the founder's transitional tier-2 grant after its height-1008 expiry.

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

   `dispute_id` is the reporter's own free-form payload string, required and not bound to a hash (`:1275-1280`, used at `:1610`). A colluding reporter grinds it offline. For *n* = 7 (`:506`), computed hypergeometrically over a uniform draw, 6 colluders in a pool of 12 get a 4-of-7 majority in 50% of honest draws; 10 in a pool of 30 get it in 14%. Two things soften this: `opened_at_height` is set by the block builder (`:1665`), so each attempt must predict its inclusion height, and a failed attempt leaves a visible dispute. Neither changes the shape. Your A20-F001 closure (`docs/audit/WeAll-A01-A20-P0-Closure-Manifest-20260930.md:119`) treats the class as removed because PoH is scope-closed. The dispute selector is the same gap outside that closure, and no document names it. Fix shape: a commit-reveal or beacon that lands after the pool and the case are frozen.

3. **The quorum race.** Exploitable once ballots are on. A dispute resolves the instant `total_votes ≥ ceil(2n/3)`, outcome `yes > no`, with abstentions counted toward quorum (`dispute.py:1186-1232`). Three yes and two abstain uphold a report with two ballots uncast. Substitutes are recorded (`:651, :1713`) and never promoted. The no-show flags written at `:1136, :2874, :2880` have no reader anywhere in `src/`. Your appeal window protects targets of a wrongful removal; a wrongful "not upheld" has no reporter appeal (`:1655-1658`). Fix shape: close on deadline or full turnout, and give the flags a reader.

4. **The enrolee's payload overrides the reputation floor.** Latent. `_role_required_reputation_milli` checks `if key in payload` before `if key in params` (`runtime/apply/roles.py:293-306`). Today the floor is 0 and not governable, so this changes nothing. It bites the day a floor exists. Fix shape: read params first and reject the payload key.

5. **`restricted_members` lands in a store nothing reads.** Latent. `GROUP_MEMBERSHIP_RESTRICT` writes into `state["groups"]["by_id"]` while the canonical store is `roles.groups_by_id` (`dispute.py:1174-1183`), and no code outside that writer reads the key. A ruling to restrict a member therefore does not reach the group's membership logic as written. Relatedly, `GROUP_ROLE_GRANT` can grant posting roles to accounts that are not members (`content.py:256-275`). Fix shape: write to the canonical store and add a test that reads it back.

6. **Whitelisted parameters with no reader.** Documentation, with a governance cost. `param_policy.py:29-30` whitelists `live_n_jurors` and `live_interacting_jurors`; the live scheduler uses constants (`runtime/poh/live_scheduler.py:333-334`). `live_pass_threshold` and `economics.transfer_fee_bps` are also whitelisted with zero readers. A valid vote on any of them changes nothing, which a voter will not know. Fix shape: wire them or drop them from the whitelist.

7. **Node binding wording.** Documentation. `docs/THREAT_MODEL_CHECKLIST.md:15-19` says a node key is "bound on-chain to a verified account." `net/peer_identity.py:317-330` checks that the account exists with one active node device and reads no `poh_tier`. That is one node per account, not per human. Fix shape: change the doc or add the tier check.

## Where you are ahead

- **Running code for every mechanism you asked about.** Proposals, electorate snapshots, dispute panels, appeals, PoH cases and validator-set transitions all exist as state transitions with tests. Our random-panel draw is still a rejected stub in our own validation code.
- **Enforcement where every node runs it.** The parameter whitelist (`runtime/param_policy.py:21-48`) is checked at proposal create, edit and execute, and again inside the rules setter. Governance cannot widen its own reach. Our equivalent validators never read who authored a governance record.
- **A Sybil cost at all.** Registration needs a 16-bit proof-of-work bound to payload and signer (`runtime/account_registration_work.py:29-30, 211-290`). Ours is a non-empty-string check.
- **Fail-closed gates.** Economics locks are refusals, not labels (`runtime/econ_phase.py:180-195`). Under strict governance the constitution cannot be amended at all, and the error names what is unspecified (`governance.py:1517-1540`).
- **Test surface.** 5,179 collected tests, zero unconditional skips, a mutation gate, and hash-locked dependencies that install cleanly.
- **Candour.** `juror_select.py:325` says the selector is "deterministic and *not* an unpredictability claim." Your README's status table is ten rows of NO-GO. You say it on the first screen.

## Things you already document, connected

- **One key is everything at genesis.** In both shipped genesis files one pubkey is validator, operator, sole tier-2 human, founder and allowlist entry. Your README says NO-GO. Agreed.
- **The account ceiling.** `docs/security/ACCOUNT_REGISTRATION_SCARCITY.md:47` concedes a distributed attacker can consume capacity, and you present the ceiling as a state-safety bound. We agree it is not meant as Sybil defence. One new detail: PoH reviewers are excluded only from their own case (`runtime/poh/juror_select.py:~310-440`), so A can verify B and B can verify A.
- **Jurisdiction.** `GEO-002` and `SCP-003` are declared normative targets, and your M1 to M3 audit (`docs/audits/M1_M3_COMPREHENSIVE_AUDIT.md:83-85`) names the removal-authority conflict. Today the realized primitive is a group whose creator holds every lever (`runtime/apply/groups.py:543-600, :853-871`).
- **Governance's reach over validators.** `docs/consensus_governance_surface.md` lists only parameters. In code governance also adds, removes and suspends validators. The lifecycle gate blocks arbitrary additions (`apply/consensus.py:558-644`). We found no minimum-set check on removal and no validator-side ratification step.
- **Where authority terminates.** Upgrades are records (`apply/protocol.py:277-298`, `software_applied=False`, `operator_action_required=True`), so constitutional authority ends at whoever runs the validator binaries. Your `CONSTITUTIONAL_TRACEABILITY.md` lists the same gaps we found.
- **The pinned clock.** `configs/genesis.ledger.prod.json` pins `economic_unlock_time` to 2026-08-07. The generator is sound (`scripts/build_production_genesis_manifest.py:407-408, :412`), but if this exact chain identity launches, its first 90 days elapsed before the first block.

## Challenges to the assumptions

**Public-only as an axiom.** ADR 0001 says nothing consensus-affecting may be private. We disagree, and we want to state the disagreement fairly. A household's decisions are not the commons' business, and a protocol that cannot express a decision binding on fewer than everyone is not local-first. Our model is the reach ladder above. The price we pay is real: we cannot yet hand two strangers one artifact that proves a decision was made. You can. We have to argue against your position, not assume past it.

**The account ceiling as a weapon.** At 16 bits, the whole 10,000-account ceiling costs one laptop an afternoon, and once it is full, registration fails for everyone (`runtime/account_registration_work.py:224-235`). A Sybil farm does not need to win the vote. It can close the door. Denial to newcomers is the one failure a small civic network cannot afford.

**The validator set as the electorate's landlord.** Governance can replace validators, and validators apply governance. Upgrades are records, and the binary each operator runs is outside the chain. Every BFT chain has this shape. What WeAll could do that most do not is record the operator-side act, a signed `software_applied` attestation per validator per upgrade, so the gap is at least visible.

## What we owe you

Your frame puts three questions to us, and we are carrying them. Two terms first: an *integrity zome* is the validation code every Holochain peer runs on every record and cannot swap without changing the network's identity; an *elohim* is a bounded software agent that acts for a person or a community under a written constitution.

1. Name one finished Elohim governance decision and the single artifact two strangers with no access grants can compute and compare byte for byte.
2. Which integrity-zome file stops one person's 100 free keys from casting 100 votes, writing a `constitutional`-grade precedent record, or upholding a challenge? If none, will our register say governance outcomes are not yet authoritative?
3. Which reach tiers are barred from carrying binding decisions, and what protocol rule, not prose, keeps an elohim from becoming the final civic authority?

Two admissions. Our governance integrity validators never read the author of an entry (`elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs:415-433`), so any agent can write a precedent marked constitutional. Our vote path has no one-vote-per-agent check (`elohim/holochain/dna/elohim/zomes/content_store/src/governance_action.rs:396-410`).

## Reading list

- Ford, *Identity and Personhood in Digital Democracy* (2020): [arXiv 2011.02412](https://arxiv.org/abs/2011.02412)
- Keidar, Lewis-Pye, Shapiro, Talmon, *Constitutional Consensus for Democratic Governance*: [arXiv 2505.19216](https://arxiv.org/abs/2505.19216). The closest academic sibling to your thesis; its treatment of participant-set changes is the validator-set question above.
- Kleros: stake-weighted drawing, appeal-by-doubling, and the *p + ε* bribery analysis.
- Proof of Humanity (Kleros registry), BrightID, Idena, Duniter, including the ring-plus-deepfake attack documented on the PoH forum.
- DAO participation data: the Complutense study reported by The Defiant, and the Compound "Humpy" quorum capture.
- Ostrom's design principles, especially nested enterprises and graduated sanctions.

## Close

The full internal review is available on request. The clone we read sits at `genesis/research/repos/WeAll-Protocol` in our repository. Please tell us which findings you dispute.

Matthew Dowell
Elohim Protocol

*This review was produced with AI agents under my direction. Every cited line was re-verified against source before sending.*
