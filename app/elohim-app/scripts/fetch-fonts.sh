#!/bin/bash
# Download bundled fonts for offline capability
# Fetches Material Icons, Font Awesome 6.4.0, and Google Fonts (Noto Sans, Source Sans 3)
# Can be run from anywhere; finds paths relative to script location

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_DIR="$(dirname "$SCRIPT_DIR")"   # app/elohim-app (script lives in app/elohim-app/scripts/)
FONTS_DIR="$APP_DIR/src/assets/fonts"

FA_VERSION="6.4.0"
UA="Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/120"

# One download path for every CDN fetch. Bounded (connect 15s, transfer 120s),
# retried (5x, every curl error class — a CDN stall on the CI agent surfaced
# 2026-09-22 as elohim/dev #1717 `script returned exit code 28` with no URL
# named), fails on HTTP errors instead of saving the error page as a font, and
# writes through a temp file so a partial download can never satisfy the
# "already present" short-circuit on the next run. Names the URL on failure.
fetch_url() {
  local url="$1" out="$2"
  local tmp="$out.part" rc=0
  curl -fsSL \
      --connect-timeout 15 --max-time 120 \
      --retry 5 --retry-delay 3 --retry-all-errors \
      -H "User-Agent: $UA" \
      -o "$tmp" "$url" || rc=$?
  if [ "$rc" -ne 0 ]; then
    rm -f "$tmp"
    echo "fetch-fonts: FAILED (curl exit $rc) after retries: $url" >&2
    return "$rc"
  fi
  if [ ! -s "$tmp" ]; then
    rm -f "$tmp"
    echo "fetch-fonts: FAILED (empty body): $url" >&2
    return 1
  fi
  mv -f "$tmp" "$out"
}

# ── Material Icons ──────────────────────────────────────────────────
fetch_material_icons() {
  local dir="$FONTS_DIR/material-icons"
  if [ -f "$dir/material-icons.woff2" ]; then
    echo "Material Icons: already present"
    return
  fi
  echo "Material Icons: downloading..."
  mkdir -p "$dir"
  fetch_url 'https://fonts.gstatic.com/s/materialicons/v145/flUhRq6tzZclQEJ-Vdg-IuiaDsNc.woff2' \
    "$dir/material-icons.woff2"
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
  fetch_url "https://cdnjs.cloudflare.com/ajax/libs/font-awesome/$FA_VERSION/css/all.min.css" \
    "$css_dir/all.min.css"
  local fonts=(
    fa-brands-400.woff2 fa-brands-400.ttf
    fa-regular-400.woff2 fa-regular-400.ttf
    fa-solid-900.woff2 fa-solid-900.ttf
    fa-v4compatibility.woff2 fa-v4compatibility.ttf
  )
  for font in "${fonts[@]}"; do
    fetch_url "https://cdnjs.cloudflare.com/ajax/libs/font-awesome/$FA_VERSION/webfonts/$font" \
      "$wf_dir/$font"
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
  fetch_url \
    'https://fonts.googleapis.com/css2?family=Noto+Sans:ital,wght@0,100..900;1,100..900&family=Source+Sans+3:ital,wght@0,200..900;1,200..900&display=swap' \
    "$dir/google-fonts.remote.css"
  css=$(cat "$dir/google-fonts.remote.css")
  rm -f "$dir/google-fonts.remote.css"

  # Extract and download only latin, latin-ext, and vietnamese subset files
  # Use grep to find the font URLs following subset comments
  local urls
  urls=$(echo "$css" | grep -A8 -E '/\* (latin-ext|latin|vietnamese) \*/' \
    | grep -oP 'url\(\Khttps://[^)]+' || true)
  if [ -z "$urls" ]; then
    echo "fetch-fonts: FAILED: Google Fonts CSS carried no latin/latin-ext/vietnamese woff2 URLs (UA or CSS shape changed?)" >&2
    return 1
  fi

  for url in $urls; do
    local fname
    fname=$(basename "$url")
    fetch_url "$url" "$dir/$fname"
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
