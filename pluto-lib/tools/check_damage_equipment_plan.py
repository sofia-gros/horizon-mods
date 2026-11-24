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


# Reviewed 441f49d5 inputs. creatures.rs is the exact direct-host output:
# its sole delta forwards MobEvents to the particle adapter; hurt_player_from
# is unchanged. The other two files are pristine Git blobs. Pinning the entire
# prepared file also rejects any future direct rewrite until it is reviewed.
NATIVE_SOURCES = {
    'crates/horizon-client/src/game/creatures.rs': 'e2a6a83ce2f210596cc76a9886965f89198a416ea1c3a4673a3bee42ecb4002c',
    'crates/horizon-client/src/game/explosion.rs': '002b5dc60b4064e8c1349c91823f669d1129b190f6fea8c9c3b8440b918e8674',
    'crates/horizon-client/src/game/item_durability.rs': '09057717874b509f168bd267cf67086cd8d5dec8ff3c41c68615b9f06cfdec61',
}
NATIVE_TARGETS = {
    "horizon_client::game::creatures::Game<'_>::hurt_player_from",
    "horizon_client::game::explosion::Game<'_>::explosion_hurt_player",
    *(f"horizon_client::game::item_durability::Game<'_>::{name}" for name in
      ('hurt_player_equipment', 'push_seen_with_equipment_hurt', 'wear_armor',
       'hurt_armor', 'hurt_equipment')),
}
UNATTACHED_FILES = {
    'crates/horizon-client/src/game/explosion.rs',
    'crates/horizon-client/src/game/item_durability.rs',
}
# Reviewed exceptions to the file-level ban in check_native_order: a HEAD hook
# anchored in a preserved file whose target is not an equipment function and
# which the pinned damage callers cannot bypass. Pinned by identity, target,
# function fingerprint and packaged helper bytes; any drift re-trips the guard.
PERMITTED_ATTACHMENTS = {
    'pluto-hanging-frame-explosion-return': {
        'target': "horizon_client::game::explosion::Game<'_>::explosion_breaks_decorations",
        'function_fingerprint': '46df438c823f03a83f64360159fb2415d27995fc1a80158b8c09d3122cde29f1',
        'helper': 'pluto-hanging-frame-explosion-return.rs',
        'helper_sha256': '4ddfc734dde11454e93679c551478c19d09759f75979e3c2aeabe37708b3805b',
    },
}


def permitted_attachment(entry):
    expected = PERMITTED_ATTACHMENTS.get(entry.get('injection_id'))
    if expected is None:
        return False
    if (entry.get('target') != expected['target']
            or entry.get('function_fingerprint') != expected['function_fingerprint']):
        return False
    raw = (ROOT / 'mod' / expected['helper']).read_bytes()
    return (hashlib.sha256(raw).hexdigest() == expected['helper_sha256']
            and entry.get('helper_source', '').encode() == raw)


def native_bundle(plan):
    return any(entry.get('mod_id') == 'pluto_lib'
               for section in ('injections', 'modules', 'states')
               for entry in plan.get(section, []))


def check_native_order(plan, entries):
    if plan.get('schema') != 6:
        raise ValueError('unreviewed official plan schema for native equipment')
    targets = NATIVE_TARGETS | {entry['target'] for entry in entries}
    methods = {target.rsplit('::', 1)[-1] for target in targets}
    for section in ('injections', 'modules', 'states'):
        for entry in plan.get(section, []):
            if any(c.get('file') in NATIVE_SOURCES for c in entry.get('constructors', [])):
                raise ValueError('state constructor changes a preserved native equipment source')
            if entry.get('mod_id') == 'pluto_native_creative':
                raise ValueError('native equipment cannot compose the retired marker bundle')
            if (entry.get('file') in UNATTACHED_FILES
                    or entry.get('target', '').rsplit('::', 1)[-1] in methods):
                if not permitted_attachment(entry):
                    raise ValueError('native equipment boundary must remain unattached')
    # Unrelated creature spawn/drop/particle attachments are checked by their
    # own mandatory guards, not certified as equipment behavior here.


def check_order(plan, entries):
    if native_bundle(plan):
        check_native_order(plan, entries)
        return
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
    workspace = Path(workspace)
    if native_bundle(plan):
        check_native_order(plan, entries)
        for name, expected in NATIVE_SOURCES.items():
            source = workspace / name
            if source.is_symlink() or not source.is_file():
                raise ValueError(f'native equipment source is not a regular file: {name}')
            source.resolve().relative_to(workspace.resolve())
            if plan.get('inputs', {}).get(name) != expected:
                raise ValueError(f'native equipment plan source differs: {name}')
            if hashlib.sha256(source.read_bytes()).hexdigest() != expected:
                raise ValueError(f'native equipment workspace source differs: {name}')
        return
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
    print("Equipment boundary verified; native preservation supplies no custom protection or wear-observer API.")
