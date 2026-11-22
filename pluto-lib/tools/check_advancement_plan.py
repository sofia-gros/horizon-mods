#!/usr/bin/env python3
"""Fail closed on composition or source drift at copied advancement boundaries.

Use the complete installed official plan and pristine selected host workspace.
Predicate callbacks with Unsupported/native fallback are a separate API contract;
this guard applies only to the two conditional full-body replacements.

Owner decision, 2026-11-22: the pluto_lib bundle is now the composition, so
the preservation branch below also admits the advancement event drain and
the two replacements when they match the reviewed bundle_attachments in
advancement-replacements.json exactly (pinned identity, phase, priority,
bindings, function fingerprint, packaged helper bytes and pristine file
digest). The bundle's forwarders already install into these hooks; without
them queued events and component-aware inventory triggers have nothing to
feed. Everything else at this file and these targets remains unattached.
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


BUNDLE_MOD_ID = "pluto_lib"


def bundle_entries():
    data = json.loads((ROOT / "tools/advancement-replacements.json").read_text())
    return data.get("bundle_attachments", [])


def _bundle_match(entry, meta, helpers):
    """Exact reviewed bundle attachment: pinned identity and phase fields,
    and the plan-carried helper bytes bound to the packaged bundle file."""
    if entry.get("mod_id") != BUNDLE_MOD_ID:
        return False
    for key in ("injection_id", "target", "file", "kind", "cancellable",
                "priority", "bindings", "function_fingerprint"):
        if entry.get(key) != meta[key]:
            return False
    packaged = (Path(helpers) / meta["helper"]).read_bytes()
    return (digest(packaged) == meta["helper_sha256"]
            and entry.get("helper_source", "").encode() == packaged)


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


NATIVE_FILE = 'crates/horizon-client/src/game/advancements.rs'
NATIVE_SHA256 = 'a15053aee938826d4181c32fb2a6882ef7b524dffaac5c9e951a8fb95f42f3c9'


def check(plan, workspace):
    metadata = entries()
    if any(entry.get('mod_id') == 'pluto_lib'
           for section in ('injections', 'modules', 'states') for entry in plan.get(section, [])):
        # The active trait bundle composes exactly the reviewed advancement
        # attachments pinned in bundle_attachments (owner decision,
        # 2026-11-22): the event drain, the two conditional replacements
        # and the tag predicate of the tag-index overlay,
        # admitted by exact identity below. The pinned native index/trigger
        # sources and native event order are otherwise preserved, and any
        # other attachment at this file or these targets still fails closed.
        if plan.get('schema') != 6:
            raise ValueError('unreviewed official plan schema for native advancements')
        reviewed = bundle_entries()
        targets = {entry['target'] for entry in metadata} | {entry['target'] for entry in reviewed}
        matched = {}
        for section in ('injections', 'modules', 'states'):
            for entry in plan.get(section, []):
                if any(c.get('file') == NATIVE_FILE for c in entry.get('constructors', [])):
                    raise ValueError('state constructor changes a preserved native advancement source')
                if (entry.get('file') == NATIVE_FILE or entry.get('target') in targets
                        or entry.get('mod_id') == MOD_ID):
                    hit = None
                    if section == 'injections':
                        hit = next((meta for meta in reviewed
                                    if _bundle_match(entry, meta, ROOT / 'mod')), None)
                    if hit is None:
                        raise ValueError('native advancement caller must remain unattached; no mixed legacy replacement')
                    matched[hit['injection_id']] = matched.get(hit['injection_id'], 0) + 1
        source = Path(workspace) / NATIVE_FILE
        if source.is_symlink() or not source.is_file():
            raise ValueError('native advancement source must be a regular file')
        source.resolve().relative_to(Path(workspace).resolve())
        if plan.get('inputs', {}).get(NATIVE_FILE) != NATIVE_SHA256:
            raise ValueError('native advancement plan source differs')
        if digest(source.read_bytes()) != NATIVE_SHA256:
            raise ValueError('native advancement workspace source differs')
        # Once the native inventory gates are composed (pluto-inventory
        # module present), every reviewed attachment is required exactly
        # once, so the grant paths cannot silently lose their hooks.
        if any(entry.get('module_id') == 'pluto-inventory' for entry in plan.get('modules', [])):
            for meta in reviewed:
                if matched.get(meta['injection_id'], 0) != 1:
                    raise ValueError(f"reviewed advancement attachment missing or duplicated: {meta['injection_id']}")
        return
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
    print("Advancement boundary verified; only the reviewed bundle attachments extend the native caller.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
