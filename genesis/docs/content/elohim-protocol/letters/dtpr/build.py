"""Build the DTPR letter's html5-app bundle from source/ into site/, and pack ../dtpr.zip.

Usage (from anywhere):  python3 -I genesis/docs/content/elohim-protocol/letters/dtpr/build.py

Steps:
  1. attach provenance and trust marks to the three datachains (source/*.final.json),
  2. render the letter with its datachains (source/build_page.py) into site/index.html,
     wrapped as a complete document that loads only its own files,
  3. copy fonts/ beside it,
  4. pack site/ into ../dtpr.zip with fixed timestamps and sorted names, so the same
     source always yields the same bytes and therefore the same blobHash.
Prints the zip's sha256, which genesis/data/lamad/content/letter-to-dtpr.json carries.
"""

import hashlib
import shutil
import subprocess
import sys
import zipfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
SRC = HERE / "source"
SITE = HERE / "site"
ZIP = HERE.parent / "dtpr.zip"
PY = [sys.executable, "-I"]

HEAD = """<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
<link rel="stylesheet" href="fonts/fonts.css">
<style>
:root { padding-top: env(safe-area-inset-top, 0px); padding-bottom: env(safe-area-inset-bottom, 0px); }
body { margin: 0; }
img { max-width: 100%; }
[hidden] { display: none !important; }
</style>
"""


def run(*args):
    subprocess.run([*PY, *args], cwd=SRC, check=True)


def main():
    for name in ("own-seat", "hosted", "pricing"):
        run("add_provenance.py", f"datachain-{name}.json", f"marks-{name}.json", f"datachain-{name}.final.json")
    if SITE.exists():
        shutil.rmtree(SITE)
    SITE.mkdir()
    fragment = SITE / "fragment.html"
    run("build_page.py", ".", str(fragment))
    body = fragment.read_text()
    fragment.unlink()
    # Drop the hosted-artifact-only lines; fonts come from fonts/fonts.css instead.
    keep = [ln for ln in body.splitlines()
            if "fonts.googleapis.com" not in ln and "fonts.gstatic.com" not in ln
            and "claude:viewport-fit-sides" not in ln]
    split = next(i for i, ln in enumerate(keep) if ln.startswith('<div class="wrap">'))
    page = HEAD + "\n".join(keep[:split]) + "\n</head>\n<body>\n" + "\n".join(keep[split:])
    (SITE / "index.html").write_text(page + "\n</body>\n</html>\n")
    shutil.copytree(HERE / "fonts", SITE / "fonts")

    with zipfile.ZipFile(ZIP, "w", zipfile.ZIP_DEFLATED) as z:
        for f in sorted(p for p in SITE.rglob("*") if p.is_file()):
            info = zipfile.ZipInfo(str(f.relative_to(SITE)), date_time=(1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o644 << 16
            z.writestr(info, f.read_bytes())
    digest = hashlib.sha256(ZIP.read_bytes()).hexdigest()
    print(f"built site/ and {ZIP.name}: sha256-{digest}")


if __name__ == "__main__":
    main()
