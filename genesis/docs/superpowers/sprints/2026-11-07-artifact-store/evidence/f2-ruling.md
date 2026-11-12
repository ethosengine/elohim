---
id: f2-astra-rereview-ruling
cites: []
---
# F2 after the astra re-review: orchestrator ruling (2026-11-08)

Reviewer: Codex gpt-6-astra, evidence/f2-astra-rereview.md (BLOCK, 13 items). Ruling by the sprint orchestrator (Claude Opus 5.5) under the operator's latitude and the standing Sybil ruling.

## Threat model the ruling applies
F2 is a developer tool on the steward's own workspaces. A process running as the same account already holds every capability these attacks would win: it can write `mesh-bin` directly, edit the sources and `~/.cargo`, write brit notes, and read or write the fetch queue. F2 cannot defend what the account already controls, and the first review scoped item 5 the same way ("not an established cross-UID privilege escalation"). Same-account findings are therefore named residuals, not gates. What still gates: an honest build reusing the wrong bytes under one key, and anything a remote holder can do.

Admitted holders today are only the steward's own devices (the tier's devices gate; the record crosses only the steward channel). A malicious admitted builder is a compromised device of the same principal. Independent computation evidence is the release plane's job (F5/I6, plan §8 out of scope). Item 10 is a named frontier, not a gate.

## Per item
| # | Item | Class | Ruling |
|---|---|---|---|
| 1 | H1 in-place overwrite / rename race | same-account | residual |
| 2 | H2 forged Installed, cancel entry identity | same-account | residual |
| 3 | H3 PATH and native tools unkeyed | honest build | **FIX**: fixed PATH in the allowlist, recorded in the descriptor; resolved linker/cc path plus its version output in the key |
| 4 | H4 edit-and-restore, node_modules | same-account / honest-rare | residual (an edit not restored is caught by the dirty check) |
| 5 | H5 parent symlink race, queued selection trusted | same-account | residual |
| 6 | M6 sync publish after deadline, withdraw race | operational / same-account | residual |
| 7 | M7 undercount on outer deadline | operational | follow-up |
| 8 | M8 relay class by advertised address | operational | follow-up (needs the connection's actual path) |
| 9 | M9 missing marker fails open | honest misconfig | **FIX**: serving lock held and marker missing or unreadable refuses |
| 10 | residual 5, admitted builder fabricates | same principal | frontier F5/I6 |
| 11 | 20 ms window misclassifies | operational | **FIX**: name the bound that expired, no tolerance window |
| 12 | crash-orphaned staging | operational | **FIX**: sweep this destination's stale staging dirs before a new install |
| 13 | residual 4, no real build under env -i | validation | carried to F2r (first real build) |

Close: the four fixes, then the orchestrator's delta review of them (not a third astra round). Residuals 1, 2, 4, 5, 6, 10 and follow-ups 7, 8 go to the report and the backlog.
