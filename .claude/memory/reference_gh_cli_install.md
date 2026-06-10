---
name: reference_gh_cli_install
description: GitHub CLI is NOT preinstalled in this container; how to restore it and which token is set
metadata: 
  node_type: memory
  type: reference
  originSessionId: 8923b36c-6604-4b09-8e13-c4bd730428cd
---

`gh` is **not preinstalled** in the Eclipse Che dev container (no `apt`/`brew`; it's Fedora/RHEL-based with `dnf`/`yum`, and `gh` is NOT a pnpm/npm package — it's a Go binary). It "works at one time" then vanishes because containers are ephemeral.

**Restore (no root needed):** download the release tarball and drop the binary into `/home/user/bin` (on PATH, writable):
```bash
VER=$(curl -s -H "Authorization: Bearer $GH_TOKEN" https://api.github.com/repos/cli/cli/releases/latest | python3 -c "import json,sys;print(json.load(sys.stdin)['tag_name'].lstrip('v'))")
curl -sL "https://github.com/cli/cli/releases/download/v${VER}/gh_${VER}_linux_amd64.tar.gz" | tar xz -C /tmp
cp /tmp/gh_${VER}_linux_amd64/bin/gh /home/user/bin/gh && chmod +x /home/user/bin/gh
```

**Auth:** `GH_TOKEN` env var is already set — `gh` picks it up automatically (no `gh auth login`). It authenticates as the **EthosengineBot** account (classic PAT, scopes incl. `repo`, `admin:org`, `workflow`), so `gh issue close`/`comment`, PR ops, and org actions all work and appear authored by the bot. The repo is `ethosengine/elohim`. The raw GitHub REST API via `curl -H "Authorization: Bearer $GH_TOKEN"` works without installing anything.
