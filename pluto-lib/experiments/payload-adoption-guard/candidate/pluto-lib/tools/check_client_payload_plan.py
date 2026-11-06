#!/usr/bin/env python3
"""Fail closed on source/order drift for the read-only client payload observer.

Requires the complete actual package plan and pristine official workspace.
No transform, source copy or native packet handling is performed here.
"""
import hashlib
import json
import re
from pathlib import Path
import sys
ROOT = Path(__file__).resolve().parents[1]


def metadata():
    return json.loads((ROOT / "tools/client-payload-source.json").read_text())


def check(plan, meta):
    observation = meta.get("candidate_observation")
    if (not isinstance(observation, dict)
        or observation.get("horizon_revision") != meta.get("horizon_revision")
        or any(not isinstance(observation.get(key), str) or re.fullmatch("[0-9a-f]{64}", observation[key]) is None
               for key in ("plan_sha256", "index_result_sha256"))
        or not isinstance(observation.get("evidence_commit"), str)
        or re.fullmatch("[0-9a-f]{40}", observation["evidence_commit"]) is None):
        raise ValueError("payload candidate has no reviewed actual plan/Index observation")
    if (set(meta["caller_fingerprints"]) != set(meta["callers"])
        or any(not isinstance(fp, str) or re.fullmatch("[0-9a-f]{64}", fp) is None
               for fp in meta["caller_fingerprints"].values())):
        raise ValueError("payload caller fingerprint is unobserved")
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
    # The private updater is part of this reviewed RETURN, never a second
    # packet hook. Match the exact module ID or placement so aliases and a
    # foreign collision cannot hide another implementation at this path.
    update = meta["internal_updates"]
    canonical_ids = {update["module_id"], "pluto_native_creative-" + update["module_id"]}
    modules = [m for m in plan.get("modules", [])
        if m.get("module_id") in canonical_ids
        or (m.get("file") == update["file"] and m.get("name") == update["name"])]
    if len(modules) != 1:
        raise ValueError("payload update requires exactly its reviewed private module")
    module = modules[0]
    if (module.get("mod_id") != "pluto_native_creative"
        or module.get("module_id") not in canonical_ids
        or module.get("file") != update["file"] or module.get("name") != update["name"]
        or module.get("visibility", "pub(crate)") != update["visibility"]):
        raise ValueError("payload update module owner or placement changed")
    update_raw = (ROOT / "mixins/pluto_native_creative" / update["source"]).read_bytes()
    if (hashlib.sha256(update_raw).hexdigest() != update["sha256"]
        or module.get("source") != update_raw.decode()):
        raise ValueError("payload update module differs from reviewed source")
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
