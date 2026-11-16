#!/usr/bin/env python3
"""Guard synchronous current-book operations against native/composition drift."""
import hashlib
import json
from pathlib import Path
import sys
ROOT=Path(__file__).resolve().parents[1]

# Exact pinned 441f49d5 book implementation. The active trait bundle does not
# expose the retired current-book compare-and-apply capability. Native page
# bounds, edit-page creation, lectern button dispatch and sounds stay native.
NATIVE_FILE = 'crates/horizon-client/src/game/books.rs'
NATIVE_SHA256 = '2c18f1db9ce129c7977a1bbf2b190ded816360956354eb6d08c5af5a9a8592d8'


def native_bundle(plan):
    return any(entry.get('mod_id') == 'pluto_lib'
               for section in ('injections', 'modules', 'states')
               for entry in plan.get(section, []))


def metadata(plan=None):
    if plan is not None and native_bundle(plan):
        return {'native_preservation': True, 'official_sources': {NATIVE_FILE: NATIVE_SHA256}}
    return json.loads((ROOT/"tools/current-book-source.json").read_text())

def check(plan, meta):
    if native_bundle(plan):
        if plan.get('schema') != 6 or meta != metadata(plan):
            raise ValueError('unreviewed native current-book schema or metadata')
        for section in ('injections', 'modules', 'states'):
            for entry in plan.get(section, []):
                if any(c.get('file') == NATIVE_FILE for c in entry.get('constructors', [])):
                    raise ValueError('state constructor changes a preserved native book source')
                if (entry.get('mod_id') == 'pluto_native_creative'
                        or entry.get('file') == NATIVE_FILE
                        or entry.get('target', '').rsplit('::', 1)[-1] == 'turn_page'):
                    raise ValueError('native current-book must remain unattached; no mixed legacy adapter')
        if plan.get('inputs', {}).get(NATIVE_FILE) != NATIVE_SHA256:
            raise ValueError('native current-book plan source differs')
        return
    modules=[m for m in plan.get("modules",[]) if m.get("mod_id")=="pluto_native_creative" and m.get("module_id","").endswith(meta["module_id"])]
    if len(modules)!=1:raise ValueError("expected exactly one current-book adapter")
    module=modules[0]
    if module["file"]!=meta["module_file"] or module["name"]!=meta["module_name"]:
        raise ValueError("current-book native placement changed")
    source=(ROOT/"mixins/pluto_native_creative"/meta["source"]).read_bytes()
    if hashlib.sha256(source).hexdigest()!=meta["source_sha256"] or module["source"]!=source.decode():
        raise ValueError("plan differs from the reviewed current-book adapter")
    for hook in plan.get("injections",[]):
        if hook["target"]==meta["turn_target"] or hook.get("function_fingerprint")==meta["turn_fingerprint"]:
            raise ValueError("native page-turn hooks require separate compare-and-apply composition review")
    for file,digest in meta["official_sources"].items():
        if plan.get("inputs",{}).get(file)!=digest:
            raise ValueError(f"plan does not use reviewed current-book source: {file}")

def check_source(workspace,meta):
    workspace = Path(workspace)
    if meta.get('native_preservation') and meta['official_sources'] != {NATIVE_FILE: NATIVE_SHA256}:
        raise ValueError('unreviewed native current-book source metadata')
    for file,digest in meta["official_sources"].items():
        source = workspace / file
        if source.is_symlink():
            raise ValueError('current-book source must not be a symlink')
        source.resolve().relative_to(workspace.resolve())
        if hashlib.sha256(source.read_bytes()).hexdigest()!=digest:
            raise ValueError(f"official current-book source drifted: {file}")

if __name__=="__main__":
    try:
        if len(sys.argv)!=3:raise ValueError("usage: check_current_book_plan.py COMPLETE_PLAN.json PRISTINE_OFFICIAL_WORKSPACE")
        plan=json.loads(Path(sys.argv[1]).read_text())
        meta=metadata(plan);check(plan,meta);check_source(Path(sys.argv[2]),meta)
    except (ValueError,OSError,KeyError) as error:
        print(f"current-book adapter rejected: {error}",file=sys.stderr);raise SystemExit(1)
    print("Current-book source and native turn composition verified.")
