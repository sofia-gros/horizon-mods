#!/usr/bin/env python3
"""Fail closed on composition or source drift at copied advancement boundaries.

Use the complete installed official plan and pristine selected host workspace.
Predicate callbacks with Unsupported/native fallback are a separate API contract;
this guard applies only to the two conditional full-body replacements.
"""
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
MOD_ID = "pluto_native_creative"


def digest(value):
    return hashlib.sha256(value).hexdigest()


def entries():
    return json.loads((ROOT / "tools/advancement-replacements.json").read_text())["boundaries"]


def check_order(plan, metadata):
    if plan.get("schema") != 6:
        raise ValueError("unreviewed official plan schema for advancement replacements")
    injections = plan.get("injections", [])
    for meta in metadata:
        selected = [hook for hook in injections if hook["target"] == meta["target"]]
        ids = {meta["injection_id"], MOD_ID + "-" + meta["injection_id"]}
        own = [hook for hook in selected if hook.get("mod_id") == MOD_ID
               and hook.get("injection_id") in ids]
        if len(own) != 1:
            raise ValueError(f"expected exactly one advancement replacement: {meta['target']}")
        # Current adapters have ordinary HEAD priority. A later HEAD, an
        # interior operation or a RETURN modifier may observe different paths
        # depending on runtime opt-in. Require explicit redesign, not a guessed
        # safe ordering. This also rejects duplicate hooks from our own bundle.
        if len(selected) != 1:
            raise ValueError(f"competing hook could be bypassed by advancement replacement: {meta['target']}")
        hook = own[0]
        if (hook.get("kind") != "HEAD" or hook.get("cancellable") is not True
                or hook.get("priority") != meta["priority"]):
            raise ValueError("advancement replacement phase/priority/cancellation changed")
        if hook.get("function_fingerprint") != meta["function_fingerprint"]:
            raise ValueError("selected native advancement function fingerprint drifted")
        if hook.get("file") != meta["file"] or hook.get("bindings") != meta["bindings"]:
            raise ValueError("advancement target file or actor binding drifted")


def verify_source(plan, workspace, metadata, helper_root=None):
    workspace = Path(workspace)
    helper_root = ROOT / "mixins/pluto_native_creative" if helper_root is None else Path(helper_root)
    for meta in metadata:
        original = (workspace / meta["file"]).read_bytes()
        if digest(original) != meta["file_sha256"]:
            raise ValueError(f"pristine official advancement source drifted: {meta['file']}")
        if plan.get("inputs", {}).get(meta["file"]) != meta["file_sha256"]:
            raise ValueError("plan does not bind the reviewed pristine advancement source")
        helper = (helper_root / meta["helper"]).read_bytes()
        if digest(helper) != meta["helper_sha256"]:
            raise ValueError(f"reviewed advancement adapter changed: {meta['helper']}")
        ids = {meta["injection_id"], MOD_ID + "-" + meta["injection_id"]}
        selected = next(hook for hook in plan["injections"]
                        if hook["target"] == meta["target"] and hook.get("mod_id") == MOD_ID
                        and hook.get("injection_id") in ids)
        if selected.get("helper_source", "").encode() != helper:
            raise ValueError("plan does not contain the exact reviewed advancement adapter")


def check(plan, workspace):
    metadata = entries()
    check_order(plan, metadata)
    verify_source(plan, workspace, metadata)


def main(argv=None):
    argv = sys.argv[1:] if argv is None else argv
    try:
        if len(argv) != 2:
            raise ValueError("usage: check_advancement_plan.py COMPLETE_PLAN.json PRISTINE_OFFICIAL_WORKSPACE")
        check(json.loads(Path(argv[0]).read_text()), Path(argv[1]))
    except (ValueError, OSError, KeyError, StopIteration, TypeError) as error:
        print(f"advancement composition rejected: {error}", file=sys.stderr)
        return 1
    print("Copied advancement boundaries, exact source and complete-plan exclusivity verified.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
