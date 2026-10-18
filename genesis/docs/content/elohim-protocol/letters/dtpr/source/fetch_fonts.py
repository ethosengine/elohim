"""Vendor the page's four typefaces as local woff2 files (run once; the results are checked in).

Usage: python3 -I fetch_fonts.py <bundle-dir>

All four families are SIL Open Font License 1.1 (Google Fonts). The page is served by
doorways under `default-src 'self'`, so the fonts ship inside the bundle instead of
loading from a third party. Writes <bundle-dir>/fonts/*.woff2 and fonts/fonts.css
(the @font-face rules, latin subset only).
"""

import re
import sys
import urllib.request
from pathlib import Path

CSS_URL = ("https://fonts.googleapis.com/css2?"
           "family=Fraunces:opsz,wght@9..144,400;9..144,600"
           "&family=Source+Serif+4:ital,opsz,wght@0,8..60,400;0,8..60,600;1,8..60,400"
           "&family=DM+Sans:wght@400;500;600"
           "&family=Red+Hat+Text:wght@400;500;600;700&display=swap")
UA = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36"


def get(url):
    req = urllib.request.Request(url, headers={"User-Agent": UA})
    with urllib.request.urlopen(req, timeout=30) as r:
        return r.read()


def main(bundle):
    out = Path(bundle) / "fonts"
    out.mkdir(parents=True, exist_ok=True)
    css = get(CSS_URL).decode()
    blocks = re.findall(r"/\* ([a-z-]+) \*/\s*(@font-face \{.*?\})", css, flags=re.S)
    rules = []
    for subset, block in blocks:
        if subset != "latin":
            continue
        family = re.search(r"font-family: '([^']+)'", block).group(1)
        style = re.search(r"font-style: (\w+)", block).group(1)
        weight = re.search(r"font-weight: ([\d ]+);", block).group(1).strip()
        url = re.search(r"url\((https://[^)]+\.woff2)\)", block).group(1)
        name = f"{family.lower().replace(' ', '-')}-{style}.woff2"  # variable fonts: one file per family/style
        if not (out / name).exists():
            (out / name).write_bytes(get(url))
        rules.append(re.sub(r"url\(https://[^)]+\)", f"url({name})", block))
    (out / "fonts.css").write_text("\n".join(dict.fromkeys(rules)) + "\n")
    print(f"vendored {len(set(rules))} faces into {out}")


if __name__ == "__main__":
    main(sys.argv[1])
