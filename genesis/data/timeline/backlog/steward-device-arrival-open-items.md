

## Row 61, delta (2026-11-21): a fenced hand-held lease is handed back and claimed afresh

The cause is in berth's settle step (`_settle_steward` in `genesis/agentic/bin/berth`): the
one-device override that `--offline-ok` writes is taken only for a lease that is not yet fenced.
A lease claimed by hand before hours of gates is already past its fence window when the hook
renews it, so the renewal is refused where a fresh claim would have written the override and
waited for it. berth's behaviour is unchanged.

The cure is the first of the three owed, in the hook: when the pushing session's renewal is
refused with exit 3 and berth's reason is `self-demoted: no head from …`, `phase_l_claim`
releases the session's lease, says so in one line, and lets the landing agent claim afresh under
`--offline-ok`; the session does not hold the lease afterwards. A failed release refuses with
both reasons. Every other refusal of a renewal reads as before. The match on berth's phrase
lives in one function (`phase_l_fenced_reason`), and `genesis/agentic/berth_test.py` already pins
the phrase. Three tests in `genesis/orchestrator/prepush-berth.test.mjs` drive it against the
fake berth: the order claim, release, claim; a failed release; another device holding the lease.

Not observed against a real lease: the tests fake berth. The other two cures stay open, and the
second (a device with no live session still sends a head inside the fence window) is the one
that would also keep an idle device visible as a carrier.
