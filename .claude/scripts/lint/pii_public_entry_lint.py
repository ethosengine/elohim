"""Flag PII-lexicon fields on PUBLIC Holochain integrity entries.
The DHT is world-readable + append-only: PII on a public entry is an un-recallable leak.
Public is the Holochain default; only `#[entry_type(visibility = "private")]` is exempt."""
import json, re, sys, glob
from dataclasses import dataclass

PII_LEXICON = re.compile(
    r"(name|bio|location|address|avatar|photo|lat|lng|longitude|latitude|"
    r"context|external_identifier|note|email|phone|birth|dob|gps)", re.I)
# A field is SAFE if it is plainly a hash/cid/key/pointer/commitment/timestamp.
SAFE_TYPE = re.compile(r"(Hash|Cid|CID|AgentPubKey|Timestamp|EntryHash|ActionHash|Signature|\[u8|commitment|nonce|encrypted)", re.I)
STRUCT = re.compile(r"(#\[entry_type\([^)]*\)\]\s*)?pub struct (\w+)\s*\{([^}]*)\}", re.S)
FIELD = re.compile(r"pub (\w+)\s*:\s*([^,\n]+)")

@dataclass(frozen=True)
class Hit:
    struct: str
    field: str
    ftype: str

def scan_source(src: str, visibility: str = None):
    hits = []
    for m in STRUCT.finditer(src):
        attr, name, body = m.group(1) or "", m.group(2), m.group(3)
        is_private = visibility == "private" or 'visibility = "private"' in attr
        if is_private:
            continue
        for fm in FIELD.finditer(body):
            fname, ftype = fm.group(1), fm.group(2).strip()
            if PII_LEXICON.search(fname) and not SAFE_TYPE.search(ftype):
                hits.append(Hit(name, fname, ftype))
    return hits

def scan_tree():
    hits = []
    for path in glob.glob("elohim/holochain/dna/**/*_integrity/src/**/*.rs", recursive=True):
        with open(path) as f:
            for h in scan_source(f.read()):
                hits.append({"path": path, "struct": h.struct, "field": h.field})
    return hits

def main(argv):
    baseline_path = ".claude/scripts/lint/pii_public_entry_baseline.json"
    hits = scan_tree()
    if "--update-baseline" in argv:
        json.dump(sorted(hits, key=lambda h: (h["path"], h["struct"], h["field"])),
                  open(baseline_path, "w"), indent=2)
        print(f"baseline updated: {len(hits)} known PII-on-public-entry fields")
        return 0
    try:
        baseline = json.load(open(baseline_path))
    except FileNotFoundError:
        baseline = []
    baseset = {(h["path"], h["struct"], h["field"]) for h in baseline}
    new = [h for h in hits if (h["path"], h["struct"], h["field"]) not in baseset]
    for h in hits:
        print(f"PII-ON-PUBLIC-ENTRY: {h['struct']}.{h['field']}  ({h['path']})")
    if new:
        print(f"\n*** {len(new)} NEW PII field(s) on public entries — not in baseline ***")
        for h in new:
            print(f"  NEW: {h['struct']}.{h['field']}  ({h['path']})")
    if "--check" in argv:
        return 1 if new else 0           # warn-mode: only NEW violations fail the build
    return 0

if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
