#!/usr/bin/env python3
"""Run only the named PlutoLib guards against a complete plan.

For a reduced PlutoLib bundle (see tools/slice_manifest.py) the guards of
features that were removed must not run, but every guard of a kept feature
must still pass. `check_native_plan.py` remains the check for the full bundle.
Usage: check_sliced_plan.py PLAN.json PRISTINE_WORKSPACE guard[,guard...]
"""
import importlib.util
import json
from pathlib import Path
import sys

here = Path(__file__).with_name("check_native_plan.py")
spec = importlib.util.spec_from_file_location("check_native_plan", here)
native = importlib.util.module_from_spec(spec)
spec.loader.exec_module(native)

GUARDS = {
    "protection": lambda p, w: native.protection.check(p, w),
    "advancement": lambda p, w: native.advancement.check(p, w),
    "equipment": lambda p, w: (native.equipment.check_order(p, native.equipment.entries()),
                               native.equipment.verify_source(p, w, native.equipment.entries())),
    "payload": lambda p, w: (native.payload.check(p, native.payload.metadata(p)),
                             native.payload.check_source(w, native.payload.metadata(p))),
    "placement": lambda p, w: native.placement.check(p, w),
    "current_book": lambda p, w: (native.current_book.check(p, native.current_book.metadata(p)),
                                  native.current_book.check_source(w, native.current_book.metadata(p))),
    "wire_snapshot": lambda p, w: native.wire_snapshot.check(p, w),
    "matching": lambda p, w: native.matching.check(p, w),
    "terrain": lambda p, w: (native.terrain.check(p, native.terrain.metadata(p)),
                             native.terrain.check_source(w, native.terrain.metadata(p))),
    "cloud": lambda p, w: native.cloud.check(p, w),
    "station": lambda p, w: native.station.check(p, w),
    "structures": lambda p, w: native.structure.check(p, w),
    "block_entities": lambda p, w: native.block_entities.check(p, w),
}


def main(argv):
    if len(argv) not in (3, 4):
        print("usage: check_sliced_plan.py PLAN.json INPUT_WORKSPACE guard[,guard...] [DIRECT_RECEIPT.json]", file=sys.stderr)
        return 2
    plan = json.loads(Path(argv[0]).read_text())
    names = [n for n in argv[2].split(",") if n]
    receipt = json.loads(Path(argv[3]).read_text(encoding='utf-8')) if len(argv) == 4 else None
    if receipt is not None and not {'structures', 'block_entities'} <= set(names):
        print('direct attachments require the structures and block_entities guards', file=sys.stderr)
        return 2
    kept_structure_boundary = any(entry.get('mod_id') == 'pluto_lib'
                                  and entry.get('module_id', entry.get('injection_id', '')).startswith('pluto-structure-')
                                  for entry in plan.get('modules', []) + plan.get('injections', []))
    if kept_structure_boundary and 'structures' not in names:
        print('kept structure adapters require the structures guard', file=sys.stderr)
        return 2
    if native.block_entities.kept(plan) and 'block_entities' not in names:
        print('kept custom block-entity factory requires the block_entities guard', file=sys.stderr)
        return 2
    unknown = [n for n in names if n not in GUARDS]
    if unknown or not names:
        print(f"unknown or empty guard list: {unknown or names}", file=sys.stderr)
        return 2
    failed = 0
    for name in names:
        try:
            if receipt is not None and name in ('structures', 'block_entities'):
                guard = native.structure if name == 'structures' else native.block_entities
                guard.check(plan, Path(argv[1]), direct_receipt=receipt)
            else:
                GUARDS[name](plan, Path(argv[1]))
            print(f"guard OK: {name}")
        except Exception as error:  # report every guard, then fail
            failed += 1
            print(f"guard FAILED: {name}: {error}", file=sys.stderr)
    skipped = sorted(set(GUARDS) - set(names))
    print("guards NOT run for removed features: " + (", ".join(skipped) or "none"))
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
