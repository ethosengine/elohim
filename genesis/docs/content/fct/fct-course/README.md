# Foundations for Christians and Technology — HTML5 bundle

A Christian companion course to the Center for Humane Technology's
*Foundations of Humane Technology*: fifteen modules grouped into five
"movements" (Waking Up · Turning · Reordering Loves · Rebuilding Common
Life · Abundant Life & Sending), plus a verse appendix and a bibliography.
This directory is for anyone editing the course text or rebuilding the
browsable site from it.

## Prerequisites (for building and editing)

Python 3 and the `markdown` package (`pip install markdown`). Nothing else.
Pushing the course to the learning platform needs more; see that section.

## First run

The built site is **not** checked in. Build it first, then open it:

    python3 build.py        # prints "built site/: 20 files" on success

then open `site/index.html` in a browser, or host the `site/` folder as-is.

## What's here (checked in)

    content/                 the markdown source of truth: m01.md … m15.md,
                             overview.md, verse-appendix.md, bibliography.md
                             (exported from the course document, 2026-09-16)
    Foundations-for-Christians-and-Technology.docx
                             the course document itself (review draft v2) — the
                             editable original that content/ was exported from;
                             build.py copies it into site/ and links it from the
                             course home so the site carries its own source
    build.py                 regenerates site/ from content/ (and the .docx)
    README.md                this file

## What build.py produces (generated, not checked in)

    site/
      index.html             course home: overview, table of contents, link to the .docx
      modules/NN-slug.html   one page per module (01–15)
      appendix/              verses.html, bibliography.html
      css/base.css           ~15 lines of legibility CSS; delete or replace freely
      Foundations-for-Christians-and-Technology.docx   copied from this directory

## Which copy is authoritative

The `.docx` is the author's working document. `content/*.md` is its export
and is what `build.py` reads. When the two disagree, the `.docx` wins. The
export is manual (there is no script for it): the author copies each
module's text out of the document into its `content/mNN.md`, one file per
module, then rebuilds. If you only edit `content/*.md`, you are editing
downstream of the `.docx`; tell the author so the document can be updated.

## Design intent

The markup is deliberately minimal and semantic so you can layer your own
design, scope-and-sequence, and packaging on top without rework:

- Elements only — no classes, no ids, no JavaScript, no framework.
- Each module page: `<nav>` (prev/next/home), `<main> > <article>`, with the
  module preamble in `<header>` and every `##` block wrapped in `<section>`.
- Hierarchy hooks on `<body>`: `data-module`, `data-day`, `data-movement`
  (movements: 1 Waking Up · 2 Turning · 3 Reordering Loves ·
  4 Rebuilding Common Life · 5 Abundant Life & Sending).
- Relative links throughout — the folder works from disk, a zip, or any
  static host; nothing external is loaded.

## Editing

Edit `content/*.md`, run `python3 build.py`, open `site/index.html`, done.

## How the site reaches the learning platform

The learning platform this repository builds (lamad, served by the Elohim
Protocol) seeds its content from JSON files under
`genesis/data/lamad/content/`, one file per piece of content (a "content
node"). Content that is a whole website, like this course, is stored as a
zip of the built site (an "html5-app" node) and identified by the sha256 of
that zip, recorded in the node as `blobHash`. The seeder uploads the zip
under that hash, so the hash in the JSON must match the zip on disk. If it
does not, the course's page on the platform answers with a 404 (or a
"syncing" status that never resolves) instead of the course home.

If you only edit and preview the course locally, you can stop reading here.

This workflow needs, beyond the build prerequisites: the `just` task runner
(used for every repository command; https://github.com/casey/just, or
`cargo install just`), and a running local development
instance of the platform (the repository calls it the "household mesh";
`just mesh start` from the repository root brings one up).

For this course (paths relative to this directory unless stated):

- `../fct-course.zip` — the contents of the built `site/` folder, packed
  with `index.html` at the zip root. This is what gets seeded; it is
  checked in.
- `../../../data/lamad/content/fct-course.json` (from the repository root:
  `genesis/data/lamad/content/fct-course.json`) — the content node. Its
  `blobHash` is the zip's sha256 (with a `sha256-` prefix), and its
  `metadata.resources` names the `.docx` that ships inside the zip.

After changing anything in `content/` or the `.docx`, rebuild and repack,
from this directory:

    python3 build.py
    ( cd site && zip -rX ../../fct-course.zip . )   # produces ../fct-course.zip;
                                                    # -X drops owner/permission
                                                    # metadata so the hash is
                                                    # reproducible across machines
    sha256sum ../fct-course.zip                     # macOS: shasum -a 256

The zip must contain exactly one `index.html`. elohim-storage, the
platform's blob server that unpacks the zip, finds files by path suffix, so
a second `index.html` anywhere in the zip could shadow the course home.
Packing from inside `site/`, as above, guarantees this.

Then, in the content node JSON named above, set the new hash (keeping the
`sha256-` prefix) in the two fields that carry it: the top-level `blobHash`
(read by the seeder and the platform) and `metadata.blobHash` (a copy kept
for the node's own record, following the other html5-app nodes). They must
agree.

Finally, from the repository root, check and seed:

    just seed validate              # schema check, writes nothing
    just seed apply mesh content    # seeds the local development instance

## Relationship to the earlier version

`../v1/` holds the 2025 lesson-plan markdown this course was rewritten from.
It is kept for reference only; do not edit it. The per-module content nodes
named `fct-module-*` (and their `fct-bible-*` / `fct-media-*` children) were
generated from that v1 text and have not yet been regenerated from v2. Both
exist on the platform without conflict: the bundle (the `fct-course` node)
is the current course, and the v1 nodes are the older per-module outline
that the course path still lists after it. As of 2026-09-23 they have not
been regenerated from v2; the `fct-course` node and this directory are the
only v2 wiring.
