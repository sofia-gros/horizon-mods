#!/usr/bin/env python3
"""Reserve the canonical deny-only crafting-match boundary in a complete plan.

An earlier cancellation could bypass admission; a later rewrite could resurrect
an output. Reuse the established exclusive-boundary validator without changing
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
    return json.loads((ROOT / 'tools/crafting-match-boundaries.json').read_text())['boundaries']


def check_order(plan, metadata):
    try:
        _shared.check_order(plan, metadata)
    except ValueError as error:
        raise ValueError(str(error).replace('placement', 'crafting-match')) from error


def verify_source(plan, workspace, metadata, helpers=None):
    try:
        _shared.verify_source(plan, workspace, metadata, helpers)
    except ValueError as error:
        raise ValueError(str(error).replace('placement', 'crafting-match')) from error


def check(plan, workspace):
    metadata = entries()
    check_order(plan, metadata)
    verify_source(plan, workspace, metadata)


def main(argv=None):
    args = sys.argv[1:] if argv is None else argv
    try:
        if len(args) != 2:
            raise ValueError('usage: check_crafting_match_plan.py COMPLETE_PLAN.json PRISTINE_OFFICIAL_WORKSPACE')
        check(json.loads(Path(args[0]).read_text()), Path(args[1]))
    except (ValueError, OSError, KeyError, TypeError, StopIteration) as error:
        print('Crafting-match composition rejected: ' + str(error), file=sys.stderr)
        return 1
    print('Crafting-match veto ownership, native fingerprint and helper source verified.')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
