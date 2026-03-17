#!/usr/bin/env bash
set -euo pipefail

# research.sh — Hydrate/dehydrate research repos for deep-context work.
#
# Usage:
#   ./ekklesia/research.sh status    # Show what's cloned vs available
#   ./ekklesia/research.sh clone     # Clone all repos from manifest
#   ./ekklesia/research.sh clone polis  # Clone specific repo
#   ./ekklesia/research.sh clean     # Remove all cloned repos (reclaim space)
#   ./ekklesia/research.sh clean polis  # Remove specific repo
#   ./ekklesia/research.sh pull      # Pull latest on all cloned repos
#   ./ekklesia/research.sh size      # Show disk usage of research repos

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
MANIFEST="$SCRIPT_DIR/research-manifest.json"

if ! command -v jq &>/dev/null; then
  echo "error: jq is required. Install with: apt install jq / brew install jq"
  exit 1
fi

if [[ ! -f "$MANIFEST" ]]; then
  echo "error: manifest not found at $MANIFEST"
  exit 1
fi

count=$(jq '.repos | length' "$MANIFEST")

status() {
  echo "Research repos ($count in manifest):"
  echo ""
  for i in $(seq 0 $((count - 1))); do
    name=$(jq -r ".repos[$i].name" "$MANIFEST")
    path=$(jq -r ".repos[$i].path" "$MANIFEST")
    relevance=$(jq -r ".repos[$i].relevance" "$MANIFEST")
    pillar=$(jq -r ".repos[$i].pillar" "$MANIFEST")
    full_path="$REPO_ROOT/$path"

    if [[ -d "$full_path/.git" ]]; then
      size=$(du -sh "$full_path" 2>/dev/null | cut -f1)
      branch=$(git -C "$full_path" branch --show-current 2>/dev/null || echo "detached")
      echo "  ✓ $name ($size, $branch) [$pillar]"
    else
      echo "  ○ $name (not cloned) [$pillar]"
    fi
    echo "    $relevance"
    echo ""
  done
}

clone_repo() {
  local filter="$1"
  for i in $(seq 0 $((count - 1))); do
    name=$(jq -r ".repos[$i].name" "$MANIFEST")
    url=$(jq -r ".repos[$i].url" "$MANIFEST")
    path=$(jq -r ".repos[$i].path" "$MANIFEST")
    full_path="$REPO_ROOT/$path"

    if [[ -n "$filter" && "$filter" != "$name" ]]; then
      continue
    fi

    if [[ -d "$full_path/.git" ]]; then
      echo "  skip: $name (already cloned at $path)"
    else
      echo "  clone: $name → $path"
      mkdir -p "$(dirname "$full_path")"
      git clone --depth 1 "$url" "$full_path"
      echo "  done: $name"
    fi
  done
}

clean_repo() {
  local filter="$1"
  for i in $(seq 0 $((count - 1))); do
    name=$(jq -r ".repos[$i].name" "$MANIFEST")
    path=$(jq -r ".repos[$i].path" "$MANIFEST")
    full_path="$REPO_ROOT/$path"

    if [[ -n "$filter" && "$filter" != "$name" ]]; then
      continue
    fi

    if [[ -d "$full_path" ]]; then
      size=$(du -sh "$full_path" 2>/dev/null | cut -f1)
      echo "  clean: $name ($size) at $path"
      rm -rf "$full_path"
      echo "  done: $name removed"
    else
      echo "  skip: $name (not cloned)"
    fi
  done
}

pull_repos() {
  for i in $(seq 0 $((count - 1))); do
    name=$(jq -r ".repos[$i].name" "$MANIFEST")
    path=$(jq -r ".repos[$i].path" "$MANIFEST")
    full_path="$REPO_ROOT/$path"

    if [[ -d "$full_path/.git" ]]; then
      echo "  pull: $name"
      git -C "$full_path" pull --ff-only 2>/dev/null || echo "    (pull failed — may need manual intervention)"
    else
      echo "  skip: $name (not cloned)"
    fi
  done
}

show_size() {
  total=0
  for i in $(seq 0 $((count - 1))); do
    name=$(jq -r ".repos[$i].name" "$MANIFEST")
    path=$(jq -r ".repos[$i].path" "$MANIFEST")
    full_path="$REPO_ROOT/$path"

    if [[ -d "$full_path" ]]; then
      size=$(du -sh "$full_path" 2>/dev/null | cut -f1)
      bytes=$(du -sb "$full_path" 2>/dev/null | cut -f1)
      total=$((total + bytes))
      echo "  $name: $size"
    fi
  done
  if [[ $total -gt 0 ]]; then
    echo ""
    echo "  total: $(numfmt --to=iec $total 2>/dev/null || echo "${total} bytes")"
  else
    echo "  (no repos cloned)"
  fi
}

case "${1:-status}" in
  status) status ;;
  clone)  clone_repo "${2:-}" ;;
  clean)  clean_repo "${2:-}" ;;
  pull)   pull_repos ;;
  size)   show_size ;;
  *)
    echo "Usage: $0 {status|clone [name]|clean [name]|pull|size}"
    exit 1
    ;;
esac
