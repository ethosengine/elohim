"""Schema-contract test for .epr-meta. Run:
python3 .claude/scripts/_lib/__tests__/epr_meta_schema_test.py  (exit 0 = pass)"""
import json, sys
from pathlib import Path

here = Path(__file__).resolve()
for _ in range(8):
    if (here / ".claude" / "scripts" / "_lib").is_dir():
        break
    here = here.parent
REPO = here  # dir holding .claude/

_passed = 0
def check(label, cond):
    global _passed
    assert cond, f"FAIL: {label}"
    _passed += 1
    print(f"  ✅ {label}")

s = json.loads((REPO / "elohim/sdk/schemas/v1/objects/epr-meta.schema.json").read_text())
check("$schema draft 2020-12", s["$schema"] == "https://json-schema.org/draft/2020-12/schema")
check("$id", s["$id"] == "epr:schema:objects:epr-meta")
check("title EprMeta", s["title"] == "EprMeta")
check("additionalProperties false", s["additionalProperties"] is False)
check("declares Source of truth:", "Source of truth:" in s["description"])
check("declares Category C", "Category C" in s["description"])
check("self-contained leaf (no $ref)", "$ref" not in json.dumps(s))
check("requires epr-meta-version", "epr-meta-version" in s["required"])
check("rule class enum is the canonical 5",
      s["properties"]["rules"]["items"]["properties"]["class"]["enum"]
      == ["deny", "ask", "inject", "measure", "dispatch"])
rule_schema = s["properties"]["rules"]["items"]
check("rule schema admits package-style policy bindings", "policy" in rule_schema["properties"])
check("rule schema admits tailored dispatch parameters", "parameters" in rule_schema["properties"])

print(f"\n  {_passed} assertions passed ✅")
