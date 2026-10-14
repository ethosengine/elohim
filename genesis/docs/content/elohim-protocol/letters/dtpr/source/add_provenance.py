"""Attach authoring provenance and trust marks to a DTPR datachain draft.

Usage: python3 -I add_provenance.py <datachain.json> <marks.json> <out.json>

marks.json maps a placement key (element_instance_id, else element_id) to
{"backed": peer|signed|people|promise, "today": live|building|designed,
 "confidence": high|medium|low, "rationale": str, "today_note": str,
 "sources": [{"quote": str, "context": str}]}.

The two marks lead the rationale, so DTPR's renderer shows them in its
"AI proposal context" panel and any reader of the JSON sees them too.
"""

import json
import sys

BACKED = {
    "peer": "Enforced by every peer",
    "signed": "Signed and witnessed",
    "people": "Held by people",
    "promise": "Our commitment",
}
TODAY = {"live": "Live", "building": "Building", "designed": "Designed"}


def main(chain_path, marks_path, out_path):
    chain = json.load(open(chain_path))
    marks = json.load(open(marks_path))
    keys = [e.get("element_instance_id") or e["element_id"] for e in chain["elements"]]
    missing = [k for k in marks if k not in keys]
    if missing:
        sys.exit(f"marks name placements not in the chain: {missing}")
    provenance = {}
    for key, m in marks.items():
        lead = f"Backed by: {BACKED[m['backed']]}. Today: {TODAY[m['today']]}"
        if m.get("today_note"):
            lead += f" ({m['today_note']})"
        entry = {"rationale": f"{lead}. {m['rationale']}", "confidence": m["confidence"]}
        if m.get("sources"):
            entry["source_references"] = m["sources"]
        provenance[key] = entry
    chain["authoring_provenance"] = {
        "kind": "ai_generated",
        "model": "claude-opus-5-5",
        "generated_at": chain["created_at"],
        "element_provenance": provenance,
    }
    json.dump(chain, open(out_path, "w"), indent=2, ensure_ascii=False)
    unmarked = [k for k in keys if k not in marks]
    print(f"wrote {out_path}: {len(provenance)} marked, unmarked: {unmarked}")


if __name__ == "__main__":
    main(*sys.argv[1:4])
