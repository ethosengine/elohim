"""Build the shareable page: the letter, with the validated DTPR datachains in place.

Usage: python3 -I build_page.py <campaign-dir> <out.html>

Reads letter-to-dtpr.md (the single source of the prose), the two validated
thin datachains, their trust marks, the proposals, the schema export and the
downloaded DTPR icons, and writes one self-contained HTML file (icons inlined,
no network requests).
"""

import html
import json
import re
import sys
from pathlib import Path

D = Path(sys.argv[1])
OUT = Path(sys.argv[2])

schema = json.load(open(D / "dtpr-schema.json"))["data"]
catalog = {}
for line in open(D / "dtpr-element-catalog.md"):
    m = re.match(r"- ([a-z0-9_]+): (.+?) — ", line)
    if m:
        catalog[m.group(1)] = m.group(2)


def en(arr):
    for x in arr or []:
        if x.get("locale") == "en":
            return x["value"]
    return ""


CATS = {c["id"]: c for c in schema["categories"]}
ORDER = schema["datachain_type"]["categories"]
DATA_FLOW = set(schema["datachain_type"]["subchains"][0]["categories"])
CTX = {}
for c in schema["categories"]:
    if c.get("element_context"):
        for v in c["element_context"]["values"]:
            CTX[(c["id"], v["id"])] = en(v["name"])

BACKED = {
    "peer": ("Enforced by every peer", "Every computer that stores a record checks this rule and refuses records that break it."),
    "signed": ("Signed and witnessed", "A signed record anyone can read, check and challenge."),
    "people": ("Held by people", "A person can always overturn a machine here: an appeal, a council, a recovery group."),
    "promise": ("Our commitment", "What we commit to in production; the network does not enforce it by itself."),
}
TODAY = {"live": "Live", "building": "Building", "designed": "Designed"}

UNDERNEATH = {
    "own": {
        "accountable": "Your label expects an organization here. From underneath, the first deployer is the person, and anyone with authority over them answers to them.",
        "affected": "Your choices assume someone else's system acts on people. Here the person granted the system its permissions and can take them back.",
        "action": "The two actions we lean on most are many peers checking the same rules, and asking a person. Your list does not have either yet.",
        "processing": "Your category groups algorithms and models as techniques. Here they are different kinds of thing: an algorithm is a governed record anyone can inspect by its address, and a model is a participant whose identity and capabilities are layered (base model, a tuning layer, and the constitutional context in force when something was authored or gated) and attested the way a person's are.",
        "output_dataset": "What a person makes alongside an elohim carries both contributions from the start: their own direction, rewarded as theirs, and what came from the commons, cited and shared back.",
        "rights": "Rights here are things a person does (withdraw, leave, copy the rules), not requests to an operator.",
        "access": "Read from the person's seat, access is what they grant, not what an operator allows.",
        "retention": "Retention is not an operator's setting here. It depends on where a person put something. A gift to the commons becomes something others rely on, so taking it back is a request honoured as far as it can be, and the gate asks first.",
        "storage": "Your own element already describes our architecture: governance built into the structure of the data itself.",
    },
    "pricing": {
        "collection": "The telemetry is real, but it is for the household that owns the devices, and it travels only as far as the household chooses.",
        "access": "Sharing is not the taboo it is elsewhere. Privacy is the default, and what a household chooses to share earns for the household and the commons, not for a platform that sells it.",
        "action": "The elohim working with the household reads the offer. The offer does not read the household.",
        "input_dataset": "No seller learns who is in need. The floor is met inside the network.",
        "manifestation": "In most labels this icon means a price a company sets by watching you. Here everyday needs carry no price to weigh at all, and only ceiling choices carry a felt limit, set by what the commons can carry.",
        "risks_mitigation": "The toll of the network returns to the commons instead of becoming profit.",
    },
    "hosted": {
        "accountable": "At this stage, and only at this stage, your middle-out reading is the accurate one. The label says so.",
        "retention": "Hosting is a first stage. Leaving it should leave nothing behind that could act as you.",
    },
}

icons = {}
for f in (D / "icons").glob("*.svg"):
    svg = f.read_text()
    svg = re.sub(r'\swidth="36" height="36"', "", svg, count=1)
    icons[f.stem] = svg


def icon(el, variant):
    base = variant or "default"
    light = icons.get(f"{el}.{base}")
    dark = icons.get(f"{el}.{base}.dark" if base != "default" else f"{el}.dark")
    if not light:
        return '<span class="ico"></span>'
    return (f'<span class="ico" aria-hidden="true"><span class="ico-l">{light}</span>'
            f'<span class="ico-d">{dark or light}</span></span>')


