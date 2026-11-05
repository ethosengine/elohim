---
epr-habit-version: 1
id: rust-style
invariant: >
  The native Rust style gate refuses new unexplained lint suppressions, requires one
  finding of paydown from each touched indebted crate after enrollment, and preserves
  zero-debt strict module scopes. A pushed commit cannot borrow a dirty working-tree repair.
status: green
active: false
checks:
  - "just gate rust-style (native syntax, Git ratchet, strict scopes, checker fmt/Clippy/tests and agent adapter contracts)"
  - "a2o @concern:rust-style: cd genesis/a2o && pnpm exec cucumber-js --profile rust-style (six isolated-Git acceptance scenarios, including independent addressed-policy holdings)"
  - "node --test genesis/orchestrator/rust-toolchain-pin.test.mjs (CI/local checker toolchain agreement)"
guard: >
  Never equate equal counts with no new findings; never use dirty source for a pushed object;
  never call a missing baseline or a stale checker clean. Scope and owner moves cannot transfer
  debt, and lint/configuration edits must select the gate. Attribute reason presence does not
  establish justification or certify all guide rules. Runtime trust and remote CI execution
  remain separate from local contract evidence.
refs:
  - "guide: genesis/docs/rust-style-guide.md"
  - "checker: genesis/orchestrator/rust-style/"
retire-when: >
  when all first-party crates have zero active-rule debt and the ordinary compiler/governance
  gates enforce these same properties without a separate Git debt ratchet
---

DELTA 2026-10-22 (GREEN, local enforcement contract): `just gate rust-style` passed with
1,901 Rust files in 79 crates and 411 inherited RS-GOV-04 findings, zero new findings;
the checker itself has six strict files and zero debt. Eight new attributes in current work
were supplied with their existing design rationale, with broader decisions recorded locally.
Native checker: 18 tests passed, fmt and Clippy `-D warnings` passed. Shared agent adapter:
seven tests passed, including Codex freeform events and first-build CI baseline refusal.
Live Write probes denied an unexplained suppression and permitted an explained one; the
native `epr govern` provider independently returned `refuse` then `permit`. A2O: three
scenarios/nine steps passed. Policy pins: 25 verified; validator accounting: 11 assertions;
package projections: 2,194 checks passed. The first policy enrollment preserves no-new checks;
ordinary per-touch paydown begins when enrollment is in the integration baseline. CI is wired
before downstream dispatch; this delta does not claim a remote Jenkins run.

Story review cost: three fresh blind-reader rounds; correctness findings 2, 1, 0;
interpretability findings 0, 1, 0; preference findings 0, 0, 0. All findings resolved;
final reader READY. The stories now distinguish new strict-module debt from inherited
crate debt and explicitly establish the accepted repair as the next integration baseline.

DELTA 2026-10-22 (final hardening): `just gate rust-style` remains GREEN with 19 native
tests, including new-child strict-scope shadowing refusal, plus seven adapter/baseline tests.
All 15 repository-validator tests and the native host's Clippy `-D warnings` pass. The tested
`epr` executable is installed on the shared PATH; live strict refuse/permit probes pass through
that installed host. Build-manifest validation reports 24 manifests, 45 steps, zero errors.
Durable local receipts: `genesis/a2o/reports/recovery/rust-style/`. The feature-branch CI checkout
now explicitly fetches its integration ref; no history or hook-availability gap is called clean.

DELTA 2026-10-24 (ownership diagnostic): `python3 -m unittest discover -s genesis/orchestrator/rust-style -p hook_test.py` passed 8 tests; normal and `--hook` entrypoints now name the safe.directory cure and exit 2 before pool/Cargo on Git ownership refusal. No Cargo build or full native gate was run in this bounded fix; habit status unchanged.

DELTA 2026-11-02 (portable discipline, partial delivery; status unchanged): 35 metadata
tests and 4 portable CLI tests pass, including independent local standing, explicit
successor adoption and corrupt/missing-policy refusal; touched-package Clippy and
workspace formatting pass. Rust-style recipe passes 19 native and 8 adapter tests,
2,088 files / 84 crates; publisher tests pass 32 cases and precheck verifies existing
packages with the changed metadata version absent. Independent source review found
no remaining local correctness blocker. Two fresh repository representatives reached
a conditional scoped trial recommendation (task-srd-4-report.md), not Brit adoption.
Full gates refuse an unrelated duplicate manifest before recipes; standalone checker
publication/install, full discipline closure and network authority/witnessing remain
open. No commit, push, remote acceptance or new strict scope is claimed.

