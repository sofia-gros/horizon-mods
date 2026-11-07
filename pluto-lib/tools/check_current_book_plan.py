#!/usr/bin/env python3
"""Guard synchronous current-book operations against native/composition drift."""
import hashlib
import json
from pathlib import Path
import sys
ROOT=Path(__file__).resolve().parents[1]

def metadata():
    return json.loads((ROOT/"tools/current-book-source.json").read_text())

def check(plan, meta):
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
    for file,digest in meta["official_sources"].items():
        if hashlib.sha256((workspace/file).read_bytes()).hexdigest()!=digest:
            raise ValueError(f"official current-book source drifted: {file}")

if __name__=="__main__":
    try:
        if len(sys.argv)!=3:raise ValueError("usage: check_current_book_plan.py COMPLETE_PLAN.json PRISTINE_OFFICIAL_WORKSPACE")
        meta=metadata();check(json.loads(Path(sys.argv[1]).read_text()),meta);check_source(Path(sys.argv[2]),meta)
    except (ValueError,OSError,KeyError) as error:
        print(f"current-book adapter rejected: {error}",file=sys.stderr);raise SystemExit(1)
    print("Current-book source and native turn composition verified.")