def esc(s):
    return html.escape(s, quote=True)


def var(placement, vid):
    for v in placement.get("variables", []):
        if v["id"] == vid:
            return en(v["value"])
    return ""


def card(p, marks, proposed=None):
    el = p["element_id"]
    key = p.get("element_instance_id") or el
    cat = proposed["category_id"] if proposed else None
    if proposed:
        title = en(proposed["title"])
    elif el in ("organization", "people", "institution"):
        title = var(p, "title")
    else:
        title = catalog.get(el, el)
    body = (var(p, "description") or var(p, "additional_description") or var(p, "rights")
            or var(p, "risk"))
    extra = ""
    if el == "data_retained" or (proposed and proposed["id"] == "given_to_the_commons"):
        title = title.replace("{{duration}}", var(p, "duration"))
    if var(p, "mitigation"):
        extra = f'<p class="mit"><span>Mitigation</span> {esc(var(p, "mitigation"))}</p>'
    if el == "shared_storage_and_governance":
        extra += f'<p class="mit"><span>How long</span> {esc(var(p, "duration"))}</p>'
    ctx_cat = cat or next((c for c in CATS if c == p.get("_cat")), None)
    chips = ""
    if p.get("context_type_id"):
        label = CTX.get((p["_cat"], p["context_type_id"]), p["context_type_id"])
        chips += f'<span class="ctx">{esc(label)}</span>'
    if proposed:
        chips += '<span class="ctx proposed-chip">Proposed</span>'
    m = marks.get(key)
    marks_html = ""
    detail = ""
    if m:
        b, bdesc = BACKED[m["backed"]]
        t = TODAY[m["today"]]
        note = f'<span class="tnote">{esc(m["today_note"])}</span>' if m.get("today_note") else ""
        marks_html = (f'<div class="marks"><span class="mark b-{m["backed"]}" title="{esc(bdesc)}">'
                      f'<i></i>{esc(b)}</span><span class="mark t-{m["today"]}"><i></i>Today: {t}</span>{note}</div>')
        srcs = "".join(
            f'<blockquote><p>&ldquo;{esc(s["quote"])}&rdquo;</p><cite>{esc(s.get("context", ""))}</cite></blockquote>'
            for s in m.get("sources", []))
        detail = (f'<details><summary>Why we can say this</summary><p>{esc(m["rationale"])}</p>{srcs}</details>')
    elif proposed:
        detail = f'<details open><summary>Why it is needed</summary><p>{esc(proposed["_why"])}</p></details>'
    ico = icon(proposed["_icon"] if proposed else el,
               p.get("context_type_id") if not proposed else proposed.get("_variant"))
    cls = "dtpr-el proposed" if proposed else "dtpr-el"
    return (f'<article class="{cls}"><header>{ico}<div class="tb"><h4>{esc(title)}</h4>'
            f'<div class="chips">{chips}</div></div></header>'
            f'<p class="desc">{esc(body)}</p>{extra}{marks_html}{detail}</article>')


def datachain(chain, marks, seat):
    by_cat = {}
    for p in chain["elements"]:
        cat = None
        el = p["element_id"]
        for c in ORDER:
            pass
        by_cat.setdefault(None, [])
    # category of each element from the catalog file headings
    cat_of = {}
    current = None
    for line in open(D / "dtpr-element-catalog.md"):
        h = re.match(r"### ([a-z_]+)", line)
        if h:
            current = h.group(1)
        m = re.match(r"- ([a-z0-9_]+):", line)
        if m:
            cat_of[m.group(1)] = current
    groups = {}
    for p in sorted(chain["elements"], key=lambda e: e.get("priority", 0)):
        c = cat_of[p["element_id"]]
        p = dict(p, _cat=c)
        groups.setdefault(c, []).append(p)
    out = [f'<div class="dtpr-chain" data-seat="{seat}"><div class="chain-head">'
           f'<p class="kicker">DTPR datachain &middot; {esc(chain["schema_version"])}</p>'
           f'<h3>{esc(en(chain["title"]))}</h3><p>{esc(en(chain["description"]))}</p></div>']
    flow_open = False
    for c in ORDER:
        if c not in groups:
            continue
        if c in DATA_FLOW and not flow_open:
            out.append('<div class="flow"><p class="flow-label">Data flow</p>')
            flow_open = True
        if c not in DATA_FLOW and flow_open:
            out.append("</div>")
            flow_open = False
        cat = CATS[c]
        note = UNDERNEATH[seat].get(c)
        note_html = f'<p class="under"><b>From underneath.</b> {esc(note)}</p>' if note else ""
        out.append(f'<section class="dtpr-cat"><div class="cat-head"><h4>{esc(en(cat["name"]))}</h4>'
                   f'<p>{esc(en(cat.get("prompt", [])))}</p></div>{note_html}<div class="grid">'
                   + "".join(card(p, marks) for p in groups[c]) + "</div></section>")
    if flow_open:
        out.append("</div>")
    out.append("</div>")
    return "".join(out)


