#!/usr/bin/env python3
"""Reserve the station-ingredient parser boundary in a complete plan.

An earlier cancellation could bypass parser dispatch; a later rewrite could
replace a conflict/invalid proposal with an accepted ingredient. Reuse the established exclusive-boundary validator without changing
its placement metadata or relaxing either group's composition requirements.
"""
import importlib.util
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
_spec = importlib.util.spec_from_file_location(
    '_pluto_exclusive_placement_boundary', ROOT / 'tools/check_recipe_placement_plan.py')
_shared = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_shared)
digest = _shared.digest


def entries():
    return json.loads((ROOT / 'tools/station-ingredient-boundaries.json').read_text())['boundaries']


def check_order(plan, metadata):
    try:
        _shared.check_order(plan, metadata)
    except ValueError as error:
        raise ValueError(str(error).replace('placement', 'station-ingredient')) from error


def verify_source(plan, workspace, metadata, helpers=None):
    try:
        _shared.verify_source(plan, workspace, metadata, helpers)
    except ValueError as error:
        raise ValueError(str(error).replace('placement', 'station-ingredient')) from error


NATIVE_SOURCES = {
    'crates/horizon-client/src/stations.rs': '2de858164ed879132d89c1eacd2792872d931ff5404ef6bfa378c995af2f85a8',
    'crates/horizon-client/src/crafting.rs': '298e65ea0417a37ccc73c29c23e25b396c369faeb9b17f0fef7f32b097c2243e',
}


def check(plan, workspace):
    if _shared.native_bundle(plan):
        # Keep both the station caller and its Ingredient::parse implementation.
        # This is not the retired custom station-parser registration capability.
        # (The shared validator's reviewed crafting-match exception covers the
        # crafting.rs module/veto only; it does not extend to this boundary.)
        _shared.check_native(plan, workspace, NATIVE_SOURCES, {'ingredient'})
        return
    metadata = entries()
    check_order(plan, metadata)
    verify_source(plan, workspace, metadata)


def main(argv=None):
    args = sys.argv[1:] if argv is None else argv
    try:
        if len(args) != 2:
            raise ValueError('usage: check_station_ingredient_plan.py COMPLETE_PLAN.json PRISTINE_OFFICIAL_WORKSPACE')
        check(json.loads(Path(args[0]).read_text()), Path(args[1]))
    except (ValueError, OSError, KeyError, TypeError, StopIteration) as error:
        print('Station-ingredient composition rejected: ' + str(error), file=sys.stderr)
        return 1
    print('Station-ingredient boundary verified; native preservation supplies no custom parser registration.')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