DELTA 2026-11-02 (GREEN, local shared-discipline trial): normal Rust-style, orchestrator and EPRFS gates pass; Rust-style has 30 native / 9 hook / 4 distribution tests and scans 2,090 files in 84 crates; EPRFS has 1,970 passing Rust tests, zero failures or ignored tests, with workspace fmt/Clippy green. Six a2o scenarios / 18 steps pass. Brit's actual standalone offline install verifies the same policy/guide/method CIDs as Elohim, and its independently selected `brit-build-ref` trial measures 14 files with zero debt; committed fixtures refuse new suppressions, dirty repairs and corrupt holdings. Both story files had two fresh-reader rounds, each 0/3/0 then 0/2/0 correctness/interpretability/preference findings, final READY; remaining interpretation findings are retained in task-srd-5-report.md. Durable proof: genesis/a2o/reports/recovery/shared-rust-discipline-2026-11-02/. Network authority/witnessing and scope/checker migration remain open; no commit, push or remote CI claimed. The 72-minute EPRFS gate leaves performance finding 330923a9dd23 open.

DELTA 2026-11-03 (integration): upstream dev reconciled; published eprfs-meta 0.3.3 collision detected by the precheck, assigned 0.3.4 and regenerated the addressed checker method/source without overwriting the prior holdings. Full Rust-style gate passes on 2,092 files / 84 crates, six a2o scenarios / 18 steps pass, publisher precheck and 32 publisher tests pass. Proof: genesis/local-dev/integration-20261103/. Network authority and the separate native-source sprint remain unclaimed.

DELTA 2026-11-03 (reach correction): the pre-push seam-matrix harness exposed the unmapped eprfs-meta registry and an upstream Python test citation outside the registry schema. Routed portable policy evaluation to the governing/meta column and cited the existing Rust integration wrapper for the Python suite; all 38 matrix assertions pass. Evidence: genesis/local-dev/integration-20261103/seam-matrix-repaired.log. No policy or licensing restriction changed.

DELTA 2026-11-03 (native sprint integration): EPRFS core 0.3.3 requires the checker's exact dependency pin to advance; rebuilt and re-exported the complete method/source closure while retaining prior addressed holdings and the same guide/rules. The normal Rust-style gate passes on the combined tree, including the Windows storage branch; publisher precheck preserves existing immutable packages and 36 publisher tests pass. Evidence: genesis/local-dev/integration-20261103/native-rust-style-gate.log and native-checker-discipline.json.

DELTA 2026-11-05 (status unchanged; a freeze and a false green found): the adopted method hashes eprfs-core and eprfs-meta source, and v1 refuses replacing adopted content, so any edit to those crates is UNMEASURED with no path forward. A rust-style PASS on an integration branch that changed eprfs-core was false: the shared family/native target slot embeds absolute include paths into whichever checkout built it, so the checker hashed the main checkout's sources. A migration station that admitted "dependency-only" method changes was rejected by adversarial review (the checker executes that code; Cargo.toml can swap the entry point). Steward ruling: decouple rust-style onto the published eprfs-core/eprfs-meta and land it as one recorded bootstrap override. Until then Track A (eprfs-core) and the eprfs-meta integrity lint are held on native-dev/integrate.

DELTA 2026-11-05 (status unchanged; one-time registry-decoupled bootstrap, steward-authorized override): the checker no longer compiles the tree it polices. It builds published, checksummed eprfs-core 0.3.3, eprfs-meta 0.3.4 and seam-contracts 0.1.2 from the elohim registry, declares `build = false`, and its syntax scanner is extracted into the path-only `elohim-rust-style-scan` crate with no EPRFS or seam-contract dependency; epr-cli now consumes that scanner directly, so its workspace graph never crosses into the registry-backed checker. The adopted method inventories its nine checker inputs, including the scan crate's manifest and source (policy bafkreib6zgok5h2gtmoz2zipvssjjzlzml2j53vu2xua3f6qevtbednsfy, method bafkreif2lxmrsvrqy2e3umw2auo5gq7oxkz3r6aqnwr2salcivq367uudm; guide unchanged; landed old objects retained and the two unlanded intermediate objects replaced). The wrappers enter the repository root, refuse legacy and nested Cargo configuration, pin the compiler and wrappers, and wire anonymous registry reads for a cold cache. Both the old and new checkers refuse this replacement by design, so this single commit lands with hooks bypassed under steward human:matthew's covenant clause bafyreib72ths5psd6mpiowmhsf7v5sr6akr3jkr2lf3a5xmlflbaaryspi; its CI rust-style check is expected red once. Probe: cargo 32, hook 15 and distribution 4 tests, workspace fmt and Clippy `-D warnings`, epr-cli Clippy, credential-free offline tree resolution, and all 1,301 epr-cli tests after building their declared `bin-launch` fixture in the same target; the package test used no registry credentials. Three gpt-6-astra reviews covered the prior bootstrap (final finding, nested-config linker, fixed with an adversarial marker-linker test). After it, an edit confined to eprfs-core or eprfs-meta no longer changes the method. Open: effective Cargo configuration accounting (workspace-discipline row 84).
