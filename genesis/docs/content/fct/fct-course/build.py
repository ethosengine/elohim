#!/usr/bin/env python3
"""Build the Foundations for Christians and Technology static site.

Reads content/*.md (source of truth), writes clean semantic HTML5 into site/.
Deliberately unopinionated: element-level markup only, no classes, no JS,
one small optional stylesheet. Regenerate any time with:  python3 build.py
Requires:  pip install markdown
"""
import html
import os
import re
import shutil

import markdown

MD = lambda t: markdown.markdown(t, extensions=["tables"])

COURSE = "Foundations for Christians and Technology"
TAGLINE = ("A Christian companion to the Center for Humane Technology's "
           "Foundations of Humane Technology — fourteen days, fifteen modules, "
           "five movements, walking the shape of the gospel from lament to commissioning.")

MOVEMENTS = {
    1: "Waking Up — Lament & Sight",
    2: "Turning — Repentance & Repair",
    3: "Reordering Loves",
    4: "Rebuilding Common Life",
    5: "Abundant Life & Sending",
}

# (number, source, slug, title, movement, day)
MODULES = [
    (1,  "m01", "01-the-church-dilemma",              "The Church Dilemma",                          1, 0),
    (2,  "m02", "02-systems-thinking",                "Systems Thinking",                            1, 0),
    (3,  "m03", "03-a-problem-well-stated",           "A Problem Well Stated",                       1, 1),
    (4,  "m04", "04-wisdom-and-trust",                "The Wisdom Gap & the Conditions for Trust",   1, 2),
    (5,  "m05", "05-our-attention-is-sacred",         "Our Attention is Sacred",                     2, 3),
    (6,  "m06", "06-externalities-and-the-commons",   "Externalities & the Commons",                 2, 4),
    (7,  "m07", "07-regeneration-and-reconciliation", "Regeneration & Reconciliation",               2, 5),
    (8,  "m08", "08-centering-love",                  "Centering Love",                              3, 6),
    (9,  "m09", "09-fruit-as-metric",                 "Fruit as Metric",                             3, 7),
    (10, "m10", "10-recognizing-distortions",         "Recognizing Distortions",                     4, 8),
    (11, "m11", "11-collaboration-and-sensemaking",   "Collaboration & Sensemaking",                 4, 9),
    (12, "m12", "12-fairness-and-justice",            "Fairness & Justice: Fill My House",           4, 10),
    (13, "m13", "13-helping-people-thrive",           "Helping People Thrive",                       5, 11),
    (14, "m14", "14-ready-to-act",                    "Ready to Act",                                5, 12),
    (15, "m15", "15-digital-discipleship-retrospective", "Digital Discipleship Retrospective",       5, 13),
]

# Co-located source document: the course doc itself, shipped beside index.html
# so the bundle carries its own editable original (linked from the course home).
DOCUMENT = "Foundations-for-Christians-and-Technology.docx"

APPENDICES = [
    ("verse-appendix", "verses",       "Verse Appendix"),
    ("bibliography",   "bibliography", "Bibliography & Media"),
]


def wrap_sections(body_html: str) -> str:
    """Wrap each <h2>-delimited region in <section>; anything before the
    first <h2> becomes the article's <header>."""
    parts = re.split(r"(?=<h2)", body_html)
    out = []
    if parts and not parts[0].startswith("<h2"):
        lead = parts.pop(0).strip()
        if lead:
            out.append(f"<header>\n{lead}\n</header>")
    for part in parts:
        out.append(f"<section>\n{part.strip()}\n</section>")
    return "\n\n".join(out)


def page(title, body, css_rel, body_attrs="", lang="en"):
    return f"""<!DOCTYPE html>
<html lang="{lang}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{html.escape(title)}</title>
<link rel="stylesheet" href="{css_rel}css/base.css">
</head>
<body{body_attrs}>
{body}
</body>
</html>
"""


def nav_html(prev_item, next_item, up_rel):
    links = [f'<a href="{up_rel}index.html" rel="up">Course home</a>']
    if prev_item:
        n, _, slug, title, _, _ = prev_item
        links.append(f'<a href="{slug}.html" rel="prev">&#8592; Module {n}: {html.escape(title)}</a>')
    if next_item:
        n, _, slug, title, _, _ = next_item
        links.append(f'<a href="{slug}.html" rel="next">Module {n}: {html.escape(title)} &#8594;</a>')
    return '<nav aria-label="Course navigation">\n' + "\n".join(links) + "\n</nav>"


