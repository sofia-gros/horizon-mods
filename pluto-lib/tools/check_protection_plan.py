#!/usr/bin/env python3
"""Validate the protection wrapper's ordering in a complete official Mixins plan."""
import json
from pathlib import Path
import sys

HIT = "horizon_client_lib::health::PlayerMotion::hurt_caused_by"
STATUS = "horizon_client_render::effects::StatusFlags::protection"
KERNEL = "horizon_world::combat_rules::after_magic"


def check(plan):
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
        if len(sys.argv) != 2:
            raise ValueError("usage: check_protection_plan.py COMPLETE_PLAN.json")
        check(json.loads(Path(sys.argv[1]).read_text()))
    except (ValueError, OSError, KeyError) as error:
        print(f"protection ordering rejected: {error}", file=sys.stderr)
        raise SystemExit(1)
    print("Protection ordering verified for the supplied complete plan.")
