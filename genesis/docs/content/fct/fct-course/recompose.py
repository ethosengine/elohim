#!/usr/bin/env python3
"""Recompose the course markdown into linked lamad atoms and the path walk.

The course text in content/mNN.md is the teacher's source of truth (exported from
the .docx; see README.md). This script projects it into the learning platform's
seed data: one small atom per piece a learner meets (lesson, story, discussion,
practice lanes, homework, assessments), typed relationships between them, and the
path "foundations-christian-technology" as one walk over those atoms.

What the prose cannot say lives in recompose/mNN.yaml beside the markdown:
assessment questions and the simulation labs a module points to. Everything else
(scripture roles, callbacks between modules, the three practice lanes) is read
from the markdown itself, so revising the course and re-running this regenerates
the graph.

Output is deterministic: the same inputs always write the same bytes, so seeding
can recognise an unchanged atom and leave it alone.

    python3 recompose.py              # the whole course (modules 1-15)
    python3 recompose.py --modules 1-4
    python3 recompose.py --check      # exit 1 if any output would change
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

import yaml

HERE = Path(__file__).resolve().parent
CONTENT = HERE / "content"
SIDECARS = HERE / "recompose"
REPO = HERE.parents[4]
LAMAD = REPO / "genesis" / "data" / "lamad"
OUT_CONTENT = LAMAD / "content"
OUT_PATH = LAMAD / "paths" / "foundations-christian-technology.json"

PATH_ID = "foundations-christian-technology"
COURSE_ID = PATH_ID
WORKBOOK_ID = "fct-course"
COURSE_VERSION = "review draft v2, 2026-09-16"
RECOMPOSED_AT = "2026-09-27T00:00:00.000Z"
GENERATOR = "fct-course/recompose.py"
REACH = "commons"  # the developers' declaration: this course is fit for the commons

# Module number -> the id its lesson has carried since the 2025 edition. Ids are
# stable identities that other stories and peers already reference; the lesson is
# upgraded in place rather than re-minted.
LESSON_IDS = {
    1: "fct-module-01-church-dilemma",
    2: "fct-module-02-systems-thinking",
    3: "fct-module-03-problem-well-stated",
    4: "fct-module-04-scaling-wisdom",
    5: "fct-module-05-respecting-human-nature",
    6: "fct-module-06-harmful-consequences",
    7: "fct-module-07-regeneration",
    8: "fct-module-08-centering-love",
    9: "fct-module-09-fruits-of-spirit",
    10: "fct-module-10-recognize-distortions",
    11: "fct-module-11-collaboration-sensemaking",
    12: "fct-module-12-fairness-justice",
    13: "fct-module-13-helping-thrive",
    14: "fct-module-14-ready-to-act",
    15: "fct-module-15-retrospective",
}

LANES = {
    "tech workers": ("tech-workers", "Tech workers"),
    "households": ("households", "Households"),
    "church leaders": ("church-leaders", "Church leaders"),
}

ROMAN = {"I": 1, "II": 2, "III": 3, "IV": 4, "V": 5}

# The relationship vocabulary is the lamad manifest's, never a copy of it.
RELATIONSHIP_TYPES = frozenset(json.loads(
    (REPO / "elohim" / "sdk" / "domains" / "lamad" / "manifest" / "relationships.json").read_text()))


# ---------------------------------------------------------------------------
# Parsing the course markdown
# ---------------------------------------------------------------------------


@dataclass
class Block:
    heading: str
    body: str
    children: list["Block"] = field(default_factory=list)


@dataclass
class Module:
    number: int
    title: str
    movement: int
    movement_title: str
    day: str
    arc: str
    texts: dict[str, str]  # label -> raw text ("Anchor text", "Supporting", "Story", "Lab", ...)
    preamble: str
    lead: str
    blocks: list[Block]
    source: str


def split_blocks(text: str, level: int) -> tuple[str, list[Block]]:
    """Split markdown at headings of exactly `level` hashes; return (lead, blocks)."""
    marker = "#" * level + " "
    lead: list[str] = []
    blocks: list[Block] = []
    current: Block | None = None
    buf: list[str] = []
    for line in text.splitlines():
        if line.startswith(marker):
            if current is None:
                lead = buf
            else:
                current.body = "\n".join(buf).strip()
                blocks.append(current)
            current = Block(heading=line[len(marker):].strip(), body="")
            buf = []
        else:
            buf.append(line)
    if current is None:
        lead = buf
    else:
        current.body = "\n".join(buf).strip()
        blocks.append(current)
    return "\n".join(lead).strip(), blocks


def module_titles() -> dict[int, tuple[str, int, str]]:
    """From overview.md: number -> (title, movement number, movement title)."""
    titles: dict[int, tuple[str, int, str]] = {}
    for line in (CONTENT / "overview.md").read_text().splitlines():
        m = re.match(r"\*\*([IVX]+)\. (.+?)\*\* \([^)]*\): (.+)$", line)
        if not m:
            continue
        roman, mtitle, rest = m.groups()
        for part in rest.split(" · "):
            n = re.match(r"(\d+) (.+)$", part.strip())
            if n:
                titles[int(n.group(1))] = (n.group(2).strip(), ROMAN[roman], mtitle.strip())
    return titles


def parse_module(number: int, titles: dict[int, tuple[str, int, str]]) -> Module:
    path = CONTENT / f"m{number:02d}.md"
    text = path.read_text()
    lead, blocks = split_blocks(text, 2)
    for block in blocks:
        block.body, block.children = split_blocks(block.body, 3)
    head = re.match(r"\*\*Movement ([IVX]+) — (.+?) · (Day .+?)\*\*", lead)
    if not head:
        raise SystemExit(f"{path.name}: first line is not '**Movement N — Title · Day N**'")
    arc = re.search(r"^\*Arc: (.+?)\*$", lead, re.M)
    texts_line = re.search(r"^\*((?:Anchor|Commissioning) text: .+?)\*$", lead, re.M)
    texts: dict[str, str] = {}
    if texts_line:
        for part in texts_line.group(1).split(" · "):
            label, _, value = part.partition(":")
            texts[label.strip()] = value.strip()
    # The preamble is the lead minus the three metadata lines.
    preamble = "\n".join(
        l for l in lead.splitlines()
        if not (l.startswith("**Movement") or l.startswith("*Arc:") or re.match(r"^\*(Anchor|Commissioning) text:", l))
    ).strip()
    title, movement, movement_title = titles[number]
    return Module(
        number=number,
        title=title,
        movement=movement,
        movement_title=movement_title,
        day=head.group(3),
        arc=arc.group(1) if arc else "",
        texts=texts,
        preamble=preamble,
        lead=lead,
        blocks=blocks,
        source=f"docs/content/fct/fct-course/content/{path.name}",
    )


# ---------------------------------------------------------------------------
# Scripture references
# ---------------------------------------------------------------------------

REF = re.compile(
    r"((?:[123] )?[A-Z][a-z]+(?: of [A-Z][a-z]+)?) (\d+)(?::(\d+)(?:[-–](\d+))?)?(?:[-–](\d+))?"
)


def scripture_refs(text: str) -> list[tuple[str, str]]:
    """Every 'Book C:V-V' reference in text -> [(atom id, display)]."""
    out = []
    for m in REF.finditer(text):
        book, ch, v1, v2, ch2 = m.groups()
        slug = book.lower().replace(" ", "-")
        if slug == "psalms":
            slug = "psalm"
        parts = [slug, ch] + [p for p in (v1, v2) if p] + ([ch2] if ch2 and not v1 else [])
        display = m.group(0).replace("-", "–") if not v1 else m.group(0)
        out.append(("fct-bible-" + "-".join(parts), display))
    return out


def role_refs(texts: dict[str, str]) -> list[tuple[str, str, str]]:
    """(atom id, display, role) for anchor, supporting, story and refrain texts."""
    out: list[tuple[str, str, str]] = []
    for label, role in (("Anchor text", "anchor"), ("Commissioning text", "commissioning"),
                        ("Supporting", "supporting"), ("Story", "story-text")):
        raw = texts.get(label, "")
        for chunk in re.split(r";", raw):
            chunk_role = "refrain" if role == "supporting" and "refrain" in chunk else role
            for atom_id, display in scripture_refs(chunk):
                out.append((atom_id, display, chunk_role))
    seen, unique = set(), []
    for item in out:
        if item[0] not in seen:
            seen.add(item[0])
            unique.append(item)
    return unique


def scripture_atom(atom_id: str, display: str) -> dict:
    query = display.replace(" ", "+").replace(":", "%3A").replace("–", "-")
    return {
        "id": atom_id,
        "contentType": "reference",
        "title": display,
        "description": f"Scripture: {display}",
        "content": f"## {display}\n\nRead {display} in your own Bible, or "
                   f"[online](https://www.biblegateway.com/passage/?search={query}).\n",
        "contentFormat": "markdown",
        "reach": REACH,
        "tags": ["bible", "scripture", "fct"],
        "metadata": {"category": "scripture", "courseId": COURSE_ID, "generatedBy": GENERATOR},
        "createdAt": RECOMPOSED_AT,
        "updatedAt": RECOMPOSED_AT,
    }


# ---------------------------------------------------------------------------
# Atoms
# ---------------------------------------------------------------------------


def slug(text: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", text.lower()).strip("-")


def minutes(text: str) -> str:
    words = len(text.split())
    low = max(5, round(words / 200 / 5) * 5)
    return f"{low}-{low + 10} minutes"


def first_sentence(text: str) -> str:
    plain = re.sub(r"[*_>`#]", "", text).strip().replace("\n", " ")
    m = re.match(r"(.+?[.!?])(\s|$)", plain)
    return (m.group(1) if m else plain)[:240]


def module_callbacks(text: str, own: int) -> list[int]:
    """Module numbers a passage points to ('Module 6', 'Modules 6, 8, and 9')."""
    found: set[int] = set()
    for m in re.finditer(r"Modules? ((?:\d+(?:, | and |, and |[–-]| & )?)+)", text):
        span = m.group(1)
        numbers = {int(n) for n in re.findall(r"\d+", span)}
        for a, b in re.findall(r"(\d+)[–-](\d+)", span):  # "Modules 6–9" means 6, 7, 8 and 9
            numbers.update(range(int(a), int(b) + 1))
        found.update(n for n in numbers if 1 <= n <= 15 and n != own)
    return sorted(found)


def media_index() -> list[tuple[str, str]]:
    """(title, atom id) for the course's books, films, talks and podcasts.

    Titles shorter than eight characters ("13th") are too ambiguous to find in prose,
    so they are linked by hand if at all.
    """
    out = []
    for path in sorted(OUT_CONTENT.glob("fct-media-*.json")) + sorted(OUT_CONTENT.glob("fct-video-*.json")):
        atom = json.loads(path.read_text())
        title = re.sub(r"\s*\(.*\)$", "", atom["title"]).strip()
        if len(title) >= 8:
            out.append((title, atom["id"]))
    return out


class Composer:
    def __init__(self, modules: dict[int, Module]):
        self.modules = modules
        self.atoms: dict[str, dict] = {}
        self.report: list[str] = []
        self.media = media_index()
        if not self.media:
            self.report.append("no fct-media-*/fct-video-* atoms found; lessons get no media links")

    # -- helpers -----------------------------------------------------------

    def existing(self, atom_id: str) -> dict:
        path = OUT_CONTENT / f"{atom_id}.json"
        return json.loads(path.read_text()) if path.exists() else {}

    def base(self, mod: Module, atom_id: str, content_type: str, title: str, body: str,
             form: str, extra_meta: dict | None = None, content_format: str = "markdown") -> dict:
        prior = self.existing(atom_id)
        lesson_prior = self.existing(LESSON_IDS[mod.number])
        atom = {
            "id": atom_id,
            "contentType": content_type,
            "title": title,
            "description": first_sentence(body) if content_format == "markdown" else title,
            "content": body,
            "contentFormat": content_format,
            "sourcePath": mod.source,
            "reach": REACH,
            "tags": ["fct", "christian-technology", "humane-tech", f"movement-{mod.movement}",
                     f"module-{mod.number}", form],
            "metadata": {
                "category": "fct",
                "courseId": COURSE_ID,
                "courseVersion": COURSE_VERSION,
                "sourceWorkbook": WORKBOOK_ID,
                "moduleNumber": mod.number,
                "movement": mod.movement,
                "movementTitle": mod.movement_title,
                "day": mod.day,
                "form": form,
                **(extra_meta or {}),
            },
            "relationships": [],
            "createdAt": prior.get("createdAt", RECOMPOSED_AT),
            "updatedAt": RECOMPOSED_AT,
        }
        # Carry the atom's standing in the network (who stewards it, its identifiers).
        for key in ("stewardedBy", "contributors", "did", "activityPubType", "linkedData"):
            if key in prior:
                atom[key] = prior[key]
            elif key == "stewardedBy" and key in lesson_prior:
                atom[key] = lesson_prior[key]
        return atom

    def ensure_scripture(self, ref_id: str, display: str) -> None:
        """A scripture atom for every reference an edge names, so no edge dangles.

        Atoms that already exist and were not written by this script are curated (the
        2025 import carries verse text); they are inputs here, never overwritten. Atoms
        this script wrote are regenerated from their reference alone.
        """
        existing = self.existing(ref_id)
        if ref_id not in self.atoms and (not existing or existing.get("metadata", {}).get("generatedBy") == GENERATOR):
            self.add(scripture_atom(ref_id, display))

    def edge(self, atom: dict, target: str, rtype: str, role: str) -> None:
        if rtype not in RELATIONSHIP_TYPES:
            raise SystemExit(f"{atom['id']}: relationship type {rtype} is not in the lamad manifest")
        rel = {"target": target, "type": rtype, "role": role}
        if rel not in atom["relationships"]:
            atom["relationships"].append(rel)

    def callbacks(self, atom: dict, text: str, own: int) -> None:
        for n in module_callbacks(text, own):
            self.edge(atom, LESSON_IDS[n], "RELATES_TO", "callback")

    def link_media(self, mod: Module, lesson: dict) -> None:
        """REFERENCES from the lesson to each book, film or talk the module names."""
        homework = "\n".join(b.body for b in mod.blocks if b.heading.startswith("Homework"))
        go_deeper = "\n".join(b.body for b in mod.blocks if b.heading.startswith("Go Deeper"))
        for title, media_id in self.media:
            pattern = re.compile(r"(?<![\w])" + re.escape(title) + r"(?![\w])", re.I)
            if pattern.search(homework):
                role = "assignment"
            elif pattern.search(go_deeper):
                role = "go-deeper"
            elif any(pattern.search(b.body) for b in mod.blocks):
                role = "media"
            else:
                continue
            self.edge(lesson, media_id, "REFERENCES", role)

    def add(self, atom: dict) -> str:
        self.atoms[atom["id"]] = atom
        return atom["id"]

    # -- one module --------------------------------------------------------

    def compose(self, mod: Module) -> list[dict]:
        """Build every atom of a module; return its steps on the path, in order."""
        lesson_id = LESSON_IDS[mod.number]
        sidecar_path = SIDECARS / f"m{mod.number:02d}.yaml"
        sidecar = yaml.safe_load(sidecar_path.read_text()) if sidecar_path.exists() else {}
        blocks = {b.heading: b for b in mod.blocks}

        # The lesson opens exactly as the course does: its movement line, arc and texts.
        lesson_parts: list[str] = [mod.lead]

        children: list[tuple[str, str, dict]] = []  # (atom id, role, step fields)
        objectives: list[str] = []

        for block in mod.blocks:
            h = block.heading
            if h.startswith("Learning Objectives") or h.startswith("Purpose & Objectives"):
                objectives = [re.sub(r"^\d+\.\s*", "", l).strip()
                              for l in block.body.splitlines() if re.match(r"^\d+\.", l)]
                lesson_parts.append(f"## {h}\n\n{block.body}")
            elif h == "Teaching":
                teach = [f"## Teaching"]
                for sub in block.children:
                    story = re.match(r"(Counter-Story|Story): (.+)$", sub.heading)
                    if story:
                        kind, stitle = story.groups()
                        form = "counter-story" if kind == "Counter-Story" else "story"
                        atom = self.base(mod, f"{lesson_id}-{form}", "article",
                                         f"{kind}: {stitle}", f"## {stitle}\n\n{sub.body}", form)
                        for ref_id, display in scripture_refs(sub.heading + " " + mod.texts.get("Story", "")):
                            self.edge(atom, ref_id, "REFERENCES", "story-text")
                            self.ensure_scripture(ref_id, display)
                        self.callbacks(atom, sub.body, mod.number)
                        children.append((self.add(atom), form, {
                            "stepTitle": f"{kind}: {stitle}",
                            "stepNarrative": first_sentence(sub.body),
                            "estimatedTime": minutes(sub.body),
                        }))
                    else:
                        teach.append(f"### {sub.heading}\n\n{sub.body}")
                if block.body:
                    teach.insert(1, block.body)
                lesson_parts.append("\n\n".join(teach))
            elif h.startswith("Discussion"):
                atom = self.base(mod, f"{lesson_id}-discussion", "discussion", f"Discussion: {mod.title}",
                                 f"## {h}\n\n{block.body}", "discussion")
                self.callbacks(atom, block.body, mod.number)
                children.append((self.add(atom), "discussion", {
                    "stepTitle": "Gather: discussion and activity",
                    "stepNarrative": first_sentence(block.body),
                    "estimatedTime": minutes(block.body),
                }))
            elif h.startswith("Application"):
                children.extend(self.application(mod, lesson_id, h, block))
            elif h.startswith("Homework"):
                atom = self.base(mod, f"{lesson_id}-homework", "practice", f"Homework: {mod.title}",
                                 f"## {h}\n\n{block.body}", "homework")
                self.callbacks(atom, block.body, mod.number)
                children.append((self.add(atom), "homework", {
                    "stepTitle": "The one assignment",
                    "stepNarrative": first_sentence(block.body),
                    "estimatedTime": "varies",
                }))
            elif h.startswith(("Opening", "Humane Tech Concepts", "Conclusion", "Go Deeper")):
                lesson_parts.append(f"## {h}\n\n{block.body}")
            else:
                # A module-specific practice (e.g. The Class Covenant) stands as its own atom.
                atom = self.base(mod, f"{lesson_id}-{slug(re.sub(r'^(The|A) ', '', h))}", "practice", h,
                                 f"## {h}\n\n{block.body}", "practice")
                self.callbacks(atom, block.body, mod.number)
                children.append((self.add(atom), "practice", {
                    "stepTitle": h,
                    "stepNarrative": first_sentence(block.body),
                    "estimatedTime": minutes(block.body),
                }))

        children.extend(self.assessments(mod, lesson_id, sidecar))

        lesson_body = "\n\n".join(lesson_parts) + "\n"
        lesson = self.base(mod, lesson_id, "lesson", f"FCT {mod.number}: {mod.title}", lesson_body,
                           "lesson", {"learningObjectiveCount": len(objectives)})
        lesson["description"] = mod.arc or first_sentence(mod.preamble)
        lesson["learningObjectives"] = objectives
        for ref_id, display, role in role_refs(mod.texts):
            self.edge(lesson, ref_id, "REFERENCES", role)
            self.ensure_scripture(ref_id, display)
        for child_id, role, _ in children:
            if role == "lane":
                continue  # a lane belongs to its module's practice hub, not the lesson
            self.edge(lesson, child_id, "CONTAINS", role)
            self.edge(self.atoms[child_id], lesson_id, "BELONGS_TO", role)
        for lab in sidecar.get("labs", []):
            self.edge(lesson, lab, "RELATES_TO", "lab")
        if mod.number > 1:
            self.edge(lesson, LESSON_IDS[mod.number - 1], "FOLLOWS", "sequence")
        self.edge(lesson, WORKBOOK_ID, "REFERENCES", "source-workbook")
        self.link_media(mod, lesson)
        self.callbacks(lesson, lesson_body, mod.number)
        lesson["children"] = [c for c, role, _ in children if role != "lane"] + [r for r, _, _ in role_refs(mod.texts)]
        lesson["relatedNodeIds"] = sorted({r["target"] for r in lesson["relationships"]
                                           if r["type"] == "RELATES_TO"})
        self.add(lesson)

        steps = [{
            "resourceId": lesson_id,
            "stepTitle": mod.title,
            "stepNarrative": lesson["description"],
            "learningObjectives": objectives,
            "completionCriteria": ["view"],
            "estimatedTime": minutes(lesson_body),
        }]
        for lab in sidecar.get("labs", []):
            steps.append({
                "resourceId": lab,
                "stepTitle": sidecar.get("labTitles", {}).get(lab, lab),
                "stepNarrative": sidecar.get("labNarratives", {}).get(lab, ""),
                "completionCriteria": ["interaction"],
                "estimatedTime": "30 minutes",
            })
        for child_id, role, fields in children:
            step = {"resourceId": child_id, **fields}
            step["completionCriteria"] = ["score"] if role in ("quiz", "reflection") else ["view"]
            steps.append(step)
        # The workbook bundle is the SOURCE of this path, never a step on it: a module's structure is
        # the path's structure, and a step that carries the whole course subdivided inside itself
        # would put a second, unwalkable table of contents inside module 1 (operator, 2026-10-08).
        # Each lesson keeps its REFERENCES source-workbook edge as provenance.
        return steps

    def application(self, mod: Module, lesson_id: str, heading: str, block: Block) -> list:
        """The three practice lanes become one atom per audience, held by a lane hub."""
        lines = block.body.splitlines()
        lane_texts: dict[str, list[str]] = {}
        intro: list[str] = []
        current = None
        for line in lines:
            m = re.match(r"- \*\*([^:*]+):\*\*\s*(.*)$", line)
            if m:
                current = m.group(1).strip()
                lane_texts[current] = [m.group(2)]
            elif current and line.strip() and not line.startswith("- "):
                lane_texts[current].append(line.strip())
            elif current is None:
                intro.append(line)
        hub = self.base(mod, f"{lesson_id}-application", "practice", f"Practice: {mod.title}",
                        f"## {heading}\n\n" + ("\n".join(intro).strip() or block.body), "practice-lanes")
        if not lane_texts:
            hub["content"] = f"## {heading}\n\n{block.body}"
        standard_lanes = bool(lane_texts) and all(label.lower() in LANES for label in lane_texts)
        # A module that widens the three lanes (Module 14's sending circles) is titled
        # by its own heading rather than invited to "choose a lane".
        widened_title = "Practice: " + re.sub(r"^Application\s*[—-]\s*", "", heading)
        steps = [(self.add(hub), "practice", {
            "stepTitle": "Practice: choose your lane" if standard_lanes else (widened_title if lane_texts else heading),
            "stepNarrative": first_sentence("\n".join(intro) or block.body),
            "estimatedTime": minutes(block.body),
        })]
        self.callbacks(hub, block.body, mod.number)
        for label, text_lines in lane_texts.items():
            # The three audiences keep one slug across the course; a module that widens
            # them (Module 14's sending circles) names its own.
            lane_slug, lane_title = LANES.get(label.lower(), (slug(re.sub(r"^The ", "", label)), label))
            if label.lower() not in LANES:
                self.report.append(f"m{mod.number:02d}: lane '{label}' is not one of the three audiences")
            text = " ".join(text_lines)
            lane = self.base(mod, f"{lesson_id}-lane-{lane_slug}", "practice",
                             f"{lane_title}: {mod.title}", f"## {lane_title}\n\n{text}", "practice-lane",
                             {"audience": lane_slug})
            self.edge(lane, hub["id"], "BELONGS_TO", "lane")
            self.edge(hub, lane["id"], "CONTAINS", "lane")
            self.callbacks(lane, text, mod.number)
            self.add(lane)
            steps.append((lane["id"], "lane", {
                "stepTitle": f"Practice for {lane_title.lower()}",
                "stepNarrative": first_sentence(text),
                "optional": True,
                "estimatedTime": "one week",
            }))
        return steps

    def assessments(self, mod: Module, lesson_id: str, sidecar: dict) -> list:
        out = []
        quiz = sidecar.get("quiz") or []
        if quiz:
            questions = [self.radio_question(lesson_id, i, q) for i, q in enumerate(quiz, 1)]
            atom = self.base(mod, f"{lesson_id}-quiz", "assessment", f"Check your understanding: {mod.title}",
                             questions, "quiz", {"assessmentType": "mastery", "questionCount": len(questions)},
                             content_format="sophia-quiz-json")
            atom["description"] = f"Mastery check for Module {mod.number}, drawn from its learning objectives."
            self.edge(atom, lesson_id, "VALIDATES", "mastery")
            out.append((self.add(atom), "quiz", {
                "stepTitle": "Check your understanding",
                "stepNarrative": atom["description"],
                "estimatedTime": f"{max(5, len(questions) * 2)} minutes",
            }))
        for i, r in enumerate(sidecar.get("reflections") or [], 1):
            key = r.get("id", f"reflection-{i}" if i > 1 else "reflection")
            question = self.free_response(lesson_id, key, r)
            atom = self.base(mod, f"{lesson_id}-{key}", "assessment", r["title"], [question], "reflection",
                             {"assessmentType": "reflection", "questionCount": 1},
                             content_format="sophia-quiz-json")
            atom["description"] = r["prompt"][:240]
            self.edge(atom, lesson_id, "VALIDATES", "reflection")
            out.append((self.add(atom), "reflection", {
                "stepTitle": r["title"],
                "stepNarrative": r.get("narrative", "Not graded: your answer is for you and your group."),
                "estimatedTime": r.get("time", "10 minutes"),
            }))
        # A sidecar that names `quiz:` with nothing under it declares the absence
        # (Module 15 is consecration, not content); a sidecar that never mentions it is a gap.
        if not quiz and "quiz" not in sidecar:
            self.report.append(f"m{mod.number:02d}: no quiz authored in recompose/m{mod.number:02d}.yaml")
        return out

    @staticmethod
    def radio_question(lesson_id: str, n: int, q: dict) -> dict:
        qid = f"{lesson_id}-q{n}"
        choices = []
        for j, c in enumerate(q["choices"]):
            correct = c.endswith(" *")
            choices.append({"content": c[:-2] if correct else c, "correct": correct, "id": f"{qid}-c{j}"})
        if sum(c["correct"] for c in choices) != 1:
            raise SystemExit(f"{qid}: mark exactly one choice correct with a trailing ' *'")
        return {
            "id": qid,
            "purpose": "mastery",
            "content": {
                "content": f"{q['q']}\n\n[[☃ radio 1]]",
                "images": {},
                "widgets": {"radio 1": {
                    "type": "radio",
                    "options": {"choices": choices, "randomize": True, "multipleSelect": False},
                    "graded": True,
                    "version": {"major": 0, "minor": 0},
                }},
            },
            "hints": [{"content": h, "images": {}, "widgets": {}} for h in q.get("hints", [])],
            "metadata": {
                "sourceContentId": f"{lesson_id}-quiz",
                "assessesContentId": lesson_id,
                "bloomsLevel": q.get("bloom", "understand"),
                "difficulty": q.get("difficulty", "medium"),
                "estimatedTimeSeconds": 60,
                "questionType": "core",
                "tags": ["fct"],
            },
        }

    @staticmethod
    def free_response(lesson_id: str, key: str, r: dict) -> dict:
        return {
            "id": f"{lesson_id}-{key}",
            "purpose": "reflection",
            "content": {
                "content": "[[☃ free-response 1]]",
                "images": {},
                "widgets": {"free-response 1": {
                    "type": "free-response",
                    "options": {
                        "allowUnlimitedCharacters": False,
                        "characterLimit": r.get("characterLimit", 1500),
                        "placeholder": r.get("placeholder", ""),
                        "question": r["prompt"],
                        "scoringCriteria": [{"text": c} for c in r.get("criteria", [])],
                    },
                    "graded": False,
                    "version": {"major": 0, "minor": 0},
                }},
            },
            "metadata": {
                "sourceContentId": f"{lesson_id}-{key}",
                "assessesContentId": lesson_id,
                "bloomsLevel": r.get("bloom", "evaluate"),
                "estimatedTimeSeconds": 600,
                "questionType": "open-response",
                "tags": ["fct", "reflection"],
            },
        }


# ---------------------------------------------------------------------------
# The path walk
# ---------------------------------------------------------------------------


def legacy_steps(number: int, title: str) -> list[dict]:
    return [{
        "resourceId": LESSON_IDS[number],
        "stepTitle": title,
        "stepNarrative": "The 2025 edition of this module. Its recomposition from the v2 course is next.",
        "completionCriteria": ["view"],
        "estimatedTime": "30-45 minutes",
    }]


def build_path(recomposed: dict[int, list[dict]], titles: dict[int, tuple[str, int, str]],
               modules: dict[int, Module]) -> dict:
    prior = json.loads(OUT_PATH.read_text())
    chapters = []
    for mv in range(1, 6):
        numbers = [n for n, (_, m, _) in sorted(titles.items()) if m == mv]
        mtitle = titles[numbers[0]][2]
        mods = []
        for order, n in enumerate(numbers):
            steps = recomposed.get(n) or legacy_steps(n, titles[n][0])
            mods.append({
                "id": f"fct-m{n:02d}",
                "title": f"{n}. {titles[n][0]}",
                "description": modules[n].arc if n in modules else "",
                "order": order,
                "steps": [{"order": i, **s} for i, s in enumerate(steps)],
            })
        chapters.append({
            "id": f"fct-movement-{mv}",
            "title": f"Movement {['I', 'II', 'III', 'IV', 'V'][mv - 1]}: {mtitle}",
            "description": MOVEMENT_ARCS[mv],
            "order": mv - 1,
            "modules": mods,
        })
    flat = [s for ch in chapters for m in ch["modules"] for s in m["steps"]]
    path = {k: v for k, v in prior.items() if k not in ("chapters", "steps")}
    path.update({
        "version": "2.0.0",
        "description": "A Christian companion to the Center for Humane Technology's Foundations of "
                       "Humane Technology: fifteen modules in five movements, walking the shape of the "
                       "gospel story from lament to commissioning.",
        "reach": REACH,
        "updatedAt": RECOMPOSED_AT,
        "chapters": chapters,
        "steps": [{**s, "order": i} for i, s in enumerate(flat)],
    })
    path["metadata"] = {**prior.get("metadata", {}), "courseVersion": COURSE_VERSION,
                        "courseBundleNodeId": WORKBOOK_ID, "recomposedModules": sorted(recomposed)}
    return path


MOVEMENT_ARCS = {
    1: "Lament & sight: from assuming the church stands outside the crisis to seeing it clearly.",
    2: "Repentance & repair: turning from what captures our attention, and repairing what it cost.",
    3: "Reordering loves: love as the measure, and fruit as the metric.",
    4: "Rebuilding common life: truth, collaboration and justice among us.",
    5: "Abundant life & sending: thriving, acting, and being sent.",
}


# ---------------------------------------------------------------------------


def dump(obj: dict) -> str:
    return json.dumps(obj, indent=2, ensure_ascii=False) + "\n"


def parse_range(text: str) -> list[int]:
    out: list[int] = []
    for part in text.split(","):
        a, _, b = part.partition("-")
        out.extend(range(int(a), int(b or a) + 1))
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--modules", default="1-15", help="module numbers to recompose (default 1-15, the whole course)")
    ap.add_argument("--check", action="store_true", help="write nothing; exit 1 if outputs would change")
    args = ap.parse_args()

    titles = module_titles()
    selected = parse_range(args.modules)
    modules = {n: parse_module(n, titles) for n in selected}
    composer = Composer(modules)
    recomposed = {n: composer.compose(modules[n]) for n in selected}
    outputs = {OUT_CONTENT / f"{i}.json": dump(a) for i, a in sorted(composer.atoms.items())}
    outputs[OUT_PATH] = dump(build_path(recomposed, titles, modules))

    changed = [p for p, text in outputs.items() if not p.exists() or p.read_text() != text]
    for line in composer.report:
        print(f"note: {line}")
    if args.check:
        for p in changed:
            print(f"would change: {p.relative_to(REPO)}")
        return 1 if changed else 0
    for p in changed:
        p.write_text(outputs[p])
    print(f"recomposed modules {selected}: {len(composer.atoms)} atoms, "
          f"{len(changed)} file(s) written, {len(outputs) - len(changed)} unchanged")
    return 0


if __name__ == "__main__":
    sys.exit(main())
