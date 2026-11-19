# One-baseline sprint: receipts

Kept on this hand-over ref so the journal's claims can be checked without either device's scratch.

| File | What |
|---|---|
| `follow-unmarked.*` | line 2: shem's `EPR_FOLLOW_SIBLING=1 epr bin follow --tool epr` at `71069b1f5` on its real store; the refusal of ethosengine's unmarked release, then the result of shem's own build |
| `codex-fleet-roll-review.receipt.md` | first independent review of the fleet-roll check (`2080441f5..82993d1b3`): two reproduced false passes |
| `codex-fleet-roll-rereview.receipt.md` | second review (`82993d1b3..bb2a6ab74`): the answer depended on the developer's checkout |
| `codex-fleet-roll-final.receipt.md` | final check (`bb2a6ab74..220b5642f`): twelve probes right; a hidden-submodule diff setting, shallow history, edge cases |
| `codex-e11-view-review.receipt.md` | first review of the E11 view rework (`7edd733a0..f6a22524f`) |
| `codex-e11-view-rereview.receipt.md` | second review (`6e1c98cd2..e7450722e`) |

The fleet-roll check landed on `dev` as `0f48e02fb`, squashed from `self/nt-fleet-roll` (`b4e451a14`, fourteen commits),
after a bounded last round accepted against a fixed checklist rather than a fourth review.

Added later the same day:

| File | What |
|---|---|
| `codex-l2a-review.receipt.md` | review of the producer half (`self/nt-l2a` `22a337c18`): the marker was not bound to the C that was linked |
| `codex-e11-view-final.receipt.md` | narrow check of the E11 view after the receipt file was removed: the exit-code protocol could not carry two facts |
| `codex-e11-view-final2.receipt.md` | narrow re-check of the per-attempt result design; closed by ruling |
| `codex-brit-readonly-review.receipt.md` | review of the brit read-only extension (also beside its patches) |

The E11 rework landed on `dev` as `0fd724eb2`, squashed from `self/nt-e11-view` (`b916cd1fe`, sixteen commits); the squash's
tree is identical to that head's.

