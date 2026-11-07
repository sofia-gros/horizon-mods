#!/usr/bin/env python3
"""Fail closed on drift or hooks bypassed by the pinned damage callers.

Run before application, with the complete combined official plan and pristine
source workspace. This guard is not a replacement for native Cargo checks.
"""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]


def digest(text):
    return hashlib.sha256(text.encode()).hexdigest()


def check_order(plan, entries):
    for meta in entries:
        selected = [p for p in plan.get("injections", []) if p["target"] == meta["target"]]
        own = [p for p in selected if p.get("mod_id") == "pluto_native_creative"
               and p.get("injection_id", "").endswith(meta["injection_id"])]
        if len(own) != 1:
            raise ValueError(f"expected one equipment replacement for {meta['target']}")
        replacement = own[0]
        if (replacement["kind"] != "HEAD" or replacement["priority"] != -2147483648
                or not replacement.get("cancellable")):
            raise ValueError("equipment replacement requires the reserved lowest cancellable HEAD")
        if replacement["function_fingerprint"] != meta["function_fingerprint"]:
            raise ValueError("native damage function fingerprint drifted")
        for hook in selected:
            if hook is replacement:
                continue
            if hook["kind"] not in {"HEAD", "RETURN"}:
                raise ValueError("an interior damage hook would be bypassed by the copied caller")
            if hook["kind"] == "HEAD" and hook["priority"] <= replacement["priority"]:
                raise ValueError("equipment replacement must follow every other HEAD")
        # HEAD Break invokes native RETURN modifiers once. The helper itself
        # does not recursively invoke its target, so they are not duplicated.


def verify_source(plan, workspace, entries):
    for meta in entries:
        text = (workspace / meta["file"]).read_text()
        if digest(text) != meta["file_sha256"]:
            raise ValueError(f"official source drifted: {meta['file']}")
        start = text.index(meta["start"])
        end = text.index(meta["end"], start)
        original = text[start:end].rstrip()
        if digest(original) != meta["function_sha256"]:
            raise ValueError("native function extraction differs from reviewed source")
        body = original.replace(meta["declaration"], "fn original(", 1)
        body = body.replace("&mut self,", "game: &mut crate::game::Game<'_>,", 1)
        body = re.sub(r"\bself\b", "game", body)
        for change in meta["changes"]:
            if body.count(change["old"]) != 1:
                raise ValueError("reviewed damage adapter hunk no longer matches exactly once")
            body = body.replace(change["old"], change["new"])
        expected = meta["header"] + body + meta["footer"]
        formatted = subprocess.run(
            ["rustfmt", "--edition", "2024", "--emit", "stdout"],
            input=expected, text=True, capture_output=True, check=True,
        ).stdout
        helper = ROOT / "mixins/pluto_native_creative" / meta["helper"]
        if helper.read_text() != formatted:
            raise ValueError(f"copied caller has changes outside reviewed hunks: {meta['helper']}")
        selected = next(p for p in plan["injections"]
                        if p.get("mod_id") == "pluto_native_creative"
                        and p.get("injection_id", "").endswith(meta["injection_id"]))
        if selected["helper_source"] != formatted:
            raise ValueError("plan does not contain the reviewed copied helper")


def entries():
    return [json.loads((ROOT / "tools" / name).read_text()) for name in
            ["ordinary-equipment-source.json", "explosion-equipment-source.json", "armor-wear-source.json"]]


if __name__ == "__main__":
    try:
        if len(sys.argv) != 3:
            raise ValueError("usage: check_damage_equipment_plan.py COMPLETE_PLAN.json PRISTINE_OFFICIAL_WORKSPACE")
        plan = json.loads(Path(sys.argv[1]).read_text())
        metadata = entries()
        check_order(plan, metadata)
        verify_source(plan, Path(sys.argv[2]), metadata)
    except (ValueError, OSError, KeyError, StopIteration, subprocess.CalledProcessError) as error:
        print(f"damage equipment adapter rejected: {error}", file=sys.stderr)
        raise SystemExit(1)
    print("Damage equipment source, adapter hunks and combined hook ordering verified.")
