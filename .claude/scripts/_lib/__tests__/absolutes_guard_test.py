#!/usr/bin/env python3
"""Harness for the absolutes guard (`epr:validator-absolutes-guard`, `_lib/absolutes.py`).

The mirror of `sovereignty_guard_test.py` for absolute promises on trust surfaces. It locks:
  1. the lexicon fires word-bounded over the folded shadow (homoglyphs included), never inside a
     longer word or a hyphenated compound (`nevertheless`, `always-on`);
  2. only trust surfaces and only prose: `.md` prose, `.feature` steps and `#` narrative, `.html` text nodes and visible
     attributes, UI string literals in `.ts`/`.js`; never code comments, identifiers, single-word
     enum literals, logs, CSS strings, fenced/inline code, frontmatter, Gherkin tags or tables;
  3. exemptions: a RESOLVING enforcer citation (repo path with optional suffix, `policy:<id>@<v>`,
     `seam:<id>`) in the same sentence (prose) or the same/preceding line (code, templates); a
     dangling citation is reported, not honoured; testimony (blockquote, quoted speech,
     `testimony: true`); a negated capability needs the same citation;
  4. net-new only (multiset line delta);
  5. the validator: carries no class of its own, names the frame atom, mints no CID, and an
     unreadable prior fails toward surfacing (the guard is advisory);
  6. the frame atom's CID is pinned here, and the policy row's `frame-ref` matches it.

Run: python3 .claude/scripts/_lib/__tests__/absolutes_guard_test.py   (no pytest in this repo)
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from _lib import absolutes  # noqa: E402
from _lib import epr_meta as em  # noqa: E402
from _lib import frame_atoms  # noqa: E402

REPO = Path(__file__).resolve().parents[4]
GUARD = em.REFERENCE_VALIDATORS["epr:validator-absolutes-guard"]
ATOM_CID = "bafyreifu5muluyfeiay74fvopftanyh4xvbs65iuxya55dpgevbwy7236u"
checks = 0
failures = []


def check(name, got, expected):
    global checks
    checks += 1
    if got != expected:
        failures.append(f"{name}: got {got!r}, expected {expected!r}")


def hits(rel, post, prior=""):
    r = absolutes.classify(rel, post, prior, REPO)
    return [] if r is None else [(h["line"], h["phrase"]) for h in r["hits"]]


MD = "genesis/docs/specs/zz-absolutes-fixture.md"
FEAT = "genesis/a2o/features/protocol/zz-fixture.feature"
HTML = "app/elohim-app/src/app/zz-fixture.component.html"
TS = "app/elohim-elements/elohim-core/src/zz-fixture.ts"
CHROME = "elohim/elohim-chrome-asset/src/omni-element.js"

# ── 1. lexicon, word-bounded over the folded shadow ───────────────────────────────────────────
check("always fires", hits(MD, "Your data is always yours.\n"), [(1, "always")])
check("multiword phrase fires", hits(MD, "No one can see it.\n"), [(1, "no one can")])
check("100% fires", hits(MD, "It is 100% private.\n"), [(1, "100%")])
check("case-insensitive", hits(MD, "NEVER shared.\n"), [(1, "never")])
check("homoglyph folds", hits(MD, "It is nеver shared.\n"), [(1, "never")])
check("not inside a longer word", hits(MD, "Nevertheless, it works. Everything holds.\n"), [])
check("not in a hyphenated compound", hits(MD, "An always-on node; a never-ending story.\n"), [])
check("two in one line", hits(MD, "Always on, never off.\n"), [(1, "always"), (1, "never")])

# ── 2. trust surfaces and prose only ──────────────────────────────────────────────────────────
check("off-surface path is quiet", absolutes.classify("CLAUDE.md", "never\n", "", REPO), None)
check("spec test file is not a surface", absolutes.surface_kind(
    "app/elohim-elements/elohim-core/src/x.spec.ts"), None)
check("chrome element is a code surface", absolutes.surface_kind(CHROME), "code")
check("feature is a surface", absolutes.surface_kind(FEAT), "feature")
check("md fenced + inline code + frontmatter skipped",
      hits(MD, "---\ndescription: never\n---\n```\nalways\n```\nUse `never`.\n"), [])
check("md link target skipped", hits(MD, "See [the doc](never/always.md).\n"), [])
check("feature step UI copy fires",
      hits(FEAT, "Feature: x\n  Then I see \"always your own choice\"\n"), [(2, "always")])
check("feature tags and tables skipped",
      hits(FEAT, "@never\nFeature: x\n  | never |\n"), [])
check("feature # comments are the story's prose and fire",
      hits(FEAT, "Feature: x\n  # Camera is always the person's own choice.\n"), [(2, "always")])
check("feature (enforced:) citation inside a # comment exempts its sentence",
      hits(FEAT, "Feature: x\n  # Camera is always asked (enforced: .claude/scripts/_lib/absolutes.py).\n"),
      [])
check("ts CSS declaration string skipped",
      hits(TS, "const c = 'align-items: center; width: 100%; padding: 0;';\n"), [])
check("html text node and title attribute fire",
      hits(HTML, '<p title="Always private">Kept <b>forever</b></p>\n'),
      [(1, "always"), (1, "forever")])
check("html tags, bindings, interpolation and control flow skipped",
      hits(HTML, '<div class="never" [title]="always">{{ never }}</div>\n'
                 '@if (always) {\n<span></span>\n}\n'), [])
check("ts UI string literal fires",
      hits(TS, "const copy = { label: 'Always your own choice' };\n"), [(1, "always")])
check("ts comments, enum literals, logs skipped",
      hits(TS, "// never shown\n/* always */\nconst m = 'never';\n"
               "console.log('never ever happens here');\n"), [])
check("lit html template text fires, css template skipped",
      hits(TS, "return html`<p>Kept forever, ${x}</p>`;\nconst s = css`.never { a: b }`;\n"),
      [(1, "forever")])

# ── 3. exemptions ─────────────────────────────────────────────────────────────────────────────
check("resolving path citation exempts",
      hits(MD, "No one can read it (enforced: .claude/scripts/_lib/absolutes.py).\n"), [])
check("path citation with a ::test suffix resolves",
      hits(MD, "It is never sold (enforced: .claude/scripts/_lib/absolutes.py::classify).\n"), [])
check("policy citation resolves",
      hits(MD, "It is never apex (enforced: policy:sovereignty-ontology-guard@5).\n"), [])
check("unknown policy version does not resolve",
      hits(MD, "It is never apex (enforced: policy:sovereignty-ontology-guard@99).\n"),
      [(1, "never")])
check("dangling path citation is reported",
      absolutes.classify(MD, "No one can (enforced: no/such/enforcer.rs).\n", "", REPO)
      ["hits"][0]["citation"], "no/such/enforcer.rs")
check("citation covers the same sentence across a wrapped line",
      hits(MD, "Consent is always\nasked first (enforced: .claude/scripts/_lib/absolutes.py).\n"), [])
check("citation does not cover the next sentence",
      hits(MD, "It is checked (enforced: .claude/scripts/_lib/absolutes.py). It is never\nsold.\n"),
      [(1, "never")])
check("code: absolute comment on the preceding line exempts",
      hits(TS, "// absolute: .claude/scripts/_lib/absolutes.py\n"
               "const copy = { label: 'Always your own choice' };\n"), [])
check("html: absolute comment on the same line exempts",
      hits(HTML, "<p>Never shared</p> <!-- absolute: .claude/scripts/_lib/absolutes.py -->\n"), [])
check("blockquote is testimony", hits(MD, "> I will never forget.\n"), [])
check("quoted speech is testimony", hits(MD, 'She said "we always help".\n'), [])
check("curly-quoted speech in a feature is testimony",
      hits(FEAT, "Feature: x\n  She says “I never give up”\n"), [])
check("testimony frontmatter exempts the doc",
      absolutes.classify(MD, "---\ntestimony: true\n---\nNever again.\n", "", REPO), None)
check("negated capability needs the citation too", hits(MD, "Nobody can revoke it.\n"),
      [(1, "nobody can")])

# ── 4. net-new only ───────────────────────────────────────────────────────────────────────────
check("unchanged is quiet", hits(MD, "It is never sold.\n", "It is never sold.\n"), [])
check("rewording a line that already carried it is quiet",
      hits(MD, "It is never resold.\n", "It is never sold.\n"), [])
check("removing an absolute is quiet", hits(MD, "It is sold by default.\n", "It is never sold.\n"),
      [])
check("adding one more fires",
      hits(MD, "It is never sold.\nKept forever.\n", "It is never sold.\n"), [(2, "forever")])

# ── 5. the validator ──────────────────────────────────────────────────────────────────────────
_v = GUARD({"content": "Your data is always yours.\n", "is_new": True,
            "path": str(REPO / MD)})
check("validator fires on a trust surface", _v is not None, True)
check("verdict carries no class of its own (the rule's declared class applies)", _v.cls, None)
check("evidence names the frame atom", _v.evidence["frameRef"], ATOM_CID)
check("no classification CID minted in Python", _v.evidence["classificationCid"], None)
check("reason names the repair", "(enforced:" in _v.reason and "by default" in _v.reason, True)
check("validator quiet off-surface",
      GUARD({"content": "always\n", "is_new": True, "path": str(REPO / "README.md")}), None)
check("unreadable prior surfaces (advisory fails toward surfacing)",
      GUARD({"content": "It is guaranteed.\n", "is_new": False,
             "path": str(REPO / "genesis/docs/specs/zz-missing-absolutes.md")}) is not None, True)

# ── 4b. removing an adjacent citation is net-new ──────────────────────────────────────────────
_CITE = "(enforced: .claude/scripts/_lib/absolutes.py)"
check("removing a citation on the preceding line (html) surfaces the unchanged absolute",
      hits(HTML, "<p>Consent is always asked.</p>\n", f"<!-- absolute: {_CITE[11:-1]} -->\n<p>Consent is always asked.</p>\n"),
      [(1, "always")])
check("removing the wrapped-sentence citation line (md) surfaces the unchanged absolute",
      hits(MD, "Consent is always\nasked first.\n", f"Consent is always\nasked first {_CITE}.\n"),
      [(1, "always")])
check("removing a citation line only (md, no added line) surfaces",
      hits(MD, "Consent is always asked.\n\nOther.\n", f"Consent is always asked\n{_CITE}.\n\nOther.\n"),
      [(1, "always")])
check("an unchanged cited absolute stays quiet",
      hits(MD, f"Consent is always\nasked first {_CITE}.\nNew line.\n", f"Consent is always\nasked first {_CITE}.\n"), [])
check("an unchanged uncited absolute is not re-surfaced by an unrelated edit",
      hits(MD, "Consent is always asked.\nNew line.\n", "Consent is always asked.\n"), [])

# ── 6. the atom pin and the policy row ────────────────────────────────────────────────────────
_frames = frame_atoms.load_frames(REPO)
check("atom CID pinned", _frames["epr:validator-absolutes-guard"][1], ATOM_CID)
_pol, _errs = em.load_policies(REPO)
_row = _pol.get("absolutes-guard@1") or {}
check("policy row is dispatch", _row.get("class"), "dispatch")
check("policy row frame-ref matches the atom", _row.get("frame-ref"), ATOM_CID)
check("policy row binds the validator", _row.get("validator"), "epr:validator-absolutes-guard")

print(f"{checks - len(failures)}/{checks} checks passed")
for f in failures:
    print(f"  FAIL {f}")
sys.exit(1 if failures else 0)
