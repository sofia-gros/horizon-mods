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


# Pinned 441f49d5 Git blobs, including the extracted polling loop. There is
# no PlutoLib payload observer in the trait bundle. This contract preserves
# native dispatch only; it does not supply a public payload transport API.
NATIVE_SOURCES = {
    'crates/horizon-client/src/game/native_hooks.rs': '1edb8ccc33f7b046428176755c3ffa59ad6184a1d044b45dabf9d865e5fb7012',
    'crates/horizon-client/src/game/remote.rs': '19119b8e8a930e35d8aa3b2ac3ce9f2de4b58297ecf68f81fba19cf9f21b1eab',
    'crates/horizon-client/src/game/remote_poll.rs': 'fa5c8b5d821b140f96f5943ce60bed358daa0013289f289f1ac95b1ace6cd03b',
    'crates/horizon-client/src/game/execution_role.rs': '81457295dc2a098d24047cab5d1b7b55230539420a0def68ef587db4827bc4bd',
}
NATIVE_METHODS = {'native_client_packet', 'handle_packet', 'poll_remote', 'poll_remote_events'}


def native_bundle(plan):
    return any(entry.get('mod_id') == 'pluto_lib'
               for section in ('injections', 'modules', 'states')
               for entry in plan.get(section, []))


def metadata(plan=None):
    if plan is not None and native_bundle(plan):
        return {'native_preservation': True, 'source_sha256': dict(NATIVE_SOURCES)}
    return json.loads((ROOT / "tools/client-payload-source.json").read_text())


def check(plan, meta):
    if native_bundle(plan):
        if plan.get('schema') != 6 or meta != metadata(plan):
            raise ValueError('unreviewed schema or metadata for native payload preservation')
        for section in ('injections', 'modules', 'states'):
            for entry in plan.get(section, []):
                if any(c.get('file') in NATIVE_SOURCES for c in entry.get('constructors', [])):
                    raise ValueError('state constructor changes a preserved native packet source')
                if entry.get('mod_id') == 'pluto_native_creative':
                    raise ValueError('native payload cannot compose the retired marker bundle')
                if (entry.get('target', '').rsplit('::', 1)[-1] in NATIVE_METHODS
                        or entry.get('file') in {
                            'crates/horizon-client/src/game/remote_poll.rs',
                            'crates/horizon-client/src/game/execution_role.rs'}):
                    raise ValueError('native packet boundary must remain unattached')
        for path, expected in NATIVE_SOURCES.items():
            if plan.get('inputs', {}).get(path) != expected:
                raise ValueError(f'native packet plan source differs: {path}')
        return
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
    workspace = Path(workspace)
    if meta.get('native_preservation') and meta['source_sha256'] != NATIVE_SOURCES:
        raise ValueError('unreviewed native packet source metadata')
    for path, digest in meta["source_sha256"].items():
        source = workspace / path
        if source.is_symlink():
            raise ValueError(f'packet source must not be a symlink: {path}')
        source.resolve().relative_to(workspace.resolve())
        if hashlib.sha256(source.read_bytes()).hexdigest() != digest:
            raise ValueError(f"official packet source drifted: {path}")


if __name__ == "__main__":
    try:
        if len(sys.argv) != 3:
            raise ValueError("usage: check_client_payload_plan.py COMPLETE_PLAN.json PRISTINE_OFFICIAL_WORKSPACE")
        plan = json.loads(Path(sys.argv[1]).read_text())
        meta = metadata(plan)
        check(plan, meta)
        check_source(Path(sys.argv[2]), meta)
    except (ValueError, OSError, KeyError) as error:
        print(f"client payload observer rejected: {error}", file=sys.stderr)
        raise SystemExit(1)
    print("Client payload source and native observer ordering verified.")
