# Letter to the DTPR community — HTML5 bundle

A letter from the Elohim Protocol to Helpful Places and the DTPR (Digital Trust for
Places and Routines) Community of Practice. It describes the protocol in DTPR's own
visual language, as three DTPR *datachains* (DTPR's structured disclosure labels), and
offers one piece of advice: don't label the AI; label the parties who arrive carrying it.
This directory is for anyone editing the letter or its labels, or rebuilding the page.

**Read it through a doorway** (the protocol's web gateways; there are several, so no single server is the only way in):

- https://alpha.elohim.host/apps/letter-to-dtpr/index.html
- https://doorway-alpha.elohim.host/apps/letter-to-dtpr/index.html

Both addresses serve the same bundle from the alpha test network (the project's staging
network; production is paused). The bundle is content-addressed: its address is its
sha256, and peers and doorways verify the bytes against that address before serving.

## Prerequisites

Python 3, standard library only. Building needs no network access; the fonts and DTPR's
schema and icons are checked in. Re-validating the labels against DTPR needs network
access to `api.dtpr.io` (see "Checking the labels").

## First run

The built page is **not** checked in. Build it, then open it:

    python3 -I build.py     # exits 0 and prints "built site/ and dtpr.zip: sha256-…"

then open `site/index.html` in a browser. The page loads only its own files. (`-I` runs
Python isolated from the current directory and user site-packages; it is a safety habit, not a requirement.)

## What's here (checked in)

    source/letter.md          the letter's prose: the single source the page is built from
    source/datachain-*.json   the three labels, in DTPR's DatachainInstance shape:
                              own-seat (a person running their own node), hosted
                              (a doorway's first stage), pricing (solidarity pricing)
    source/marks-*.json       each claim's trust marks: what backs it (enforced by every
                              peer, signed and witnessed, held by people, our commitment)
                              and where it stands today (live, building, designed),
                              with the source lines that support it
    source/build_proposals.py the five elements proposed to DTPR, in DTPR's Element shape
    source/build_page.py      renders the letter and labels into one page
    source/add_provenance.py  attaches the trust marks as DTPR authoring provenance
    source/page-template.html the page's layout and styles
    source/dtpr-schema.json   DTPR schema dtpr@2026-09-01-beta (categories, contexts)
    source/dtpr-element-catalog.md  DTPR's element titles and descriptions, by category
    source/icons/             DTPR's composed icons for every element used (light, dark)
    source/fetch_fonts.py     how fonts/ was vendored (run once; not part of the build)
    fonts/                    the four typefaces as local files, with their @font-face rules
    build.py                  builds site/ and packs ../dtpr.zip

    ../dtpr.zip               one level up, beside this directory (where the content record's
                              localZipPath points): the built site as one zip, the protocol's
                              "html5-app" format for a self-contained web page

Build outputs (not checked in): `site/`, and `source/datachain-*.final.json`, the three
labels with their trust marks attached as DTPR authoring provenance.

The content node is `genesis/data/lamad/content/letter-to-dtpr.json`: the protocol's
record of this artifact, which the seeder publishes and doorways serve from. Its
`blobHash` must equal the zip's sha256, because peers fetch and verify the bundle by that
hash; a mismatch means the doorway serves a stale page or none. Its reach is `commons`
(the widest of the protocol's visibility levels: meant for anyone). After any change: run `build.py`, put the printed
hash into that file's `blobHash`, and commit the source, the zip and the record together.
`build.py` writes the zip with fixed timestamps and sorted names, so the same source
always gives the same hash.

How a change reaches the doorways: once the commit lands on `dev`, the Genesis pipeline's
seeder uploads the zip and the record to the network's storage, and doorways fetch the
zip by its hash and serve it. No other step is manual.

### After any change

1. Edit the source (see Editing).
2. Run `python3 -I build.py` and note the printed hash.
3. Check the labels (see Checking the labels).
4. Put the hash into `blobHash` in `genesis/data/lamad/content/letter-to-dtpr.json`.
5. Commit the source, `../dtpr.zip` and the record together.

## Editing

- **The prose** lives only in `source/letter.md`. Its `## ` headings become the page's
  sections; the labels appear inside "The same questions, asked from underneath", the
  pricing label inside "When prices move", and the proposals inside "Five elements".
- **A label claim** lives in `source/datachain-*.json`; its marks in the matching
  `marks-*.json`, keyed by the placement's `element_instance_id` (or its `element_id`
  when it appears once). Both are fields of DTPR's DatachainInstance schema (dtpr.ai).
- **A new DTPR element** needs its icons in `source/icons/`: fetch
  `https://api.dtpr.io/api/v2/schemas/dtpr@2026-09-01-beta/elements/<id>/icon.<variant>.svg`
  for the light variant and `.dark` for the dark one.

## Checking the labels

Each built label is checked against DTPR's own validator. After `build.py`:

    curl -s -X POST -H 'content-type: application/json' \
      --data-binary @source/datachain-own-seat.final.json \
      'https://api.dtpr.io/api/v2/schemas/dtpr@2026-09-01-beta/validate'

`{"ok":true,"errors":[],"warnings":[]}` is a pass. The page's "Check it yourself" panel
gives the same JSON to readers so they can paste it into DTPR's visualizer.

## Credits

DTPR's icons, taxonomy and datachain structure are by the Digital Transparency in the
Public Realm contributors, used under CC BY 4.0; portions of the icons derive from
Material Symbols (Apache 2.0). Fraunces, Source Serif 4, DM Sans and Red Hat Text are
under the SIL Open Font License 1.1. The page is a letter from the Elohim Protocol and is
not published by DTPR or Helpful Places. It was drafted with Claude (Anthropic) and sent
by Matthew Dowell in his own name.
