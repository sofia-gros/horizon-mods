#!/usr/bin/env python3
"""Strict pre-apply guard for the isolated guide lifetime experiment only."""
from pathlib import Path
import hashlib
import json
import sys
ROOT = Path(__file__).resolve().parent
MOD = "pluto_guide_lifetime_experiment"

def sha(data):
    return hashlib.sha256(data).hexdigest()

def identity_matches(hook, expected):
    return (hook.get("target") == expected["target"]
            or hook.get("function_fingerprint") == expected["function_fingerprint"]
            or (hook.get("file") == expected["file"] and hook.get("node") == expected["node"]))

def check(plan, metadata):
    for file, digest in metadata["official_sources"].items():
        if plan.get("inputs", {}).get(file) != digest:
            raise ValueError("official source drift: " + file)
    for file, digest in metadata["experiment_sources"].items():
        if sha((ROOT/file).read_bytes()) != digest:
            raise ValueError("experiment source drift: " + file)
    owned = [h for h in plan.get("injections", []) if h.get("mod_id") == MOD]
    if len(owned) != len(metadata["hooks"]):
        raise ValueError("unexpected experiment hook count")
    for expected in metadata["hooks"]:
        selected = [h for h in plan.get("injections", []) if identity_matches(h, expected)]
        own = [h for h in selected if h.get("mod_id") == MOD]
        if len(own) != 1:
            raise ValueError("exactly one reviewed invalidation required")
        hook = own[0]
        for field in ["file", "node", "function_fingerprint", "kind", "cancellable", "priority", "bindings", "helper_source"]:
            if hook.get(field) != expected[field]:
                raise ValueError("invalidation contract drift: " + field)
        for foreign in [h for h in selected if h.get("mod_id") != MOD]:
            # Only these three byte-for-byte reviewed disjoint library clear
            # helpers are admitted. Other native entries are exclusive.
            if foreign not in metadata["allowed_disjoint_clear_hooks"]:
                raise ValueError("unreviewed hook can bypass/alter lifetime")
    for expected in metadata["allowed_disjoint_modules"]:
        found = [m for m in plan.get("modules", []) if m.get("mod_id") == expected["mod_id"] and m.get("name") == expected["name"]]
        if found != [expected]:
            raise ValueError("reviewed disjoint clear-state module changed")
    modules = [m for m in plan.get("modules", []) if m.get("mod_id") == MOD]
    if len(modules) != len(metadata["modules"]):
        raise ValueError("unexpected experiment module count")
    for expected in metadata["modules"]:
        selected = [m for m in modules if m.get("name") == expected["name"]]
        if len(selected) != 1:
            raise ValueError("missing experiment module")
        for field in ["file", "name", "visibility", "source"]:
            actual = selected[0].get(field, "pub(crate)" if field == "visibility" else None)
            if actual != expected[field]:
                raise ValueError("module placement/source drift: " + field)
    states = [s for s in plan.get("states", []) if s.get("mod_id") == MOD]
    if len(states) != 1:
        raise ValueError("one owned state required")
    state = states[0]
    for field, value in metadata["state"].items():
        if state.get(field) != value:
            raise ValueError("owned state/constructor drift: " + field)

if __name__ == "__main__":
    try:
        if len(sys.argv) != 3:
            raise ValueError("usage: check_plan.py COMPLETE_PLAN.json PRISTINE_OFFICIAL_WORKSPACE")
        metadata = json.loads((ROOT/"source.json").read_text())
        check(json.loads(Path(sys.argv[1]).read_text()), metadata)
        for file, digest in metadata["official_sources"].items():
            if sha((Path(sys.argv[2])/file).read_bytes()) != digest:
                raise ValueError("pristine source drift: " + file)
    except (OSError, KeyError, ValueError) as error:
        print("guide lifetime experiment rejected: " + str(error), file=sys.stderr)
        raise SystemExit(1)
    print("Experimental guide source, owned state and invalidation composition verified.")
