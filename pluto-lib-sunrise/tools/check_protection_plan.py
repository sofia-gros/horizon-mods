#!/usr/bin/env python3
"""Verify native protection preservation or the retained marker wrapper's ordering."""
import hashlib
import json
from pathlib import Path
import sys

HIT = "sunrise_client_lib::health::PlayerMotion::hurt_caused_by"
STATUS = "sunrise_client_render::effects::StatusFlags::protection"
KERNEL = "sunrise_world::combat_rules::after_magic"


# Pinned 441f49d5 Git blobs. The trait bundle has no marker/protection API:
# preserve the native hit, status lookup and scalar calculation instead of
# requiring a retired wrapper to be installed. This certifies no custom EPF.
NATIVE_SOURCES = {
    'crates/horizon-client/src/health.rs': '493e3f31f821779594cd9ddb00a35e0029f129d8e48d6e446b6282439fd2e3bd',
    'crates/horizon-client-render/src/effects.rs': '7f0e546d908c362d7dda4bc2ae09a7b27f937c7151e33a6e8799d041985470ed',
    'crates/horizon-world/src/combat_rules.rs': 'a2b3ab54355ab6a074c419b1a533452cdb394f6c92ae86af05f44c4883f3f22c',
}


def check_native(plan, workspace):
    if plan.get('schema') != 6:
        raise ValueError('unreviewed official plan schema for native protection')
    if workspace is None:
        raise ValueError('native protection preservation requires the input workspace')
    for section in ('injections', 'modules', 'states'):
        for entry in plan.get(section, []):
            if any(c.get('file') in NATIVE_SOURCES for c in entry.get('constructors', [])):
                raise ValueError('state constructor changes a preserved native protection source')
            if entry.get('file') in NATIVE_SOURCES or entry.get('target') in (HIT, STATUS, KERNEL):
                raise ValueError('native protection files must remain unattached; mixed marker composition is forbidden')
            if entry.get('mod_id') == 'pluto_native_creative':
                raise ValueError('native protection preservation cannot compose the retired marker bundle')
    root = Path(workspace)
    for name, expected in NATIVE_SOURCES.items():
        source = root / name
        if source.is_symlink() or not source.is_file():
            raise ValueError(f'native protection source is not a regular file: {name}')
        source.resolve().relative_to(root.resolve())
        if plan.get('inputs', {}).get(name) != expected:
            raise ValueError(f'native protection plan source differs: {name}')
        if hashlib.sha256(source.read_bytes()).hexdigest() != expected:
            raise ValueError(f'native protection workspace source differs: {name}')


def check(plan, workspace=None):
    if any(entry.get('mod_id') == 'pluto_lib'
           for section in ('injections', 'modules', 'states') for entry in plan.get(section, [])):
        check_native(plan, workspace)
        return
    injections = plan.get("injections", [])
    def own(entry, suffix):
        return entry.get("mod_id") == "pluto_native_creative" and entry.get("injection_id", "").endswith(suffix)
    heads = [p for p in injections if p["target"] == HIT and p["kind"] == "HEAD"]
    wrappers = [p for p in heads if own(p, "protection-player-scope")]
    if len(wrappers) != 1:
        raise ValueError("expected exactly one Pluto player protection wrapper")
    if wrappers[0]["priority"] != 2147483647:
        raise ValueError("protection wrapper must have the reserved maximum HEAD priority")
    if any(p is not wrappers[0] and p["priority"] >= wrappers[0]["priority"] for p in heads):
        raise ValueError("another maximum-priority hit HEAD can execute twice on reentry")
    if any(p["target"] == HIT and p["kind"] == "RETURN" for p in injections):
        raise ValueError("hit RETURN hooks require a different wrapper design")
    returns = [p for p in injections if p["target"] == STATUS and p["kind"] == "RETURN"]
    projections = [p for p in returns if own(p, "protection-project")]
    if len(projections) != 1:
        raise ValueError("expected exactly one protection projection")
    if any(p is not projections[0] and p["priority"] <= projections[0]["priority"] for p in returns):
        raise ValueError("numeric status RETURN hooks must precede the protection marker projection")
    if any(p["target"] == KERNEL and not own(p, "protection-kernel") for p in injections):
        raise ValueError("the marker-consuming scalar boundary must not have another hook that could observe the private marker")
    if not any(p["target"] == KERNEL and own(p, "protection-kernel") for p in injections):
        raise ValueError("protection scalar consumer is missing")


if __name__ == "__main__":
    try:
        if len(sys.argv) not in (2, 3):
            raise ValueError("usage: check_protection_plan.py COMPLETE_PLAN.json [INPUT_WORKSPACE]")
        check(json.loads(Path(sys.argv[1]).read_text()), Path(sys.argv[2]) if len(sys.argv) == 3 else None)
    except (ValueError, OSError, KeyError) as error:
        print(f"protection ordering rejected: {error}", file=sys.stderr)
        raise SystemExit(1)
    print("Protection boundary verified for the supplied plan; no custom-protection capability claim.")
