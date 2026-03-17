# Ekklesia — Governance Research & References

Deep-context research repos for the qahal governance pillar. Cloned on demand, gitignored.

## Usage

```bash
./ekklesia/research.sh status          # Show what's cloned vs available
./ekklesia/research.sh clone           # Clone all repos from manifest
./ekklesia/research.sh clone polis     # Clone specific repo
./ekklesia/research.sh clean           # Remove all (reclaim space)
./ekklesia/research.sh clean polis     # Remove specific repo
./ekklesia/research.sh pull            # Pull latest on cloned repos
./ekklesia/research.sh size            # Show disk usage
```

## Adding a new research repo

Edit `research-manifest.json`:

```json
{
  "name": "repo-name",
  "url": "https://github.com/org/repo.git",
  "path": "ekklesia/research/repo-name",
  "relevance": "Why this matters for the protocol",
  "pillar": "qahal|lamad|shefa|imagodei|elohim"
}
```

Then run `./ekklesia/research.sh clone repo-name`.

## Current repos

| Repo | Pillar | Relevance |
|------|--------|-----------|
| `compdemocracy/polis` | qahal | Sensemaking algorithm reference — clustering, PCA, consensus, bridging statements |
