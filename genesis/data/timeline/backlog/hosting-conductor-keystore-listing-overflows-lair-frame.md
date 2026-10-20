---
id: "backlog-hosting-conductor-keystore-listing-overflows-lair-frame"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A hosting conductor stops being able to install ANY hosted agent once its keystore holds too many seeds: install_app with a supplied agent key lists every lair seed in one 8 KiB frame — doorway now lets the conductor mint the key; the fork and the fleet still carry the trap"
slug: "hosting-conductor-keystore-listing-overflows-lair-frame"
written: "2026-10-20"
author: "overnight shift 2026-10-20 (ci-runner-story-green)"
status: "open"
priority: "critical"
jobs: [elohim-edge, household-mesh]
tags: [doorway, conductor, lair, hosting, holochain-fork]
---

**What happened (household, 2026-10-20 07:00Z).** After a night of hosted registrations (the
ci-runner story sponsors ~5 runners per run, each a new agent key; ~15 runs), every
`install_app` on matthew's conductor failed in ~6 ms with `Other({"error":"ConnectionAborted"})`;
the conductor log shows lair `incoming channel error: {"error":"Other","message":"internal"}` then
`lair connection recv loop ended`. `generate_agent_pub_key` alone still worked; the same direct
install on jessica's conductor (few seeds) installed, ran and uninstalled cleanly. Restarting the
conductor did not help.

**Root cause (read in source).** `install_app` with `agent_key: Some(k)` checks the key is in the
keystore with `self.keystore.list_public_keys()` (`elohim/holochain-conductor/crates/holochain/src/conductor/conductor.rs:1330`),
which calls lair `list_entries()` (`crates/holochain_keystore/src/meta_lair_client.rs:247`) and
gets EVERY entry back in one message. lair's encrypted stream caps a frame at 8 KiB
(`lair_keystore_api-0.7.1/src/sodium_secretstream.rs` `MAX_FRAME = 1024 * 8`). Uninstalling a
hosted cell never deletes its seed, so a hosting conductor's seed count only grows; past the
point where the listing exceeds 8 KiB, every keyed install fails, forever. The doorway always
supplied a key it had just generated, so it walked straight into this.

**Fixed here (doorway, landed with this shift).** `TypedAdminClient::install_app_minting_key`
installs with `agent_key: None` and reads the minted key from the returned `AppInfo`; the
provisioner uses it. A conductor-minted key skips the listing. Verified live: matthew's
conductor hosted again immediately, and the ci-runner lane passed twice in a row afterwards.

**Still open.**
1. **The fleet.** alpha doorways host up to 32–50 agents per conductor and churn
   (reinstalls, prologue re-runs). Any hosting pod past the threshold refuses every new hosted
   human until a doorway with this fix deploys. Check the alpha pods' install errors for lair
   `ConnectionAborted` before assuming anything else.
2. **The fork.** `list_public_keys` should not be one unbounded message: page it, or confirm a
   single key with a per-key lookup (e.g. a probe signature). Also seeds are never reclaimed on
   uninstall — a lifecycle gap of its own. Owner: `elohim/holochain-conductor`.
3. **Fault classification.** `provisioner.rs` `is_transport_fault` reads this lair failure's
   `ConnectionAborted` as a dropped admin socket and re-offers the install to "another
   conductor"; it is a keystore verdict, not a transport fault.