def tally(marks):
    from collections import Counter
    b = Counter(m["backed"] for m in marks.values())
    t = Counter(m["today"] for m in marks.values())
    rows = "".join(f'<li><span class="mark b-{k}"><i></i>{BACKED[k][0]}</span><b>{b.get(k, 0)}</b><small>{BACKED[k][1]}</small></li>'
                   for k in BACKED)
    rows_t = " &middot; ".join(f'{TODAY[k]} {t.get(k, 0)}' for k in TODAY)
    return f'<ul class="tally">{rows}</ul><p class="tally-today">Where those claims stand today: {rows_t}.</p>'


# ---- proposals ---------------------------------------------------------
sys.path.insert(0, str(D))
src = (D / "build_proposals.py").read_text()
ns = {}
exec(compile(src.split("SIBLING =")[0], "build_proposals", "exec"), ns)
PROPS = ns["PROPOSALS"]
ICON_FOR = {"checks": ("decides", "autonomous"), "refers": ("recommends", "human_decides"),
            "available_to_those_i_share_with": ("available_to_me", None),
            "given_to_the_commons": ("data_retained", None), "right_to_leave": ("right_object", None)}
CAT_FOR = {"checks": "action", "refers": "action", "available_to_those_i_share_with": "access",
           "given_to_the_commons": "retention", "right_to_leave": "rights"}


def proposals_html():
    cards = []
    for p in PROPS:
        el = dict(p["element"])
        el["_why"] = p["why"]
        el["_icon"], el["_variant"] = ICON_FOR[el["id"]]
        placement = {"element_id": el["id"], "_cat": CAT_FOR[el["id"]], **p["placement"]}
        desc = en(el["description"])
        desc = re.sub(r"^[^—]+— ", "", desc)
        placement_desc = dict(placement)
        placement_desc["variables"] = [{"id": "additional_description", "value": [{"locale": "en", "value": desc}]}]
        c = card(placement_desc, {}, proposed=el)
        cat_name = en(CATS[CAT_FOR[el["id"]]]["name"])
        cards.append(c.replace('<div class="chips">', f'<div class="chips"><span class="ctx">{esc(cat_name)}</span>', 1))
    return '<div class="grid proposals">' + "".join(cards) + "</div>"


# ---- letter markdown -----------------------------------------------------
def inline(s):
    s = esc(s)
    s = re.sub(r"`([^`]+)`", r"<code>\1</code>", s)
    s = re.sub(r"\*\*([^*]+)\*\*", r"<strong>\1</strong>", s)
    s = re.sub(r"(?<![\w*])\*([^*]+)\*(?![\w*])", r"<em>\1</em>", s)
    return s


letter = (D / "letter.md").read_text()
front, body = letter.split("---", 2)[1:]
sections = re.split(r"^## ", body.split("\n# ", 1)[1], flags=re.M)
title_line, opening = sections[0].split("\n", 1)


def md_block(text):
    out, para, items = [], [], []
    def flush():
        if para:
            out.append("<p>" + inline(" ".join(para)) + "</p>"); para.clear()
        if items:
            tag = "ol" if items[0][0] else "ul"
            out.append(f"<{tag}>" + "".join(f"<li>{inline(t)}</li>" for _, t in items) + f"</{tag}>"); items.clear()
    for line in text.splitlines():
        s = line.strip()
        m = re.match(r"^(?:- |(\d+)\. )(.*)", s)
        if not s:
            flush(); continue
        if m:
            if para:
                out.append("<p>" + inline(" ".join(para)) + "</p>"); para.clear()
            items.append((m.group(1), m.group(2))); continue
        if items and line.startswith("   "):
            items[-1] = (items[-1][0], items[-1][1] + " " + s); continue
        flush() if items else None
        para.append(s)
    flush()
    return "".join(out)


