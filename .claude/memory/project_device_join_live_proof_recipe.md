---
index: false
name: project-device-join-live-proof-recipe
title: "Device-join live proof recipe (i1006 approver, generated as…"
description: "How the one-command device join was proven live 2026-10-07 — i1006 workspace approves, a generated sandbox asks; what each failure meant. Bites on any device-ceremony proof."
id: project-device-join-live-proof-recipe
metadata:
  node_type: memory
  title: "Device-join live proof recipe (i1006 approver, generated asker)"
  type: project
  originSessionId: b792cf88-2ad3-4526-8b27-3869349776fe
  modified: 2026-10-07T16:57:27.062Z
cites:
  - elohim/eprfs/epr-cli/src/device/
  - "one-command-device-join-design | One command on the new device, one yes on a device that is already yours | sha256:f0cebf3702df05b1 | path: genesis/docs/superpowers/specs/2026-10-07-one-command-device-join-design.md"
---

Proving `epr device join` → `epr device approve --yes` live on one machine (2026-10-07):

- **The che workspace node (`:8095`) holds no person** (`/auth/identity/standing` → `hasIdentity:false`):
  Matthew's alpha identity was minted by the doorway, so on alpha only the doorway speaks for him.
  Don't use che as an approver. The **i1006 workspace peer** (`genesis/local-dev/i1006`, storage
  `:8093`, conductor admin 42201/app 4475) speaks for "Matthew" on the i1006 network and approves.
- **A "device" is a conductor agent key**, not a storage identity.key: a second storage dir on the
  same conductor is the same device. A fresh device = `hc sandbox generate` in
  `genesis/local-dev/i1006/conductors` (`-f <admin> … -d <name> <happ> network --bootstrap
  http://localhost:8888/bootstrap quic http://localhost:3340/`), then `hc sandbox run <idx> -p=<app>`
  with `echo test |` piped (never `< /dev/null` — lair needs the passphrase), then storage from
  `storage-restart/matthew.environ` with STORAGE_DIR/HOLOCHAIN_*_URL/P2P_PORT overridden and
  SELF_HUMAN_ID/GENESIS_SELF_HEAL_IDENTITY/AGENT_PUBKEY dropped.
- **The packed happ must carry current coordinators**: `sign_device_enrollment` (mishpat) and
  `get_human_root_evidence` (imagodei) "zome function doesn't exist" = stale `workdir/*.dna`.
  `just pack` in each DNA dir, then `hc app pack dna/elohim/workdir -o …/elohim.happ`. The storage's
  `--happ-path` hot-swap did not fire for an external conductor within the proof window.
- **"identity commitment unavailable" = no network**: i1006 conductors home to bootstrap `:8888`
  (served by a doorway: `doorway --dev-mode --listen 0.0.0.0:8888 --bootstrap-enabled …` runs
  without mongo) and iroh-relay `:3340` (`iroh-relay --dev -c i1006/iroh-relay.toml`). The first
  enroll after a conductor connects can still be early; the kept consent + `join` again covers it.
- `pkill -f '<pattern>'` kills the Bash tool's own shell (exit 144); use `pgrep -f '[p]attern'`.

**Why:** three hours of the proof went to these environment facts, none of which were code gaps.
**How to apply:** reuse this topology for any device-ceremony proof; see
`genesis/docs/superpowers/specs/2026-10-07-one-command-device-join-design.md` for the design and
`genesis/local-dev/testdrive-20261007/` for the logs. Related: [[project_campaign14_leg2_lineage_and_root_author]].
