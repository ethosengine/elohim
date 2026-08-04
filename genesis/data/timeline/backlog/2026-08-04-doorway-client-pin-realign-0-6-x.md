---
id: "backlog-doorway-client-pin-realign-0-6-x"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Realign doorway-service holochain client pins from the unreleased 0.9.0-dev/0.7.0-dev family to the 0.8.3/0.6.3 stable family matching the 0.6.x conductor"
slug: "doorway-client-pin-realign-0-6-x"
written: "2026-08-04"
author: "holochain-iroh convergence campaign (Wave 1 Lane C)"
status: "backlog"
priority: "medium"
tags: [doorway, holochain-client, dependency-pins, wire-format, wave-1, codex-claimable]
cites:
  - genesis/docs/superpowers/plans/2026-08-04-holochain-iroh-convergence-upgrade-campaign.md
---

# Doorway client-pin realign + 0.6.1 API-reshape audit (Wave 1, Lane C — claimable by any agent)

Task C1 of the convergence campaign plan (see cite — its Global Constraints govern).
Write-set: `doorway/doorway-service/**` only — disjoint from Lanes A/B/D.

## Context you need (no session context assumed)

- `doorway/doorway-service/Cargo.toml:32-45` currently pins `holochain_client 0.9.0-dev.5`, `holochain_zome_types 0.7.0-dev.5`, `holochain_types 0.7.0-dev.11`, `holochain_websocket 0.7.0-dev.11`, `holochain_conductor_api 0.7.0-dev.11` — the UNRELEASED 0.7 wire-protocol family, against a conductor that is 0.6.x. Discovered 2026-08-04; it works today by narrowness of the calls, unverified.
- Correct stable pairing for a 0.6.3 conductor: `holochain_client = "0.8.3"`, and `holochain_zome_types` / `holochain_types` / `holochain_websocket` / `holochain_conductor_api` = `"0.6.3"`.
- The 0.6.1 Rust client reshaped `AppInfoStatus` → a new `AppStatus` union (removed "paused"/"running" cases; added "enabled" and nested "disabled" reasons) and `get_agent_activity` gained a `GetOptions` parameter returning `Vec<SignedWarrant>` — compile-driven fixes may be needed.
- Do NOT rename `CapAccess`→`CapAccessType` (`src/conductor/typed_admin.rs:18,224`): that is a 0.7-line change; the 0.8.3 client still uses `CapAccess`. Add a `// 0.7 migration:` breadcrumb comment only.

## Steps

1. Inventory before bumping: `grep -rn "AppInfoStatus\|AppStatus\|Paused\|paused" doorway/doorway-service/src/`.
2. Bump the five pins; `RUSTFLAGS="" cargo build --release 2>&1 | head -40; echo EXIT=$?` (set `CARGO_TARGET_DIR` to the doorway pool slot from the session preflight — never bare `target/`); fix compile-driven.
3. Gates: `RUSTFLAGS="" cargo test --lib --bins 2>&1 | tail -20; echo EXIT=$?` · `cargo clippy -- -D warnings 2>&1 | tail -5; echo EXIT=$?` · `cargo fmt --check; echo EXIT=$?`.
4. If the 0.6.x family fails to resolve against another doorway dep, STOP, revert, and report the exact conflict — an explicit documented hold is a valid outcome; a silent partial bump is not.

## DoD

All gates EXIT=0 with output pasted (or the documented-conflict report), committed path-limited to `doorway/` on a work branch. Commit-only; the orchestrating session reviews before integration (done = composes, not compiles).