own = json.load(open(D / "datachain-own-seat.final.json"))
hosted = json.load(open(D / "datachain-hosted.final.json"))
pricing = json.load(open(D / "datachain-pricing.final.json"))
m_pricing = json.load(open(D / "marks-pricing.json"))
m_own = json.load(open(D / "marks-own-seat.json"))
m_hosted = json.load(open(D / "marks-hosted.json"))

labels_html = f"""
<div class="labels" id="labels">
  <div class="seat-tabs" role="tablist" aria-label="Which seat to read the label from">
    <button role="tab" id="tab-own" aria-selected="true" aria-controls="panel-own">From your own seat</button>
    <button role="tab" id="tab-hosted" aria-selected="false" aria-controls="panel-hosted">While a doorway hosts you</button>
  </div>
  <div role="tabpanel" id="panel-own" aria-labelledby="tab-own">{datachain(own, m_own, "own")}</div>
  <div role="tabpanel" id="panel-hosted" aria-labelledby="tab-hosted" hidden>{datachain(hosted, m_hosted, "hosted")}</div>
</div>"""

trust_html = f"""
<div class="trust-box"><p class="kicker">The label above, counted</p>
<p>Of the {len(m_own)} claims in the first label that carry a mark, here is what backs each one.</p>{tally(m_own)}</div>"""

verify_html = f"""
<div class="verify" id="verify">
  <h3>Check it yourself</h3>
  <p>All three labels passed DTPR's validator at api.dtpr.io for <code>dtpr@2026-09-01-beta</code> with no errors and no warnings, and each proposal passed the resolved-chain validator against your canonical snapshot. Copy any label and paste it into the <a href="https://dtpr.ai/en/tools/datachain" target="_blank" rel="noopener">DTPR datachain visualizer</a> to see it in your own renderer.</p>
  <div class="copy-row">
    <button type="button" class="copy" data-src="json-own" id="copy-own">Copy “From your own seat” JSON</button>
    <button type="button" class="copy" data-src="json-hosted" id="copy-hosted">Copy “While a doorway hosts you” JSON</button>
    <button type="button" class="copy" data-src="json-pricing" id="copy-pricing">Copy “When prices move” JSON</button>
    <span class="copied" role="status" aria-live="polite"></span>
  </div>
  <p class="hash">Schema content hash <code>{schema["manifest"]["content_hash"]}</code></p>
  <textarea id="json-own" class="json" readonly aria-label="From your own seat datachain JSON">{esc(json.dumps(own, ensure_ascii=False))}</textarea>
  <textarea id="json-pricing" class="json" readonly aria-label="When prices move datachain JSON">{esc(json.dumps(pricing, ensure_ascii=False))}</textarea>
  <textarea id="json-hosted" class="json" readonly aria-label="While a doorway hosts you datachain JSON">{esc(json.dumps(hosted, ensure_ascii=False))}</textarea>
</div>"""

parts = []
for sec in sections[1:]:
    head, text = sec.split("\n", 1)
    block = md_block(text)
    sid = re.sub(r"[^a-z]+", "-", head.lower()).strip("-")
    if head.startswith("The same questions"):
        block += labels_html
    if head.startswith("When prices move"):
        block += f'<div class="labels single">{datachain(pricing, m_pricing, "pricing")}</div>'
    if head.startswith("Making a label trustworthy"):
        block += trust_html
    if head.startswith("Five elements"):
        first, rest = block.split("</p>", 1)
        rest_ul = re.sub(r"^<ul>.*?</ul>", "", rest, count=1, flags=re.S)
        block = first + "</p>" + proposals_html() + rest_ul
    if head.startswith("What we are asking"):
        sig_split = block.split("<p>With respect and gratitude,", 1)
        block = sig_split[0] + verify_html + ("<div class=\"sig\"><p>With respect and gratitude," + sig_split[1] + "</div>" if len(sig_split) > 1 else "")
    parts.append(f'<section class="letter-sec" id="{sid}"><h2>{esc(head)}</h2>{block}</section>')

opening_html = md_block(opening)
meta = dict(re.findall(r"^(\w+): (.+)$", front, flags=re.M))

page = (D / "page-template.html").read_text()
page = (page.replace("{{TITLE}}", esc(title_line.strip()))
        .replace("{{TO}}", esc(meta["to"]))
        .replace("{{LEDE}}", esc(meta.get("lede", "")))
        .replace("{{OPENING}}", opening_html)
        .replace("{{SECTIONS}}", "".join(parts)))
OUT.write_text(page)
print(f"wrote {OUT} ({len(page)//1024} KB)")
