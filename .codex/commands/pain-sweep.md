---
description: On-demand pain sweep — dispatch the algedonic-designer to sense burden no declared measure sees and register at most three sensors for it.
argument-hint: "[--senses waiting,repetition,...] [--window-days 2-14] [--seed N]"
---

# /pain-sweep

Dispatches the `algedonic-designer` subagent (Opus) for one firing. It reads raw pre-measure
exhaust through its nerve endings, skips every pain a declared measure already covers, and
registers at most three sensors, each a 4-tuple: measure id@version · bound · concern address ·
reader. It never fixes anything, never files a report, and never guesses an address.

This is the on-demand entry. The same persona also runs as the `pain-sweep` station of
`/delivery-stasis` (when the conveyor has room) and as a daily surprise-auditor routine that
fires with probability 0.2. It never runs from a hook or at SessionStart.

## Steps

1. **Draw.** Use the senses, window and seed the arguments name, if any. Otherwise draw fresh
   ones: a random seed, 3 of the 7 nerve endings (waiting, repetition, effort with no fruit,
   interruption, contradiction, apology and rant, numbness) and a look-back window of 2–14 days:
   ```bash
   python3 -c 'import random,secrets; s=secrets.randbits(32); r=random.Random(s); n=["waiting","repetition","effort-with-no-fruit","interruption","contradiction","apology-and-rant","numbness"]; print(s, ",".join(sorted(r.sample(n,3), key=n.index)), r.randint(2,14))'
   ```
2. **Dispatch** the `algedonic-designer` agent. The prompt carries only the draw: `seed=<s>
   senses=<a,b,c> window_days=<d> origin=pain-sweep`. The persona holds its own method, so do not
   restate it or hint at which pain to find.
3. **Relay** what it returns: the draw, each minted sensor's 4-tuple and fold, the skipped
   candidates with the measure that already covers each, and any `address=absent` pains. Add
   nothing of your own. A firing that mints nothing is a valid result.
