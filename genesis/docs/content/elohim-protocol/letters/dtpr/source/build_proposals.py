"""Add Elohim's proposed elements to a resolved DTPR datachain.

Usage: python3 -I build_proposals.py <resolved.json> <full-snapshot.json> <out.json>

<full-snapshot.json> is any resolved chain whose snapshot holds the sibling
elements and categories; each proposal borrows its sibling's variable
declarations and, where the category has one, the category's element_context.

The resolved chain comes from DTPR's /resolve endpoint. Each proposal becomes a
`suggested_elements` entry, a placement in `elements`, and a provenance entry,
which is the shape DTPR's renderer marks as "proposed".
"""

import json
import sys


def loc(text):
    return [{"locale": "en", "value": text}]


PROPOSALS = [
    {
        "element": {
            "id": "checks",
            "category_id": "action",
            "symbol_id": "dm_accept-or-deny",
            "title": loc("Checks"),
            "description": loc(
                "Checks — many independent computers each test a record against shared, "
                "published rules before accepting it. No single one decides, and no one can "
                "change the rules alone. Distinct from Decides, where one system's outcome takes effect."
            ),
        },
        "placement": {"context_type_id": "autonomous", "variables": [
            {"id": "additional_description", "value": loc(
                "Every peer that stores a record checks it against the network's rules; a record that breaks them is refused everywhere.")}]},
        "why": "The protocol's trust does not rest on one system deciding well but on many independent peers checking the same rules. Decides names one system's outcome taking effect; nothing in the vocabulary names plural validation.",
    },
    {
        "element": {
            "id": "refers",
            "category_id": "action",
            "symbol_id": "dm_priority-ranking",
            "title": loc("Asks a person"),
            "description": loc(
                "Asks a person — passes a question it cannot settle to a named person and holds it "
                "open until that person answers. An unanswered question is never treated as a no. "
                "Distinct from Recommends, where a person picks from options the system offers."
            ),
        },
        "placement": {"context_type_id": "human_decides", "variables": [
            {"id": "additional_description", "value": loc(
                "Every decision has three outcomes: yes, no, or ask a person. The rules forbid turning \"ask a person\" into \"no\".")}]},
        "why": "Autonomy levels say who completes the loop, but not what happens when a system reaches the edge of what it may decide. A first-class referral that cannot decay into refusal is how a human floor is kept in practice.",
    },
    {
        "element": {
            "id": "available_to_those_i_share_with",
            "category_id": "access",
            "symbol_id": "available",
            "title": loc("Available to those I share with"),
            "description": loc(
                "Shared only with the people and groups I choose, as far as I choose, and I can take "
                "it back. The person decides who sees it, not the organization running the system."
            ),
        },
        "placement": {"variables": [
            {"id": "additional_description", "value": loc(
                "Each thing you publish carries a reach you chose: just you, your household, a group, or everyone.")}]},
        "why": "Every access element is written from the organization's side: available to me means an operator lets me see my own records. When the person holds the data, access is something they grant, and the vocabulary needs that direction.",
    },
    {
        "element": {
            "id": "given_to_the_commons",
            "category_id": "retention",
            "symbol_id": "is_retained",
            "title": loc("Given to the commons"),
            "description": loc(
                "Published as a gift to a shared record that others may come to rely on. You can ask "
                "to withdraw it, and the request is honoured as far as it can be: it may drop to a "
                "narrower reach or stay where others depend on it, and copies already made cannot be "
                "guaranteed to return. Distinct from Retained, where an operator deletes on a schedule."
            ),
        },
        "placement": {"variables": [
            {"id": "duration", "value": loc("for as long as others rely on it")},
            {"id": "additional_description", "value": loc(
                "The reach gate asks before you publish, so you know what you are giving and what it may mean to take it back.")}]},
        "why": "Retained for a duration, or not retained, both assume one party can delete on its own. A gift to a shared record is a third kind: once others rely on it, withdrawing it becomes a request the network honours as far as it can, not a guarantee. People deserve to know that before they give, which is why the gate comes first.",
    },
    {
        "element": {
            "id": "right_to_leave",
            "category_id": "rights",
            "symbol_id": "hand_raised",
            "title": loc("Right to leave and take it with you"),
            "description": loc(
                "The right to move your identity and everything you signed to another provider or "
                "your own device, and for a community to copy the rules and run its own network. "
                "Separate from the right to access, which lets you read your data, and from the right "
                "to be forgotten, which deletes it."
            ),
        },
        "placement": {"variables": [
            {"id": "rights", "value": loc(
                "Take your key and your signed records to your own device or another doorway at any time; a community can copy the rules and continue on its own.")}]},
        "why": "Portability and exit are missing from the rights list, and they are the right that disciplines every operator: a host you can leave with everything is a host that must keep earning your trust.",
    },
]


SIBLING = {"action": "acts", "access": "available_to_me", "retention": "data_retained", "rights": "right_object"}


def main(resolved_path, full_path, out_path):
    chain = json.load(open(resolved_path))
    snap = json.load(open(full_path))["schema_snapshot"]
    els = {e["id"]: e for e in snap["elements"]}
    cats = {c["id"]: c for c in snap["categories"]}
    chain.setdefault("suggested_elements", [])
    prov = chain["authoring_provenance"]["element_provenance"]
    for i, p in enumerate(PROPOSALS):
        el = dict(p["element"])
        el["variables"] = els[SIBLING[el["category_id"]]].get("variables", [])
        if cats[el["category_id"]].get("element_context"):
            el["context"] = cats[el["category_id"]]["element_context"]
        chain["suggested_elements"].append(el)
        placement = {"element_id": el["id"], "priority": 10 + i, **p["placement"]}
        chain["elements"].append(placement)
        prov[el["id"]] = {
            "rationale": "Proposed by Elohim Protocol for DTPR's consideration. " + p["why"],
            "confidence": "medium",
        }
    json.dump(chain, open(out_path, "w"), indent=2, ensure_ascii=False)
    print(f"wrote {out_path} with {len(PROPOSALS)} proposals")


if __name__ == "__main__":
    main(*sys.argv[1:4])
