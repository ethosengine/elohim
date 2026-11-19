# Review receipts, native-throughput sprint (2026-11-18/19)

Raw outputs of the reviews run from shem. Each JSON is one fan-out's result: findings carry
`where` (file:line), `evidence` (the skeptic's verification) and a proposed `fix`. Read them
selectively with python or jq. They describe the tree at the commit each names, not dev today.

| File | What |
|---|---|
| `wf-row29-2-decision.result.json` | row 29.2 decision panel: `ground` (six fact finders), `options` (five proposals, fifteen critiques), `rankings`, `memo` |
| `wf-row28-audit.result.json` | reads the reuse-enrolled Rust gates perform outside their key: `confirmed` (53), `refuted` (34), `critic` |
| `wf-e11-review.result.json` | review of `self/nt-e11` 9c203f96: `blocking`, `followUp`, `refuted`, `design` |
| `wf-e10-multilens.result.json` | eight-lens review of `claude/nt-isa` baa210967 (branch not landing): `survivors`, `refuted` |
| `codex-e10-*.receipt.md` | two earlier independent reviews of the same branch |
| `codex-r5-third.receipt.md` | third review of the first-epr verifier (`claude/nt-r5-first-epr`) |

This branch is a hand-over ref for reading. It is not meant to merge.
