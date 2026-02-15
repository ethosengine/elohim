#!/bin/bash
# Download bundled fonts for offline capability
# Fetches Material Icons, Font Awesome 6.4.0, and Google Fonts (Noto Sans, Source Sans 3)
# Can be run from anywhere; finds paths relative to script location

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
FONTS_DIR="$PROJECT_ROOT/elohim-app/src/assets/fonts"

FA_VERSION="6.4.0"
UA="Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/120"

# ── Material Icons ──────────────────────────────────────────────────
fetch_material_icons() {
  local dir="$FONTS_DIR/material-icons"
  if [ -f "$dir/material-icons.woff2" ]; then
    echo "Material Icons: already present"
    return
  fi
  echo "Material Icons: downloading..."
  mkdir -p "$dir"
  curl -sL 'https://fonts.gstatic.com/s/materialicons/v145/flUhRq6tzZclQEJ-Vdg-IuiaDsNc.woff2' \
    -o "$dir/material-icons.woff2"
  cat > "$dir/material-icons.css" <<'CSS'
@font-face {
  font-family: 'Material Icons';
  font-style: normal;
  font-weight: 400;
  src: url(material-icons.woff2) format('woff2');
}
.material-icons {
  font-family: 'Material Icons';
  font-weight: normal;
  font-style: normal;
  font-size: 24px;
  line-height: 1;
  letter-spacing: normal;
  text-transform: none;
  display: inline-block;
  white-space: nowrap;
  word-wrap: normal;
  direction: ltr;
  -webkit-font-feature-settings: 'liga';
  -webkit-font-smoothing: antialiased;
}
CSS
  echo "Material Icons: done"
}

# ── Font Awesome 6.4.0 ─────────────────────────────────────────────
fetch_fontawesome() {
  local css_dir="$FONTS_DIR/fontawesome"
  local wf_dir="$FONTS_DIR/webfonts"
  if [ -f "$wf_dir/fa-solid-900.woff2" ]; then
    echo "Font Awesome: already present"
    return
  fi
  echo "Font Awesome $FA_VERSION: downloading..."
  mkdir -p "$css_dir" "$wf_dir"
  curl -sL "https://cdnjs.cloudflare.com/ajax/libs/font-awesome/$FA_VERSION/css/all.min.css" \
    -o "$css_dir/all.min.css"
  local fonts=(
    fa-brands-400.woff2 fa-brands-400.ttf
    fa-regular-400.woff2 fa-regular-400.ttf
    fa-solid-900.woff2 fa-solid-900.ttf
    fa-v4compatibility.woff2 fa-v4compatibility.ttf
  )
  for font in "${fonts[@]}"; do
    curl -sL "https://cdnjs.cloudflare.com/ajax/libs/font-awesome/$FA_VERSION/webfonts/$font" \
      -o "$wf_dir/$font"
  done
  echo "Font Awesome $FA_VERSION: done"
}

# ── Google Fonts (Noto Sans + Source Sans 3, Latin/Latin-ext/Vietnamese) ─
fetch_google_fonts() {
  local dir="$FONTS_DIR/google"
  if [ -f "$dir/google-fonts.css" ] && [ "$(find "$dir" -name '*.woff2' | wc -l)" -ge 12 ]; then
    echo "Google Fonts: already present"
    return
  fi
  echo "Google Fonts: downloading..."
  mkdir -p "$dir"

  # Fetch the CSS with a modern UA to get woff2 format
  local css
  css=$(curl -s \
    'https://fonts.googleapis.com/css2?family=Noto+Sans:ital,wght@0,100..900;1,100..900&family=Source+Sans+3:ital,wght@0,200..900;1,200..900&display=swap' \
    -H "User-Agent: $UA")

  # Extract and download only latin, latin-ext, and vietnamese subset files
  # Use grep to find the font URLs following subset comments
  local urls
  urls=$(echo "$css" | grep -A8 -E '/\* (latin-ext|latin|vietnamese) \*/' \
    | grep -oP 'url\(\Khttps://[^)]+')

  for url in $urls; do
    local fname
    fname=$(basename "$url")
    curl -sL "$url" -o "$dir/$fname"
  done

  # Generate local CSS: keep only latin/latin-ext/vietnamese blocks,
  # rewrite URLs to local filenames
  local in_block=0
  while IFS= read -r line; do
    if echo "$line" | grep -qP '/\* (latin-ext|latin|vietnamese) \*/'; then
      in_block=1
      echo "$line"
    elif echo "$line" | grep -qP '/\* [a-z]'; then
      in_block=0
    elif [ "$in_block" -eq 1 ]; then
      echo "$line" | sed 's|url(https://[^)]*/\([^)]*\))|url(\1)|g'
    fi
  done <<< "$css" > "$dir/google-fonts.css"

  echo "Google Fonts: done ($(find "$dir" -name '*.woff2' | wc -l) files)"
}

# ── Main ────────────────────────────────────────────────────────────
echo "Fetching bundled fonts for offline capability..."
fetch_material_icons
fetch_fontawesome
fetch_google_fonts
echo "All fonts ready in $FONTS_DIR"
