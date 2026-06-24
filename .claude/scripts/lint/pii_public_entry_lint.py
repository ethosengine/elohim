"""Flag PII-lexicon fields on PUBLIC Holochain integrity entries.

The DHT is world-readable + append-only: PII on a public entry is an un-recallable leak.
Public is the Holochain default; only variants decorated with
    #[entry_type(visibility = "private")]
in the `EntryTypes` enum are exempt.  The visibility attribute lives on the ENUM VARIANT,
NOT on the pub struct — so we parse EntryTypes enums first to build a private-struct set,
then skip those structs when scanning fields.
"""
import json, re, sys, glob, os
from dataclasses import dataclass

# ---------------------------------------------------------------------------
# PII lexicon — short ambiguous tokens (lat, lng, gps, dob) are guarded with
# negative letter-lookbehind + negative letter-lookahead so they don't fire
# as substrings of longer words (e.g. lat→re*lat*ed, flat, template, platform).
# Other tokens are matched as substrings because their embeddings ARE PII:
#   name → display_name, risk_name, username; bio → bio_notes; note → footnote.
# ---------------------------------------------------------------------------
PII_LEXICON = re.compile(
    r"(name|bio|location|address|avatar|photo|"
    r"(?<![a-zA-Z])lat(?![a-zA-Z])|(?<![a-zA-Z])lng(?![a-zA-Z])|"
    r"longitude|latitude|context|external_identifier|note|email|phone|birth|"
    r"(?<![a-zA-Z])dob(?![a-zA-Z])|(?<![a-zA-Z])gps(?![a-zA-Z]))",
    re.I,
)

# A field is SAFE if its TYPE is plainly a hash/cid/key/pointer/commitment/timestamp.
SAFE_TYPE = re.compile(
    r"(Hash|Cid|CID|AgentPubKey|Timestamp|EntryHash|ActionHash|Signature|\[u8|commitment|nonce|encrypted)",
    re.I,
)

# ---------------------------------------------------------------------------
# Regex patterns
# ---------------------------------------------------------------------------

# Match an EntryTypes enum declaration and capture its body.
# We look for:
#   #[unit_enum(...)]          (optional — may or may not precede)
#   pub enum EntryTypes { ... }
ENTRY_TYPES_ENUM = re.compile(
    r"pub\s+enum\s+EntryTypes\s*\{([^}]*(?:\{[^}]*\}[^}]*)*)\}",
    re.S,
)

# Within the EntryTypes body, match a variant that has a
# #[entry_type(visibility = "private")] attribute immediately before it.
PRIVATE_VARIANT = re.compile(
    r'#\[entry_type\s*\([^)]*visibility\s*=\s*"private"[^)]*\)\]\s*\n\s*(\w+)\s*\(',
    re.S,
)

# Match a pub struct and its body.
# We deliberately do NOT try to capture an entry_type attribute here — visibility
# is declared on the enum variant, not on the struct.  We match everything from
# "pub struct NAME {" to the matching closing "}", tolerating nested braces for
# generic types like Vec<(String, u32)>.
STRUCT_HEADER = re.compile(r"\bpub\s+struct\s+(\w+)\s*\{")
FIELD = re.compile(r"\bpub\s+(\w+)\s*:\s*([^,\n}]+)")


def _find_structs(src: str):
    """Yield (struct_name, body_str) for every `pub struct` in src.

    We walk char-by-char from each struct header to find the matching `}`,
    which handles nested braces inside field types correctly.
    """
    for m in STRUCT_HEADER.finditer(src):
        name = m.group(1)
        start = m.end()  # position just after '{'
        # Walk forward to find the matching closing brace.
        depth = 1
        i = start
        while i < len(src) and depth > 0:
            c = src[i]
            if c == "{":
                depth += 1
            elif c == "}":
                depth -= 1
            i += 1
        body = src[start : i - 1]
        yield name, body


