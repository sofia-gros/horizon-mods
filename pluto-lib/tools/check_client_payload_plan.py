#!/usr/bin/env python3
"""Fail closed on source/order drift for the read-only client payload observer.

Requires the complete actual package plan and pristine official workspace.
No transform, source copy or native packet handling is performed here.
"""
import hashlib
import json
from pathlib import Path
import sys
ROOT = Path(__file__).resolve().parents[1]


def metadata():
    return json.loads((ROOT / "tools/client-payload-source.json").read_text())


def check(plan, meta):
    hooks = plan.get("injections", [])
    # Compare resolved node/fingerprint too: alternate textual target spellings
    # must not evade composition checks for the same official function.
    selected = [p for p in hooks if p["target"] == meta["target"]
        or (p.get("file") == meta["location"]["file"] and p.get("node") == meta["location"]["node"])
        or p.get("function_fingerprint") == meta["function_fingerprint"]]
    own = [p for p in selected if p.get("mod_id") == "pluto_native_creative"
           and p.get("injection_id", "").endswith(meta["injection_id"])]
    if len(own) != 1 or len(selected) != 1:
        raise ValueError("payload observer requires exactly its reviewed RETURN; other target hooks need separate composition review")
    hook = own[0]
    if hook["kind"] != "RETURN" or hook.get("cancellable"):
        raise ValueError("payload notification must not cancel the native observer")
    if hook["function_fingerprint"] != meta["function_fingerprint"]:
        raise ValueError("native packet observer fingerprint drifted")
    if (hook.get("file") != meta["location"]["file"] or hook.get("node") != meta["location"]["node"]):
        raise ValueError("native observer resolved location changed")
    if hook["bindings"] != meta["bindings"]:
        raise ValueError("payload notification bindings changed")
    helper = ROOT / "mixins/pluto_native_creative" / meta["helper"]
    raw = helper.read_bytes()
    if hashlib.sha256(raw).hexdigest() != meta["helper_sha256"] or hook["helper_source"] != raw.decode():
        raise ValueError("plan/helper differs from reviewed observer")
    for hook in hooks:
        if (hook["target"] in meta["callers"] or
            hook.get("function_fingerprint") in meta["caller_fingerprints"].values()) and (
            hook["kind"] not in {"HEAD", "RETURN"} or
            hook.get("cancellable")
        ):
            raise ValueError("a caller hook can bypass or reorder the native packet observer")
    for path, digest in meta["source_sha256"].items():
        if plan.get("inputs", {}).get(path) != digest:
            raise ValueError(f"plan does not use reviewed official packet source: {path}")


def check_source(workspace, meta):
    for path, digest in meta["source_sha256"].items():
        if hashlib.sha256((workspace/path).read_bytes()).hexdigest() != digest:
            raise ValueError(f"official packet source drifted: {path}")


if __name__ == "__main__":
    try:
        if len(sys.argv) != 3:
            raise ValueError("usage: check_client_payload_plan.py COMPLETE_PLAN.json PRISTINE_OFFICIAL_WORKSPACE")
        meta = metadata()
        check(json.loads(Path(sys.argv[1]).read_text()), meta)
        check_source(Path(sys.argv[2]), meta)
    except (ValueError, OSError, KeyError) as error:
        print(f"client payload observer rejected: {error}", file=sys.stderr)
        raise SystemExit(1)
    print("Client payload source and native observer ordering verified.")