def build():
    os.makedirs("site/modules", exist_ok=True)
    os.makedirs("site/appendix", exist_ok=True)
    os.makedirs("site/css", exist_ok=True)

    # -- module pages -----------------------------------------------------
    for i, item in enumerate(MODULES):
        num, src, slug, title, mv, day = item
        raw = open(f"content/{src}.md", encoding="utf-8").read()
        body = wrap_sections(MD(raw))
        nav = nav_html(MODULES[i - 1] if i else None,
                       MODULES[i + 1] if i + 1 < len(MODULES) else None, "../")
        inner = f"""{nav}
<main>
<article>
<h1>Module {num} &#183; {html.escape(title)}</h1>
{body}
</article>
</main>
{nav}"""
        attrs = (f' data-module="{num}" data-day="{day}" data-movement="{mv}"')
        page_title = f"Module {num} · {title} — {COURSE}"
        open(f"site/modules/{slug}.html", "w", encoding="utf-8").write(
            page(page_title, inner, "../", attrs))

    # -- appendix pages ---------------------------------------------------
    for src, slug, title in APPENDICES:
        raw = open(f"content/{src}.md", encoding="utf-8").read()
        body = wrap_sections(MD(raw))
        nav = '<nav aria-label="Course navigation">\n<a href="../index.html" rel="up">Course home</a>\n</nav>'
        inner = f"""{nav}
<main>
<article>
{body}
</article>
</main>
{nav}"""
        open(f"site/appendix/{slug}.html", "w", encoding="utf-8").write(
            page(f"{title} — {COURSE}", inner, "../"))

    # -- index ------------------------------------------------------------
    overview = MD(open("content/overview.md", encoding="utf-8").read())
    toc = []
    for mv in sorted(MOVEMENTS):
        toc.append(f"<h2>Movement {'IVX'[mv-1] if False else ['I','II','III','IV','V'][mv-1]}. "
                   f"{html.escape(MOVEMENTS[mv])}</h2>")
        toc.append("<ol>")
        for num, _, slug, title, m, day in MODULES:
            if m == mv:
                toc.append(f'<li value="{num}"><a href="modules/{slug}.html">{html.escape(title)}</a>'
                           f" <small>(Day {day})</small></li>")
        toc.append("</ol>")
    toc.append("<h2>Appendices</h2>\n<ul>")
    for _, slug, title in APPENDICES:
        toc.append(f'<li><a href="appendix/{slug}.html">{html.escape(title)}</a></li>')
    toc.append("</ul>")

    doc_link = ""
    if os.path.exists(DOCUMENT):
        shutil.copyfile(DOCUMENT, f"site/{DOCUMENT}")
        doc_link = (f'\n<p><a href="{DOCUMENT}" download>Download the course document (.docx)</a>'
                    " &#8212; the editable original this site is built from.</p>")
    inner = f"""<header>
<h1>{COURSE}</h1>
<p>{TAGLINE}</p>{doc_link}
</header>
<main>
<article>
{overview}
</article>
<nav aria-label="Course contents">
{chr(10).join(toc)}
</nav>
</main>"""
    open("site/index.html", "w", encoding="utf-8").write(page(COURSE, inner, ""))

    # -- css (optional, deletable) ----------------------------------------
    open("site/css/base.css", "w", encoding="utf-8").write("""\
/* Minimal legibility layer only — delete or replace freely.
   No classes are used anywhere in the markup; style by element. */
body { max-width: 72ch; margin: 0 auto; padding: 1rem;
       font-family: system-ui, sans-serif; line-height: 1.6; }
nav[aria-label="Course navigation"] { display: flex; flex-wrap: wrap;
       gap: 1rem; justify-content: space-between; margin: 1rem 0; }
table { border-collapse: collapse; }
th, td { border: 1px solid currentColor; padding: 0.35em 0.6em;
         text-align: left; vertical-align: top; }
""")
    print("built site/:", sum(len(f) for _, _, f in os.walk("site")), "files")


if __name__ == "__main__":
    build()