def _collect_private_struct_names(src: str) -> set:
    """Parse EntryTypes enum(s) in src and return names of private-variant structs."""
    private = set()
    for em in ENTRY_TYPES_ENUM.finditer(src):
        enum_body = em.group(1)
        for vm in PRIVATE_VARIANT.finditer(enum_body):
            # The variant name and the inner type name are the same by Holochain convention:
            #   #[entry_type(visibility = "private")]
            #   AttentionTending(AttentionTending),
            private.add(vm.group(1))
    return private


@dataclass(frozen=True)
class Hit:
    struct: str
    field: str
    ftype: str


def scan_source(src: str, visibility: str = None, private_structs: set = None):
    """Scan a Rust source string for PII fields on public structs.

    Parameters
    ----------
    src:             Raw Rust source text.
    visibility:      Legacy unit-test shim.  Pass "private" to treat ALL structs
                     in src as private (exempting them).  Pass "public" or None to
                     scan normally (private_structs set governs per-struct exemption).
    private_structs: Set of struct names that are private per the EntryTypes enum.
                     Populated by scan_tree(); unit tests may pass None to skip.
    """
    if private_structs is None:
        private_structs = set()

    # Legacy shim: if caller passes visibility="private" treat the whole file as private.
    if visibility == "private":
        return []

    hits = []
    for name, body in _find_structs(src):
        if name in private_structs:
            continue
        for fm in FIELD.finditer(body):
            fname, ftype = fm.group(1), fm.group(2).strip()
            if PII_LEXICON.search(fname) and not SAFE_TYPE.search(ftype):
                hits.append(Hit(name, fname, ftype))
    return hits


def scan_tree():
    """Walk all *_integrity/src/**/*.rs files and return a list of hit dicts.

    For each file we first parse any EntryTypes enum declarations in the SAME
    file (the primary lib.rs contains them) so we can exempt private-variant
    structs, even when those structs live in a separate file within the same
    integrity crate.  We collect private names across ALL integrity files in a
    crate directory before scanning individual files.
    """
    hits = []
    # Collect per-crate private struct name sets.
    # Key = crate root dir (parent of "src"), value = set of private struct names.
    crate_private: dict[str, set] = {}

    integrity_files = glob.glob(
        "elohim/holochain/dna/**/*_integrity/src/**/*.rs", recursive=True
    )

    for path in integrity_files:
        # Crate src dir is the ancestor "src" folder.
        parts = path.replace("\\", "/").split("/")
        try:
            src_idx = max(i for i, p in enumerate(parts) if p == "src")
        except ValueError:
            continue
        crate_dir = "/".join(parts[: src_idx - 1])  # one above "src"
        if crate_dir not in crate_private:
            # Build the private-names set from ALL files in this crate's src tree.
            crate_src = "/".join(parts[: src_idx + 1])
            private_names: set = set()
            for rs_path in glob.glob(f"{crate_src}/**/*.rs", recursive=True):
                with open(rs_path) as f:
                    private_names |= _collect_private_struct_names(f.read())
            crate_private[crate_dir] = private_names

    for path in integrity_files:
        parts = path.replace("\\", "/").split("/")
        try:
            src_idx = max(i for i, p in enumerate(parts) if p == "src")
        except ValueError:
            continue
        crate_dir = "/".join(parts[: src_idx - 1])
        private_names = crate_private.get(crate_dir, set())

        with open(path) as f:
            src = f.read()
        for h in scan_source(src, private_structs=private_names):
            hits.append({"path": path, "struct": h.struct, "field": h.field})

    return hits


def main(argv):
    baseline_path = ".claude/scripts/lint/pii_public_entry_baseline.json"
    hits = scan_tree()
    if "--update-baseline" in argv:
        sorted_hits = sorted(hits, key=lambda h: (h["path"], h["struct"], h["field"]))
        with open(baseline_path, "w") as f:
            json.dump(sorted_hits, f, indent=2)
            f.write("\n")
        print(f"baseline updated: {len(hits)} known PII-on-public-entry fields")
        return 0
    try:
        with open(baseline_path) as f:
            baseline = json.load(f)
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
        return 1 if new else 0  # warn-mode: only NEW violations fail the build
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
